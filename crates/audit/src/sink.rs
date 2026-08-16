use std::future::Future;

use p4inz_errors::AppResult;

use crate::event::AuditEvent;

/// The recording contract for [`AuditEvent`]s.
///
/// No concrete implementation lives here — writing events to PostgreSQL,
/// structured logs, or anywhere else is an infrastructure concern. Returns
/// `impl Future + Send` rather than using `async fn` in the trait directly,
/// for the same reason as `p4inz_application::ProjectRepository`: `async
/// fn` in traits cannot express a `Send` bound on stable Rust.
pub trait AuditSink {
    fn record(&self, event: &AuditEvent) -> impl Future<Output = AppResult<()>> + Send;
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::event::{AuditAction, AuditActor, AuditOutcome};

    #[derive(Default)]
    struct InMemoryAuditSink {
        events: Mutex<Vec<AuditEvent>>,
    }

    impl AuditSink for InMemoryAuditSink {
        async fn record(&self, event: &AuditEvent) -> AppResult<()> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn records_events_in_order() {
        let sink = InMemoryAuditSink::default();

        let first = AuditEvent::new(
            AuditAction::parse("project.register").unwrap(),
            AuditActor::User("discord:1".to_string()),
            AuditOutcome::Success,
        );
        let second = AuditEvent::new(
            AuditAction::parse("authorize").unwrap(),
            AuditActor::User("discord:1".to_string()),
            AuditOutcome::Failure { reason: "missing permission".to_string() },
        );

        sink.record(&first).await.unwrap();
        sink.record(&second).await.unwrap();

        let recorded = sink.events.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0], first);
        assert_eq!(recorded[1], second);
    }
}
