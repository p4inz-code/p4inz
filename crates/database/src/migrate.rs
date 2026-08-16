use sqlx::postgres::PgPool;

use crate::error::DatabaseError;

/// Applies any pending migrations from the workspace `migrations/`
/// directory (see `migrations/README.md` for the naming convention).
///
/// Safe to call on every startup: SQLx tracks applied migrations in its own
/// `_sqlx_migrations` table and only runs new ones. This never runs
/// destructive SQL of its own — it only executes whatever has been
/// deliberately checked into `migrations/`.
pub async fn run_migrations(pool: &PgPool) -> Result<(), DatabaseError> {
    sqlx::migrate!("../../migrations").run(pool).await.map_err(DatabaseError::Migrate)
}
