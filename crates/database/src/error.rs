use thiserror::Error;

/// Errors from establishing or using a database connection.
///
/// Each variant carries a fixed, generic message and preserves the
/// underlying `sqlx` error only as [`source`](std::error::Error::source) —
/// never interpolated into the top-level [`Display`](std::fmt::Display)
/// text — so a connection string embedded in credentials is never echoed
/// through this type's own message.
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("failed to connect to the database")]
    Connect(#[source] sqlx::Error),

    #[error("database migration failed")]
    Migrate(#[source] sqlx::migrate::MigrateError),

    #[error("database query failed")]
    Query(#[source] sqlx::Error),
}
