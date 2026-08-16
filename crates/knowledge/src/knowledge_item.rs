use std::time::SystemTime;

use p4inz_domain::Id;

use crate::category::KnowledgeCategory;
use crate::content::{Body, Title};
use crate::provenance::Provenance;
use crate::publication_state::PublicationState;
use crate::source::Source;
use crate::workflow::{WorkflowError, is_allowed_transition};

/// Identifies a [`KnowledgeItem`].
pub type KnowledgeItemId = Id<KnowledgeItem>;

/// A single piece of authoritative knowledge (`docs/PROJECT_SPEC.md`
/// section 6, ADR-004: "Knowledge Is the Source of Truth").
///
/// Represents unstructured content (title + body) — e.g. an FAQ entry, a
/// policy notice, an announcement — as distinct from `p4inz_domain::Project`,
/// which models Projects' *structured* fields (name, status, repository
/// link, etc.). A `KnowledgeItem` in the `Projects` category describes a
/// project in prose; it does not replace `Project`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeItem {
    id: KnowledgeItemId,
    category: KnowledgeCategory,
    title: Title,
    body: Body,
    source: Source,
    state: PublicationState,
    provenance: Provenance,
}

impl KnowledgeItem {
    /// Creates a new knowledge item as of `now`. Always starts as
    /// [`PublicationState::Draft`] with version 1 — the public
    /// constructor cannot produce a published item directly, enforcing
    /// "AI cannot directly publish authoritative knowledge" and
    /// "Controlled updates" (`docs/PROJECT_SPEC.md` section 6/7) at the
    /// type level. Advancing the state is Knowledge Workflow's concern
    /// (Milestone 18).
    pub fn new(
        id: KnowledgeItemId,
        category: KnowledgeCategory,
        title: Title,
        body: Body,
        source: Source,
        now: SystemTime,
    ) -> Self {
        Self {
            id,
            category,
            title,
            body,
            source,
            state: PublicationState::Draft,
            provenance: Provenance::new(now),
        }
    }

