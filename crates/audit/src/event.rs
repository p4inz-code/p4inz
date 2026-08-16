use std::fmt;
use std::time::SystemTime;

use thiserror::Error;

/// Maximum accepted length for an [`AuditAction`], in characters.
pub const AUDIT_ACTION_MAX_LEN: usize = 100;

/// A short, stable identifier for what happened (e.g. `"project.register"`,
/// `"authorize"`). Left open-ended rather than a closed enum for the same
/// reason as `p4inz_security::Permission`: the specification does not
/// enumerate a fixed vocabulary of auditable actions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuditAction(String);

impl AuditAction {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, AuditActionError> {
        let trimmed = raw.as_ref().trim();

        if trimmed.is_empty() {
            return Err(AuditActionError::Empty);
        }
        if trimmed.chars().count() > AUDIT_ACTION_MAX_LEN {
            return Err(AuditActionError::TooLong { max: AUDIT_ACTION_MAX_LEN });
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AuditActionError {
    #[error("audit action must not be empty")]
    Empty,
    #[error("audit action must be at most {max} characters")]
    TooLong { max: usize },
}

/// Who or what performed the audited action.
///
/// An opaque identifier rather than a full identity type: P4inz has no
/// unified user/identity model yet (Discord and web identities are mapped
/// separately, later — see `docs/development/implementation_plan.md`
/// section 12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditActor {
    /// A human-initiated action, identified by an opaque, source-specific
    /// id (e.g. a Discord user id).
    User(String),
    /// An automated/system-initiated action (e.g. a scheduled sync).
    System,
}

/// Whether the audited action succeeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditOutcome {
    Success,
    Failure { reason: String },
}

impl AuditOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }
}

/// A single audit record.
///
/// `docs/PROJECT_SPEC.md` section 9 requires audit logging as a first-class
/// safety control; `docs/development/implementation_plan.md` section 16
/// requires security/audit events to stay separated from ordinary logs —
/// this type is that dedicated record shape, deliberately independent of
/// how it is eventually stored (see [`crate::AuditSink`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    action: AuditAction,
    actor: AuditActor,
    outcome: AuditOutcome,
    occurred_at: SystemTime,
    target: Option<String>,
}

impl AuditEvent {
    pub fn new(action: AuditAction, actor: AuditActor, outcome: AuditOutcome) -> Self {
        Self { action, actor, outcome, occurred_at: SystemTime::now(), target: None }
    }

    #[must_use]
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    #[must_use]
    pub fn with_occurred_at(mut self, occurred_at: SystemTime) -> Self {
        self.occurred_at = occurred_at;
        self
    }

    pub fn action(&self) -> &AuditAction {
        &self.action
    }

    pub fn actor(&self) -> &AuditActor {
        &self.actor
    }

    pub fn outcome(&self) -> &AuditOutcome {
        &self.outcome
    }

    pub fn occurred_at(&self) -> SystemTime {
        self.occurred_at
    }

    pub fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_action_rejects_empty() {
        assert_eq!(AuditAction::parse(""), Err(AuditActionError::Empty));
    }

    #[test]
    fn audit_action_rejects_too_long() {
        let too_long = "a".repeat(AUDIT_ACTION_MAX_LEN + 1);
        assert_eq!(
            AuditAction::parse(too_long),
            Err(AuditActionError::TooLong { max: AUDIT_ACTION_MAX_LEN })
        );
    }

    #[test]
    fn event_defaults_to_no_target() {
        let event = AuditEvent::new(
            AuditAction::parse("project.register").unwrap(),
            AuditActor::User("discord:123".to_string()),
            AuditOutcome::Success,
        );

        assert!(event.target().is_none());
        assert!(event.outcome().is_success());
    }

    #[test]
    fn with_target_sets_target() {
        let event = AuditEvent::new(
            AuditAction::parse("project.register").unwrap(),
            AuditActor::System,
            AuditOutcome::Failure { reason: "duplicate".to_string() },
        )
        .with_target("project:p4inz");

        assert_eq!(event.target(), Some("project:p4inz"));
        assert!(!event.outcome().is_success());
    }
}
