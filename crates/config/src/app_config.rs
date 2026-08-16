use p4inz_common::Secret;

use crate::environment::Environment;
use crate::error::ConfigError;

const RUST_LOG_KEY: &str = "RUST_LOG";
const DATABASE_URL_KEY: &str = "DATABASE_URL";
const DISCORD_TOKEN_KEY: &str = "DISCORD_TOKEN";
const DISCORD_APPLICATION_ID_KEY: &str = "DISCORD_APPLICATION_ID";
const GITHUB_TOKEN_KEY: &str = "GITHUB_TOKEN";
const AI_PROVIDER_KEY: &str = "AI_PROVIDER";
const AI_MODEL_KEY: &str = "AI_MODEL";
const AI_API_KEY_KEY: &str = "AI_API_KEY";
const AI_BASE_URL_KEY: &str = "AI_BASE_URL";

const DEFAULT_LOG_FILTER: &str = "info";

/// Core runtime settings that apply regardless of which subsystems are enabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreConfig {
    pub environment: Environment,
    pub log_filter: String,
}

/// PostgreSQL connection settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub url: Secret,
}

/// Discord bot credentials.
///
/// Required: P4inz is fundamentally a Discord application
/// (`docs/PROJECT_SPEC.md`, sections 1 and 13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordConfig {
    pub token: Secret,
    pub application_id: String,
}

/// Optional GitHub API access, used for higher rate limits and/or private
/// repository access (`docs/PROJECT_SPEC.md` section 5).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitHubConfig {
    pub token: Option<Secret>,
}

/// Optional AI provider selection.
///
/// `docs/PROJECT_SPEC.md` section 7 / the locked technology stack: local AI
/// is first-class, online AI is optional. Provider/model selection is not
/// yet validated against a closed set of supported providers — that belongs
/// to the AI provider abstraction (Milestone 24).
///
/// `api_key`/`base_url` are only meaningful for an online provider
/// (Optional Online Provider, Milestone 26) — a local provider
/// (Milestone 25) is configured directly with its own base URL/model
/// rather than through this struct, since it needs no credential.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AiConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<Secret>,
    pub base_url: Option<String>,
}

/// The fully assembled, validated application configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub core: CoreConfig,
    pub database: DatabaseConfig,
    pub discord: DiscordConfig,
    pub github: GitHubConfig,
    pub ai: AiConfig,
}

impl AppConfig {
    /// Loads configuration from the process environment.
    ///
    /// Fails closed: a missing required variable or an invalid value
    /// returns a [`ConfigError`] rather than falling back to a guessed
    /// default.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_source(|key| std::env::var(key).ok())
    }

    /// Loads configuration from an arbitrary key lookup function.
    ///
    /// [`from_env`](Self::from_env) calls this with `std::env::var`; tests
    /// call it directly with an in-memory map so they never need to mutate
    /// process-wide environment variables (which would be racy under
    /// `cargo test`'s parallel execution).
    pub fn from_source(source: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let environment = match optional(&source, Environment::ENV_KEY) {
            Some(raw) => Environment::parse(&raw)?,
            None => Environment::default(),
        };
        let log_filter =
            optional(&source, RUST_LOG_KEY).unwrap_or_else(|| DEFAULT_LOG_FILTER.to_string());

        let database = DatabaseConfig { url: Secret::new(required(&source, DATABASE_URL_KEY)?) };

        let discord = DiscordConfig {
            token: Secret::new(required(&source, DISCORD_TOKEN_KEY)?),
            application_id: required(&source, DISCORD_APPLICATION_ID_KEY)?,
        };

        let github = GitHubConfig { token: optional(&source, GITHUB_TOKEN_KEY).map(Secret::new) };

        let ai = AiConfig {
            provider: optional(&source, AI_PROVIDER_KEY),
            model: optional(&source, AI_MODEL_KEY),
            api_key: optional(&source, AI_API_KEY_KEY).map(Secret::new),
            base_url: optional(&source, AI_BASE_URL_KEY),
        };

        Ok(Self { core: CoreConfig { environment, log_filter }, database, discord, github, ai })
    }
}

