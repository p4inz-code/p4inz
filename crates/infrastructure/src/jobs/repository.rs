use std::time::SystemTime;

use p4inz_errors::{AppResult, ErrorKind, IntoAppError};
use p4inz_jobs::{Job, JobId, JobKind, JobRepository, JobStatus};
use sqlx::PgPool;
use sqlx::Row;
use sqlx::postgres::PgRow;
use time::OffsetDateTime;

/// A PostgreSQL-backed [`JobRepository`] (`docs/architecture/
/// dependency-rules.md`: infrastructure implements contracts required by
/// application/domain/knowledge/jobs).
///
/// [`claim_next`](Self::claim_next) uses `SELECT ... FOR UPDATE SKIP
/// LOCKED` inside a single atomic `UPDATE ... WHERE id = (subquery)`
/// statement so that concurrent workers polling at the same time can never
/// both claim the same job — the database itself is the mutual-exclusion
/// mechanism, not applcation-level coordination
/// (`docs/development/implementation_plan.md` section 15: "Concurrency
/// limits"; this run's job-safety requirements: "prevent duplicate
/// execution").
pub struct PgJobRepository {
    pool: PgPool,
}

impl PgJobRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl JobRepository for PgJobRepository {
    async fn enqueue(&self, job: &Job) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO jobs (
                id, kind, payload, status, run_at,
                attempts, max_attempts, last_error, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(job.id().into_uuid())
        .bind(job.kind().as_str())
        .bind(job.payload())
        .bind(job.status().as_str())
        .bind(OffsetDateTime::from(job.run_at()))
        .bind(job.attempts() as i32)
        .bind(job.max_attempts() as i32)
        .bind(job.last_error())
        .bind(OffsetDateTime::from(job.created_at()))
        .bind(OffsetDateTime::from(job.updated_at()))
        .execute(&self.pool)
        .await
        .into_app_error(ErrorKind::Internal, "failed to enqueue job")?;

        Ok(())
    }

    async fn claim_next(&self, now: SystemTime) -> AppResult<Option<Job>> {
        let now = OffsetDateTime::from(now);

        let row = sqlx::query(
            r#"
            UPDATE jobs
            SET status = 'running', updated_at = $1
            WHERE id = (
                SELECT id FROM jobs
                WHERE status = 'pending' AND run_at <= $1
                ORDER BY run_at
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING *
            "#,
        )
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .into_app_error(ErrorKind::Internal, "failed to claim next job")?;

        row.as_ref()
            .map(row_to_job)
            .transpose()
            .into_app_error(ErrorKind::Internal, "stored job is invalid")
    }

    async fn mark_succeeded(&self, id: JobId, now: SystemTime) -> AppResult<()> {
        sqlx::query("UPDATE jobs SET status = 'succeeded', updated_at = $1 WHERE id = $2")
            .bind(OffsetDateTime::from(now))
            .bind(id.into_uuid())
            .execute(&self.pool)
            .await
            .into_app_error(ErrorKind::Internal, "failed to mark job succeeded")?;

        Ok(())
    }

    async fn mark_failed(&self, id: JobId, error: &str, now: SystemTime) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET status = 'failed', attempts = attempts + 1, last_error = $1, updated_at = $2
            WHERE id = $3
            "#,
        )
        .bind(error)
        .bind(OffsetDateTime::from(now))
        .bind(id.into_uuid())
        .execute(&self.pool)
        .await
        .into_app_error(ErrorKind::Internal, "failed to mark job failed")?;

        Ok(())
    }

    async fn reschedule(
        &self,
        id: JobId,
        run_at: SystemTime,
        error: &str,
        now: SystemTime,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET status = 'pending', run_at = $1, attempts = attempts + 1,
                last_error = $2, updated_at = $3
            WHERE id = $4
            "#,
        )
        .bind(OffsetDateTime::from(run_at))
        .bind(error)
        .bind(OffsetDateTime::from(now))
        .bind(id.into_uuid())
        .execute(&self.pool)
        .await
        .into_app_error(ErrorKind::Internal, "failed to reschedule job")?;

        Ok(())
    }

    async fn find_by_id(&self, id: JobId) -> AppResult<Option<Job>> {
        let row = sqlx::query("SELECT * FROM jobs WHERE id = $1")
            .bind(id.into_uuid())
            .fetch_optional(&self.pool)
            .await
            .into_app_error(ErrorKind::Internal, "failed to look up job by id")?;

        row.as_ref()
            .map(row_to_job)
            .transpose()
            .into_app_error(ErrorKind::Internal, "stored job is invalid")
    }

    async fn count_by_status(&self, status: JobStatus) -> AppResult<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE status = $1")
            .bind(status.as_str())
            .fetch_one(&self.pool)
            .await
            .into_app_error(ErrorKind::Internal, "failed to count jobs by status")?;

        Ok(count as u64)
    }
}

