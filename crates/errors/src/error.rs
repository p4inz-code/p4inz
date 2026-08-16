use thiserror::Error;

use crate::kind::ErrorKind;

/// A typed, composable error for crossing domain/application/infrastructure/
/// API boundaries.
///
/// `AppError` pairs an [`ErrorKind`] (how the error should be handled) with
/// a `message` describing what happened, and optionally preserves the
/// original error as its [`source`](std::error::Error::source) for
/// diagnostics. It intentionally does not model HTTP status codes, response
/// bodies, or any other transport-specific shape — those belong to the
/// adapters that eventually handle an `AppError` (e.g. the API crate), not
/// to this shared foundation.
///
/// `message` is surfaced through [`Display`](std::fmt::Display) and may be
/// shown to callers/logs; do not put secrets in it.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct AppError {
    kind: ErrorKind,
    message: String,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

/// Convenience alias for `Result<T, AppError>`.
pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into(), source: None }
    }

    /// Attaches an underlying error, preserving it in the `source` chain.
    #[must_use]
    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Validation, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Conflict, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unauthorized, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Forbidden, message)
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::RateLimited, message)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unavailable, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }
}

/// Wraps a fallible result's error into an [`AppError`] of a given
/// [`ErrorKind`], preserving the original error as the source.
///
/// This is how domain/infrastructure errors are expected to cross into the
/// shared taxonomy at application/adapter boundaries, e.g.:
///
/// ```
/// use p4inz_errors::{ErrorKind, IntoAppError};
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("invalid value")]
/// struct SomeDomainError;
///
/// fn parse() -> Result<(), SomeDomainError> {
///     Err(SomeDomainError)
/// }
///
/// let result = parse().into_app_error(ErrorKind::Validation, "could not parse value");
/// assert_eq!(result.unwrap_err().kind(), ErrorKind::Validation);
/// ```
pub trait IntoAppError<T> {
    fn into_app_error(self, kind: ErrorKind, message: impl Into<String>) -> AppResult<T>;
}

impl<T, E> IntoAppError<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_app_error(self, kind: ErrorKind, message: impl Into<String>) -> AppResult<T> {
        self.map_err(|source| AppError::new(kind, message).with_source(source))
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

    #[derive(Debug, Error)]
    #[error("boom")]
    struct BoomError;

    #[test]
    fn new_has_no_source() {
        let err = AppError::new(ErrorKind::Internal, "failed");
        assert_eq!(err.kind(), ErrorKind::Internal);
        assert_eq!(err.message(), "failed");
        assert!(err.source().is_none());
    }

    #[test]
    fn with_source_preserves_chain() {
        let err = AppError::new(ErrorKind::Unavailable, "provider down").with_source(BoomError);
        assert_eq!(err.kind(), ErrorKind::Unavailable);
        assert!(err.source().is_some());
        assert_eq!(err.source().unwrap().to_string(), "boom");
    }

    #[test]
    fn display_shows_message_not_kind() {
        let err = AppError::new(ErrorKind::NotFound, "project not found");
        assert_eq!(err.to_string(), "project not found");
    }

    #[test]
    fn kind_constructors_set_expected_kind() {
        assert_eq!(AppError::validation("x").kind(), ErrorKind::Validation);
        assert_eq!(AppError::not_found("x").kind(), ErrorKind::NotFound);
        assert_eq!(AppError::conflict("x").kind(), ErrorKind::Conflict);
        assert_eq!(AppError::unauthorized("x").kind(), ErrorKind::Unauthorized);
        assert_eq!(AppError::forbidden("x").kind(), ErrorKind::Forbidden);
        assert_eq!(AppError::rate_limited("x").kind(), ErrorKind::RateLimited);
        assert_eq!(AppError::unavailable("x").kind(), ErrorKind::Unavailable);
        assert_eq!(AppError::internal("x").kind(), ErrorKind::Internal);
    }

    #[test]
    fn into_app_error_wraps_and_preserves_source() {
        let result: Result<(), BoomError> = Err(BoomError);
        let wrapped = result.into_app_error(ErrorKind::Internal, "operation failed");

        let err = wrapped.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Internal);
        assert_eq!(err.message(), "operation failed");
        assert_eq!(err.source().unwrap().to_string(), "boom");
    }
}
