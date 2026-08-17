//! Migration correctness (`docs/development/implementation_plan.md`
//! Milestone 60: Database Tests — "Migration/repository correctness").
//!
//! Requires a live PostgreSQL instance — not run by `cargo test
//! --workspace` (see each test's `#[ignore]`), matching the convention
//! `crates/database/src/pool.rs`'s own `connects_and_passes_health_check`
//! already established. Run explicitly against a real, disposable
//! database:
//!
//! ```text
//! DATABASE_URL=postgres://user:pass@localhost/p4inz_test \
//!     cargo test -p p4inz-database -- --ignored
//! ```

use p4inz_common::Secret;
use p4inz_config::DatabaseConfig;
use p4inz_database::{PoolSettings, connect, run_migrations};

fn database_url() -> String {
    std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test")
}

/// `run_migrations`' doc comment claims it's "safe to call on every
/// startup" — every binary in this workspace (`apps/p4inz`,
/// `apps/p4inz-worker`) relies on that being true, since both call it
/// unconditionally at every startup, not just on a fresh database. This
/// is the only test that actually exercises that claim against a real
/// database rather than trusting the comment.
#[tokio::test]
#[ignore = "requires a live PostgreSQL instance; see module doc comment"]
async fn migrations_apply_cleanly_and_are_idempotent() {
    let config = DatabaseConfig { url: Secret::new(database_url()) };
    let pool = connect(&config, PoolSettings::default()).await.unwrap();

    run_migrations(&pool).await.unwrap();
    // A second call must be a safe no-op, not an error or a re-run of
    // already-applied migrations.
    run_migrations(&pool).await.unwrap();

    let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE success")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(applied, 3, "expected exactly the 3 migrations checked into migrations/");
}

/// Verifies the migrated schema actually has the shape every repository
/// implementation (`p4inz_search::PgKnowledgeRepository`,
/// `p4inz_infrastructure::jobs::PgJobRepository`) assumes when it reads
/// and writes specific column names — catching a migration/repository
/// drift (a renamed or dropped column the Rust code still expects) that
/// no purely in-memory test could.
#[tokio::test]
#[ignore = "requires a live PostgreSQL instance; see module doc comment"]
async fn migrated_schema_has_the_columns_repositories_depend_on() {
    let config = DatabaseConfig { url: Secret::new(database_url()) };
    let pool = connect(&config, PoolSettings::default()).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let columns_of = |table: &str| {
        let table = table.to_string();
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, String>(
                "SELECT column_name FROM information_schema.columns WHERE table_name = $1",
            )
            .bind(&table)
            .fetch_all(&pool)
            .await
            .unwrap()
        }
    };

    // knowledge_items (migrations/0001_knowledge_items.sql).
    let knowledge_columns = columns_of("knowledge_items").await;
    for expected in [
        "id",
        "category",
        "title",
        "body",
        "source_kind",
        "source_reference",
        "publication_state",
        "version",
        "created_at",
        "updated_at",
        "synchronized_at",
    ] {
        assert!(
            knowledge_columns.iter().any(|c| c == expected),
            "knowledge_items is missing expected column '{expected}'"
        );
    }

    // jobs (migrations/0002_jobs.sql, migrations/0003_job_retries.sql).
    let job_columns = columns_of("jobs").await;
    for expected in [
        "id",
        "kind",
        "payload",
        "status",
        "run_at",
        "created_at",
        "updated_at",
        "attempts",
        "max_attempts",
        "last_error",
    ] {
        assert!(
            job_columns.iter().any(|c| c == expected),
            "jobs is missing expected column '{expected}'"
        );
    }
}
