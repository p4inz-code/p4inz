use p4inz_common::Secret;

use crate::environment::Environment;
use crate::error::ConfigError;

const RUST_LOG_KEY: &str = "RUST_LOG";
const DATABASE_URL_KEY: &str = "DATABASE_URL";
const DISCORD_TOKEN_KEY: &str = "DISCORD_TOKEN";
const DISCORD_APPLICATION_ID_KEY: &str = "DISCORD_APPLICATION_ID";
const GITHUB_TOKEN_KEY: &str = "GITHUB_TOKEN";
const GITHUB_REPOSITORIES_KEY: &str = "GITHUB_REPOSITORIES";
const AI_PROVIDER_KEY: &str = "AI_PROVIDER";
const AI_MODEL_KEY: &str = "AI_MODEL";
const AI_API_KEY_KEY: &str = "AI_API_KEY";
const AI_BASE_URL_KEY: &str = "AI_BASE_URL";
const API_PORT_KEY: &str = "API_PORT";
const API_ALLOWED_ORIGINS_KEY: &str = "API_ALLOWED_ORIGINS";
const DISCORD_CLIENT_SECRET_KEY: &str = "DISCORD_CLIENT_SECRET";
const AUTH_REDIRECT_URI_KEY: &str = "AUTH_REDIRECT_URI";
const AUTH_SESSION_SECRET_KEY: &str = "AUTH_SESSION_SECRET";
const ADMIN_USER_IDS_KEY: &str = "ADMIN_USER_IDS";

const DEFAULT_LOG_FILTER: &str = "info";
const DEFAULT_API_PORT: u16 = 8080;

/// Minimum length for [`AuthConfig::session_secret`] (Milestone 56:
/// Production Configuration — "Environment hardening"). It signs every
/// session token via HMAC-SHA256 (`p4inz_api::auth::session`); a short
/// secret is the weak link in an otherwise-sound signature scheme — 32
/// bytes matches SHA-256's own output size, the point past which a longer
/// key stops adding meaningful resistance to brute-force. Enforced
/// whenever the secret is set at all, not only in production: a weak
/// session-signing key is just as forgeable in staging.
const MIN_SESSION_SECRET_LENGTH: usize = 32;

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
///
/// `repositories` lists the `"owner/repo"` references GitHub Jobs
/// (Milestone 35) synchronizes on a schedule — empty means nothing is
/// scheduled (a deployment can still trigger a sync manually for a
/// specific reference without configuring anything here).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitHubConfig {
    pub token: Option<Secret>,
    pub repositories: Vec<String>,
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

/// HTTP API settings (`docs/development/implementation_plan.md` section
/// 10: "API Architecture").
///
/// `allowed_origins` is the explicit CORS allowlist ("CORS is explicitly
/// configured") — empty by default, so no cross-origin browser request is
/// permitted until a deployment configures the website's actual origin.
/// Fails closed the same way an unconfigured [`crate::AppConfig::github`]
/// schedules nothing: an empty allowlist is a valid, safe, restrictive
/// default, not a missing value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiConfig {
    pub port: u16,
    pub allowed_origins: Vec<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self { port: DEFAULT_API_PORT, allowed_origins: Vec::new() }
    }
}

