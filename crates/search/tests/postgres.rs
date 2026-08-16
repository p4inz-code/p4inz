//! Integration tests against a real PostgreSQL instance.
//!
//! Not run by default (`cargo test --workspace`) — this environment has no
//! PostgreSQL server available. Run explicitly against a disposable
//! database with:
//!
//! ```text
//! DATABASE_URL=postgres://... cargo test -p p4inz-search -- --ignored
//! ```
//!
//! Each test creates its own knowledge item (a fresh random id) and
//! deletes it afterward, so tests can run repeatedly against the same
//! database without accumulating rows.

use std::time::SystemTime;

use p4inz_domain::Link;
use p4inz_knowledge::{
    Body, KnowledgeCategory, KnowledgeItem, KnowledgeItemId, KnowledgeRepository, PublicationState,
    Source, SourceKind, Title,
};
use p4inz_search::{PgKnowledgeRepository, search_published};

async fn connected_repository() -> (PgKnowledgeRepository, sqlx::PgPool) {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(&url).await.unwrap();
    p4inz_database::run_migrations(&pool).await.unwrap();
    (PgKnowledgeRepository::new(pool.clone()), pool)
}

fn published_item(id: KnowledgeItemId, reference: Option<Link>) -> KnowledgeItem {
    let now = SystemTime::now();
    KnowledgeItem::new(
        id,
        KnowledgeCategory::Community,
        Title::parse("Support").unwrap(),
        Body::parse("Contact us in #support.").unwrap(),
        Source::new(SourceKind::Administrator, reference),
        now,
    )
    .transition_to(PublicationState::Review, now)
    .unwrap()
    .transition_to(PublicationState::Published, now)
    .unwrap()
}

async fn cleanup(pool: &sqlx::PgPool, id: KnowledgeItemId) {
    sqlx::query("DELETE FROM knowledge_items WHERE id = $1")
        .bind(id.into_uuid())
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a live PostgreSQL instance; see module doc comment"]
async fn save_and_find_by_id_round_trips() {
    let (repository, pool) = connected_repository().await;
    let item = published_item(KnowledgeItemId::new(), None);

    repository.save(&item).await.unwrap();
    let found = repository.find_by_id(item.id()).await.unwrap();

    assert_eq!(found, Some(item.clone()));
    cleanup(&pool, item.id()).await;
}

#[tokio::test]
#[ignore = "requires a live PostgreSQL instance; see module doc comment"]
async fn find_by_source_reference_correlates_by_link() {
    let (repository, pool) = connected_repository().await;
    let link = Link::parse("https://github.com/p4inz-code/p4inz-test-fixture").unwrap();
    let item = published_item(KnowledgeItemId::new(), Some(link.clone()));

    repository.save(&item).await.unwrap();
    let found = repository.find_by_source_reference(link.as_str()).await.unwrap();

    assert_eq!(found.map(|i| i.id()), Some(item.id()));
    cleanup(&pool, item.id()).await;
}

#[tokio::test]
#[ignore = "requires a live PostgreSQL instance; see module doc comment"]
async fn search_finds_published_items_by_text() {
    let (repository, pool) = connected_repository().await;
    let id = KnowledgeItemId::new();
    let now = SystemTime::now();
    let item = KnowledgeItem::new(
        id,
        KnowledgeCategory::Community,
        Title::parse("Zylophone Support Policy").unwrap(),
        Body::parse("Details about zylophone-related support requests.").unwrap(),
        Source::new(SourceKind::Administrator, None),
        now,
    )
    .transition_to(PublicationState::Review, now)
    .unwrap()
    .transition_to(PublicationState::Published, now)
    .unwrap();

    repository.save(&item).await.unwrap();
    let results = search_published(&pool, "zylophone", 10).await.unwrap();

    assert!(results.iter().any(|found| found.id() == id));
    cleanup(&pool, id).await;
}

#[tokio::test]
#[ignore = "requires a live PostgreSQL instance; see module doc comment"]
async fn search_excludes_unpublished_items() {
    let (repository, pool) = connected_repository().await;
    let id = KnowledgeItemId::new();
    let item = KnowledgeItem::new(
        id,
        KnowledgeCategory::Community,
        Title::parse("Unpublished Wibblefrotz Draft").unwrap(),
        Body::parse("This draft should never appear in search.").unwrap(),
        Source::new(SourceKind::Administrator, None),
        SystemTime::now(),
    );

    repository.save(&item).await.unwrap();
    let results = search_published(&pool, "wibblefrotz", 10).await.unwrap();

    assert!(results.iter().all(|found| found.id() != id));
    cleanup(&pool, id).await;
}
