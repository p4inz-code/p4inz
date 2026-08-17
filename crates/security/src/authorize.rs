use p4inz_audit::{AuditAction, AuditActor, AuditEvent, AuditOutcome, AuditSink};
use p4inz_errors::{AppError, AppResult};

use crate::permission::Permission;
use crate::permission_set::PermissionSet;

/// Checks `required` against `granted` and unconditionally records the
/// outcome to `sink` before returning — every authorization decision,
/// success or failure, is audited
/// (`docs/development/implementation_plan.md` section 12: "... ->
/// Application Authorization -> Action -> Audit", both for Discord and web
/// identities).
///
/// Fails closed: any error recording the audit event itself is also
/// propagated rather than silently ignored, and a missing permission
/// always returns `Err`, never a bypassable warning.
pub async fn authorize(
    granted: &PermissionSet,
    required: &Permission,
    actor: AuditActor,
    sink: &impl AuditSink,
) -> AppResult<()> {
    let allowed = granted.contains(required);

    let outcome = if allowed {
        AuditOutcome::Success
    } else {
        AuditOutcome::Failure { reason: format!("missing permission '{required}'") }
    };

    let action = AuditAction::parse("authorize").expect("static action is valid");
    let event = AuditEvent::new(action, actor, outcome).with_target(required.as_str());

    sink.record(&event).await?;

    if allowed {
        Ok(())
    } else {
        Err(AppError::forbidden(format!("missing required permission '{required}'")))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use p4inz_audit::AuditEvent;
    use p4inz_errors::ErrorKind;

    use super::*;
    use crate::role::{Role, RoleName};

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<AuditEvent>>,
    }

    impl AuditSink for RecordingSink {
        async fn record(&self, event: &AuditEvent) -> AppResult<()> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    fn permission(raw: &str) -> Permission {
        Permission::parse(raw).unwrap()
    }

    #[tokio::test]
    async fn allows_and_audits_when_permission_is_granted() {
        let role =
            Role::new(RoleName::parse("moderator").unwrap(), [permission("project:register")]);
        let granted = PermissionSet::from_roles([&role]);
        let sink = RecordingSink::default();

        let result =
            authorize(&granted, &permission("project:register"), AuditActor::System, &sink).await;

        assert!(result.is_ok());
        let events = sink.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].outcome().is_success());
    }

    #[tokio::test]
    async fn denies_and_audits_when_permission_is_missing() {
        let granted = PermissionSet::empty();
        let sink = RecordingSink::default();

        let result =
            authorize(&granted, &permission("project:register"), AuditActor::System, &sink).await;

        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Forbidden);

        let events = sink.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert!(!events[0].outcome().is_success());
    }

    /// A denied audit event that doesn't say *which* permission was
    /// missing is close to useless for a real security investigation —
    /// this locks in that the recorded event actually identifies it, not
    /// just that a denial happened.
    #[tokio::test]
    async fn denied_audit_event_identifies_the_missing_permission() {
        let granted = PermissionSet::empty();
        let sink = RecordingSink::default();

        let _ = authorize(&granted, &permission("admin:manage"), AuditActor::System, &sink).await;

        let events = sink.events.lock().unwrap();
        assert_eq!(events[0].target(), Some("admin:manage"));
        match events[0].outcome() {
            AuditOutcome::Failure { reason } => assert!(reason.contains("admin:manage")),
            AuditOutcome::Success => panic!("expected a Failure outcome"),
        }
    }

    /// The doc comment's central security claim: "Fails closed: any error
    /// recording the audit event itself is also propagated rather than
    /// silently ignored." Without this, a broken/unreachable audit sink
    /// would silently turn into "authorization checks stop happening" —
    /// exactly the failure mode "every authorization decision, success or
    /// failure, is audited" exists to prevent. This is the only test that
    /// actually exercises that claim; every other test here uses a sink
    /// that never fails.
    #[tokio::test]
    async fn a_failing_audit_sink_denies_access_even_when_permission_would_be_granted() {
        struct FailingSink;
        impl AuditSink for FailingSink {
            async fn record(&self, _event: &AuditEvent) -> AppResult<()> {
                Err(AppError::internal("audit backend unreachable"))
            }
        }

        let role =
            Role::new(RoleName::parse("moderator").unwrap(), [permission("project:register")]);
        let granted = PermissionSet::from_roles([&role]);

        let result =
            authorize(&granted, &permission("project:register"), AuditActor::System, &FailingSink)
                .await;

        assert!(
            result.is_err(),
            "a permission that would be granted must still be denied if the decision can't be audited"
        );
    }
}
