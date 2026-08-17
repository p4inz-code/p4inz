use std::future::Future;
use std::time::Duration;

use p4inz_errors::AppResult;

/// How long a provider implementation ([`crate::LocalAiProvider`],
/// [`crate::OnlineAiProvider`]) waits for a response before giving up.
///
/// Without this, a hung or unresponsive provider blocks the calling
/// request indefinitely — `reqwest::Client` applies no timeout by default
/// (`Failure Tests`, Milestone 66: "outage/retry/recovery"; `docs/
/// PROJECT_SPEC.md` section 7: "Deterministic features must continue
/// working when... A provider times out" only holds if a timeout is ever
/// actually enforced). 60 seconds is generous enough for real local/online
/// model generation, while still bounding the worst case so
/// [`p4inz_application::AiQuestionHandler`]'s deterministic fallback
/// (Milestone 31) is reached in bounded time rather than never.
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// A request to generate a completion from an AI provider.
///
/// Deliberately minimal — no evidence/citation/confidence fields. Prompt
/// assembly (combining a question with retrieved knowledge) is AI
/// Context's job (Milestone 27); this type only needs to carry whatever
/// text the caller has already assembled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionRequest {
    pub prompt: String,
    /// A soft cap on response length. `None` lets the provider use its
    /// own default; providers are not required to honor an exact value.
    pub max_tokens: Option<u32>,
}

impl CompletionRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self { prompt: prompt.into(), max_tokens: None }
    }

    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
}

/// A completed response from an AI provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionResponse {
    pub text: String,
}

/// A pluggable AI backend (ADR-003: "AI providers are accessed through an
/// internal abstraction... Local models and future providers must be
/// interchangeable without changing domain logic").
///
/// No concrete implementation lives here. `docs/PROJECT_SPEC.md` section 7
/// requires the core application to keep working when a provider is
/// unavailable — implementations should surface that as
/// `ErrorKind::Unavailable` rather than panicking, so callers (AI
/// Fallback, Milestone 31) can react uniformly regardless of which
/// provider is behind this trait.
pub trait AiProvider {
    fn complete(
        &self,
        request: CompletionRequest,
    ) -> impl Future<Output = AppResult<CompletionResponse>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_no_max_tokens_by_default() {
        let request = CompletionRequest::new("hello");
        assert_eq!(request.prompt, "hello");
        assert_eq!(request.max_tokens, None);
    }

    #[test]
    fn with_max_tokens_sets_it() {
        let request = CompletionRequest::new("hello").with_max_tokens(256);
        assert_eq!(request.max_tokens, Some(256));
    }
}
