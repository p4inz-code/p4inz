use std::time::SystemTime;

use p4inz_errors::{AppResult, ErrorKind, IntoAppError};

use crate::category::KnowledgeCategory;
use crate::content::{Body, Title};
use crate::knowledge_item::{KnowledgeItem, KnowledgeItemId};
use crate::repository::KnowledgeRepository;
use crate::source::Source;
use crate::source_adapter::{RawDocument, SourceAdapter};

/// What synchronizing a source resulted in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncOutcome {
    Created(KnowledgeItem),
    Updated(KnowledgeItem),
    /// The fetched content was identical to what's already stored —
    /// nothing was written, and the existing item's version was not
    /// bumped.
    Unchanged(KnowledgeItem),
}

impl SyncOutcome {
    pub fn item(&self) -> &KnowledgeItem {
        match self {
            Self::Created(item) | Self::Updated(item) | Self::Unchanged(item) => item,
        }
    }
}

/// Decides what to do with freshly fetched content relative to an existing
/// item (if any) — pure decision logic, no repository access, so it's
/// cheap to test exhaustively.
///
/// Never destroys anything (`docs/PROJECT_SPEC.md` section 6: "Source
/// changes must not automatically destroy valid existing knowledge",
/// "Historical versions must not be silently destroyed"): a `None`
/// existing item always creates; a present one is only ever updated in
/// place ([`KnowledgeItem::with_content`]) or left as [`SyncOutcome::Unchanged`].
pub fn plan_sync(
    existing: Option<KnowledgeItem>,
    category: KnowledgeCategory,
    fetched: RawDocument,
    source: Source,
    now: SystemTime,
) -> AppResult<SyncOutcome> {
    let title = Title::parse(fetched.title)
        .into_app_error(ErrorKind::Validation, "fetched title is invalid")?;
    let body = Body::parse(fetched.body)
        .into_app_error(ErrorKind::Validation, "fetched body is invalid")?;

    match existing {
        None => {
            let item =
                KnowledgeItem::new(KnowledgeItemId::new(), category, title, body, source, now);
            Ok(SyncOutcome::Created(item))
        }
        Some(item) => {
            if *item.title() == title && *item.body() == body {
                Ok(SyncOutcome::Unchanged(item))
            } else {
                Ok(SyncOutcome::Updated(item.with_content(title, body, now)))
            }
        }
    }
}

