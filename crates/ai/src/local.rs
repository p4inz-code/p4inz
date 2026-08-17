use p4inz_errors::{AppResult, ErrorKind, IntoAppError};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::provider::{AiProvider, CompletionRequest, CompletionResponse, REQUEST_TIMEOUT};

/// Where to reach a local inference server, and which model to ask it for.
///
/// Targets the Ollama HTTP API (`POST {base_url}/api/generate`) — the most
/// common self-hosted local-model runner, and a natural fit for "Local AI:
/// First-class" + zero-cost/self-hosted (`docs/PROJECT_SPEC.md` sections 7
/// and 11) without this crate embedding its own inference engine.
/// `base_url` and `model` are both required rather than defaulted: which
/// model to run is a deployment choice this crate should not guess at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAiProviderConfig {
    pub base_url: String,
    pub model: String,
}

impl LocalAiProviderConfig {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self { base_url: base_url.into(), model: model.into() }
    }
}

/// An [`AiProvider`] backed by a local Ollama-compatible HTTP server.
pub struct LocalAiProvider {
    http: Client,
    config: LocalAiProviderConfig,
}

impl LocalAiProvider {
    pub fn new(config: LocalAiProviderConfig) -> AppResult<Self> {
        let http = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .into_app_error(ErrorKind::Internal, "failed to build local AI HTTP client")?;
        Ok(Self { http, config })
    }

    /// Test-only escape hatch for injecting a client with a shorter
    /// timeout than [`REQUEST_TIMEOUT`] — a real outage test can't wait
    /// out a 60-second production timeout to prove one is enforced.
    #[cfg(test)]
    fn with_client(config: LocalAiProviderConfig, http: Client) -> Self {
        Self { http, config }
    }
}

#[derive(Debug, Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<GenerateOptions>,
}

#[derive(Debug, Serialize)]
struct GenerateOptions {
    num_predict: u32,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

impl AiProvider for LocalAiProvider {
    async fn complete(&self, request: CompletionRequest) -> AppResult<CompletionResponse> {
        let body = GenerateRequest {
            model: &self.config.model,
            prompt: &request.prompt,
            stream: false,
            options: request.max_tokens.map(|num_predict| GenerateOptions { num_predict }),
        };

        let response = self
            .http
            .post(format!("{}/api/generate", self.config.base_url))
            .json(&body)
            .send()
            .await
            .into_app_error(ErrorKind::Unavailable, "failed to reach the local AI provider")?;

        let response = response
            .error_for_status()
            .into_app_error(ErrorKind::Unavailable, "the local AI provider returned an error")?;

        let parsed: GenerateResponse = response.json().await.into_app_error(
            ErrorKind::Internal,
            "failed to parse the local AI provider's response",
        )?;

        Ok(CompletionResponse { text: parsed.response })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes_without_options_when_max_tokens_is_none() {
        let body = GenerateRequest { model: "llama3", prompt: "hi", stream: false, options: None };
        let json = serde_json::to_string(&body).unwrap();
        assert!(!json.contains("options"));
    }

    #[test]
    fn request_serializes_num_predict_when_max_tokens_is_set() {
        let body = GenerateRequest {
            model: "llama3",
            prompt: "hi",
            stream: false,
            options: Some(GenerateOptions { num_predict: 128 }),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"num_predict\":128"));
    }

    #[test]
    fn response_deserializes_the_text_field() {
        let parsed: GenerateResponse =
            serde_json::from_str(r#"{"response":"hello there","done":true}"#).unwrap();
        assert_eq!(parsed.response, "hello there");
    }

    /// Requires a real local Ollama (or compatible) server. Not run by
    /// default — this environment has none available. Run explicitly with
    /// `cargo test -p p4inz-ai -- --ignored` against a real instance.
    #[tokio::test]
    #[ignore = "requires a live local Ollama-compatible server; see doc comment"]
    async fn completes_against_a_real_local_server() {
        let config = LocalAiProviderConfig::new("http://localhost:11434", "llama3");
        let provider = LocalAiProvider::new(config).unwrap();

        let response =
            provider.complete(CompletionRequest::new("Say hello in one word.")).await.unwrap();
        assert!(!response.text.is_empty());
    }

    /// AI Evaluation (Milestone 63: "Deterministic evaluation") — the
    /// three tests below exercise [`LocalAiProvider::complete`]'s actual
    /// HTTP request/response handling (success, an HTTP error status,
    /// malformed JSON) deterministically, against an in-process mock
    /// server instead of a live Ollama instance. Before this, everything
    /// past request serialization in `complete` was only covered by
    /// `completes_against_a_real_local_server`, which never runs in this
    /// environment (no live server) or in ordinary CI.
    async fn spawn_mock_server(status: axum::http::StatusCode, body: impl Into<String>) -> String {
        let body = body.into();
        let app = axum::Router::new().route(
            "/api/generate",
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

    #[tokio::test]
    async fn complete_returns_the_response_text_on_success() {
        let base_url =
            spawn_mock_server(axum::http::StatusCode::OK, r#"{"response":"mocked answer"}"#).await;
        let provider =
            LocalAiProvider::new(LocalAiProviderConfig::new(base_url, "llama3")).unwrap();

        let response = provider.complete(CompletionRequest::new("hi")).await.unwrap();

        assert_eq!(response.text, "mocked answer");
    }

    #[tokio::test]
    async fn complete_maps_an_http_error_status_to_unavailable() {
        let base_url = spawn_mock_server(axum::http::StatusCode::INTERNAL_SERVER_ERROR, "").await;
        let provider =
            LocalAiProvider::new(LocalAiProviderConfig::new(base_url, "llama3")).unwrap();

        let err = provider.complete(CompletionRequest::new("hi")).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Unavailable);
    }

    #[tokio::test]
    async fn complete_maps_malformed_json_to_internal() {
        let base_url = spawn_mock_server(axum::http::StatusCode::OK, "not json").await;
        let provider =
            LocalAiProvider::new(LocalAiProviderConfig::new(base_url, "llama3")).unwrap();

        let err = provider.complete(CompletionRequest::new("hi")).await.unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Internal);
    }

    /// A mock server that never responds — simulates a hung/unresponsive
    /// local inference server for the outage test below.
    async fn spawn_hanging_mock_server() -> String {
        let app = axum::Router::new().route(
            "/api/generate",
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

    /// Failure Tests (Milestone 66: "outage/retry/recovery") — before
    /// `REQUEST_TIMEOUT` was added to [`LocalAiProvider::new`], a
    /// hung/unresponsive provider made `complete` hang indefinitely
    /// instead of erroring, silently defeating
    /// [`p4inz_application::AiQuestionHandler`]'s deterministic fallback
    /// (Milestone 31) — a stuck request never reaches the point where
    /// fallback logic would run. Uses [`LocalAiProvider::with_client`] to
    /// inject a much shorter timeout than the real 60-second default, so
    /// this test resolves in well under a second instead of tying up the
    /// test suite for a minute.
    #[tokio::test]
    async fn complete_times_out_rather_than_hanging_on_an_unresponsive_server() {
        let base_url = spawn_hanging_mock_server().await;
        let http =
            Client::builder().timeout(std::time::Duration::from_millis(200)).build().unwrap();
        let provider =
            LocalAiProvider::with_client(LocalAiProviderConfig::new(base_url, "llama3"), http);

        let started = std::time::Instant::now();
        let err = provider.complete(CompletionRequest::new("hi")).await.unwrap_err();

        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "complete() should time out quickly, not hang until the server eventually responds"
        );
        assert_eq!(err.kind(), ErrorKind::Unavailable);
    }
}
