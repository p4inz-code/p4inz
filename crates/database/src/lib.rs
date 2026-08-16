//! P4inz database subsystem.
//!
//! The PostgreSQL connection/pool foundation and migration runner. SQLx and
//! PostgreSQL types stay behind this crate's boundary — `domain` and
//! `application` must not depend on them directly
//! (`docs/architecture/dependency-rules.md`).
//!
//! This crate deliberately does not define any product schema or
//! repositories; it only establishes the mechanism (connecting, pooling,
//! running migrations, a health check) that later milestones build on.

mod error;
mod migrate;
mod pool;

pub use error::DatabaseError;
pub use migrate::run_migrations;
pub use pool::{PoolSettings, connect, health_check};

/// Re-exported so downstream crates can hold a pool without depending on
/// `sqlx` directly for this one type.
pub use sqlx::PgPool;
