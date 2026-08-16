use std::future::Future;

use p4inz_audit::AuditActor;
use p4inz_errors::AppResult;
use p4inz_security::PermissionSet;

use crate::question::Question;

/// Answers a natural-language [`Question`] on behalf of a caller.
///
/// `granted`/`actor` identify the caller and the permissions already
/// resolved for them (e.g. from Discord guild roles) — an implementation
/// that retrieves knowledge must check these before returning results
/// (`docs/PROJECT_SPEC.md` section 8: "the search system must never rely
/// on the AI to decide whether a user may access a result"), which is why
/// they are part of this trait's contract rather than something only some
/// implementations happen to need.
///
/// [`UnavailableQuestionHandler`] is the safe default when no real
/// implementation is wired up, so the Discord pipeline (and, later, the
/// API) always has something real to call.
///
/// Returns `impl Future + Send` rather than using `async fn` in the trait
/// directly, for the same reason as `ProjectRepository`.
pub trait QuestionHandler {
    fn answer(
        &self,
        question: &Question,
        granted: &PermissionSet,
        actor: AuditActor,
    ) -> impl Future<Output = AppResult<String>> + Send;
}

/// Answers every question with a fixed, honest "not available yet" message.
///
/// `docs/PROJECT_SPEC.md` section 7 requires deterministic functionality to
/// keep working when AI/knowledge retrieval is unavailable; before those
/// subsystems are wired up, "unavailable" is simply always the case.
pub struct UnavailableQuestionHandler;

impl QuestionHandler for UnavailableQuestionHandler {
    async fn answer(
        &self,
        _question: &Question,
        _granted: &PermissionSet,
        _actor: AuditActor,
    ) -> AppResult<String> {
        Ok("I can't answer questions yet — that capability hasn't been built. \
            Try a slash command instead."
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unavailable_handler_always_succeeds() {
        let question = Question::parse("What is P4inz?").unwrap();
        let answer = UnavailableQuestionHandler
            .answer(&question, &PermissionSet::empty(), AuditActor::System)
            .await
            .unwrap();
        assert!(!answer.is_empty());
    }
}
