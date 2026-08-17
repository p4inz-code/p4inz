//! P4inz infrastructure subsystem.
//!
//! External integrations, kept behind the ports domain/application/
//! knowledge/jobs define — nothing here is depended on by those crates
//! (`docs/architecture/dependency-rules.md`: "Infrastructure implements
//! contracts required by application/domain").
//!
//! - [`github::GitHubSourceAdapter`]: a `p4inz_knowledge::SourceAdapter`
//!   implementation for GitHub-sourced knowledge (Milestone 19).
//! - [`jobs::PgJobRepository`]: a `p4inz_jobs::JobRepository`
//!   implementation backed by PostgreSQL (Milestone 33).
//! - [`discord_oauth::DiscordOAuthClient`][]: the "Sign in with Discord"
//!   OAuth2 exchange for web/admin authentication (Milestone 40).

mod discord_oauth;
pub mod github;
pub mod jobs;

pub use discord_oauth::{DiscordIdentity, DiscordOAuthClient};
