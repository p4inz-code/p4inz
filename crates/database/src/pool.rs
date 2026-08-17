use std::time::Duration;

use p4inz_config::DatabaseConfig;
use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::error::DatabaseError;

/// Connection pool tuning.
///
/// Defaults are conservative and suitable for a self-hosted single-instance
/// deployment (`docs/architecture/zero-cost.md`); override for
/// higher-throughput production use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolSettings {
    pub max_connections: u32,
    pub acquire_timeout: Duration,
}

impl Default for PoolSettings {
    fn default() -> Self {
        Self { max_connections: 5, acquire_timeout: Duration::from_secs(10) }
    }
}

/// Opens a PostgreSQL connection pool.
///
/// Never logs `config.url` — it may contain credentials. On failure, the
/// underlying `sqlx::Error` is preserved only as [`DatabaseError::Connect`]'s
/// source, not interpolated into the returned error's message.
pub async fn connect(
    config: &DatabaseConfig,
    settings: PoolSettings,
) -> Result<PgPool, DatabaseError> {
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .acquire_timeout(settings.acquire_timeout)
        .connect(config.url.expose_secret())
        .await
        .map_err(DatabaseError::Connect)
}

/// Builds a pool without performing any I/O — connecting is deferred to
/// first actual use. For callers that need a real `PgPool` *value* to
/// satisfy a type signature without a live PostgreSQL instance (e.g. a
/// test exercising HTTP routing that never itself queries the database);
/// production code should use [`connect`] instead, which fails fast if
/// the database is unreachable rather than deferring that discovery.
pub fn connect_lazy(
    config: &DatabaseConfig,
    settings: PoolSettings,
) -> Result<PgPool, DatabaseError> {
    PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .acquire_timeout(settings.acquire_timeout)
        .connect_lazy(config.url.expose_secret())
        .map_err(DatabaseError::Connect)
}

/// Verifies the pool can reach the database.
///
/// Intended for readiness/health checks (`docs/development/
/// implementation_plan.md` section 16, "Health checks"), not as a general
/// query mechanism.
pub async fn health_check(pool: &PgPool) -> Result<(), DatabaseError> {
    sqlx::query("SELECT 1").execute(pool).await.map(|_| ()).map_err(DatabaseError::Query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_conservative() {
        let settings = PoolSettings::default();
        assert_eq!(settings.max_connections, 5);
        assert_eq!(settings.acquire_timeout, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn connect_lazy_succeeds_without_a_reachable_database() {
        let config = DatabaseConfig {
            url: p4inz_common::Secret::new("postgres://user:pass@localhost/p4inz"),
        };
        let result = connect_lazy(&config, PoolSettings::default());
        assert!(result.is_ok());
    }

    /// Requires a real, reachable PostgreSQL instance at `DATABASE_URL`.
    /// Not run by default (`cargo test --workspace`) — this environment has
    /// no PostgreSQL server available. Run explicitly with
    /// `cargo test -p p4inz-database -- --ignored` against a real database.
    #[tokio::test]
    #[ignore = "requires a live PostgreSQL instance; see doc comment"]
    async fn connects_and_passes_health_check() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
        let config = DatabaseConfig { url: p4inz_common::Secret::new(url) };

        let pool = connect(&config, PoolSettings::default()).await.unwrap();
        health_check(&pool).await.unwrap();
    }
}
