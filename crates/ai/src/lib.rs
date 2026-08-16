//! P4inz ai subsystem.
//!
//! AI orchestration/provider abstraction (`docs/architecture/overview.md`,
//! ADR-003). [`AiProvider`] is the pluggable backend contract.
//! [`LocalAiProvider`] (Milestone 25) implements it against a local
//! Ollama-compatible HTTP server — "Local AI: First-class"
//! (`docs/PROJECT_SPEC.md` section 7). [`OnlineAiProvider`] (Milestone 26)
//! implements it against an OpenAI-compatible online API — "Online AI:
//! Optional", never mandatory.

mod local;
mod online;
mod provider;

pub use local::{LocalAiProvider, LocalAiProviderConfig};
pub use online::{OnlineAiProvider, OnlineAiProviderConfig};
pub use provider::{AiProvider, CompletionRequest, CompletionResponse};
