use std::time::SystemTime;

use p4inz_domain::Id;
use thiserror::Error;

/// Identifies a [`Job`].
pub type JobId = Id<Job>;

/// The maximum length of a [`JobKind`]. Generous enough for a descriptive
/// dotted name (e.g. `"knowledge:github_sync"`) without being unbounded.
pub const JOB_KIND_MAX_LEN: usize = 100;

/// What kind of work a [`Job`] represents (e.g. `"knowledge:github_sync"`),
/// used to look the right [`crate::JobHandler`] up in a
/// [`crate::JobHandlerRegistry`] at execution time.
///
/// Deliberately a validated string rather than a closed enum: the job
/// system itself must not know about specific job types (GitHub
/// synchronization, or any job kind added later) — that would couple this
/// crate to infrastructure/knowledge concerns it has no business depending
/// on (`docs/development/implementation_plan.md` section 23: the structure
/// must support "Additional jobs" without rewriting the core).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JobKind(String);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum JobKindError {
    #[error("job kind must not be empty")]
    Empty,
    #[error("job kind must be at most {JOB_KIND_MAX_LEN} characters")]
    TooLong,
}

impl JobKind {
    pub fn parse(raw: impl Into<String>) -> Result<Self, JobKindError> {
        let trimmed = raw.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(JobKindError::Empty);
        }
        if trimmed.chars().count() > JOB_KIND_MAX_LEN {
            return Err(JobKindError::TooLong);
        }
        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for JobKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A [`Job`]'s lifecycle state.
///
/// `Failed` is currently terminal — bounded retries/backoff before a job
/// is allowed to reach `Failed` are Milestone 34's concern (Retry System),
/// not this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("'{0}' is not a valid job status")]
pub struct JobStatusError(String);

impl std::str::FromStr for JobStatus {
    type Err = JobStatusError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            other => Err(JobStatusError(other.to_string())),
        }
    }
}

/// The default cap on how many times a job is attempted before it is
/// treated as permanently failed (dead-lettered), for callers that don't
/// need a different value. `docs/development/implementation_plan.md`
/// section 15 requires "Bounded retries" without prescribing a number;
/// five is a conventional, moderate default (enough to ride out a brief
/// outage, not so many it turns a truly broken job into a long-running
/// retry storm).
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;

/// A unit of background work (`docs/development/implementation_plan.md`
/// section 15: "Persistent/reliable jobs where required").
///
/// `payload` is an opaque, job-kind-specific string (typically JSON) —
/// this crate has no reason to understand its shape, only to carry it from
/// enqueue time to the [`crate::JobHandler`] registered for `kind`.
///
/// `attempts` counts failures recorded so far (`docs/development/
/// implementation_plan.md` section 15: "Bounded retries", "Failure state
/// tracking"); Milestone 34 (Retry System) uses it together with
/// `max_attempts` to decide whether a failure should be retried
/// ([`crate::execute::process_next`]) or is terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    id: JobId,
    kind: JobKind,
    payload: String,
    status: JobStatus,
    run_at: SystemTime,
    attempts: u32,
    max_attempts: u32,
    last_error: Option<String>,
    created_at: SystemTime,
    updated_at: SystemTime,
}

impl Job {
    /// Creates a new, pending job scheduled to run at `run_at` (pass `now`
    /// for "as soon as a worker is free"), retried up to
    /// [`DEFAULT_MAX_ATTEMPTS`] times on failure.
    pub fn new(
        kind: JobKind,
        payload: impl Into<String>,
        run_at: SystemTime,
        now: SystemTime,
    ) -> Self {
        Self::with_max_attempts(kind, payload, run_at, now, DEFAULT_MAX_ATTEMPTS)
    }

