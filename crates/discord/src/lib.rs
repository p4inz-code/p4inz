//! P4inz discord subsystem.
//!
//! The Discord gateway connection/reconnect lifecycle (Milestone 11),
//! slash-command dispatch framework (Milestone 12), natural-language
//! question pipeline (Milestone 13), guild-role-to-permission mapping
//! (Milestone 14), and consistent failure UX (Milestone 15).
//! `serenity` types stay behind this crate's boundary; `domain` and
//! `application` must not depend on them
//! (`docs/architecture/dependency-rules.md`).
//!
//! No concrete slash commands, real question-answering logic, or
//! per-guild configuration storage are defined here — only the frameworks
//! ([`SlashCommand`], [`CommandRegistry`], [`GuildRoleMapping`],
//! [`error_ux::describe`]).

mod client;
mod command;
mod error;
pub mod error_ux;
mod handler;
mod permissions;
mod registry;

pub use client::{build, default_intents, run};
pub use command::SlashCommand;
pub use error::DiscordError;
pub use permissions::GuildRoleMapping;
pub use registry::CommandRegistry;
