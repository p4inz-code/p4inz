mod session;

use session::verify_session;
pub(crate) use session::{SessionClaims, issue_session};

use axum::Json;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use p4inz_errors::AppError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{ApiState, AuthState};

const OAUTH_STATE_COOKIE: &str = "p4inz_oauth_state";
const SESSION_COOKIE: &str = "p4inz_session";

pub(crate) fn require_auth(state: &ApiState) -> Result<&AuthState, ApiError> {
    state.auth.as_deref().ok_or_else(|| {
        ApiError::from(AppError::unavailable("web/admin authentication is not configured"))
    })
}

/// Reads and verifies the session cookie in `jar`, returning its claims.
/// Shared by every handler that needs "who is calling, if anyone" —
/// [`me`], and API Authorization's admin actions (Milestone 41).
pub(crate) fn require_session(
    auth: &AuthState,
    jar: &CookieJar,
) -> Result<SessionClaims, ApiError> {
    let token = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| ApiError::from(AppError::unauthorized("no session")))?;
    verify_session(&auth.session_secret, token.value()).map_err(ApiError::from)
}

/// Begins "Sign in with Discord" (`docs/development/implementation_plan.md`
/// Milestone 40: Authentication) by redirecting the browser to Discord's
/// OAuth2 consent screen. Sets a short-lived, `HttpOnly` CSRF cookie whose
/// value the callback must see echoed back unchanged in the `state` query
/// parameter — the standard double-submit-cookie mitigation: an attacker
/// can craft a callback URL with their own `state`, but can't make the
/// victim's browser carry a matching cookie unless the victim genuinely
/// started the flow through this endpoint.
#[utoipa::path(
    get,
    path = "/auth/discord/login",
    tag = "auth",
    responses(
        (status = 303, description = "Redirects to Discord's OAuth2 consent screen"),
        (status = 503, description = "Web/admin authentication is not configured"),
    ),
)]
pub async fn login(
    State(state): State<ApiState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, ApiError> {
    let auth = require_auth(&state)?;

    let csrf_state = Uuid::new_v4().to_string();
    let redirect_url = auth.oauth.authorize_url(&csrf_state);

    let state_cookie = Cookie::build((OAUTH_STATE_COOKIE, csrf_state))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::minutes(10))
        .build();

    Ok((jar.add(state_cookie), Redirect::to(&redirect_url)))
}

#[derive(Deserialize)]
pub(crate) struct CallbackParams {
    code: String,
    state: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct AuthenticatedBody {
    username: String,
}

/// Completes the OAuth2 flow: verifies the CSRF `state`, exchanges the
/// authorization `code` for the caller's Discord identity, and issues a
/// signed session cookie.
#[utoipa::path(
    get,
    path = "/auth/discord/callback",
    tag = "auth",
    responses(
        (status = 200, description = "Signed in", body = AuthenticatedBody),
        (status = 401, description = "CSRF state mismatch, or Discord rejected the code"),
        (status = 503, description = "Web/admin authentication is not configured"),
    ),
)]
pub async fn callback(
    State(state): State<ApiState>,
    Query(params): Query<CallbackParams>,
    jar: CookieJar,
) -> Result<impl IntoResponse, ApiError> {
    let auth = require_auth(&state)?;

    let expected_state = jar.get(OAUTH_STATE_COOKIE).map(|cookie| cookie.value().to_string());
    if expected_state.as_deref() != Some(params.state.as_str()) {
        return Err(ApiError::from(AppError::unauthorized(
            "OAuth state parameter did not match — please try signing in again",
        )));
    }

    let identity = auth.oauth.exchange_code(&params.code).await?;
    let token = issue_session(&auth.session_secret, &identity)?;

    let session_cookie = Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::hours(24))
        .build();

    let jar = jar.remove(Cookie::from(OAUTH_STATE_COOKIE)).add(session_cookie);

    Ok((jar, Json(AuthenticatedBody { username: identity.username })))
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct MeBody {
    user_id: String,
    username: String,
}

/// Returns the current session's identity — proves the authentication
/// mechanism actually works end-to-end without yet wiring any
/// permission-based decision (API Authorization, Milestone 41).
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "The current session's identity", body = MeBody),
        (status = 401, description = "No valid session"),
        (status = 503, description = "Web/admin authentication is not configured"),
    ),
)]
pub async fn me(State(state): State<ApiState>, jar: CookieJar) -> Result<Json<MeBody>, ApiError> {
    let auth = require_auth(&state)?;
    let claims = require_session(auth, &jar)?;

    Ok(Json(MeBody { user_id: claims.sub, username: claims.username }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn require_auth_fails_when_unconfigured() {
        let config = p4inz_config::DatabaseConfig {
            url: p4inz_common::Secret::new("postgres://user:pass@localhost/p4inz"),
        };
        let pool =
            p4inz_database::connect_lazy(&config, p4inz_database::PoolSettings::default()).unwrap();

        let state = ApiState::new(pool);

        assert!(require_auth(&state).is_err());
    }
}
