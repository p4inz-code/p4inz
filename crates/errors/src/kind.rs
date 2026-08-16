use std::fmt;

/// Classifies an [`AppError`](crate::AppError) by how it should generally be
/// handled, independent of which layer produced it.
///
/// This is the shared vocabulary every boundary (domain, application,
/// infrastructure, API, Discord) maps its own errors onto so that callers
/// can react consistently (e.g. an API adapter mapping [`ErrorKind`] to an
/// HTTP status) without needing to know the originating error's concrete
/// type. Each variant traces to an explicit, already-locked requirement:
///
/// - [`Validation`](ErrorKind::Validation) — input validation
///   (`docs/security/security-model.md`).
/// - [`NotFound`](ErrorKind::NotFound) — safe failure when information is
///   unavailable (`docs/PROJECT_SPEC.md` V1 definition).
/// - [`Conflict`](ErrorKind::Conflict) — knowledge conflict detection
///   (`docs/PROJECT_SPEC.md` section 6).
/// - [`Unauthorized`](ErrorKind::Unauthorized) /
///   [`Forbidden`](ErrorKind::Forbidden) — authentication vs. authorization
///   (`docs/security/security-model.md`).
/// - [`RateLimited`](ErrorKind::RateLimited) — rate limiting
///   (`docs/PROJECT_SPEC.md` section 9).
/// - [`Unavailable`](ErrorKind::Unavailable) — graceful handling of
///   unavailable providers/dependencies (`docs/PROJECT_SPEC.md` section 7).
/// - [`Internal`](ErrorKind::Internal) — catch-all for unexpected internal
///   failures.
///
/// Marked `#[non_exhaustive]` so new kinds can be added later without a
/// breaking change; match arms must include a wildcard.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// Input failed validation.
    Validation,
    /// The requested resource does not exist (or is not visible to the caller).
    NotFound,
    /// The request conflicts with the current state of a resource.
    Conflict,
    /// The caller is not authenticated.
    Unauthorized,
    /// The caller is authenticated but lacks permission.
    Forbidden,
    /// The caller has exceeded an allowed rate.
    RateLimited,
    /// A required dependency is temporarily unavailable.
    Unavailable,
    /// An unexpected internal failure occurred.
    Internal,
}

impl ErrorKind {
    /// A short, stable, machine-readable identifier for this kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "validation",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::RateLimited => "rate_limited",
            Self::Unavailable => "unavailable",
            Self::Internal => "internal",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_is_stable_and_lowercase() {
        assert_eq!(ErrorKind::Validation.as_str(), "validation");
        assert_eq!(ErrorKind::NotFound.as_str(), "not_found");
        assert_eq!(ErrorKind::RateLimited.as_str(), "rate_limited");
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(ErrorKind::Internal.to_string(), ErrorKind::Internal.as_str());
    }
}
