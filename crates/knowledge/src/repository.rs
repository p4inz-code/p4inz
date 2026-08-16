use std::future::Future;

use p4inz_errors::AppResult;

use crate::knowledge_item::{KnowledgeItem, KnowledgeItemId};

/// The persistence contract for [`KnowledgeItem`], owned by this crate the
/// same way `p4inz_application::ProjectRepository` is owned by
/// `application` — so infrastructure implements it rather than the other
/// way around (`docs/architecture/dependency-rules.md`).
///
/// No concrete implementation lives here; a PostgreSQL-backed one belongs
/// to `p4inz-database`/`p4inz-infrastructure` (Knowledge Search, Milestone
/// 21).
pub trait KnowledgeRepository {
    fn save(&self, item: &KnowledgeItem) -> impl Future<Output = AppResult<()>> + Send;

    fn find_by_id(
        &self,
        id: KnowledgeItemId,
    ) -> impl Future<Output = AppResult<Option<KnowledgeItem>>> + Send;

    /// Finds the item previously synchronized from `source_reference`, if
    /// any. Synchronization (Milestone 20) uses this to decide whether a
    /// freshly fetched document creates a new item or updates an existing
    /// one.
    fn find_by_source_reference(
        &self,
        source_reference: &str,
    ) -> impl Future<Output = AppResult<Option<KnowledgeItem>>> + Send;
}
