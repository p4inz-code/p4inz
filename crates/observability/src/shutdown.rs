//! Graceful shutdown signal handling (`docs/development/
//! implementation_plan.md` Milestone 52: "Local/self-hosted deployment").
//!
//! A self-hosted deployment supervisor such as systemd (see
//! `infra/deployment/production`) stops a service by sending `SIGTERM`, not
//! `SIGINT`/Ctrl-C. `tokio::signal::ctrl_c` alone never observes `SIGTERM`
//! on Unix, so code that only awaited `ctrl_c()` never ran its graceful
//! shutdown path under `systemctl stop` — the process would only stop once
//! systemd's `TimeoutStopSec` elapsed and sent `SIGKILL`, skipping graceful
//! shutdown (and, for the worker, `p4inz_jobs::run_worker_with`'s guarantee
//! that an in-flight job finishes before stopping) entirely.

use std::io;

/// Resolves once the process receives a shutdown signal appropriate for its
/// platform — `SIGINT` or `SIGTERM` on Unix, Ctrl-C on Windows (which has
/// no direct `SIGTERM` equivalent commonly sent by a process supervisor;
/// this project's self-hosted deployment target is Linux/systemd, see
/// `infra/deployment/production`).
///
/// Returns `io::Result<()>` — matching [`tokio::signal::ctrl_c`]'s own
/// signature — so it's a drop-in replacement anywhere that already expects
/// a fallible shutdown future (`p4inz_jobs::run_worker_with`,
/// `p4inz_infrastructure::jobs::run_scheduler`).
pub async fn wait_for_shutdown_signal() -> io::Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = signal(SignalKind::terminate())?;

        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await
    }
}
