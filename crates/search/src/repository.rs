use p4inz_application::KnowledgeSearch;
use p4inz_errors::{AppResult, ErrorKind, IntoAppError};
use p4inz_knowledge::{KnowledgeItem, KnowledgeItemId, KnowledgeRepository};
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::row::row_to_item;
use crate::search::search_published;

/// A PostgreSQL-backed [`KnowledgeRepository`]
/// (`docs/architecture/dependency-rules.md`: infrastructure implements
/// contracts required by application/domain; `p4inz-knowledge` defines
/// this one directly since it plays the same "owning" role for the
/// knowledge subsystem that `p4inz-domain`/`p4inz-application` play for
/// `Project`).
pub struct PgKnowledgeRepository {
    pool: PgPool,
}

impl PgKnowledgeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl KnowledgeRepository for PgKnowledgeRepository {
    async fn save(&self, item: &KnowledgeItem) -> AppResult<()> {
        let provenance = item.provenance();

        sqlx::query(
            r#"
            INSERT INTO knowledge_items (
                id, category, title, body, source_kind, source_reference,
                publication_state, version, created_at, updated_at, synchronized_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE SET
                category = EXCLUDED.category,
                title = EXCLUDED.title,
                body = EXCLUDED.body,
                source_kind = EXCLUDED.source_kind,
                source_reference = EXCLUDED.source_reference,
                publication_state = EXCLUDED.publication_state,
                version = EXCLUDED.version,
                updated_at = EXCLUDED.updated_at,
                synchronized_at = EXCLUDED.synchronized_at
            "#,
        )
        .bind(item.id().into_uuid())
        .bind(item.category().as_str())
        .bind(item.title().as_str())
        .bind(item.body().as_str())
        .bind(item.source().kind().as_str())
        .bind(item.source().reference().map(|link| link.as_str()))
        .bind(item.state().as_str())
        .bind(provenance.version().as_u32() as i32)
        .bind(OffsetDateTime::from(provenance.created_at()))
        .bind(OffsetDateTime::from(provenance.updated_at()))
        .bind(provenance.synchronized_at().map(OffsetDateTime::from))
        .execute(&self.pool)
        .await
        .into_app_error(ErrorKind::Internal, "failed to save knowledge item")?;

        Ok(())
    }

    async fn find_by_id(&self, id: KnowledgeItemId) -> AppResult<Option<KnowledgeItem>> {
        let row = sqlx::query("SELECT * FROM knowledge_items WHERE id = $1")
            .bind(id.into_uuid())
            .fetch_optional(&self.pool)
            .await
            .into_app_error(ErrorKind::Internal, "failed to look up knowledge item by id")?;

        row.as_ref()
            .map(row_to_item)
            .transpose()
            .into_app_error(ErrorKind::Internal, "stored knowledge item is invalid")
    }

    async fn find_by_source_reference(
        &self,
        source_reference: &str,
    ) -> AppResult<Option<KnowledgeItem>> {
        let row = sqlx::query("SELECT * FROM knowledge_items WHERE source_reference = $1")
            .bind(source_reference)
            .fetch_optional(&self.pool)
            .await
            .into_app_error(
                ErrorKind::Internal,
                "failed to look up knowledge item by source reference",
            )?;

        row.as_ref()
            .map(row_to_item)
            .transpose()
            .into_app_error(ErrorKind::Internal, "stored knowledge item is invalid")
    }
}

impl KnowledgeSearch for PgKnowledgeRepository {
    async fn search(&self, query: &str, limit: u32) -> AppResult<Vec<KnowledgeItem>> {
        search_published(&self.pool, query, limit).await
    }
}
