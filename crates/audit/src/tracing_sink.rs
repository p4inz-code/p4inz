use p4inz_errors::AppResult;

use crate::event::{AuditActor, AuditEvent, AuditOutcome};
use crate::sink::AuditSink;

/// Records audit events as structured `tracing` events at the `audit`
/// target, separated from ordinary application logs by that target name
/// (`docs/development/implementation_plan.md` section 16: "Security/audit
/// events separated from ordinary logs").
///
/// A reasonable default with no infrastructure dependency (no database
/// required) — a durable, queryable sink (e.g. writing to PostgreSQL) can
/// implement [`AuditSink`] later if operators need one; this crate stays
/// usable without it.
#[derive(Debug, Clone, Copy, Default)]
pub struct TracingAuditSink;

impl AuditSink for TracingAuditSink {
    async fn record(&self, event: &AuditEvent) -> AppResult<()> {
        let actor = match event.actor() {
            AuditActor::User(id) => id.as_str(),
            AuditActor::System => "system",
        };

        let resource = event.target().unwrap_or("-");

        match event.outcome() {
            AuditOutcome::Success => {
                tracing::info!(target: "audit", action = %event.action(), %actor, %resource, "audit: success");
            }
            AuditOutcome::Failure { reason } => {
                tracing::warn!(target: "audit", action = %event.action(), %actor, %resource, %reason, "audit: failure");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::event::AuditAction;

    use super::*;

    #[tokio::test]
    async fn records_without_error() {
        let event = AuditEvent::new(
            AuditAction::parse("test.action").unwrap(),
            AuditActor::System,
            AuditOutcome::Success,
        );
        assert!(TracingAuditSink.record(&event).await.is_ok());
    }
}
