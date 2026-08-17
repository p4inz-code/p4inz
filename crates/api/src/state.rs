use std::sync::Arc;

use p4inz_common::Secret;
use p4inz_database::PgPool;
use p4inz_infrastructure::DiscordOAuthClient;
use p4inz_security::{RateLimiter, RateLimiterConfig};

/// The wiring web/admin authentication needs — present only when
/// [`p4inz_config::AuthConfig::is_configured`] was true at startup.
///
/// `admin_user_ids` is API Authorization's (Milestone 41) permission
/// source: the Discord user ids granted administrative permissions — see
/// [`p4inz_config::AuthConfig::admin_user_ids`] for why this is an
/// explicit allowlist rather than resolved Discord guild roles.
#[derive(Clone)]
pub struct AuthState {
    pub oauth: DiscordOAuthClient,
    pub session_secret: Secret,
    pub admin_user_ids: Vec<String>,
}

/// Shared state handed to every route handler.
///
/// `PgPool` is internally reference-counted (an `Arc`-backed `sqlx`
/// pool), so cloning this is cheap — required by Axum's `State` extractor,
/// which clones the state per request. `auth` and `rate_limiter` are
/// wrapped in an `Arc` for the same reason: `ApiState` is cloned per
/// request, and both need one shared instance behind that, not a fresh
/// one per clone (a fresh `RateLimiter` per request would rate-limit
/// nothing at all).
#[derive(Clone)]
pub struct ApiState {
    pub pool: PgPool,
    pub auth: Option<Arc<AuthState>>,
    pub rate_limiter: Arc<RateLimiter>,
}

impl ApiState {
    /// Builds state with web/admin authentication unavailable — `/v1/auth/*`
    /// routes will report it as such rather than mounting at all.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            auth: None,
            rate_limiter: Arc::new(RateLimiter::new(RateLimiterConfig::default())),
        }
    }

    #[must_use]
    pub fn with_auth(mut self, auth: AuthState) -> Self {
        self.auth = Some(Arc::new(auth));
        self
    }
}