/// Web/admin authentication settings — "Sign in with Discord"
/// (`docs/development/implementation_plan.md` Milestone 40:
/// Authentication). The OAuth `client_id` is `discord.application_id`
/// (Discord bot applications double as OAuth clients); no separate field
/// for it here.
///
/// Optional as a whole, the same way [`GitHubConfig`]/[`AiConfig`] are:
/// a deployment that never uses web/admin login doesn't need these set.
/// [`AuthConfig::is_configured`] tells callers (the API router) whether
/// to mount the `/v1/auth/*` routes at all — unconfigured means "this
/// deployment hasn't set up web login," not "something is broken."
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuthConfig {
    pub discord_client_secret: Option<Secret>,
    pub redirect_uri: Option<String>,
    pub session_secret: Option<Secret>,
    /// Discord user ids granted administrative permissions
    /// (`docs/development/implementation_plan.md` Milestone 41: API
    /// Authorization). An explicit allowlist rather than resolving guild
    /// roles: P4inz has no per-guild role-mapping configuration storage
    /// for the *web* identity path yet (mirroring
    /// `p4inz_discord::GuildRoleMapping`'s own documented limitation for
    /// the Discord path) — an allowlist is the smallest mechanism that is
    /// still genuinely fail-closed (unlisted users get nothing) rather
    /// than a placeholder that grants everyone or no one access.
    pub admin_user_ids: Vec<String>,
}

impl AuthConfig {
    pub fn is_configured(&self) -> bool {
        self.discord_client_secret.is_some()
            && self.redirect_uri.is_some()
            && self.session_secret.is_some()
    }
}

