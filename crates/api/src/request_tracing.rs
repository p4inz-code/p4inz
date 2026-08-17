use std::time::Instant;

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use p4inz_observability::metrics::Metrics;
use p4inz_observability::request_id;
use tracing::Instrument;

/// The header a client-supplied correlation id is read from, and every
/// response's id is written back to.
const REQUEST_ID_HEADER: &str = "x-request-id";

/// API tracing and correlation ids (`docs/development/implementation_plan.md`
/// section 16: "Correlation/request IDs", "API tracing") — every request is
/// assigned an id (reused from an incoming `X-Request-Id` header when the
/// caller already has one, e.g. a reverse proxy that assigns its own),
/// correlated under one tracing span, measured, and recorded into
/// [`Metrics`].
///
/// Registered as the outermost middleware layer (`router::build_router`),
/// so this covers every request including ones later rejected by CORS or
/// rate limiting — those are exactly the requests an operator most needs
/// correlated and counted, not just the ones that reach a handler.
pub async fn trace_requests(request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(request_id::generate);

    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let span = tracing::info_span!("http_request", request_id = %request_id, %method, %path);

    async move {
        let start = Instant::now();
        let mut response = next.run(request).await;
        let duration = start.elapsed();
        let status = response.status();

        Metrics::global().record_http_request(status.as_u16(), duration);

        let duration_ms = duration.as_millis() as u64;
        if status.is_server_error() {
            tracing::error!(status = status.as_u16(), duration_ms, "request failed");
        } else if status.is_client_error() {
            tracing::warn!(status = status.as_u16(), duration_ms, "request rejected");
        } else {
            tracing::info!(status = status.as_u16(), duration_ms, "request completed");
        }

        if let Ok(header_value) = HeaderValue::from_str(&request_id) {
            response.headers_mut().insert(REQUEST_ID_HEADER, header_value);
        }

        response
    }
    .instrument(span)
    .await
}
