use p4inz_errors::{AppResult, ErrorKind, IntoAppError};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::provider::{AiProvider, CompletionRequest, CompletionResponse};

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
            .build()
            .into_app_error(ErrorKind::Internal, "failed to build local AI HTTP client")?;
        Ok(Self { http, config })
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
}
