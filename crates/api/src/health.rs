use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::ApiState;

#[derive(Serialize, ToSchema)]
pub(crate) struct HealthBody {
    status: &'static str,
}

/// Liveness probe (`docs/development/implementation_plan.md` section 16:
/// "Health checks") — always `200` once the process can serve requests at
/// all. Deliberately checks nothing else: a liveness probe answering
/// "is the process alive" should not depend on downstream services being
/// up, or a struggling database would also take the process itself out of
/// rotation.
#[utoipa::path(
    get,
    path = "/health",
    tag = "operations",
    responses((status = 200, description = "The process is alive", body = HealthBody)),
)]
pub async fn health() -> Json<HealthBody> {
    Json(HealthBody { status: "ok" })
}

/// Readiness probe (`docs/development/implementation_plan.md` section 16:
/// "Readiness checks") — `200` only once dependencies (currently:
/// PostgreSQL) are actually reachable, `503` otherwise. Distinct from
/// [`health`]: a deployment orchestrator uses this to decide whether to
/// route traffic here, not whether to restart the process.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "operations",
    responses(
        (status = 200, description = "Dependencies are reachable", body = HealthBody),
        (status = 503, description = "A dependency is unreachable", body = HealthBody),
    ),
)]
pub async fn ready(State(state): State<ApiState>) -> (StatusCode, Json<HealthBody>) {
    match p4inz_database::health_check(&state.pool).await {
        Ok(()) => (StatusCode::OK, Json(HealthBody { status: "ok" })),
        Err(error) => {
            tracing::error!(%error, "readiness check failed: database unreachable");
            (StatusCode::SERVICE_UNAVAILABLE, Json(HealthBody { status: "unavailable" }))
        }
    }
}
