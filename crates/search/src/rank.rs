use std::time::{Duration, SystemTime};

use p4inz_knowledge::{KnowledgeItem, SourceKind};

/// Relative weight given to text relevance in [`combined_score`].
pub const RELEVANCE_WEIGHT: f64 = 0.6;
/// Relative weight given to source authority in [`combined_score`].
pub const AUTHORITY_WEIGHT: f64 = 0.25;
/// Relative weight given to freshness in [`combined_score`].
pub const FRESHNESS_WEIGHT: f64 = 0.15;

/// How long it takes a fully-fresh item's [`freshness_score`] to halve.
pub const DEFAULT_FRESHNESS_HALF_LIFE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// A source-authority score in `[0.0, 1.0]`, derived from the priority
/// order `docs/PROJECT_SPEC.md` section 5 ("Source of Truth") lists its
/// five preferred authoritative sources in — not an invented ranking, a
/// direct reading of that numbered list.
pub fn authority_score(kind: SourceKind) -> f64 {
    match kind {
        SourceKind::Repository => 1.0,
        SourceKind::OfficialDocumentation => 0.8,
        SourceKind::Administrator => 0.6,
        SourceKind::Announcement => 0.4,
        SourceKind::Other => 0.2,
    }
}

/// A freshness score in `(0.0, 1.0]`: `1.0` for content updated exactly at
/// `now`, decaying by half every `half_life` before that
/// (`docs/PROJECT_SPEC.md` section 6: "Freshness").
///
/// `now` earlier than `updated_at` (a clock going backwards) is treated as
/// zero age — still maximally fresh, not an error.
pub fn freshness_score(updated_at: SystemTime, now: SystemTime, half_life: Duration) -> f64 {
    let age = now.duration_since(updated_at).unwrap_or(Duration::ZERO);
    let half_lives = age.as_secs_f64() / half_life.as_secs_f64().max(1.0);
    0.5f64.powf(half_lives)
}

/// Combines text relevance, source authority and freshness into one
/// ranking score (`docs/development/implementation_plan.md` section 8:
/// "Search ranking may consider: Text relevance, Source authority,
/// Freshness, Knowledge state" — "Knowledge state" is already a hard
/// filter in [`crate::search_published`]'s `WHERE publication_state =
/// 'published'`, not a further ranking dimension among already-published
/// results).
///
/// The weights above are a starting heuristic, not a value the
/// specification prescribes — it names these factors without specifying
/// how to combine them. They're deliberately named constants so they can
/// be tuned later against real usage without touching this function's
/// logic.
pub fn combined_score(relevance: f64, item: &KnowledgeItem, now: SystemTime) -> f64 {
    let authority = authority_score(item.source().kind());
    let freshness =
        freshness_score(item.provenance().updated_at(), now, DEFAULT_FRESHNESS_HALF_LIFE);

    RELEVANCE_WEIGHT * relevance + AUTHORITY_WEIGHT * authority + FRESHNESS_WEIGHT * freshness
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_score_orders_by_spec_priority() {
        assert!(
            authority_score(SourceKind::Repository)
                > authority_score(SourceKind::OfficialDocumentation)
        );
        assert!(
            authority_score(SourceKind::OfficialDocumentation)
                > authority_score(SourceKind::Administrator)
        );
        assert!(
            authority_score(SourceKind::Administrator) > authority_score(SourceKind::Announcement)
        );
        assert!(authority_score(SourceKind::Announcement) > authority_score(SourceKind::Other));
    }

    #[test]
    fn authority_score_is_bounded() {
        for kind in [
            SourceKind::Repository,
            SourceKind::OfficialDocumentation,
            SourceKind::Administrator,
            SourceKind::Announcement,
            SourceKind::Other,
        ] {
            let score = authority_score(kind);
            assert!((0.0..=1.0).contains(&score));
        }
    }

    #[test]
    fn freshness_score_is_one_at_zero_age() {
        let now = SystemTime::now();
        assert_eq!(freshness_score(now, now, DEFAULT_FRESHNESS_HALF_LIFE), 1.0);
    }

    #[test]
    fn freshness_score_halves_at_the_half_life() {
        let now = SystemTime::now();
        let updated_at = now - DEFAULT_FRESHNESS_HALF_LIFE;
        let score = freshness_score(updated_at, now, DEFAULT_FRESHNESS_HALF_LIFE);
        assert!((score - 0.5).abs() < 0.001);
    }

    #[test]
    fn freshness_score_decreases_monotonically_with_age() {
        let now = SystemTime::now();
        let recent =
            freshness_score(now - Duration::from_secs(60), now, DEFAULT_FRESHNESS_HALF_LIFE);
        let older =
            freshness_score(now - Duration::from_secs(3600), now, DEFAULT_FRESHNESS_HALF_LIFE);
        assert!(recent > older);
    }

    #[test]
    fn freshness_score_treats_future_updated_at_as_zero_age() {
        let now = SystemTime::now();
        let future = now + Duration::from_secs(3600);
        assert_eq!(freshness_score(future, now, DEFAULT_FRESHNESS_HALF_LIFE), 1.0);
    }

    #[test]
    fn combined_score_weights_sum_to_one() {
        assert!((RELEVANCE_WEIGHT + AUTHORITY_WEIGHT + FRESHNESS_WEIGHT - 1.0).abs() < 1e-9);
    }

    #[test]
    fn combined_score_prefers_higher_relevance_all_else_equal() {
        use p4inz_knowledge::{
            Body, KnowledgeCategory, KnowledgeItemId, Source, SourceKind, Title,
        };

        let now = SystemTime::now();
        let item = KnowledgeItem::new(
            KnowledgeItemId::new(),
            KnowledgeCategory::Community,
            Title::parse("Support").unwrap(),
            Body::parse("Contact us.").unwrap(),
            Source::new(SourceKind::Administrator, None),
            now,
        );

        assert!(combined_score(0.9, &item, now) > combined_score(0.1, &item, now));
    }
}