/// The fully assembled, validated application configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub core: CoreConfig,
    pub database: DatabaseConfig,
    pub discord: DiscordConfig,
    pub github: GitHubConfig,
    pub ai: AiConfig,
    pub api: ApiConfig,
    pub auth: AuthConfig,
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

        let github = GitHubConfig {
            token: optional(&source, GITHUB_TOKEN_KEY).map(Secret::new),
            repositories: optional(&source, GITHUB_REPOSITORIES_KEY)
                .map(|raw| {
                    raw.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
        };

        let ai = AiConfig {
            provider: optional(&source, AI_PROVIDER_KEY),
            model: optional(&source, AI_MODEL_KEY),
            api_key: optional(&source, AI_API_KEY_KEY).map(Secret::new),
            base_url: optional(&source, AI_BASE_URL_KEY),
        };

        let api = ApiConfig {
            port: match optional(&source, API_PORT_KEY) {
                Some(raw) => raw.parse::<u16>().map_err(|_| ConfigError::Invalid {
                    key: API_PORT_KEY,
                    reason: "must be a valid port number (1-65535)".to_string(),
                })?,
                None => DEFAULT_API_PORT,
            },
            allowed_origins: optional(&source, API_ALLOWED_ORIGINS_KEY)
                .map(|raw| {
                    raw.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
        };

        let auth = AuthConfig {
            discord_client_secret: optional(&source, DISCORD_CLIENT_SECRET_KEY).map(Secret::new),
            redirect_uri: optional(&source, AUTH_REDIRECT_URI_KEY),
            session_secret: optional(&source, AUTH_SESSION_SECRET_KEY).map(Secret::new),
            admin_user_ids: optional(&source, ADMIN_USER_IDS_KEY)
                .map(|raw| {
                    raw.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
        };

        validate_hardening(environment, &api, &auth)?;

        Ok(Self {
            core: CoreConfig { environment, log_filter },
            database,
            discord,
            github,
            ai,
            api,
            auth,
        })
    }
}

/// Production Configuration (Milestone 56: "Environment hardening") —
/// checks that apply on top of the per-field parsing above, some
/// unconditionally and some only once `environment` is
/// [`Environment::Production`]. Kept separate from field-by-field parsing
/// above since these checks reason about the *combination* of environment
/// and already-parsed values, not about any single raw string.
fn validate_hardening(
    environment: Environment,
    api: &ApiConfig,
    auth: &AuthConfig,
) -> Result<(), ConfigError> {
    if let Some(secret) = &auth.session_secret {
        if secret.expose_secret().len() < MIN_SESSION_SECRET_LENGTH {
            return Err(ConfigError::Invalid {
                key: AUTH_SESSION_SECRET_KEY,
                reason: format!(
                    "must be at least {MIN_SESSION_SECRET_LENGTH} characters long — it signs \
                     session tokens, and a short secret is brute-forceable"
                ),
            });
        }
    }

    if !environment.is_production() {
        return Ok(());
    }

    // A plaintext OAuth redirect in production would send the
    // authorization code over an unencrypted connection — acceptable for
    // `http://localhost` during local development, never in production.
    if let Some(redirect_uri) = &auth.redirect_uri {
        if !redirect_uri.starts_with("https://") {
            return Err(ConfigError::Invalid {
                key: AUTH_REDIRECT_URI_KEY,
                reason: "must use https:// in production".to_string(),
            });
        }
    }

    // Same reasoning as the redirect URI above: an allowed CORS origin
    // that isn't itself served over HTTPS can't meaningfully protect the
    // credentialed requests (session cookie) it's being granted access to.
    if let Some(insecure_origin) =
        api.allowed_origins.iter().find(|origin| !origin.starts_with("https://"))
    {
        return Err(ConfigError::Invalid {
            key: API_ALLOWED_ORIGINS_KEY,
            reason: format!("'{insecure_origin}' must use https:// in production"),
        });
    }

    Ok(())
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

    const LONG_ENOUGH_SESSION_SECRET: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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
        assert_eq!(config.api, ApiConfig::default());
        assert_eq!(config.auth, AuthConfig::default());
        assert!(!config.auth.is_configured());
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
        env.insert("GITHUB_REPOSITORIES", "p4inz-code/p4inz, p4inz-code/website");
        env.insert("AI_PROVIDER", "online");
        env.insert("AI_MODEL", "llama");
        env.insert("AI_API_KEY", "ai-secret");
        env.insert("AI_BASE_URL", "https://api.example.com/v1");
        env.insert("RUST_LOG", "debug");
        env.insert("P4INZ_ENV", "staging");
        env.insert("API_PORT", "9090");
        env.insert("API_ALLOWED_ORIGINS", "https://p4inz.dev, https://staging.p4inz.dev");

        let config = AppConfig::from_source(source_from(env)).unwrap();

        assert_eq!(config.github.token.unwrap().expose_secret(), "gh-token");
        assert_eq!(
            config.github.repositories,
            vec!["p4inz-code/p4inz".to_string(), "p4inz-code/website".to_string()]
        );
        assert_eq!(config.ai.provider.as_deref(), Some("online"));
        assert_eq!(config.ai.model.as_deref(), Some("llama"));
        assert_eq!(config.ai.api_key.unwrap().expose_secret(), "ai-secret");
        assert_eq!(config.ai.base_url.as_deref(), Some("https://api.example.com/v1"));
        assert_eq!(config.core.log_filter, "debug");
        assert_eq!(config.core.environment, Environment::Staging);
        assert_eq!(config.api.port, 9090);
        assert_eq!(
            config.api.allowed_origins,
            vec!["https://p4inz.dev".to_string(), "https://staging.p4inz.dev".to_string()]
        );
    }

    #[test]
    fn invalid_api_port_is_reported() {
        let mut env = full_valid_env();
        env.insert("API_PORT", "not-a-port");

        let err = AppConfig::from_source(source_from(env)).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { key: "API_PORT", .. }));
    }

    #[test]
    fn auth_config_is_configured_once_all_three_fields_are_present() {
        let mut env = full_valid_env();
        env.insert("DISCORD_CLIENT_SECRET", "discord-oauth-secret");
        env.insert("AUTH_REDIRECT_URI", "https://p4inz.dev/v1/auth/discord/callback");
        env.insert("AUTH_SESSION_SECRET", "session-signing-secret-that-is-long-enough");

        let config = AppConfig::from_source(source_from(env)).unwrap();

        assert!(config.auth.is_configured());
        assert_eq!(
            config.auth.discord_client_secret.unwrap().expose_secret(),
            "discord-oauth-secret"
        );
        assert_eq!(
            config.auth.redirect_uri.as_deref(),
            Some("https://p4inz.dev/v1/auth/discord/callback")
        );
    }

    #[test]
    fn auth_config_is_not_configured_when_partially_set() {
        let mut env = full_valid_env();
        env.insert("DISCORD_CLIENT_SECRET", "discord-oauth-secret");

        let config = AppConfig::from_source(source_from(env)).unwrap();

        assert!(!config.auth.is_configured());
    }

    #[test]
    fn admin_user_ids_default_to_empty() {
        let config = AppConfig::from_source(source_from(full_valid_env())).unwrap();
        assert!(config.auth.admin_user_ids.is_empty());
    }

    #[test]
    fn admin_user_ids_parses_a_comma_separated_list() {
        let mut env = full_valid_env();
        env.insert("ADMIN_USER_IDS", "111111111111111111, 222222222222222222");

        let config = AppConfig::from_source(source_from(env)).unwrap();

        assert_eq!(
            config.auth.admin_user_ids,
            vec!["111111111111111111".to_string(), "222222222222222222".to_string()]
        );
    }

    #[test]
    fn a_short_session_secret_is_rejected_regardless_of_environment() {
        let mut env = full_valid_env();
        env.insert("AUTH_SESSION_SECRET", "too-short");

        let err = AppConfig::from_source(source_from(env)).unwrap_err();

        assert!(matches!(err, ConfigError::Invalid { key: "AUTH_SESSION_SECRET", .. }));
    }

    #[test]
    fn a_long_enough_session_secret_is_accepted_outside_production() {
        let mut env = full_valid_env();
        env.insert("AUTH_SESSION_SECRET", LONG_ENOUGH_SESSION_SECRET);

        assert!(AppConfig::from_source(source_from(env)).is_ok());
    }

    #[test]
    fn production_rejects_a_plaintext_oauth_redirect_uri() {
        let mut env = full_valid_env();
        env.insert("P4INZ_ENV", "production");
        env.insert("AUTH_SESSION_SECRET", LONG_ENOUGH_SESSION_SECRET);
        env.insert("AUTH_REDIRECT_URI", "http://p4inz.dev/v1/auth/discord/callback");

        let err = AppConfig::from_source(source_from(env)).unwrap_err();

        assert!(matches!(err, ConfigError::Invalid { key: "AUTH_REDIRECT_URI", .. }));
    }

    #[test]
    fn a_plaintext_oauth_redirect_uri_is_allowed_outside_production() {
        let mut env = full_valid_env();
        env.insert("AUTH_SESSION_SECRET", LONG_ENOUGH_SESSION_SECRET);
        env.insert("AUTH_REDIRECT_URI", "http://localhost:8080/v1/auth/discord/callback");

        assert!(AppConfig::from_source(source_from(env)).is_ok());
    }

    #[test]
    fn production_rejects_a_plaintext_allowed_origin() {
        let mut env = full_valid_env();
        env.insert("P4INZ_ENV", "production");
        env.insert("API_ALLOWED_ORIGINS", "https://p4inz.dev, http://insecure.example.com");

        let err = AppConfig::from_source(source_from(env)).unwrap_err();

        assert!(matches!(err, ConfigError::Invalid { key: "API_ALLOWED_ORIGINS", .. }));
    }

    #[test]
    fn a_plaintext_allowed_origin_is_allowed_outside_production() {
        let mut env = full_valid_env();
        env.insert("API_ALLOWED_ORIGINS", "http://localhost:5173");

        assert!(AppConfig::from_source(source_from(env)).is_ok());
    }

    #[test]
    fn production_accepts_a_fully_hardened_configuration() {
        let mut env = full_valid_env();
        env.insert("P4INZ_ENV", "production");
        env.insert("AUTH_SESSION_SECRET", LONG_ENOUGH_SESSION_SECRET);
        env.insert("AUTH_REDIRECT_URI", "https://p4inz.dev/v1/auth/discord/callback");
        env.insert("API_ALLOWED_ORIGINS", "https://p4inz.dev");

        assert!(AppConfig::from_source(source_from(env)).is_ok());
    }
}
