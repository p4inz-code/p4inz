//! P4inz observability subsystem (`docs/development/implementation_plan.md`
//! Milestone 51: Observability — "Logs/metrics/health";
//! `docs/architecture/overview.md`: "observability — logs, metrics,
//! tracing, health").
//!
//! - [`logging::init`] installs the process-wide structured (JSON)
//!   `tracing` subscriber every binary calls once at startup.
//! - [`metrics::Metrics`] is a minimal, dependency-free in-process counter
//!   registry, rendered as Prometheus's plain-text exposition format.
//! - [`request_id::generate`] mints correlation ids for tagging one
//!   request/job/interaction across every log line it produces.
//!
//! Health and readiness checks live in `p4inz-api` (`/health`, `/ready`)
//! rather than here, since answering them requires reaching the database —
//! this crate stays free of infrastructure dependencies, the same as
//! `p4inz-errors`/`p4inz-common`.

pub mod logging;
pub mod metrics;
pub mod request_id;
pub mod shutdown;
