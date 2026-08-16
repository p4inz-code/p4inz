use std::future::Future;

use p4inz_audit::{AuditActor, AuditSink};
use p4inz_errors::AppResult;
use p4inz_knowledge::KnowledgeItem;
use p4inz_security::{Permission, PermissionSet, authorize};

/// The permission required to search knowledge. A single, fixed
/// capability rather than per-category permissions: the specification
/// does not define finer-grained retrieval scopes, and inventing them
/// without a concrete requirement would be speculative.
const SEARCH_PERMISSION: &str = "knowledge:search";

/// Searches published knowledge (`docs/PROJECT_SPEC.md` section 6: "the
/// search system must never rely on the AI to decide whether a user may
/// access a result"). No concrete implementation lives here — a
/// PostgreSQL-backed one belongs to `p4inz-search`.
///
/// Publication-state filtering (only published items are ever returned)
/// is enforced by the implementation itself
/// ([`p4inz_search::search_published`]), not by this port — this trait
/// only describes the capability's shape.
pub trait KnowledgeSearch {
    fn search(
        &self,
        query: &str,
        limit: u32,
    ) -> impl Future<Output = AppResult<Vec<KnowledgeItem>>> + Send;
}

/// Searches knowledge, but only after confirming (and auditing) that the
/// caller is authorized to
/// (`docs/development/implementation_plan.md` section 12: "... ->
/// Application Authorization -> Action -> Audit"; section 8: "Authorization
/// occurs before results are returned").
pub struct SearchKnowledge<'a, S: KnowledgeSearch> {
    search: &'a S,
}

impl<'a, S: KnowledgeSearch> SearchKnowledge<'a, S> {
    pub fn new(search: &'a S) -> Self {
        Self { search }
    }

    pub async fn execute(
        &self,
        query: &str,
        limit: u32,
        granted: &PermissionSet,
        actor: AuditActor,
        sink: &impl AuditSink,
    ) -> AppResult<Vec<KnowledgeItem>> {
        let permission = Permission::parse(SEARCH_PERMISSION).expect("static permission is valid");
        authorize(granted, &permission, actor, sink).await?;

        self.search.search(query, limit).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::SystemTime;

    use p4inz_audit::AuditEvent;
    use p4inz_errors::{AppError, ErrorKind};
    use p4inz_knowledge::{Body, KnowledgeCategory, KnowledgeItemId, Source, SourceKind, Title};
    use p4inz_security::{Role, RoleName};

    use super::*;

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

    struct FixedSearch(Vec<KnowledgeItem>);

    impl KnowledgeSearch for FixedSearch {
        async fn search(&self, _query: &str, _limit: u32) -> AppResult<Vec<KnowledgeItem>> {
            Ok(self.0.clone())
        }
    }

    fn sample_item() -> KnowledgeItem {
        KnowledgeItem::new(
            KnowledgeItemId::new(),
            KnowledgeCategory::Community,
            Title::parse("Support").unwrap(),
            Body::parse("Contact us.").unwrap(),
            Source::new(SourceKind::Administrator, None),
            SystemTime::now(),
        )
    }

    #[tokio::test]
    async fn granted_permission_returns_results_and_audits_success() {
        let role = Role::new(
            RoleName::parse("member").unwrap(),
            [Permission::parse(SEARCH_PERMISSION).unwrap()],
        );
        let granted = PermissionSet::from_roles([&role]);
        let search = FixedSearch(vec![sample_item()]);
        let sink = RecordingSink::default();

        let results = SearchKnowledge::new(&search)
            .execute("support", 10, &granted, AuditActor::System, &sink)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(sink.events.lock().unwrap()[0].outcome().is_success());
    }

    #[tokio::test]
    async fn missing_permission_is_denied_and_audited_without_calling_search() {
        let granted = PermissionSet::empty();
        let search = FixedSearch(vec![sample_item()]);
        let sink = RecordingSink::default();

        let err = SearchKnowledge::new(&search)
            .execute("support", 10, &granted, AuditActor::System, &sink)
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Forbidden);
        let events = sink.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert!(!events[0].outcome().is_success());
    }

    #[tokio::test]
    async fn search_failure_propagates_after_authorization_succeeds() {
        struct FailingSearch;
        impl KnowledgeSearch for FailingSearch {
            async fn search(&self, _query: &str, _limit: u32) -> AppResult<Vec<KnowledgeItem>> {
                Err(AppError::unavailable("search index down"))
            }
        }

        let role = Role::new(
            RoleName::parse("member").unwrap(),
            [Permission::parse(SEARCH_PERMISSION).unwrap()],
        );
        let granted = PermissionSet::from_roles([&role]);
        let sink = RecordingSink::default();

        let err = SearchKnowledge::new(&FailingSearch)
            .execute("support", 10, &granted, AuditActor::System, &sink)
            .await
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Unavailable);
    }
}
