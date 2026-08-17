//! P4inz configuration.
//!
//! Typed, environment-based configuration for the runtime settings already
//! declared in `.env.example`. Loading fails closed: a missing required
//! variable or an invalid value produces a [`ConfigError`], never a guessed
//! default. Secret values (tokens) are wrapped in [`p4inz_common::Secret`]
//! so they can't be accidentally logged or printed.
//!
//! Infrastructure-specific concerns (opening a database connection,
//! starting the Discord gateway, etc.) are out of scope here — this crate
//! only produces the validated settings those subsystems will consume.

mod app_config;
mod environment;
mod error;

pub use app_config::{
    AiConfig, ApiConfig, AppConfig, AuthConfig, CoreConfig, DatabaseConfig, DiscordConfig,
    GitHubConfig,
};
pub use environment::Environment;
pub use error::ConfigError;
