use std::net::SocketAddr;

use axum::extract::Request;
use axum::extract::{ConnectInfo, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::error::ApiError;
use crate::state::ApiState;

/// Per-client-IP rate limiting (`docs/development/implementation_plan.md`
/// section 10: "Rate-limited"; section 13: "Rate limiting", "Resource
/// limits") — every request, authenticated or not, spends one token from
/// the caller's bucket before reaching its handler. Uses
/// `p4inz_security::RateLimiter`, the same transport-agnostic limiter
/// Discord's natural-language questions already go through
/// ([`p4inz_discord`]'s message handler), keyed by client IP here since
/// most API traffic (the public search endpoint) is anonymous — there is
/// no per-user identity to key by until a session exists.
///
/// Keys by [`ConnectInfo`] (the direct TCP peer address), not an
/// `X-Forwarded-For` header: trusting a client-supplied header for rate
/// limiting would let any caller claim a fresh IP on every request and
/// bypass the limit entirely. A deployment that sits behind a reverse
/// proxy needs that proxy to preserve the real peer address (standard
/// practice — e.g. via `PROXY protocol` or by binding the proxy directly
/// in front of this process), not this middleware trusting an
/// unauthenticated header.
pub async fn rate_limit(
    State(state): State<ApiState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let key = format!("api:{}", addr.ip());
    state.rate_limiter.check(&key)?;
    Ok(next.run(request).await)
}
