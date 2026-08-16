use std::future::Future;
use std::io;

/// Runs the worker process until `shutdown` resolves, then returns.
///
/// This is the worker's process lifecycle only — start, run, graceful
/// stop. No jobs are scheduled or executed here
/// (`docs/development/implementation_plan.md` Milestone 33: Job System;
/// `docs/architecture/runtime.md`: the worker process is responsible for
/// synchronization/indexing/embeddings/scheduled tasks/retries/
/// reconciliation, none of which exist yet). This exists so the worker
/// binary has a real, graceful start/stop shape for later milestones to
/// plug job scheduling into, rather than inventing it themselves.
///
/// `shutdown` is injected (rather than this function calling
/// [`tokio::signal::ctrl_c`] directly) so the lifecycle is testable
/// without needing to send the process a real signal — see
/// [`run_until_shutdown`] for the production entry point, which supplies
/// `tokio::signal::ctrl_c()`.
pub async fn run_until_shutdown_with<F>(shutdown: F) -> io::Result<()>
where
    F: Future<Output = io::Result<()>>,
{
    tracing::info!("worker started");
    shutdown.await?;
    tracing::info!("worker shutting down");
    Ok(())
}

/// Runs the worker process until an external shutdown signal (Ctrl+C /
/// SIGINT) is received, then returns.
pub async fn run_until_shutdown() -> io::Result<()> {
    run_until_shutdown_with(tokio::signal::ctrl_c()).await
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[tokio::test]
    async fn returns_once_the_shutdown_future_resolves() {
        let result = run_until_shutdown_with(async { Ok(()) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn propagates_a_failing_shutdown_signal() {
        let result =
            run_until_shutdown_with(async { Err(io::Error::other("signal handler failed")) }).await;
        assert!(result.is_err());
    }
}
