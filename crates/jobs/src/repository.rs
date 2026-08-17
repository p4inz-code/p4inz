use std::future::Future;
use std::time::SystemTime;

use p4inz_errors::AppResult;

use crate::job::{Job, JobId, JobStatus};

/// Persistence port for [`Job`]s (`docs/architecture/dependency-rules.md`:
/// the port is defined in the "owning" layer; concrete implementations —
/// e.g. a PostgreSQL-backed one — live in an infrastructure crate that can
/// depend on a database driver, which this crate must not).
///
/// [`claim_next`](Self::claim_next) is the concurrency-safety boundary: an
/// implementation MUST ensure that when multiple workers call it
/// concurrently, at most one of them receives any given due job
/// (`docs/PROJECT_SPEC.md`/this run's job-safety requirements: "prevent
/// duplicate execution where required"). A naive
/// find-then-update-in-two-steps implementation would not satisfy this.
pub trait JobRepository {
    /// Persists a new job.
    fn enqueue(&self, job: &Job) -> impl Future<Output = AppResult<()>> + Send;

    /// Atomically claims and returns the oldest [`JobStatus::Pending`]
    /// job whose `run_at` is at or before `now`, transitioning it to
    /// [`JobStatus::Running`] as part of the same operation. Returns
    /// `Ok(None)` when no job is due.
    ///
    /// [`JobStatus`]: crate::job::JobStatus
    fn claim_next(&self, now: SystemTime) -> impl Future<Output = AppResult<Option<Job>>> + Send;

    /// Marks a claimed job as having completed successfully.
    fn mark_succeeded(
        &self,
        id: JobId,
        now: SystemTime,
    ) -> impl Future<Output = AppResult<()>> + Send;

    /// Marks a claimed job as having failed terminally (its retry budget
    /// is exhausted — a dead letter). `error` is a human-readable
    /// description for operator visibility — never a secret or raw
    /// internal error detail (`docs/PROJECT_SPEC.md` section 13: "Safe
    /// failure behavior").
    fn mark_failed(
        &self,
        id: JobId,
        error: &str,
        now: SystemTime,
    ) -> impl Future<Output = AppResult<()>> + Send;

    /// Records a claimed job's failure and returns it to
    /// [`JobStatus::Pending`] to be retried at `run_at`, incrementing its
    /// attempt count (`docs/development/implementation_plan.md` section
    /// 15: "Bounded retries", "Exponential backoff",
    /// "Failure state tracking"). Callers ([`crate::execute::process_next`])
    /// are responsible for only calling this when the job's retry budget
    /// is not yet exhausted — use [`mark_failed`](Self::mark_failed)
    /// instead once it is.
    ///
    /// [`JobStatus`]: crate::job::JobStatus
    fn reschedule(
        &self,
        id: JobId,
        run_at: SystemTime,
        error: &str,
        now: SystemTime,
    ) -> impl Future<Output = AppResult<()>> + Send;

    /// Looks a job up by id — the "check on a specific job" visibility
    /// primitive (`docs/development/implementation_plan.md` Milestone 36:
    /// Job Observability, "Job status and failure visibility").
    fn find_by_id(&self, id: JobId) -> impl Future<Output = AppResult<Option<Job>>> + Send;

    /// Counts jobs currently in `status` — the building block for
    /// aggregate visibility (e.g. "how many jobs are currently failed").
    /// See [`crate::stats::job_stats`] for a convenience wrapper that
    /// gathers counts for every status at once.
    fn count_by_status(&self, status: JobStatus) -> impl Future<Output = AppResult<u64>> + Send;
}
