use thiserror::Error;

/// Errors from building or running the Discord gateway client.
///
/// Each variant carries a fixed, generic message and preserves the
/// underlying `serenity::Error` only as
/// [`source`](std::error::Error::source) — never interpolated into the
/// top-level [`Display`](std::fmt::Display) text — so the bot token is
/// never echoed through this type's own message.
#[derive(Debug, Error)]
pub enum DiscordError {
    #[error("failed to build the Discord client")]
    Build(#[source] serenity::Error),

    #[error("the Discord client encountered a fatal error")]
    Run(#[source] serenity::Error),

    #[error("failed to register slash commands with Discord")]
    Register(#[source] serenity::Error),
}
