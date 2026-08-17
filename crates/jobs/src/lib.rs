//! P4inz jobs subsystem.
//!
//! The worker process's lifecycle ([`run_until_shutdown`]) and the
//! reliable job system (`docs/development/implementation_plan.md`
//! Milestone 33: [`Job`], [`JobRepository`], [`JobHandler`],
//! [`JobHandlerRegistry`], [`process_next`], [`run_worker`]). This crate
//! must stay independent of Discord, Axum and SQLx/PostgreSQL, the same
//! as `p4inz-domain`/`p4inz-application`/`p4inz-knowledge`
//! (`docs/architecture/dependency-rules.md`) — concrete persistence
//! (a PostgreSQL-backed [`JobRepository`]) and concrete job handlers (e.g.
//! GitHub synchronization, Milestone 35) live in `p4inz-infrastructure`.
//!
//! Bounded retries and exponential backoff ([`backoff_delay`]) on top of
//! this are Milestone 34 (Retry System) — a failed job is retried until
//! its retry budget is exhausted, then dead-lettered as
//! `JobStatus::Failed`.
//!
//! Job Observability (Milestone 36): [`JobRepository::find_by_id`] and
//! [`JobRepository::count_by_status`]/[`job_stats`] make job status and
//! failures queryable, and [`process_next`] emits a correlated tracing
//! span per job (`docs/development/implementation_plan.md` section 16:
//! "Job tracing").

mod backoff;
mod execute;
mod handler;
mod job;
mod registry;
mod repository;
mod runtime;
mod stats;

pub use backoff::backoff_delay;
pub use execute::process_next;
pub use handler::JobHandler;
pub use job::{
    DEFAULT_MAX_ATTEMPTS, JOB_KIND_MAX_LEN, Job, JobId, JobKind, JobKindError, JobStatus,
    JobStatusError,
};
pub use registry::JobHandlerRegistry;
pub use repository::JobRepository;
pub use runtime::{run_until_shutdown, run_until_shutdown_with, run_worker, run_worker_with};
pub use stats::{JobStats, job_stats};