fn row_to_job(row: &PgRow) -> Result<Job, sqlx::Error> {
    let id: uuid::Uuid = row.try_get("id")?;
    let kind: String = row.try_get("kind")?;
    let payload: String = row.try_get("payload")?;
    let status: String = row.try_get("status")?;
    let run_at: OffsetDateTime = row.try_get("run_at")?;
    let attempts: i32 = row.try_get("attempts")?;
    let max_attempts: i32 = row.try_get("max_attempts")?;
    let last_error: Option<String> = row.try_get("last_error")?;
    let created_at: OffsetDateTime = row.try_get("created_at")?;
    let updated_at: OffsetDateTime = row.try_get("updated_at")?;

    let kind = JobKind::parse(kind)
        .map_err(|error| sqlx::Error::Decode(Box::new(std::io::Error::other(error.to_string()))))?;
    let status = status
        .parse::<JobStatus>()
        .map_err(|error| sqlx::Error::Decode(Box::new(std::io::Error::other(error.to_string()))))?;

    Ok(Job::from_parts(
        JobId::from_uuid(id),
        kind,
        payload,
        status,
        SystemTime::from(run_at),
        attempts as u32,
        max_attempts as u32,
        last_error,
        SystemTime::from(created_at),
        SystemTime::from(updated_at),
    ))
}

#[cfg(test)]
mod tests {
    use p4inz_database::{PoolSettings, connect};

    use super::*;

    /// Requires a real, reachable PostgreSQL instance at `DATABASE_URL`
    /// with migrations applied. Not run by default — this environment has
    /// no PostgreSQL server available. Run explicitly with `cargo test -p
    /// p4inz-infrastructure -- --ignored` against a real database.
    #[tokio::test]
    #[ignore = "requires a live PostgreSQL instance; see doc comment"]
    async fn claim_next_atomically_claims_a_due_job_and_hides_it_from_other_claimants() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
        let config = p4inz_config::DatabaseConfig { url: p4inz_common::Secret::new(url) };
        let pool = connect(&config, PoolSettings::default()).await.unwrap();
        let repository = PgJobRepository::new(pool);

        let now = SystemTime::now();
        let job = Job::new(JobKind::parse("test:noop").unwrap(), "{}", now, now);
        repository.enqueue(&job).await.unwrap();

        let claimed = repository.claim_next(now).await.unwrap().unwrap();
        assert_eq!(claimed.id(), job.id());
        assert_eq!(claimed.status(), JobStatus::Running);

        let second_claim = repository.claim_next(now).await.unwrap();
        assert!(second_claim.is_none());

        repository.mark_succeeded(job.id(), now).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires a live PostgreSQL instance; see doc comment"]
    async fn a_future_run_at_is_not_claimed_early() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
        let config = p4inz_config::DatabaseConfig { url: p4inz_common::Secret::new(url) };
        let pool = connect(&config, PoolSettings::default()).await.unwrap();
        let repository = PgJobRepository::new(pool);

        let now = SystemTime::now();
        let future = now + std::time::Duration::from_secs(3600);
        let job = Job::new(JobKind::parse("test:later").unwrap(), "{}", future, now);
        repository.enqueue(&job).await.unwrap();

