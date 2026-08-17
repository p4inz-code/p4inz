use std::time::Duration;

use p4inz_config::AppConfig;
use p4inz_database::{PoolSettings, connect, run_migrations};
use p4inz_infrastructure::github::GitHubSourceAdapter;
use p4inz_infrastructure::jobs::{
    GITHUB_SYNC_JOB_KIND, GitHubSyncHandler, PgJobRepository, run_scheduler,
};
use p4inz_jobs::JobHandlerRegistry;
use p4inz_knowledge::KnowledgeCategory;
use p4inz_search::PgKnowledgeRepository;

/// How often the worker checks for due jobs while idle. Not configurable
/// yet — no concrete operational need for that has arisen
/// (`docs/development/implementation_plan.md` section 1: avoid
/// speculative infrastructure).
const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// How often the GitHub sync scheduler re-enqueues a sync job for each
/// configured repository (Milestone 35: "Scheduled GitHub
/// synchronization"). Six hours keeps GitHub content reasonably fresh
/// without meaningfully risking API rate limits, even unauthenticated.
const GITHUB_SYNC_TICK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

#[tokio::main]
async fn main() {
    // Structured logging (Milestone 51) is installed first, before
    // anything else runs, so even startup failures below are captured as
    // structured log events rather than bypassing observability entirely.
    p4inz_observability::logging::init();

    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(%error, "failed to load configuration");
            std::process::exit(1);
        }
    };

    let pool = match connect(&config.database, PoolSettings::default()).await {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!(%error, "failed to connect to the database");
            std::process::exit(1);
        }
    };

    if let Err(error) = run_migrations(&pool).await {
        tracing::error!(%error, "failed to run database migrations");
        std::process::exit(1);
    }

    let job_repository = PgJobRepository::new(pool.clone());

    let mut registry = JobHandlerRegistry::new();
    let github_adapter = match GitHubSourceAdapter::new(config.github.token.clone()) {
        Ok(adapter) => adapter,
        Err(error) => {
            tracing::error!(%error, "failed to build the GitHub source adapter");
            std::process::exit(1);
        }
    };
    registry.insert(
        GITHUB_SYNC_JOB_KIND,
        GitHubSyncHandler::new(github_adapter, PgKnowledgeRepository::new(pool)),
    );

    // SIGINT or SIGTERM (Milestone 52: self-hosted deployment supervisors
    // like systemd stop a service with SIGTERM, which plain `ctrl_c()`
    // never observes on Unix).
    let worker = p4inz_jobs::run_worker_with(
        &job_repository,
        &registry,
        POLL_INTERVAL,
        p4inz_observability::shutdown::wait_for_shutdown_signal(),
    );
    let scheduler = run_scheduler(
        &job_repository,
        &config.github.repositories,
        KnowledgeCategory::Projects,
        GITHUB_SYNC_TICK_INTERVAL,
        p4inz_observability::shutdown::wait_for_shutdown_signal(),
    );

    let (worker_result, scheduler_result) = tokio::join!(worker, scheduler);

    if let Err(error) = worker_result {
        tracing::error!(%error, "worker shutdown signal failed");
        std::process::exit(1);
    }
    if let Err(error) = scheduler_result {
        tracing::error!(%error, "GitHub sync scheduler shutdown signal failed");
        std::process::exit(1);
    }
}
