use async_trait::async_trait;
use p4inz_errors::AppResult;

/// Executes the work for one [`crate::JobKind`]. Concrete implementations
/// (e.g. a GitHub-synchronization handler, Milestone 35) live outside this
/// crate — this trait exists purely so [`crate::JobHandlerRegistry`] and
/// the execution loop ([`crate::execute::process_next`]) can dispatch
/// generically, without knowing what any specific job kind actually does.
///
/// `#[async_trait]` (rather than RPITIT) is used here, not the RPITIT
/// pattern used elsewhere in the workspace, because handlers are stored as
/// `Box<dyn JobHandler>` in [`crate::JobHandlerRegistry`] — genuine dyn
/// dispatch is required, matching the same tradeoff already made for
/// `p4inz_discord::SlashCommand`.
#[async_trait]
pub trait JobHandler {
    /// Runs the job's work for `payload`. Must be safe to invoke more than
    /// once for the same logical job (`docs/development/
    /// implementation_plan.md` section 15: "Idempotent operations") — a
    /// crash after this succeeds but before the job is marked succeeded
    /// (Milestone 34/36 recovery scenarios) means it could run again.
    async fn handle(&self, payload: &str) -> AppResult<()>;
}
