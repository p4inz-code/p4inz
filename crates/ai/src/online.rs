use p4inz_common::Secret;
use p4inz_errors::{AppResult, ErrorKind, IntoAppError};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::provider::{AiProvider, CompletionRequest, CompletionResponse};

/// Where to reach an online provider, and how to authenticate to it.
///
/// Targets the OpenAI-compatible `POST {base_url}/chat/completions` shape
/// rather than one vendor's proprietary API — many providers (and
/// self-hosted gateways) implement this exact shape for compatibility, so
/// this one adapter works with more than a single named vendor without
/// this crate picking one (`docs/PROJECT_SPEC.md` section 11: "Paid
/// providers may be supported through abstractions but must never become
/// mandatory" — ADR-003: providers must be interchangeable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineAiProviderConfig {
    pub base_url: String,
    pub api_key: Secret,
    pub model: String,
}

impl OnlineAiProviderConfig {
    pub fn new(base_url: impl Into<String>, api_key: Secret, model: impl Into<String>) -> Self {
        Self { base_url: base_url.into(), api_key, model: model.into() }
    }
}

/// An [`AiProvider`] backed by an OpenAI-compatible online HTTP API.
///
/// Optional by construction: nothing in this crate or `p4inz-config`
/// requires this provider to be configured (`docs/PROJECT_SPEC.md` section
/// 7: "Online AI: Optional"). Deterministic functionality must keep
/// working with only [`crate::LocalAiProvider`] or no provider at all —
/// enforcing that is AI Fallback's job (Milestone 31), not this type's.
pub struct OnlineAiProvider {
    http: Client,
    config: OnlineAiProviderConfig,
}

impl OnlineAiProvider {
    pub fn new(config: OnlineAiProviderConfig) -> AppResult<Self> {
        let http = Client::builder()
            .build()
            .into_app_error(ErrorKind::Internal, "failed to build online AI HTTP client")?;
        Ok(Self { http, config })
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

impl AiProvider for OnlineAiProvider {
    async fn complete(&self, request: CompletionRequest) -> AppResult<CompletionResponse> {
        let body = ChatCompletionRequest {
            model: &self.config.model,
            messages: vec![ChatMessage { role: "user", content: &request.prompt }],
            max_tokens: request.max_tokens,
        };

        let response = self
            .http
            .post(format!("{}/chat/completions", self.config.base_url))
            .bearer_auth(self.config.api_key.expose_secret())
            .json(&body)
            .send()
            .await
            .into_app_error(ErrorKind::Unavailable, "failed to reach the online AI provider")?;

        let response = response
            .error_for_status()
            .into_app_error(ErrorKind::Unavailable, "the online AI provider returned an error")?;

        let parsed: ChatCompletionResponse = response.json().await.into_app_error(
            ErrorKind::Internal,
            "failed to parse the online AI provider's response",
        )?;

        let text = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| {
                p4inz_errors::AppError::new(
                    ErrorKind::Internal,
                    "online AI provider returned no choices",
                )
            })?
            .message
            .content;

        Ok(CompletionResponse { text })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_omits_max_tokens_when_not_set() {
        let body = ChatCompletionRequest {
            model: "gpt-4",
            messages: vec![ChatMessage { role: "user", content: "hi" }],
            max_tokens: None,
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(!json.contains("max_tokens"));
    }

    #[test]
    fn request_includes_max_tokens_when_set() {
        let body = ChatCompletionRequest {
            model: "gpt-4",
            messages: vec![ChatMessage { role: "user", content: "hi" }],
            max_tokens: Some(64),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"max_tokens\":64"));
    }

    #[test]
    fn response_extracts_first_choice_content() {
        let parsed: ChatCompletionResponse = serde_json::from_str(
            r#"{"choices":[{"message":{"role":"assistant","content":"hello"}}]}"#,
        )
        .unwrap();
        assert_eq!(parsed.choices[0].message.content, "hello");
    }

    /// Requires a real, valid API key and network access to a real
    /// OpenAI-compatible endpoint. Not run by default. Run explicitly with
    /// `cargo test -p p4inz-ai -- --ignored`.
    #[tokio::test]
    #[ignore = "requires a real online AI provider and API key; see doc comment"]
    async fn completes_against_a_real_online_provider() {
        let api_key = Secret::new(std::env::var("AI_API_KEY").expect("AI_API_KEY must be set"));
        let base_url = std::env::var("AI_BASE_URL").expect("AI_BASE_URL must be set");
        let model = std::env::var("AI_MODEL").expect("AI_MODEL must be set");

        let provider =
            OnlineAiProvider::new(OnlineAiProviderConfig::new(base_url, api_key, model)).unwrap();
        let response =
            provider.complete(CompletionRequest::new("Say hello in one word.")).await.unwrap();
        assert!(!response.text.is_empty());
    }
}
