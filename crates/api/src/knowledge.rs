use axum::Json;
use axum::extract::{Query, State};
use p4inz_application::{SEARCH_PERMISSION, SearchKnowledge};
use p4inz_audit::{AuditActor, TracingAuditSink};
use p4inz_knowledge::KnowledgeItem;
use p4inz_search::PgKnowledgeRepository;
use p4inz_security::{Permission, PermissionSet, Role, RoleName};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use utoipa::{IntoParams, ToSchema};

use crate::error::ApiError;
use crate::state::ApiState;

/// Default result count when `limit` is omitted.
const DEFAULT_LIMIT: u32 = 10;

/// Hard cap on `limit`, regardless of what the caller requests —
/// "API requests have bounded resource usage"
/// (`docs/development/implementation_plan.md` section 10).
const MAX_LIMIT: u32 = 50;

#[derive(Deserialize, IntoParams)]
pub(crate) struct SearchParams {
    /// The search text.
    q: String,
    /// Maximum results to return (1-50, default 10).
    limit: Option<u32>,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct KnowledgeItemBody {
    id: String,
    category: String,
    title: String,
    body: String,
    source_kind: String,
    source_reference: Option<String>,
    version: u32,
    created_at: String,
    updated_at: String,
    synchronized_at: Option<String>,
}

/// Renders a [`std::time::SystemTime`] as an RFC 3339 string. Falls back
/// to an empty string on the (practically unreachable) case where the
/// timestamp can't be represented — never panics building an API
/// response over a formatting edge case.
fn rfc3339(time: std::time::SystemTime) -> String {
    OffsetDateTime::from(time).format(&Rfc3339).unwrap_or_default()
}

impl From<&KnowledgeItem> for KnowledgeItemBody {
    fn from(item: &KnowledgeItem) -> Self {
        let provenance = item.provenance();
        Self {
            id: item.id().to_string(),
            category: item.category().as_str().to_string(),
            title: item.title().as_str().to_string(),
            body: item.body().as_str().to_string(),
            source_kind: item.source().kind().as_str().to_string(),
            source_reference: item.source().reference().map(|link| link.as_str().to_string()),
            version: provenance.version().as_u32(),
            created_at: rfc3339(provenance.created_at()),
            updated_at: rfc3339(provenance.updated_at()),
            synchronized_at: provenance.synchronized_at().map(rfc3339),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub(crate) struct SearchResponse {
    query: String,
    results: Vec<KnowledgeItemBody>,
}

/// The fixed permission set every (currently anonymous — Authentication is
/// Milestone 40) API caller is granted: search access to published
/// knowledge only, the same authorization/audit pipeline Discord's
/// natural-language questions already go through
/// ([`p4inz_application::SearchKnowledge`]) — "Public endpoints expose
/// only public information" (`docs/development/implementation_plan.md`
/// section 10) is enforced by actually calling `authorize`, not by
/// skipping it.
fn public_permissions() -> PermissionSet {
    let role = Role::new(
        RoleName::parse("public").expect("static role name is valid"),
        [Permission::parse(SEARCH_PERMISSION).expect("static permission is valid")],
    );
    PermissionSet::from_roles([&role])
}

/// Public Knowledge API: search/browse (`docs/development/
/// implementation_plan.md` Milestone 39). Returns only published
/// knowledge — enforced by [`SearchKnowledge`]/`p4inz_search::search_published`,
/// not by anything in this handler.
#[utoipa::path(
    get,
    path = "/knowledge/search",
    tag = "knowledge",
    params(SearchParams),
    responses(
        (status = 200, description = "Matching published knowledge", body = SearchResponse),
        (status = 400, description = "The query was empty or otherwise invalid"),
    ),
)]
pub async fn search(
    State(state): State<ApiState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>, ApiError> {
    let query = params.q.trim();
    if query.is_empty() {
        return Err(ApiError::from(p4inz_errors::AppError::validation(
            "query parameter 'q' must not be empty",
        )));
    }
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    let repository = PgKnowledgeRepository::new(state.pool.clone());
    let sink = TracingAuditSink;

    let results = SearchKnowledge::new(&repository)
        .execute(
            query,
            limit,
            &public_permissions(),
            AuditActor::User("api:public".to_string()),
            &sink,
        )
        .await?;

    Ok(Json(SearchResponse {
        query: query.to_string(),
        results: results.iter().map(KnowledgeItemBody::from).collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_permissions_grants_search() {
        let permission = Permission::parse(SEARCH_PERMISSION).unwrap();
        assert!(public_permissions().contains(&permission));
    }

    #[test]
    fn rfc3339_formats_a_real_timestamp() {
        let formatted = rfc3339(std::time::SystemTime::UNIX_EPOCH);
        assert_eq!(formatted, "1970-01-01T00:00:00Z");
    }
}
