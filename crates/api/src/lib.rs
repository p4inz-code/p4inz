//! P4inz API subsystem.
//!
//! The Axum HTTP application foundation (`docs/development/
//! implementation_plan.md` Milestone 37: API Foundation; section 10: "API
//! Architecture"). [`build_router`] assembles the complete [`axum::Router`]
//! — currently `/health` (liveness) and `/ready` (readiness, backed by
//! [`p4inz_database::health_check`]) — wrapped in an explicit CORS policy
//! built from a configured origin allowlist.
//!
//! API Contracts (Milestone 38): the router is versioned
//! ([`API_V1_PREFIX`]) and self-describing — every handler's
//! `#[utoipa::path]` annotation is collected into a single OpenAPI
//! document served at `GET /openapi.json`, so the contract can never
//! silently drift from the routes that actually exist.
//!
//! Public Knowledge API (Milestone 39): `GET /v1/knowledge/search` reuses
//! the same permission-aware `p4inz_application::SearchKnowledge` use case
//! Discord's natural-language questions go through, granting every
//! (currently anonymous) API caller a fixed, minimal `PermissionSet` —
//! "Public endpoints expose only public information" is enforced by
//! actually authorizing/auditing the request, not by bypassing that
//! pipeline.
//!
//! Authentication (Milestone 40): "Sign in with Discord" — `/v1/auth/
//! discord/login` (redirect), `/v1/auth/discord/callback` (OAuth2
//! exchange, issues a signed session cookie), `/v1/auth/me` (current
//! session's identity). Reuses Discord identity rather than a separate
//! account system, matching Discord's already-central role in this
//! product's identity model — see [`auth`]'s module docs.
//!
//! API Authorization (Milestone 41): `POST /v1/admin/knowledge/sync`
//! (see [`admin`]) is the first authenticated *and* authorized action —
//! it requires a valid session (Milestone 40) whose Discord user id is on
//! the configured admin allowlist, enforced and audited through the same
//! `p4inz_security::authorize` boundary every other permission check in
//! this codebase goes through, before it's allowed to call GitHub Jobs'
//! manual-trigger action (Milestone 35).
//!
//! API Security (Milestone 42): every request spends one token from a
//! per-client-IP [`p4inz_security::RateLimiter`] bucket ([`rate_limit`])
//! before reaching any handler — the same limiter type Discord's
//! natural-language questions already use, applied here at the transport
//! boundary instead of per-command. Request bodies are validated before
//! any state-changing action runs (bounded lengths, well-formed shapes),
//! and [`error::ApiError`] (Milestone 39) already guarantees every
//! failure — validation or otherwise — renders as the same consistent,
//! safe JSON error structure.
//!
//! This crate stays behind `p4inz-database`'s boundary for persistence
//! (never `sqlx` directly) the same as `p4inz-search`/
//! `p4inz-infrastructure` (`docs/architecture/dependency-rules.md`).
//!
//! Observability (Milestone 51): every request passes through
//! [`request_tracing::trace_requests`] (outermost middleware layer), which
//! assigns a correlation id, wraps handling in a tracing span, and records
//! the outcome into `p4inz_observability::metrics::Metrics`. `GET /metrics`
//! (see [`metrics`]) exposes those counters, plus live database pool
//! gauges, as Prometheus plain text.

mod admin;
mod auth;
mod error;
mod health;
mod knowledge;
mod metrics;
mod rate_limit;
mod request_tracing;
mod router;
mod state;
mod version;

pub use router::build_router;
pub use state::{ApiState, AuthState};
pub use version::API_V1_PREFIX;
