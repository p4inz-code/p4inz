use std::cmp::Ordering;
use std::time::SystemTime;

use p4inz_errors::{AppResult, ErrorKind, IntoAppError};
use p4inz_knowledge::KnowledgeItem;
use sqlx::PgPool;

use crate::rank::combined_score;
use crate::row::{get, row_to_item};

/// How many extra candidates (as a multiple of `limit`) to pull by text
/// relevance before re-ranking by [`combined_score`] — re-ranking can only
/// surface a lower-relevance-but-more-authoritative/fresher item if it was
/// fetched in the first place.
const CANDIDATE_POOL_FACTOR: i64 = 4;

/// Full-text searches published knowledge items, ranked by a combination
/// of text relevance, source authority and freshness (see
/// [`crate::rank::combined_score`];
/// `docs/development/implementation_plan.md` section 8).
///
/// Only [`PublicationState::Published`](p4inz_knowledge::PublicationState::Published)
/// items are returned — draft/review/archived content is not searchable
/// (`docs/PROJECT_SPEC.md` section 6: retrieval must not let unverified
/// content "silently appear authoritative").
pub async fn search_published(
    pool: &PgPool,
    query: &str,
    limit: u32,
) -> AppResult<Vec<KnowledgeItem>> {
    let candidate_limit =
        i64::from(limit).saturating_mul(CANDIDATE_POOL_FACTOR).max(i64::from(limit));

    let rows = sqlx::query(
        r#"
        SELECT *, ts_rank(search_vector, plainto_tsquery('english', $1)) AS relevance
        FROM knowledge_items
        WHERE publication_state = 'published'
          AND search_vector @@ plainto_tsquery('english', $1)
        ORDER BY relevance DESC
        LIMIT $2
        "#,
    )
    .bind(query)
    .bind(candidate_limit)
    .fetch_all(pool)
    .await
    .into_app_error(ErrorKind::Internal, "knowledge search query failed")?;

    let now = SystemTime::now();
    let mut scored = rows
        .iter()
        .map(|row| {
            let relevance: f32 = get(row, "relevance")?;
            let item = row_to_item(row)?;
            let score = combined_score(f64::from(relevance), &item, now);
            Ok((score, item))
        })
        .collect::<Result<Vec<_>, crate::row::RowMappingError>>()
        .into_app_error(ErrorKind::Internal, "a stored knowledge item is invalid")?;

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    scored.truncate(limit as usize);

    Ok(scored.into_iter().map(|(_, item)| item).collect())
}
