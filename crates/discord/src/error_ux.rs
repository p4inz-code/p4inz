use p4inz_errors::{AppError, ErrorKind};

/// Renders an [`AppError`] as a short, safe, user-facing Discord reply.
///
/// Maps by [`ErrorKind`] rather than the error's own message for
/// [`ErrorKind::Internal`] specifically: internal error messages are
/// meant for logs/diagnostics, not end users
/// (`docs/development/implementation_plan.md`: "Distinguish user-facing
/// errors from internal diagnostics"). For kinds whose message is already
/// written to be safe to show (validation reasons, missing-permission
/// explanations, etc.), the message is included since it is more useful
/// than a generic phrase.
pub fn describe(error: &AppError) -> String {
    match error.kind() {
        ErrorKind::Validation => format!("That doesn't look right: {error}"),
        ErrorKind::NotFound => format!("Couldn't find that: {error}"),
        ErrorKind::Conflict => format!("That conflicts with something existing: {error}"),
        ErrorKind::Unauthorized => "You need to be signed in to do that.".to_string(),
        ErrorKind::Forbidden => format!("You don't have permission to do that: {error}"),
        ErrorKind::RateLimited => {
            "You're doing that too quickly — please wait a moment and try again.".to_string()
        }
        ErrorKind::Unavailable => {
            "That's temporarily unavailable — please try again shortly.".to_string()
        }
        // Internal, and any future kind this crate doesn't know about yet.
        _ => "Something went wrong on our end — this has been logged.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_includes_the_reason() {
        let error = AppError::validation("project name must not be empty");
        assert_eq!(describe(&error), "That doesn't look right: project name must not be empty");
    }

    #[test]
    fn forbidden_includes_the_reason() {
        let error = AppError::forbidden("missing required permission 'project:register'");
        assert_eq!(
            describe(&error),
            "You don't have permission to do that: missing required permission 'project:register'"
        );
    }

    #[test]
    fn rate_limited_is_generic() {
        let error = AppError::rate_limited("rate limit exceeded for 'user:1'");
        assert_eq!(
            describe(&error),
            "You're doing that too quickly — please wait a moment and try again."
        );
    }

    #[test]
    fn internal_never_leaks_the_raw_message() {
        let error = AppError::internal("connection string postgres://user:hunter2@host/db failed");
        let rendered = describe(&error);
        assert!(!rendered.contains("hunter2"));
        assert_eq!(rendered, "Something went wrong on our end — this has been logged.");
    }

    #[test]
    fn unauthorized_is_generic() {
        let error = AppError::unauthorized("no session");
        assert_eq!(describe(&error), "You need to be signed in to do that.");
    }

    #[test]
    fn not_found_includes_the_reason() {
        let error = AppError::not_found("project 'p4inz' does not exist");
        assert_eq!(describe(&error), "Couldn't find that: project 'p4inz' does not exist");
    }

    #[test]
    fn conflict_includes_the_reason() {
        let error = AppError::conflict("a project with that name already exists");
        assert_eq!(
            describe(&error),
            "That conflicts with something existing: a project with that name already exists"
        );
    }

    #[test]
    fn unavailable_is_generic() {
        let error = AppError::unavailable("database is down");
        assert_eq!(describe(&error), "That's temporarily unavailable — please try again shortly.");
    }
}
