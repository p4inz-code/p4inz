use p4inz_application::QuestionHandler;
use p4inz_config::DiscordConfig;
use p4inz_security::RateLimiter;
use serenity::all::GatewayIntents;
use serenity::client::Client;

use crate::error::DiscordError;
use crate::handler::Handler;
use crate::permissions::GuildRoleMapping;
use crate::registry::CommandRegistry;

/// Intents for gateway connectivity, guild lifecycle awareness, and
/// reading messages so natural-language questions can be answered
/// (`docs/PROJECT_SPEC.md` section 2/8).
///
/// `MESSAGE_CONTENT` is a privileged intent Discord requires separate
/// approval for once a bot is in 100+ guilds — this is the minimum needed
/// for the Natural Language milestone's requirement, not requested
/// speculatively.
pub fn default_intents() -> GatewayIntents {
    GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
}

/// Builds a Discord gateway client, ready to [`run`].
///
/// Building performs no network I/O and does not validate the token — it
/// only constructs the client and wires the event handler with
/// `registry`, `question_handler`, `question_rate_limiter` and
/// `guild_role_mapping`. Authentication and connection (and `serenity`'s
/// own automatic reconnect/backoff) happen in [`run`]. Registering
/// `registry`'s commands with Discord's API is separate — see
/// [`CommandRegistry::register_globally`].
pub async fn build<Q: QuestionHandler + Send + Sync + 'static>(
    config: &DiscordConfig,
    registry: CommandRegistry,
    question_handler: Q,
    question_rate_limiter: RateLimiter,
    guild_role_mapping: GuildRoleMapping,
) -> Result<Client, DiscordError> {
    Client::builder(config.token.expose_secret(), default_intents())
        .event_handler(Handler::new(
            registry,
            question_handler,
            question_rate_limiter,
            guild_role_mapping,
        ))
        .await
        .map_err(DiscordError::Build)
}

/// Runs the client until it shuts down or hits a fatal error.
///
/// Reconnection after transient disconnects is handled internally by
/// `serenity` (bounded retry/backoff); this only surfaces genuinely fatal
/// failures (e.g. an invalid token) as a typed [`DiscordError`].
pub async fn run(client: &mut Client) -> Result<(), DiscordError> {
    client.start().await.map_err(DiscordError::Run)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use p4inz_application::UnavailableQuestionHandler;
    use p4inz_common::Secret;
    use p4inz_security::RateLimiterConfig;

    use super::*;

    #[test]
    fn default_intents_includes_message_content() {
        let intents = default_intents();
        assert!(intents.contains(GatewayIntents::GUILDS));
        assert!(intents.contains(GatewayIntents::MESSAGE_CONTENT));
        assert!(intents.contains(GatewayIntents::GUILD_MESSAGES));
        assert!(intents.contains(GatewayIntents::DIRECT_MESSAGES));
    }

    #[tokio::test]
    async fn build_succeeds_without_network_or_a_valid_token() {
        let config = DiscordConfig {
            token: Secret::new("not-a-real-token"),
            application_id: "0".to_string(),
        };

        let result = build(
            &config,
            CommandRegistry::new(),
            UnavailableQuestionHandler,
            RateLimiter::new(RateLimiterConfig::default()),
            GuildRoleMapping::default(),
        )
        .await;
        assert!(result.is_ok());
    }

    /// Requires a real, valid `DISCORD_TOKEN` and network access to
    /// Discord's gateway. Not run by default — this environment has no
    /// Discord bot token configured. Run explicitly with
    /// `cargo test -p p4inz-discord -- --ignored` against a real bot.
    #[tokio::test]
    #[ignore = "requires a real Discord bot token; see doc comment"]
    async fn connects_to_the_real_gateway() {
        let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");
        let config = DiscordConfig { token: Secret::new(token), application_id: "0".to_string() };

        let mut client = build(
            &config,
            CommandRegistry::new(),
            UnavailableQuestionHandler,
            RateLimiter::new(RateLimiterConfig::default()),
            GuildRoleMapping::default(),
        )
        .await
        .unwrap();
        let outcome = tokio::time::timeout(Duration::from_secs(30), run(&mut client)).await;
        assert!(outcome.is_ok(), "did not connect within 30s");
    }
}