/// Fetches `reference` via `adapter`, plans the resulting change against
/// what `repository` already has for `source`, and persists a
/// [`SyncOutcome::Created`] or [`SyncOutcome::Updated`] result (an
/// [`SyncOutcome::Unchanged`] result is not written — nothing changed).
///
/// `reference` is the adapter-specific fetch key (e.g. an `"owner/repo"`
/// string for GitHub) — it is *not* necessarily the same string as
/// `source.reference()`, which is a [`p4inz_domain::Link`] and is what
/// existing items are correlated by. A `source` with no reference is
/// treated as never having an existing counterpart (always creates), since
/// there is nothing to correlate against.
///
/// Scheduling/triggering this on an interval is a separate, later concern
/// (GitHub Jobs, Milestone 35); this only performs one synchronization
/// when called.
pub async fn synchronize_from_source(
    adapter: &impl SourceAdapter,
    repository: &impl KnowledgeRepository,
    reference: &str,
    category: KnowledgeCategory,
    source: Source,
    now: SystemTime,
) -> AppResult<SyncOutcome> {
    let fetched = adapter.fetch(reference).await?;

    let existing = match source.reference() {
        Some(link) => repository.find_by_source_reference(link.as_str()).await?,
        None => None,
    };

    let outcome = plan_sync(existing, category, fetched, source, now)?;

    if !matches!(outcome, SyncOutcome::Unchanged(_)) {
        repository.save(outcome.item()).await?;
    }

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use crate::source::SourceKind;
    use crate::version::Version;

    use super::*;

    fn source() -> Source {
        Source::new(SourceKind::Repository, None)
    }

    fn document(title: &str, body: &str) -> RawDocument {
        RawDocument {
            title: title.to_string(),
            body: body.to_string(),
            fetched_at: SystemTime::now(),
        }
    }

    #[test]
    fn no_existing_item_creates() {
        let outcome = plan_sync(
            None,
            KnowledgeCategory::Projects,
            document("P4inz", "A bot."),
            source(),
            SystemTime::now(),
        )
        .unwrap();

        assert!(matches!(outcome, SyncOutcome::Created(_)));
        assert_eq!(outcome.item().title().as_str(), "P4inz");
    }

    #[test]
    fn changed_content_updates_and_bumps_version() {
        let now = SystemTime::now();
        let existing = KnowledgeItem::new(
            KnowledgeItemId::new(),
            KnowledgeCategory::Projects,
            Title::parse("P4inz").unwrap(),
            Body::parse("Old.").unwrap(),
            source(),
            now,
        );

        let outcome = plan_sync(
            Some(existing.clone()),
            KnowledgeCategory::Projects,
            document("P4inz", "New."),
            source(),
            now,
        )
        .unwrap();

        assert!(matches!(outcome, SyncOutcome::Updated(_)));
        assert_eq!(outcome.item().body().as_str(), "New.");
        assert_eq!(outcome.item().provenance().version(), Version::initial().next());
        assert_eq!(outcome.item().id(), existing.id());
    }

    #[test]
    fn identical_content_is_unchanged_and_does_not_bump_version() {
        let now = SystemTime::now();
        let existing = KnowledgeItem::new(
            KnowledgeItemId::new(),
            KnowledgeCategory::Projects,
            Title::parse("P4inz").unwrap(),
            Body::parse("Same.").unwrap(),
            source(),
            now,
        );

        let outcome = plan_sync(
            Some(existing.clone()),
            KnowledgeCategory::Projects,
            document("P4inz", "Same."),
            source(),
            now,
        )
        .unwrap();

        assert!(matches!(outcome, SyncOutcome::Unchanged(_)));
        assert_eq!(outcome.item().provenance().version(), Version::initial());
    }

    #[test]
    fn invalid_fetched_content_is_rejected() {
        let err = plan_sync(
            None,
            KnowledgeCategory::Projects,
            document("", "body"),
            source(),
            SystemTime::now(),
        )
        .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Validation);
    }

    mod synchronize_from_source_tests {
        use std::sync::Mutex;

        use p4inz_errors::AppError;

        use super::*;

        #[derive(Default)]
        struct InMemoryRepository {
            items: Mutex<Vec<KnowledgeItem>>,
        }

        impl KnowledgeRepository for InMemoryRepository {
            async fn save(&self, item: &KnowledgeItem) -> AppResult<()> {
                let mut items = self.items.lock().unwrap();
                items.retain(|existing| existing.id() != item.id());
                items.push(item.clone());
                Ok(())
            }

            async fn find_by_id(&self, id: KnowledgeItemId) -> AppResult<Option<KnowledgeItem>> {
                Ok(self.items.lock().unwrap().iter().find(|i| i.id() == id).cloned())
            }

            async fn find_by_source_reference(
                &self,
                reference: &str,
            ) -> AppResult<Option<KnowledgeItem>> {
                Ok(self
                    .items
                    .lock()
                    .unwrap()
                    .iter()
                    .find(|i| i.source().reference().map(|link| link.as_str()) == Some(reference))
                    .cloned())
            }
        }

        struct FixedAdapter(RawDocument);

        impl SourceAdapter for FixedAdapter {
            async fn fetch(&self, _reference: &str) -> AppResult<RawDocument> {
                Ok(self.0.clone())
            }
        }

        struct FailingAdapter;

        impl SourceAdapter for FailingAdapter {
            async fn fetch(&self, _reference: &str) -> AppResult<RawDocument> {
                Err(AppError::unavailable("source unreachable"))
            }
        }

        /// A [`Source`] carrying a real reference `Link`, so
        /// `find_by_source_reference` (keyed by that link) can correlate
        /// against it. Distinct from the crate-level `source()` helper,
        /// which has no reference and is only used by tests that don't
        /// exercise correlation.
        fn linked_source() -> Source {
            Source::new(
                SourceKind::Repository,
                Some(p4inz_domain::Link::parse("https://github.com/p4inz-code/p4inz").unwrap()),
            )
        }

        #[tokio::test]
        async fn first_sync_creates_and_persists() {
            let adapter = FixedAdapter(document("P4inz", "A bot."));
            let repository = InMemoryRepository::default();

            let outcome = synchronize_from_source(
                &adapter,
                &repository,
                "p4inz-code/p4inz",
                KnowledgeCategory::Projects,
                linked_source(),
                SystemTime::now(),
            )
            .await
            .unwrap();

            assert!(matches!(outcome, SyncOutcome::Created(_)));
            assert_eq!(repository.items.lock().unwrap().len(), 1);
        }

        #[tokio::test]
        async fn unchanged_content_is_not_written_again() {
            let adapter = FixedAdapter(document("P4inz", "A bot."));
            let repository = InMemoryRepository::default();
            let now = SystemTime::now();

            synchronize_from_source(
                &adapter,
                &repository,
                "p4inz-code/p4inz",
                KnowledgeCategory::Projects,
                linked_source(),
                now,
            )
            .await
            .unwrap();
            let outcome = synchronize_from_source(
                &adapter,
                &repository,
                "p4inz-code/p4inz",
                KnowledgeCategory::Projects,
                linked_source(),
                now,
            )
            .await
            .unwrap();

            assert!(matches!(outcome, SyncOutcome::Unchanged(_)));
            assert_eq!(repository.items.lock().unwrap().len(), 1);
        }

        #[tokio::test]
        async fn second_sync_with_changed_content_updates_in_place() {
            let repository = InMemoryRepository::default();
            let now = SystemTime::now();

            synchronize_from_source(
                &FixedAdapter(document("P4inz", "Old body.")),
                &repository,
                "p4inz-code/p4inz",
                KnowledgeCategory::Projects,
                linked_source(),
                now,
            )
            .await
            .unwrap();

            let outcome = synchronize_from_source(
                &FixedAdapter(document("P4inz", "New body.")),
                &repository,
                "p4inz-code/p4inz",
                KnowledgeCategory::Projects,
                linked_source(),
                now,
            )
            .await
            .unwrap();

            assert!(matches!(outcome, SyncOutcome::Updated(_)));
            assert_eq!(outcome.item().body().as_str(), "New body.");
            assert_eq!(repository.items.lock().unwrap().len(), 1);
        }

        #[tokio::test]
        async fn adapter_failure_propagates_without_touching_the_repository() {
            let repository = InMemoryRepository::default();

            let err = synchronize_from_source(
                &FailingAdapter,
                &repository,
                "ref",
                KnowledgeCategory::Projects,
                source(),
                SystemTime::now(),
            )
            .await
            .unwrap_err();

            assert_eq!(err.kind(), ErrorKind::Unavailable);
            assert!(repository.items.lock().unwrap().is_empty());
        }
    }
}
