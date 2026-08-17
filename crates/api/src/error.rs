use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use p4inz_errors::{AppError, ErrorKind};
use serde::Serialize;

/// Wraps [`AppError`] so it can be returned directly as an Axum handler's
/// error type (`docs/development/implementation_plan.md` section 10: "API
/// errors use consistent structures").
///
/// Maps by [`ErrorKind`] to both an HTTP status and a safe message — the
/// same reasoning `p4inz_discord::error_ux::describe` already applies to
/// Discord replies: `Internal`'s message is never surfaced (it may carry
/// diagnostic detail meant for logs, not callers — `docs/PROJECT_SPEC.md`
/// section 13: "Safe failure behavior"), every other kind's message is
/// already written to be safe to show.
pub struct ApiError(AppError);

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        Self(error)
    }
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let kind = self.0.kind();
        let (status, message) = match kind {
            ErrorKind::Validation => (StatusCode::BAD_REQUEST, self.0.to_string()),
            ErrorKind::NotFound => (StatusCode::NOT_FOUND, self.0.to_string()),
            ErrorKind::Conflict => (StatusCode::CONFLICT, self.0.to_string()),
            ErrorKind::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "authentication required".to_string())
            }
            ErrorKind::Forbidden => (StatusCode::FORBIDDEN, self.0.to_string()),
            ErrorKind::RateLimited => {
                (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded".to_string())
            }
            ErrorKind::Unavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "temporarily unavailable".to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string()),
        };

        (status, Json(ErrorBody { code: kind.as_str(), message })).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;

    async fn body_json(error: ApiError) -> (StatusCode, serde_json::Value) {
        let response = error.into_response();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn validation_maps_to_bad_request_with_the_message() {
        let (status, body) = body_json(ApiError::from(AppError::validation("bad input"))).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "validation");
        assert_eq!(body["message"], "bad input");
    }

    #[tokio::test]
    async fn forbidden_maps_to_forbidden_with_the_message() {
        let (status, _) = body_json(ApiError::from(AppError::forbidden("nope"))).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rate_limited_maps_to_too_many_requests() {
        let (status, body) = body_json(ApiError::from(AppError::rate_limited("slow down"))).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(body["message"], "rate limit exceeded");
    }

    #[tokio::test]
    async fn internal_never_leaks_the_raw_message() {
        let (status, body) = body_json(ApiError::from(AppError::internal(
            "connection string postgres://user:hunter2@host/db failed",
        )))
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["message"], "internal error");
        assert!(!body["message"].as_str().unwrap().contains("hunter2"));
    }
}
