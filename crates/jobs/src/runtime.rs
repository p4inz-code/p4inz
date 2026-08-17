use std::future::Future;
use std::io;
use std::time::{Duration, SystemTime};

use crate::execute::process_next;
use crate::registry::JobHandlerRegistry;
use crate::repository::JobRepository;

/// Runs the worker process until `shutdown` resolves, then returns.
///
/// This is the worker's process lifecycle only — start, run, graceful
/// stop. No jobs are scheduled or executed here
/// (`docs/development/implementation_plan.md` Milestone 33: Job System;
/// `docs/architecture/runtime.md`: the worker process is responsible for
/// synchronization/indexing/embeddings/scheduled tasks/retries/
/// reconciliation, none of which exist yet). This exists so the worker
/// binary has a real, graceful start/stop shape for later milestones to
/// plug job scheduling into, rather than inventing it themselves.
///
/// `shutdown` is injected (rather than this function calling
/// [`tokio::signal::ctrl_c`] directly) so the lifecycle is testable
/// without needing to send the process a real signal — see
/// [`run_until_shutdown`] for the production entry point, which supplies
/// `tokio::signal::ctrl_c()`.
pub async fn run_until_shutdown_with<F>(shutdown: F) -> io::Result<()>
where
    F: Future<Output = io::Result<()>>,
{
    tracing::info!("worker started");
    shutdown.await?;
    tracing::info!("worker shutting down");
    Ok(())
}

/// Runs the worker process until an external shutdown signal (Ctrl+C /
/// SIGINT) is received, then returns.
pub async fn run_until_shutdown() -> io::Result<()> {
    run_until_shutdown_with(tokio::signal::ctrl_c()).await
}

/// Runs the worker's job-processing loop until `shutdown` resolves.
///
/// Polls for due work via [`process_next`] at most once every
/// `poll_interval` while idle, draining every currently-due job
/// back-to-back before going idle again. `shutdown` is only ever raced
/// against the *idle wait* — never against an in-flight
/// claim-then-execute — so a job that has already been claimed (and is
/// therefore `Running` in storage) always finishes before the worker
/// stops. Racing shutdown directly against [`process_next`] would let
/// `tokio::select!` drop that future mid-flight on a badly timed signal,
/// stranding the job `Running` forever with no recovery mechanism yet
/// (`docs/development/implementation_plan.md` section 15: "Safe
/// recovery"; this run's job-safety requirements: "ensure cancellation
/// does not corrupt state").
///
/// A backlog is drained fully before shutdown is honored, rather than
/// stopping after one job — acceptable for the job volumes this system
/// handles (e.g. GitHub Jobs, Milestone 35, syncing a handful of
/// repositories), and simpler/safer than adding a second non-blocking
/// shutdown check between backlog items.
pub async fn run_worker_with<R, F>(
    repository: &R,
    registry: &JobHandlerRegistry,
    poll_interval: Duration,
    shutdown: F,
) -> io::Result<()>
where
    R: JobRepository + Sync,
    F: Future<Output = io::Result<()>>,
{
    tracing::info!("worker started");
    tokio::pin!(shutdown);

    loop {
        let processed = match process_next(repository, registry, SystemTime::now()).await {
            Ok(processed) => processed,
            Err(error) => {
                tracing::error!(%error, "job polling failed; will retry after the poll interval");
                false
            }
        };

        if processed {
            continue;
        }

        tokio::select! {
            biased;
            result = &mut shutdown => {
                result?;
                break;
            }
            () = tokio::time::sleep(poll_interval) => {}
        }
    }

    tracing::info!("worker shutting down");
    Ok(())
}

/// Runs the worker's job-processing loop until an external shutdown
/// signal (Ctrl+C / SIGINT) is received.
pub async fn run_worker<R: JobRepository + Sync>(
    repository: &R,
    registry: &JobHandlerRegistry,
    poll_interval: Duration,
) -> io::Result<()> {
    run_worker_with(repository, registry, poll_interval, tokio::signal::ctrl_c()).await
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use p4inz_errors::AppResult;

    use super::*;
    use crate::handler::JobHandler;
    use crate::job::{Job, JobId, JobKind, JobStatus};

    #[tokio::test]
    async fn returns_once_the_shutdown_future_resolves() {
        let result = run_until_shutdown_with(async { Ok(()) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn propagates_a_failing_shutdown_signal() {
        let result =
            run_until_shutdown_with(async { Err(io::Error::other("signal handler failed")) }).await;
        assert!(result.is_err());
    }

    #[derive(Default)]
    struct InMemoryRepository {
        jobs: Mutex<Vec<Job>>,
    }

    impl InMemoryRepository {
        fn with(job: Job) -> Self {
            Self { jobs: Mutex::new(vec![job]) }
        }

        fn status_of(&self, id: JobId) -> JobStatus {
            self.jobs.lock().unwrap().iter().find(|j| j.id() == id).unwrap().status()
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

    #[tokio::test]
    async fn stops_once_shutdown_resolves_when_idle() {
        let repository = InMemoryRepository::default();
        let registry = JobHandlerRegistry::new();

        let result =
            run_worker_with(&repository, &registry, Duration::from_millis(1), async { Ok(()) })
                .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn drains_a_due_job_before_checking_shutdown() {
        let now = SystemTime::now();
        let job = Job::new(JobKind::parse("test:ok").unwrap(), "{}", now, now);
        let id = job.id();
        let repository = InMemoryRepository::with(job);
        let mut registry = JobHandlerRegistry::new();
        registry.insert("test:ok", SucceedingHandler);

        let result =
            run_worker_with(&repository, &registry, Duration::from_millis(1), async { Ok(()) })
                .await;

        assert!(result.is_ok());
        assert_eq!(repository.status_of(id), JobStatus::Succeeded);
    }

    #[tokio::test]
    async fn propagates_a_failing_shutdown_signal_while_idle() {
        let repository = InMemoryRepository::default();
        let registry = JobHandlerRegistry::new();

        let result = run_worker_with(&repository, &registry, Duration::from_millis(1), async {
            Err(io::Error::other("signal handler failed"))
        })
        .await;

        assert!(result.is_err());
    }
}
