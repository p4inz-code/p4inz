use std::time::Duration;

use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use p4inz_infrastructure::jobs::PgJobRepository;
use p4inz_jobs::job_stats;
use p4inz_observability::metrics::Metrics;

use crate::state::ApiState;

/// Bounds the job-stats query below — a slow or unreachable database must
/// never make `/metrics` itself hang; a scrape target has to stay
/// responsive precisely when something is already wrong.
const JOB_STATS_TIMEOUT: Duration = Duration::from_secs(2);

/// Prometheus-format metrics (`docs/development/implementation_plan.md`
/// section 16: "Metrics", "Database health metrics"). Unversioned and
/// unauthenticated like `/health`/`/ready` — a scrape target, not a public
/// API contract — and exposes only aggregate counts/timings, never
/// per-user content ("Logs must avoid sensitive content by default"
/// applies equally to metrics).
///
/// Every gauge here is sampled live on every request rather than tracked
/// as a running counter — `sqlx::PgPool::size`/`num_idle` and
/// [`job_stats`] (Job Observability, Milestone 36 — "consumed by health/
/// metrics reporting once that surface exists," per its own doc comment;
/// this is that surface) already report current state directly, so there
/// is nothing to accumulate.
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "operations",
    responses((status = 200, description = "Current process metrics in Prometheus text format")),
)]
pub async fn metrics(State(state): State<ApiState>) -> Response {
    let mut gauges = vec![
        (
            "p4inz_database_pool_connections",
            "Current PostgreSQL connection pool size.",
            f64::from(state.pool.size()),
        ),
        (
            "p4inz_database_pool_idle_connections",
            "Currently idle PostgreSQL connections in the pool.",
            state.pool.num_idle() as f64,
        ),
    ];

    // A job-stats query failure or timeout degrades this endpoint (missing
    // gauges), it does not fail it — a scrape blip is exactly the kind of
    // moment `/metrics` needs to stay up to help diagnose, not itself 500
    // or hang on.
    let job_repository = PgJobRepository::new(state.pool.clone());
    match tokio::time::timeout(JOB_STATS_TIMEOUT, job_stats(&job_repository)).await {
        Ok(Ok(stats)) => {
            gauges.push(("p4inz_jobs_pending", "Jobs currently pending.", stats.pending as f64));
            gauges.push(("p4inz_jobs_running", "Jobs currently running.", stats.running as f64));
            gauges.push((
                "p4inz_jobs_succeeded",
                "Jobs that have succeeded.",
                stats.succeeded as f64,
            ));
            gauges.push((
                "p4inz_jobs_failed",
                "Jobs that have failed permanently.",
                stats.failed as f64,
            ));
        }
        Ok(Err(error)) => {
            tracing::warn!(%error, "failed to gather job stats for /metrics");
        }
        Err(_) => {
            tracing::warn!("timed out gathering job stats for /metrics");
        }
    }

    let body = Metrics::global().render_prometheus_text(&gauges);

    (StatusCode::OK, [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")], body)
        .into_response()
}
