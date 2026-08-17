use p4inz_common::Secret;
use p4inz_errors::{AppResult, ErrorKind, IntoAppError};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::provider::{AiProvider, CompletionRequest, CompletionResponse, REQUEST_TIMEOUT};

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
            .timeout(REQUEST_TIMEOUT)
            .build()
            .into_app_error(ErrorKind::Internal, "failed to build online AI HTTP client")?;
        Ok(Self { http, config })
    }

    /// Test-only escape hatch for injecting a client with a shorter
    /// timeout than [`REQUEST_TIMEOUT`] — see `crate::local`'s equivalent
    /// for why.
    #[cfg(test)]
    fn with_client(config: OnlineAiProviderConfig, http: Client) -> Self {
        Self { http, config }
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

    /// AI Evaluation (Milestone 63: "Deterministic evaluation") — see
    /// `crate::local`'s equivalent tests for why this mock server exists:
    /// everything past request serialization in
    /// [`OnlineAiProvider::complete`] was previously only covered by the
    /// live-provider test above, which never runs here or in ordinary CI.
    async fn spawn_mock_server(status: axum::http::StatusCode, body: impl Into<String>) -> String {
        let body = body.into();
        let app = axum::Router::new().route(
            "/chat/completions",
            axum::routing::post(move || {
                let body = body.clone();
                async move { (status, body) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    fn provider_for(base_url: String) -> OnlineAiProvider {
        OnlineAiProvider::new(OnlineAiProviderConfig::new(
            base_url,
            Secret::new("test-api-key"),
            "gpt-4",
        ))
        .unwrap()
    }

    #[tokio::test]
    async fn complete_returns_the_first_choices_content_on_success() {
        let base_url = spawn_mock_server(
            axum::http::StatusCode::OK,
            r#"{"choices":[{"message":{"role":"assistant","content":"mocked answer"}}]}"#,
        )
        .await;

        let response = provider_for(base_url).complete(CompletionRequest::new("hi")).await.unwrap();

        assert_eq!(response.text, "mocked answer");
    }

    #[tokio::test]
    async fn complete_maps_an_http_error_status_to_unavailable() {
        let base_url = spawn_mock_server(axum::http::StatusCode::UNAUTHORIZED, "").await;

        let err = provider_for(base_url).complete(CompletionRequest::new("hi")).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Unavailable);
    }

    #[tokio::test]
    async fn complete_maps_malformed_json_to_internal() {
        let base_url = spawn_mock_server(axum::http::StatusCode::OK, "not json").await;

        let err = provider_for(base_url).complete(CompletionRequest::new("hi")).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Internal);
    }

    #[tokio::test]
    async fn complete_maps_an_empty_choices_array_to_internal() {
        let base_url = spawn_mock_server(axum::http::StatusCode::OK, r#"{"choices":[]}"#).await;

        let err = provider_for(base_url).complete(CompletionRequest::new("hi")).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Internal);
    }

    /// A mock server that never responds — simulates a hung/unresponsive
    /// online provider for the outage test below.
    async fn spawn_hanging_mock_server() -> String {
        let app = axum::Router::new().route(
            "/chat/completions",
            axum::routing::post(|| async {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                axum::http::StatusCode::OK
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    /// Failure Tests (Milestone 66: "outage/retry/recovery") — see
    /// `crate::local`'s equivalent test for why this matters: without a
    /// client-side timeout, a hung provider makes `complete` hang
    /// indefinitely instead of erroring, silently defeating
    /// [`p4inz_application::AiQuestionHandler`]'s deterministic fallback.
    #[tokio::test]
    async fn complete_times_out_rather_than_hanging_on_an_unresponsive_server() {
        let base_url = spawn_hanging_mock_server().await;
        let http =
            Client::builder().timeout(std::time::Duration::from_millis(200)).build().unwrap();
        let provider = OnlineAiProvider::with_client(
            OnlineAiProviderConfig::new(base_url, Secret::new("test-api-key"), "gpt-4"),
            http,
        );

        let started = std::time::Instant::now();
        let err = provider.complete(CompletionRequest::new("hi")).await.unwrap_err();

        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "complete() should time out quickly, not hang until the server eventually responds"
        );
        assert_eq!(err.kind(), ErrorKind::Unavailable);
    }
}
