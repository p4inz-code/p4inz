use std::collections::HashMap;

use serenity::http::Http;

use crate::command::SlashCommand;
use crate::error::DiscordError;

/// Maps slash-command names to their handlers and dispatches incoming
/// interactions by name.
#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn SlashCommand>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a command. Returns the previously registered command for
    /// the same name, if any — callers should treat that as a programming
    /// error (duplicate command names).
    pub fn insert(
        &mut self,
        command: impl SlashCommand + 'static,
    ) -> Option<Box<dyn SlashCommand>> {
        self.commands.insert(command.name().to_string(), Box::new(command))
    }

    pub fn get(&self, name: &str) -> Option<&dyn SlashCommand> {
        self.commands.get(name).map(|c| c.as_ref())
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Uploads every registered command's definition to Discord as global
    /// commands, replacing whatever set was previously registered.
    ///
    /// Global command updates can take up to an hour to propagate to all
    /// clients per Discord's own API behavior; per-guild registration
    /// (for fast iteration during development) is left to the composition
    /// root that wires a specific guild id, since this framework has no
    /// concept of "the current guild".
    pub async fn register_globally(&self, http: &Http) -> Result<(), DiscordError> {
        let definitions: Vec<_> = self.commands.values().map(|c| c.register()).collect();
        serenity::all::Command::set_global_commands(http, definitions)
            .await
            .map_err(DiscordError::Register)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serenity::all::{CommandInteraction, Context, CreateCommand};

    use super::*;

    struct Ping;

    #[async_trait]
    impl SlashCommand for Ping {
        fn name(&self) -> &str {
            "ping"
        }

        fn register(&self) -> CreateCommand {
            CreateCommand::new("ping").description("Replies with pong")
        }

        async fn execute(
            &self,
            _ctx: &Context,
            _interaction: &CommandInteraction,
        ) -> p4inz_errors::AppResult<()> {
            Ok(())
        }
    }

    #[test]
    fn starts_empty() {
        let registry = CommandRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn insert_and_get_round_trip_by_name() {
        let mut registry = CommandRegistry::new();
        registry.insert(Ping);

        assert_eq!(registry.len(), 1);
        assert!(registry.get("ping").is_some());
        assert!(registry.get("missing").is_none());
    }

    #[test]
    fn inserting_same_name_replaces_and_returns_previous() {
        let mut registry = CommandRegistry::new();
        assert!(registry.insert(Ping).is_none());
        assert!(registry.insert(Ping).is_some());
        assert_eq!(registry.len(), 1);
    }
}