        let claimed = repository.claim_next(now).await.unwrap();
        assert!(claimed.is_none());
    }

    #[tokio::test]
    #[ignore = "requires a live PostgreSQL instance; see doc comment"]
    async fn reschedule_returns_a_job_to_pending_with_a_bumped_attempt_count() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
        let config = p4inz_config::DatabaseConfig { url: p4inz_common::Secret::new(url) };
        let pool = connect(&config, PoolSettings::default()).await.unwrap();
        let repository = PgJobRepository::new(pool);

        let now = SystemTime::now();
        let job = Job::new(JobKind::parse("test:retry").unwrap(), "{}", now, now);
        repository.enqueue(&job).await.unwrap();
        repository.claim_next(now).await.unwrap().unwrap();

        let retry_at = now + std::time::Duration::from_secs(30);
        repository.reschedule(job.id(), retry_at, "boom", now).await.unwrap();

        let claimed = repository.claim_next(retry_at).await.unwrap().unwrap();
        assert_eq!(claimed.attempts(), 1);
        assert_eq!(claimed.last_error(), Some("boom"));
    }

    #[tokio::test]
    #[ignore = "requires a live PostgreSQL instance; see doc comment"]
    async fn find_by_id_round_trips_an_enqueued_job() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
        let config = p4inz_config::DatabaseConfig { url: p4inz_common::Secret::new(url) };
        let pool = connect(&config, PoolSettings::default()).await.unwrap();
        let repository = PgJobRepository::new(pool);

        let now = SystemTime::now();
        let job = Job::new(JobKind::parse("test:lookup").unwrap(), "{}", now, now);
        repository.enqueue(&job).await.unwrap();

        let found = repository.find_by_id(job.id()).await.unwrap().unwrap();
        assert_eq!(found.id(), job.id());
        assert_eq!(found.status(), JobStatus::Pending);
    }

    #[tokio::test]
    #[ignore = "requires a live PostgreSQL instance; see doc comment"]
    async fn find_by_id_returns_none_for_an_unknown_id() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
        let config = p4inz_config::DatabaseConfig { url: p4inz_common::Secret::new(url) };
        let pool = connect(&config, PoolSettings::default()).await.unwrap();
        let repository = PgJobRepository::new(pool);

        let found = repository.find_by_id(JobId::new()).await.unwrap();
        assert!(found.is_none());
    }

    /// The one property this repository exists for (its own doc comment:
    /// "concurrent workers polling at the same time can never both claim
    /// the same job") and the one no other test here actually exercises —
    /// every other `claim_next` test calls it sequentially on a single
    /// connection, which would pass even with a naive `SELECT` + `UPDATE`
    /// (no `FOR UPDATE SKIP LOCKED` needed) since the first call already
    /// flips the row to `running` before the second runs. This test
    /// issues five real, concurrent claims — over five separate pool
    /// connections — against exactly five pending jobs, and checks the
    /// database's own locking actually prevented a double-claim, not just
    /// that sequential calls behave.
    #[tokio::test]
    #[ignore = "requires a live PostgreSQL instance; see doc comment"]
    async fn concurrent_claims_never_claim_the_same_job_twice() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
        let config = p4inz_config::DatabaseConfig { url: p4inz_common::Secret::new(url) };
        let settings = PoolSettings { max_connections: 5, ..PoolSettings::default() };
        let pool = connect(&config, settings).await.unwrap();
        let repository = PgJobRepository::new(pool);

        let now = SystemTime::now();
        let mut expected_ids = Vec::new();
        for i in 0..5 {
            let job =
                Job::new(JobKind::parse(format!("test:concurrent-{i}")).unwrap(), "{}", now, now);
            expected_ids.push(job.id().to_string());
            repository.enqueue(&job).await.unwrap();
        }

        let (c0, c1, c2, c3, c4) = tokio::join!(
            repository.claim_next(now),
            repository.claim_next(now),
            repository.claim_next(now),
            repository.claim_next(now),
            repository.claim_next(now),
        );

        let mut claimed_ids: Vec<String> = [c0, c1, c2, c3, c4]
            .into_iter()
            .map(|result| {
                result
                    .unwrap()
                    .expect(
                        "every one of the 5 concurrent claims should have found a job — \
                         exactly 5 were enqueued",
                    )
                    .id()
                    .to_string()
            })
            .collect();
        claimed_ids.sort();
        expected_ids.sort();

        assert_eq!(
            claimed_ids, expected_ids,
            "every enqueued job should have been claimed exactly once, by exactly one caller"
        );
    }

    #[tokio::test]
    #[ignore = "requires a live PostgreSQL instance; see doc comment"]
    async fn count_by_status_reflects_enqueued_jobs() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
        let config = p4inz_config::DatabaseConfig { url: p4inz_common::Secret::new(url) };
        let pool = connect(&config, PoolSettings::default()).await.unwrap();
        let repository = PgJobRepository::new(pool);

        let before = repository.count_by_status(JobStatus::Pending).await.unwrap();

        let now = SystemTime::now();
        let job = Job::new(JobKind::parse("test:count").unwrap(), "{}", now, now);
        repository.enqueue(&job).await.unwrap();

        let after = repository.count_by_status(JobStatus::Pending).await.unwrap();
        assert_eq!(after, before + 1);
    }
}