    /// Reconstructs a [`KnowledgeItem`] from already-valid stored values,
    /// including its actual [`PublicationState`] and [`Provenance`].
    ///
    /// For loading a previously persisted item back (Knowledge Search,
    /// Milestone 21). Unlike [`new`](Self::new), this can reconstruct an
    /// item in any state: the invariant [`new`](Self::new) enforces
    /// (every item starts as Draft) only constrains how items are
    /// *created*, not how already-legitimate stored data is read back —
    /// the state was validated by [`transition_to`](Self::transition_to)
    /// when it was originally written.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: KnowledgeItemId,
        category: KnowledgeCategory,
        title: Title,
        body: Body,
        source: Source,
        state: PublicationState,
        provenance: Provenance,
    ) -> Self {
        Self { id, category, title, body, source, state, provenance }
    }

    pub fn id(&self) -> KnowledgeItemId {
        self.id
    }

    pub fn category(&self) -> KnowledgeCategory {
        self.category
    }

    pub fn title(&self) -> &Title {
        &self.title
    }

    pub fn body(&self) -> &Body {
        &self.body
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn state(&self) -> PublicationState {
        self.state
    }

    pub fn provenance(&self) -> Provenance {
        self.provenance
    }

    /// Moves this item to `to`, if that is a legal transition
    /// (`docs/development/implementation_plan.md` section 6). Consumes
    /// `self` and returns the transitioned item, so a stale pre-transition
    /// instance can't accidentally keep being used.
    ///
    /// A successful transition also touches [`Provenance`] (bumps the
    /// version and `updated_at`): a state change is itself a meaningful,
    /// auditable event in the item's history.
    pub fn transition_to(
        mut self,
        to: PublicationState,
        now: SystemTime,
    ) -> Result<Self, WorkflowError> {
        if !is_allowed_transition(self.state, to) {
            return Err(WorkflowError { from: self.state, to });
        }
        self.state = to;
        self.provenance = self.provenance.touched(now);
        Ok(self)
    }

    /// Replaces this item's title/body with freshly synchronized content
    /// and marks [`Provenance::synchronized_at`], leaving
    /// [`PublicationState`] unchanged.
    ///
    /// Source-driven content updates apply in place rather than forcing a
    /// republished item back through Draft/Review: `docs/PROJECT_SPEC.md`
    /// section 5 describes GitHub as a source for "frequently changing
    /// project information", which only works if updates propagate
    /// automatically. This still never destroys history — every call
    /// bumps the version (`docs/PROJECT_SPEC.md` section 6: "Historical
    /// versions must not be silently destroyed").
    #[must_use]
    pub fn with_content(mut self, title: Title, body: Body, now: SystemTime) -> Self {
        self.title = title;
        self.body = body;
        self.provenance = self.provenance.synchronized(now);
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::source::SourceKind;
    use crate::version::Version;

    use super::*;

    fn valid_item_at(now: SystemTime) -> KnowledgeItem {
        KnowledgeItem::new(
            KnowledgeItemId::new(),
            KnowledgeCategory::Community,
            Title::parse("Support").unwrap(),
            Body::parse("Contact us in #support.").unwrap(),
            Source::new(SourceKind::Administrator, None),
            now,
        )
    }

    fn valid_item() -> KnowledgeItem {
        valid_item_at(SystemTime::now())
    }

    #[test]
    fn new_items_start_as_draft() {
        assert_eq!(valid_item().state(), PublicationState::Draft);
    }

    #[test]
    fn new_items_start_at_version_one() {
        assert_eq!(valid_item().provenance().version(), Version::initial());
    }

    #[test]
    fn distinct_items_have_distinct_ids() {
        let a = valid_item();
        let b = valid_item();
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn accessors_return_constructed_values() {
        let item = valid_item();
        assert_eq!(item.category(), KnowledgeCategory::Community);
        assert_eq!(item.title().as_str(), "Support");
        assert_eq!(item.source().kind(), SourceKind::Administrator);
    }

    #[test]
    fn provenance_created_at_matches_construction_time() {
        let now = SystemTime::now();
        let item = valid_item_at(now);
        assert_eq!(item.provenance().created_at(), now);
    }

    #[test]
    fn walking_the_happy_path_succeeds_and_bumps_version() {
        let created = SystemTime::now();
        let item = valid_item_at(created)
            .transition_to(PublicationState::Review, created)
            .unwrap()
            .transition_to(PublicationState::Published, created)
            .unwrap()
            .transition_to(PublicationState::Archived, created)
            .unwrap();

        assert_eq!(item.state(), PublicationState::Archived);
        assert_eq!(item.provenance().version(), Version::initial().next().next().next());
    }

    #[test]
    fn rejecting_back_to_draft_succeeds() {
        let now = SystemTime::now();
        let item = valid_item_at(now).transition_to(PublicationState::Review, now).unwrap();
        let item = item.transition_to(PublicationState::Draft, now).unwrap();
        assert_eq!(item.state(), PublicationState::Draft);
    }

    #[test]
    fn skipping_review_is_rejected() {
        let now = SystemTime::now();
        let err = valid_item_at(now).transition_to(PublicationState::Published, now).unwrap_err();
        assert_eq!(
            err,
            WorkflowError { from: PublicationState::Draft, to: PublicationState::Published }
        );
    }

    #[test]
    fn archived_items_cannot_transition_further() {
        let now = SystemTime::now();
        let archived = valid_item_at(now)
            .transition_to(PublicationState::Review, now)
            .unwrap()
            .transition_to(PublicationState::Published, now)
            .unwrap()
            .transition_to(PublicationState::Archived, now)
            .unwrap();

        assert!(archived.clone().transition_to(PublicationState::Draft, now).is_err());
        assert!(archived.transition_to(PublicationState::Published, now).is_err());
    }

    #[test]
    fn with_content_replaces_title_and_body_and_marks_synchronized() {
        let created = SystemTime::now();
        let synced_at = created + std::time::Duration::from_secs(60);

        let item = valid_item_at(created).with_content(
            Title::parse("New Title").unwrap(),
            Body::parse("New body.").unwrap(),
            synced_at,
        );

        assert_eq!(item.title().as_str(), "New Title");
        assert_eq!(item.body().as_str(), "New body.");
        assert_eq!(item.provenance().synchronized_at(), Some(synced_at));
        assert_eq!(item.provenance().version(), Version::initial().next());
    }

    #[test]
    fn with_content_preserves_publication_state() {
        let now = SystemTime::now();
        let published = valid_item_at(now)
            .transition_to(PublicationState::Review, now)
            .unwrap()
            .transition_to(PublicationState::Published, now)
            .unwrap();

        let updated = published.with_content(
            Title::parse("New Title").unwrap(),
            Body::parse("New body.").unwrap(),
            now,
        );

        assert_eq!(updated.state(), PublicationState::Published);
    }
}
