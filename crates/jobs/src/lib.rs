//! P4inz jobs subsystem.
//!
//! Currently home to the worker process's lifecycle only
//! ([`run_until_shutdown`]) — job scheduling and execution is
//! `docs/development/implementation_plan.md` Milestone 33 (Job System).

mod runtime;

pub use runtime::{run_until_shutdown, run_until_shutdown_with};
