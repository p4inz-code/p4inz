use async_trait::async_trait;
use p4inz_errors::AppResult;
use serenity::all::{CommandInteraction, Context, CreateCommand};

/// A single slash command's definition and handler.
///
/// No concrete commands are defined by this crate — this milestone is the
/// dispatch *framework* only. Product-specific commands (e.g. information
/// lookups) belong to their own later milestones.
///
/// Uses `#[async_trait]` rather than native `async fn` in trait (as
/// `p4inz_application::ProjectRepository` does) because commands are
/// looked up by name at runtime and stored as `Box<dyn SlashCommand>` in
/// [`crate::CommandRegistry`] — that dyn dispatch requires an object-safe
/// trait, which native `async fn` in traits does not provide.
///
/// `execute` returns `AppResult<()>` (not a Discord-specific error) so a
/// single [`crate::error_ux::describe`] can render failures consistently
/// across slash commands and natural-language questions alike.
#[async_trait]
pub trait SlashCommand: Send + Sync {
    /// The command's name, exactly as registered with Discord.
    fn name(&self) -> &str;

    /// Describes this command for Discord's command-registration API.
    fn register(&self) -> CreateCommand;

    /// Handles an invocation of this command.
    async fn execute(&self, ctx: &Context, interaction: &CommandInteraction) -> AppResult<()>;
}