fn required(
    source: &impl Fn(&str) -> Option<String>,
    key: &'static str,
) -> Result<String, ConfigError> {
    match source(key) {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ConfigError::Missing { key }),
    }
}

/// Blank and absent are both treated as "not set". A non-Unicode value is
/// also treated as absent rather than surfaced as a distinct error, so this
/// never fails outright: an optional variable can only be present or not.
fn optional(source: &impl Fn(&str) -> Option<String>, key: &'static str) -> Option<String> {
    source(key).filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn full_valid_env() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("DATABASE_URL", "postgres://user:pass@localhost/p4inz"),
            ("DISCORD_TOKEN", "discord-token"),
            ("DISCORD_APPLICATION_ID", "1234567890"),
        ])
    }

    fn source_from(map: HashMap<&'static str, &'static str>) -> impl Fn(&str) -> Option<String> {
        move |key| map.get(key).map(|v| v.to_string())
    }

    #[test]
    fn loads_required_fields_and_applies_defaults() {
        let config = AppConfig::from_source(source_from(full_valid_env())).unwrap();

        assert_eq!(config.database.url.expose_secret(), "postgres://user:pass@localhost/p4inz");
        assert_eq!(config.discord.token.expose_secret(), "discord-token");
        assert_eq!(config.discord.application_id, "1234567890");
        assert_eq!(config.core.environment, Environment::Development);
        assert_eq!(config.core.log_filter, "info");
        assert_eq!(config.github, GitHubConfig::default());
        assert_eq!(config.ai, AiConfig::default());
    }

    #[test]
    fn missing_database_url_is_reported() {
        let mut env = full_valid_env();
        env.remove("DATABASE_URL");

        let err = AppConfig::from_source(source_from(env)).unwrap_err();
        assert_eq!(err, ConfigError::Missing { key: "DATABASE_URL" });
    }

    #[test]
    fn missing_discord_token_is_reported() {
        let mut env = full_valid_env();
        env.remove("DISCORD_TOKEN");

        let err = AppConfig::from_source(source_from(env)).unwrap_err();
        assert_eq!(err, ConfigError::Missing { key: "DISCORD_TOKEN" });
    }

    #[test]
    fn blank_required_value_is_treated_as_missing() {
        let mut env = full_valid_env();
        env.insert("DATABASE_URL", "   ");

        let err = AppConfig::from_source(source_from(env)).unwrap_err();
        assert_eq!(err, ConfigError::Missing { key: "DATABASE_URL" });
    }

    #[test]
    fn invalid_environment_is_reported() {
        let mut env = full_valid_env();
        env.insert("P4INZ_ENV", "not-a-real-environment");

        let err = AppConfig::from_source(source_from(env)).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { key: "P4INZ_ENV", .. }));
    }

    #[test]
    fn optional_fields_are_populated_when_present() {
        let mut env = full_valid_env();
        env.insert("GITHUB_TOKEN", "gh-token");
        env.insert("AI_PROVIDER", "online");
        env.insert("AI_MODEL", "llama");
        env.insert("AI_API_KEY", "ai-secret");
        env.insert("AI_BASE_URL", "https://api.example.com/v1");
        env.insert("RUST_LOG", "debug");
        env.insert("P4INZ_ENV", "staging");

        let config = AppConfig::from_source(source_from(env)).unwrap();

        assert_eq!(config.github.token.unwrap().expose_secret(), "gh-token");
        assert_eq!(config.ai.provider.as_deref(), Some("online"));
        assert_eq!(config.ai.model.as_deref(), Some("llama"));
        assert_eq!(config.ai.api_key.unwrap().expose_secret(), "ai-secret");
        assert_eq!(config.ai.base_url.as_deref(), Some("https://api.example.com/v1"));
        assert_eq!(config.core.log_filter, "debug");
        assert_eq!(config.core.environment, Environment::Staging);
    }
}