    /// Like [`new`](Self::new), with an explicit retry cap instead of
    /// [`DEFAULT_MAX_ATTEMPTS`].
    pub fn with_max_attempts(
        kind: JobKind,
        payload: impl Into<String>,
        run_at: SystemTime,
        now: SystemTime,
        max_attempts: u32,
    ) -> Self {
        Self {
            id: JobId::new(),
            kind,
            payload: payload.into(),
            status: JobStatus::Pending,
            run_at,
            attempts: 0,
            max_attempts,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Reconstructs a [`Job`] from already-valid stored values.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: JobId,
        kind: JobKind,
        payload: String,
        status: JobStatus,
        run_at: SystemTime,
        attempts: u32,
        max_attempts: u32,
        last_error: Option<String>,
        created_at: SystemTime,
        updated_at: SystemTime,
    ) -> Self {
        Self {
            id,
            kind,
            payload,
            status,
            run_at,
            attempts,
            max_attempts,
            last_error,
            created_at,
            updated_at,
        }
    }

    pub fn id(&self) -> JobId {
        self.id
    }

    pub fn kind(&self) -> &JobKind {
        &self.kind
    }

    pub fn payload(&self) -> &str {
        &self.payload
    }

    pub fn status(&self) -> JobStatus {
        self.status
    }

    pub fn run_at(&self) -> SystemTime {
        self.run_at
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }

    pub fn updated_at(&self) -> SystemTime {
        self.updated_at
    }

    /// Whether a failure right now would exhaust the retry budget — i.e.
    /// whether the *next* failure is terminal rather than retryable.
    pub fn retries_exhausted(&self) -> bool {
        self.attempts + 1 >= self.max_attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_rejects_empty() {
        assert_eq!(JobKind::parse("").unwrap_err(), JobKindError::Empty);
        assert_eq!(JobKind::parse("   ").unwrap_err(), JobKindError::Empty);
    }

    #[test]
    fn kind_rejects_too_long() {
        let raw = "x".repeat(JOB_KIND_MAX_LEN + 1);
        assert_eq!(JobKind::parse(raw).unwrap_err(), JobKindError::TooLong);
    }

    #[test]
    fn kind_trims_surrounding_whitespace() {
        let kind = JobKind::parse("  knowledge:github_sync  ").unwrap();
        assert_eq!(kind.as_str(), "knowledge:github_sync");
    }

    #[test]
    fn status_round_trips_through_str() {
        for status in
            [JobStatus::Pending, JobStatus::Running, JobStatus::Succeeded, JobStatus::Failed]
        {
            assert_eq!(status.as_str().parse::<JobStatus>().unwrap(), status);
        }
    }

    #[test]
    fn status_rejects_unknown_string() {
        assert!("bogus".parse::<JobStatus>().is_err());
    }

    #[test]
    fn new_jobs_start_pending() {
        let now = SystemTime::now();
        let job = Job::new(JobKind::parse("test").unwrap(), "{}", now, now);
        assert_eq!(job.status(), JobStatus::Pending);
        assert_eq!(job.created_at(), now);
        assert_eq!(job.run_at(), now);
    }

    #[test]
    fn distinct_jobs_have_distinct_ids() {
        let now = SystemTime::now();
        let a = Job::new(JobKind::parse("test").unwrap(), "{}", now, now);
        let b = Job::new(JobKind::parse("test").unwrap(), "{}", now, now);
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn new_jobs_start_with_zero_attempts_and_the_default_cap() {
        let now = SystemTime::now();
        let job = Job::new(JobKind::parse("test").unwrap(), "{}", now, now);
        assert_eq!(job.attempts(), 0);
        assert_eq!(job.max_attempts(), DEFAULT_MAX_ATTEMPTS);
        assert_eq!(job.last_error(), None);
    }

    #[test]
    fn with_max_attempts_overrides_the_default_cap() {
        let now = SystemTime::now();
        let job = Job::with_max_attempts(JobKind::parse("test").unwrap(), "{}", now, now, 1);
        assert_eq!(job.max_attempts(), 1);
    }

    #[test]
    fn retries_exhausted_is_false_while_budget_remains() {
        let now = SystemTime::now();
        let job = Job::with_max_attempts(JobKind::parse("test").unwrap(), "{}", now, now, 3);
        assert!(!job.retries_exhausted());
    }

    #[test]
    fn retries_exhausted_is_true_on_the_final_attempt() {
        let now = SystemTime::now();
        let job = Job::from_parts(
            JobId::new(),
            JobKind::parse("test").unwrap(),
            "{}".to_string(),
            JobStatus::Running,
            now,
            2,
            3,
            None,
            now,
            now,
        );
        assert!(job.retries_exhausted());
    }
}
