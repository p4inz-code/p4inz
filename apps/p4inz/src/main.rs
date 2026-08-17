use std::net::SocketAddr;

use p4inz_api::{ApiState, AuthState, build_router};
use p4inz_config::AppConfig;
use p4inz_database::{PoolSettings, connect, run_migrations};
use p4inz_infrastructure::DiscordOAuthClient;

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

    let mut state = ApiState::new(pool);
    if config.auth.is_configured() {
        // `is_configured` already confirmed all three are `Some`; the
        // `unwrap`s here just extract what's already been checked.
        let oauth = DiscordOAuthClient::new(
            config.discord.application_id.clone(),
            config.auth.discord_client_secret.clone().unwrap(),
            config.auth.redirect_uri.clone().unwrap(),
        );
        match oauth {
            Ok(oauth) => {
                state = state.with_auth(AuthState {
                    oauth,
                    session_secret: config.auth.session_secret.clone().unwrap(),
                    admin_user_ids: config.auth.admin_user_ids.clone(),
                });
            }
            Err(error) => {
                tracing::error!(%error, "failed to build the Discord OAuth client");
                std::process::exit(1);
            }
        }
    }

    let router = build_router(state, &config.api.allowed_origins);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.api.port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%addr, %error, "failed to bind API listener");
            std::process::exit(1);
        }
    };

    tracing::info!(%addr, "P4inz API listening");

    // `with_connect_info` makes the real client IP available to the
    // per-IP rate limiter (Milestone 42) via Axum's `ConnectInfo`
    // extractor.
    if let Err(error) =
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async {
                // SIGINT or SIGTERM (Milestone 52: self-hosted deployment
                // supervisors like systemd stop a service with SIGTERM).
                let _ = p4inz_observability::shutdown::wait_for_shutdown_signal().await;
            })
            .await
    {
        tracing::error!(%error, "API server failed");
        std::process::exit(1);
    }
}
