use std::time::SystemTime;

use axum::Json;
use axum::extract::State;
use axum_extra::extract::cookie::CookieJar;
use p4inz_audit::{AuditActor, TracingAuditSink};
use p4inz_errors::{AppError, ErrorKind, IntoAppError};
use p4inz_infrastructure::jobs::{PgJobRepository, trigger_github_sync};
use p4inz_knowledge::KnowledgeCategory;
use p4inz_security::{Permission, PermissionSet, Role, RoleName, authorize};
use serde::{Deserialize, Serialize};

use crate::auth::{require_auth, require_session};
use crate::error::ApiError;
use crate::state::ApiState;

/// Generous upper bound on a `"owner/repo"` reference's length — GitHub
/// itself caps usernames at 39 characters and repository names at 100,
/// so any real reference is far under this; it exists only to reject
/// pathologically large request bodies before they reach the job system
/// (`docs/development/implementation_plan.md` section 10: "API requests
/// have bounded resource usage").
const MAX_REFERENCE_LEN: usize = 200;

/// Validates that `reference` has the `"owner/repo"` shape GitHub Jobs
/// expects (`docs/development/implementation_plan.md` Milestone 42: API
/// Security, "Validation-first") — checked here, at the API boundary,
/// rather than only surfacing as a GitHub API failure once the job
/// actually runs.
fn validate_reference(reference: &str) -> Result<(), AppError> {
    if reference.is_empty() || reference.chars().count() > MAX_REFERENCE_LEN {
        return Err(AppError::validation(format!(
            "reference must be 1-{MAX_REFERENCE_LEN} characters"
        )));
    }
    if reference.chars().any(char::is_whitespace) {
        return Err(AppError::validation("reference must not contain whitespace"));
    }
    let Some((owner, repo)) = reference.split_once('/') else {
        return Err(AppError::validation("reference must be in \"owner/repo\" form"));
    };
    if owner.is_empty() || repo.is_empty() || repo.contains('/') {
        return Err(AppError::validation("reference must be in \"owner/repo\" form"));
    }
    Ok(())
}

/// The permission required to manually trigger a GitHub synchronization —
/// the same job kind GitHub Jobs' scheduler (Milestone 35) enqueues, now
/// reachable as an authenticated, authorized admin action (`docs/
/// development/implementation_plan.md` Milestone 41: API Authorization;
/// section 15: "Manual synchronization trigger").
const TRIGGER_SYNC_PERMISSION: &str = "jobs:trigger_sync";

/// The [`PermissionSet`] an authenticated administrator is granted. A
/// single fixed role, the same pattern `p4inz_application::knowledge_search`
/// uses for the public search permission — finer-grained admin roles
/// aren't specified anywhere yet, and inventing them without a concrete
/// requirement would be speculative.
fn admin_permissions() -> PermissionSet {
    let role = Role::new(
        RoleName::parse("admin").expect("static role name is valid"),
        [Permission::parse(TRIGGER_SYNC_PERMISSION).expect("static permission is valid")],
    );
    PermissionSet::from_roles([&role])
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub(crate) struct TriggerSyncBody {
    /// A GitHub `"owner/repo"` reference.
    reference: String,
    /// A [`KnowledgeCategory::as_str`] value (e.g. `"projects"`).
    category: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct TriggerSyncResponse {
    enqueued: bool,
}

/// Manually triggers a GitHub synchronization job
/// (`docs/development/implementation_plan.md` Milestone 41: API
/// Authorization). Demonstrates the full "Web Authentication -> Identity
/// -> P4inz Permission -> Application Authorization -> Action -> Audit"
/// pipeline (section 12) end-to-end: a valid session identifies the
/// caller, [`admin_permissions`] resolves what an administrator may do
/// (see [`p4inz_config::AuthConfig::admin_user_ids`] for why membership
/// is an explicit allowlist), [`authorize`] enforces and audits the
/// decision, and only then is [`trigger_github_sync`] — Milestone 35's
/// job-enqueue action — actually called.
#[utoipa::path(
    post,
    path = "/admin/knowledge/sync",
    tag = "admin",
    request_body = TriggerSyncBody,
    responses(
        (status = 200, description = "Sync job enqueued", body = TriggerSyncResponse),
        (status = 400, description = "Invalid reference or category"),
        (status = 401, description = "No valid session"),
        (status = 403, description = "Authenticated but not an administrator"),
        (status = 503, description = "Authentication is not configured"),
    ),
)]
pub async fn trigger_sync(
    State(state): State<ApiState>,
    jar: CookieJar,
    Json(body): Json<TriggerSyncBody>,
) -> Result<Json<TriggerSyncResponse>, ApiError> {
    let auth = require_auth(&state)?;
    let claims = require_session(auth, &jar)?;

    let granted = if auth.admin_user_ids.contains(&claims.sub) {
        admin_permissions()
    } else {
        PermissionSet::empty()
    };
    let permission =
        Permission::parse(TRIGGER_SYNC_PERMISSION).expect("static permission is valid");
    let actor = AuditActor::User(format!("discord:{}", claims.sub));
    authorize(&granted, &permission, actor, &TracingAuditSink).await?;

    validate_reference(&body.reference)?;
    let category = KnowledgeCategory::parse(&body.category)
        .into_app_error(ErrorKind::Validation, "invalid knowledge category")?;

    let job_repository = PgJobRepository::new(state.pool.clone());
    trigger_github_sync(&job_repository, &body.reference, category, SystemTime::now()).await?;

    Ok(Json(TriggerSyncResponse { enqueued: true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_well_formed_reference() {
        assert!(validate_reference("p4inz-code/p4inz").is_ok());
    }

    #[test]
    fn rejects_an_empty_reference() {
        assert!(validate_reference("").is_err());
    }

    #[test]
    fn rejects_a_reference_without_a_slash() {
        assert!(validate_reference("p4inz-code").is_err());
    }

    #[test]
    fn rejects_a_reference_with_an_empty_owner_or_repo() {
        assert!(validate_reference("/p4inz").is_err());
        assert!(validate_reference("p4inz-code/").is_err());
    }

    #[test]
    fn rejects_a_reference_with_more_than_one_slash() {
        assert!(validate_reference("p4inz-code/p4inz/extra").is_err());
    }

    #[test]
    fn rejects_a_reference_containing_whitespace() {
        assert!(validate_reference("p4inz code/p4inz").is_err());
    }

    #[test]
    fn rejects_an_overly_long_reference() {
        let owner = "a".repeat(MAX_REFERENCE_LEN);
        assert!(validate_reference(&format!("{owner}/repo")).is_err());
    }

    #[test]
    fn admin_permissions_grants_trigger_sync() {
        let permission = Permission::parse(TRIGGER_SYNC_PERMISSION).unwrap();
        assert!(admin_permissions().contains(&permission));
    }
}
