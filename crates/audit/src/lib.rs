//! P4inz audit subsystem.
//!
//! The security/business audit event record ([`AuditEvent`]) and its
//! recording contract ([`AuditSink`]), plus [`TracingAuditSink`] — a
//! default implementation with no infrastructure dependency. This crate is
//! the dedicated home for audit events so they stay separated from
//! ordinary application logs
//! (`docs/development/implementation_plan.md` section 16).

mod event;
mod sink;
mod tracing_sink;

pub use event::{
    AUDIT_ACTION_MAX_LEN, AuditAction, AuditActionError, AuditActor, AuditEvent, AuditOutcome,
};
pub use sink::AuditSink;
pub use tracing_sink::TracingAuditSink;
