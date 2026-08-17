use std::time::SystemTime;

use p4inz_errors::AppResult;
use tracing::Instrument;

use crate::backoff::backoff_delay;
use crate::registry::JobHandlerRegistry;
use crate::repository::JobRepository;

/// Claims and executes at most one due job.
///
/// Returns `Ok(true)` if a job was claimed (regardless of whether it then
/// succeeded, was rescheduled for retry, or failed permanently — all are
/// terminal outcomes for *this* call, recorded via `repository`), or
/// `Ok(false)` if none was due. Callers (the worker poll loop,
/// [`crate::runtime::run_worker`]) use the `bool` to decide whether to
/// immediately check for more work or wait before polling again.
///
/// Retry System (Milestone 34, `docs/development/implementation_plan.md`
/// section 15): a failed job is rescheduled with exponential backoff
/// ([`backoff_delay`]) unless [`Job::retries_exhausted`] — a missing
/// handler is the one exception, treated as a permanent configuration
/// error rather than a transient one, since retrying it can never
/// succeed (the handler will still be missing next time) and would only
/// waste the job's retry budget.
///
/// [`Job::retries_exhausted`]: crate::job::Job::retries_exhausted
pub async fn process_next(
    repository: &impl JobRepository,
    registry: &JobHandlerRegistry,
    now: SystemTime,
) -> AppResult<bool> {
    let Some(job) = repository.claim_next(now).await? else {
        return Ok(false);
    };

    // Every event emitted while handling this job — including whatever
    // the registered `JobHandler` itself logs — is correlated under one
    // span (`docs/development/implementation_plan.md` section 16: "Job
    // tracing", "Correlation/request IDs").
    let span = tracing::info_span!("job", job_id = %job.id(), kind = %job.kind());
    async {
        match registry.get(job.kind().as_str()) {
            Some(handler) => match handler.handle(job.payload()).await {
                Ok(()) => {
                    tracing::info!("job succeeded");
                    repository.mark_succeeded(job.id(), now).await?;
                }
                Err(error) => {
                    let message = error.to_string();
                    if job.retries_exhausted() {
                        tracing::error!(
                            attempts = job.attempts() + 1, max_attempts = job.max_attempts(), %message,
                            "job failed permanently; retry budget exhausted"
                        );
                        repository.mark_failed(job.id(), &message, now).await?;
                    } else {
                        let delay = backoff_delay(job.attempts());
                        let run_at = now + delay;
                        tracing::warn!(
                            attempts = job.attempts() + 1, max_attempts = job.max_attempts(),
                            delay_secs = delay.as_secs(), %message,
                            "job failed; scheduling retry"
                        );
                        repository.reschedule(job.id(), run_at, &message, now).await?;
                    }
                }
            },
            None => {
                let message = format!("no handler registered for job kind '{}'", job.kind());
                tracing::error!("{message}");
                repository.mark_failed(job.id(), &message, now).await?;
            }
        }

        Ok(true)
    }
    .instrument(span)
    .await
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use async_trait::async_trait;
    use p4inz_errors::AppError;

    use super::*;
    use crate::handler::JobHandler;
    use crate::job::{Job, JobId, JobKind, JobStatus};

    #[derive(Default)]
    struct InMemoryRepository {
        jobs: Mutex<Vec<Job>>,
    }

    impl InMemoryRepository {
        fn with(job: Job) -> Self {
            Self { jobs: Mutex::new(vec![job]) }
        }

        fn get(&self, id: JobId) -> Job {
            self.jobs.lock().unwrap().iter().find(|j| j.id() == id).unwrap().clone()
        }

        fn status_of(&self, id: JobId) -> JobStatus {
            self.get(id).status()
        }
    }

    impl JobRepository for InMemoryRepository {
        async fn enqueue(&self, job: &Job) -> AppResult<()> {
            self.jobs.lock().unwrap().push(job.clone());
            Ok(())
        }

        async fn claim_next(&self, now: SystemTime) -> AppResult<Option<Job>> {
            let mut jobs = self.jobs.lock().unwrap();
            let due =
                jobs.iter_mut().find(|j| j.status() == JobStatus::Pending && j.run_at() <= now);
            match due {
                Some(job) => {
                    *job = Job::from_parts(
                        job.id(),
                        job.kind().clone(),
                        job.payload().to_string(),
                        JobStatus::Running,
                        job.run_at(),
                        job.attempts(),
                        job.max_attempts(),
                        job.last_error().map(str::to_string),
                        job.created_at(),
                        now,
                    );
                    Ok(Some(job.clone()))
                }
                None => Ok(None),
            }
        }

        async fn mark_succeeded(&self, id: JobId, now: SystemTime) -> AppResult<()> {
            let mut jobs = self.jobs.lock().unwrap();
            let job = jobs.iter_mut().find(|j| j.id() == id).unwrap();
            *job = Job::from_parts(
                job.id(),
                job.kind().clone(),
                job.payload().to_string(),
                JobStatus::Succeeded,
                job.run_at(),
                job.attempts(),
                job.max_attempts(),
                job.last_error().map(str::to_string),
                job.created_at(),
                now,
            );
            Ok(())
        }

        async fn mark_failed(&self, id: JobId, error: &str, now: SystemTime) -> AppResult<()> {
            let mut jobs = self.jobs.lock().unwrap();
            let job = jobs.iter_mut().find(|j| j.id() == id).unwrap();
            *job = Job::from_parts(
                job.id(),
                job.kind().clone(),
                job.payload().to_string(),
                JobStatus::Failed,
                job.run_at(),
                job.attempts() + 1,
                job.max_attempts(),
                Some(error.to_string()),
                job.created_at(),
                now,
            );
            Ok(())
        }

        async fn reschedule(
            &self,
            id: JobId,
            run_at: SystemTime,
            error: &str,
            now: SystemTime,
        ) -> AppResult<()> {
            let mut jobs = self.jobs.lock().unwrap();
            let job = jobs.iter_mut().find(|j| j.id() == id).unwrap();
            *job = Job::from_parts(
                job.id(),
                job.kind().clone(),
                job.payload().to_string(),
                JobStatus::Pending,
                run_at,
                job.attempts() + 1,
                job.max_attempts(),
                Some(error.to_string()),
                job.created_at(),
                now,
            );
            Ok(())
        }

        async fn find_by_id(&self, id: JobId) -> AppResult<Option<Job>> {
            Ok(self.jobs.lock().unwrap().iter().find(|j| j.id() == id).cloned())
        }

        async fn count_by_status(&self, status: JobStatus) -> AppResult<u64> {
            Ok(self.jobs.lock().unwrap().iter().filter(|j| j.status() == status).count() as u64)
        }
    }

    struct SucceedingHandler;
    #[async_trait]
    impl JobHandler for SucceedingHandler {
        async fn handle(&self, _payload: &str) -> AppResult<()> {
            Ok(())
        }
    }

    struct FailingHandler;
    #[async_trait]
    impl JobHandler for FailingHandler {
        async fn handle(&self, _payload: &str) -> AppResult<()> {
            Err(AppError::internal("handler exploded"))
        }
    }

    fn pending_job(kind: &str) -> Job {
        let now = SystemTime::now();
        Job::new(JobKind::parse(kind).unwrap(), "{}", now, now)
    }

    fn pending_job_with_max_attempts(kind: &str, max_attempts: u32) -> Job {
        let now = SystemTime::now();
        Job::with_max_attempts(JobKind::parse(kind).unwrap(), "{}", now, now, max_attempts)
    }

    #[tokio::test]
    async fn returns_false_when_nothing_is_due() {
        let repository = InMemoryRepository::default();
        let registry = JobHandlerRegistry::new();

        let processed = process_next(&repository, &registry, SystemTime::now()).await.unwrap();
        assert!(!processed);
    }

    #[tokio::test]
    async fn a_due_job_with_a_succeeding_handler_is_marked_succeeded() {
        let job = pending_job("test:ok");
        let id = job.id();
        let repository = InMemoryRepository::with(job);
        let mut registry = JobHandlerRegistry::new();
        registry.insert("test:ok", SucceedingHandler);

        let processed = process_next(&repository, &registry, SystemTime::now()).await.unwrap();

        assert!(processed);
        assert_eq!(repository.status_of(id), JobStatus::Succeeded);
    }

    #[tokio::test]
    async fn a_failing_handler_with_retry_budget_remaining_is_rescheduled_not_failed() {
        let job = pending_job_with_max_attempts("test:boom", 3);
        let id = job.id();
        let repository = InMemoryRepository::with(job);
        let mut registry = JobHandlerRegistry::new();
        registry.insert("test:boom", FailingHandler);

        let now = SystemTime::now();
        let processed = process_next(&repository, &registry, now).await.unwrap();

        assert!(processed);
        let rescheduled = repository.get(id);
        assert_eq!(rescheduled.status(), JobStatus::Pending);
        assert_eq!(rescheduled.attempts(), 1);
        assert!(rescheduled.run_at() > now);
        assert_eq!(rescheduled.last_error(), Some("handler exploded"));
    }

    #[tokio::test]
    async fn a_failing_handler_on_the_final_attempt_is_marked_failed() {
        let job = pending_job_with_max_attempts("test:boom", 1);
        let id = job.id();
        let repository = InMemoryRepository::with(job);
        let mut registry = JobHandlerRegistry::new();
        registry.insert("test:boom", FailingHandler);

        let processed = process_next(&repository, &registry, SystemTime::now()).await.unwrap();

        assert!(processed);
        let failed = repository.get(id);
        assert_eq!(failed.status(), JobStatus::Failed);
        assert_eq!(failed.attempts(), 1);
    }

    #[tokio::test]
    async fn a_job_with_no_registered_handler_is_marked_failed_without_retrying() {
        let job = pending_job_with_max_attempts("test:unknown", 5);
        let id = job.id();
        let repository = InMemoryRepository::with(job);
        let registry = JobHandlerRegistry::new();

        let processed = process_next(&repository, &registry, SystemTime::now()).await.unwrap();

        assert!(processed);
        assert_eq!(repository.status_of(id), JobStatus::Failed);
    }

    #[tokio::test]
    async fn a_not_yet_due_job_is_left_untouched() {
        let future = SystemTime::now() + Duration::from_secs(3600);
        let job = Job::new(JobKind::parse("test:later").unwrap(), "{}", future, SystemTime::now());
        let id = job.id();
        let repository = InMemoryRepository::with(job);
        let registry = JobHandlerRegistry::new();

        let processed = process_next(&repository, &registry, SystemTime::now()).await.unwrap();

        assert!(!processed);
        assert_eq!(repository.status_of(id), JobStatus::Pending);
    }
}
