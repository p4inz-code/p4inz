use p4inz_errors::AppResult;

use crate::job::JobStatus;
use crate::repository::JobRepository;

/// A snapshot count of jobs by status — the aggregate "job status and
/// failure visibility" requirement (`docs/development/
/// implementation_plan.md` Milestone 36: Job Observability). Consumed by
/// health/metrics reporting (Milestone 51: Observability) once that
/// surface exists; this milestone only establishes the data itself is
/// queryable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JobStats {
    pub pending: u64,
    pub running: u64,
    pub succeeded: u64,
    pub failed: u64,
}

/// Gathers a [`JobStats`] snapshot from `repository`.
pub async fn job_stats(repository: &impl JobRepository) -> AppResult<JobStats> {
    Ok(JobStats {
        pending: repository.count_by_status(JobStatus::Pending).await?,
        running: repository.count_by_status(JobStatus::Running).await?,
        succeeded: repository.count_by_status(JobStatus::Succeeded).await?,
        failed: repository.count_by_status(JobStatus::Failed).await?,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::SystemTime;

    use super::*;
    use crate::job::{Job, JobId, JobKind};

    #[derive(Default)]
    struct InMemoryRepository {
        jobs: Mutex<Vec<Job>>,
    }

    impl JobRepository for InMemoryRepository {
        async fn enqueue(&self, job: &Job) -> AppResult<()> {
            self.jobs.lock().unwrap().push(job.clone());
            Ok(())
        }

        async fn claim_next(&self, _now: SystemTime) -> AppResult<Option<Job>> {
            unimplemented!("not exercised by these tests")
        }

        async fn mark_succeeded(&self, _id: JobId, _now: SystemTime) -> AppResult<()> {
            unimplemented!("not exercised by these tests")
        }

        async fn mark_failed(&self, _id: JobId, _error: &str, _now: SystemTime) -> AppResult<()> {
            unimplemented!("not exercised by these tests")
        }

        async fn reschedule(
            &self,
            _id: JobId,
            _run_at: SystemTime,
            _error: &str,
            _now: SystemTime,
        ) -> AppResult<()> {
            unimplemented!("not exercised by these tests")
        }

        async fn find_by_id(&self, id: JobId) -> AppResult<Option<Job>> {
            Ok(self.jobs.lock().unwrap().iter().find(|j| j.id() == id).cloned())
        }

        async fn count_by_status(&self, status: JobStatus) -> AppResult<u64> {
            Ok(self.jobs.lock().unwrap().iter().filter(|j| j.status() == status).count() as u64)
        }
    }

    fn job_with_status(kind: &str, status: JobStatus) -> Job {
        let now = SystemTime::now();
        Job::from_parts(
            JobId::new(),
            JobKind::parse(kind).unwrap(),
            "{}".to_string(),
            status,
            now,
            0,
            5,
            None,
            now,
            now,
        )
    }

    #[tokio::test]
    async fn counts_jobs_per_status() {
        let repository = InMemoryRepository::default();
        repository.enqueue(&job_with_status("a", JobStatus::Pending)).await.unwrap();
        repository.enqueue(&job_with_status("b", JobStatus::Pending)).await.unwrap();
        repository.enqueue(&job_with_status("c", JobStatus::Running)).await.unwrap();
        repository.enqueue(&job_with_status("d", JobStatus::Failed)).await.unwrap();

        let stats = job_stats(&repository).await.unwrap();

        assert_eq!(stats, JobStats { pending: 2, running: 1, succeeded: 0, failed: 1 });
    }

    #[tokio::test]
    async fn empty_repository_reports_all_zero() {
        let repository = InMemoryRepository::default();

        let stats = job_stats(&repository).await.unwrap();

        assert_eq!(stats, JobStats::default());
    }
}
