use std::time::SystemTime;

use p4inz_domain::{Link, LinkError};
use p4inz_knowledge::{
    Body, BodyError, KnowledgeCategory, KnowledgeCategoryError, KnowledgeItem, KnowledgeItemId,
    Provenance, PublicationState, PublicationStateError, Source, SourceKind, SourceKindError,
    Title, TitleError, Version, VersionError,
};
use sqlx::Row;
use sqlx::postgres::PgRow;
use thiserror::Error;
use time::OffsetDateTime;

/// A stored row failed to reconstruct into a valid [`KnowledgeItem`].
///
/// This should only ever happen if the database contains data written by
/// something other than this crate's own [`crate::PgKnowledgeRepository`]
/// (e.g. a manual `UPDATE`) — every value this crate itself writes was
/// already validated before being stored.
#[derive(Debug, Error)]
pub enum RowMappingError {
    #[error("failed to read column '{column}'")]
    Column {
        column: &'static str,
        #[source]
        source: sqlx::Error,
    },
    #[error(transparent)]
    Category(#[from] KnowledgeCategoryError),
    #[error(transparent)]
    SourceKind(#[from] SourceKindError),
    #[error(transparent)]
    PublicationState(#[from] PublicationStateError),
    #[error(transparent)]
    Title(#[from] TitleError),
    #[error(transparent)]
    Body(#[from] BodyError),
    #[error(transparent)]
    Link(#[from] LinkError),
    #[error(transparent)]
    Version(#[from] VersionError),
}

pub(crate) fn get<'r, T: sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>>(
    row: &'r PgRow,
    column: &'static str,
) -> Result<T, RowMappingError> {
    row.try_get(column).map_err(|source| RowMappingError::Column { column, source })
}

/// Reconstructs a [`KnowledgeItem`] from a `knowledge_items` row (see
/// `migrations/0001_knowledge_items.sql`).
pub fn row_to_item(row: &PgRow) -> Result<KnowledgeItem, RowMappingError> {
    let id: uuid::Uuid = get(row, "id")?;
    let category = KnowledgeCategory::parse(&get::<String>(row, "category")?)?;
    let title = Title::parse(get::<String>(row, "title")?)?;
    let body = Body::parse(get::<String>(row, "body")?)?;
    let source_kind = SourceKind::parse(&get::<String>(row, "source_kind")?)?;
    let source_reference: Option<String> = get(row, "source_reference")?;
    let source_reference = source_reference.map(Link::parse).transpose()?;
    let state = PublicationState::parse(&get::<String>(row, "publication_state")?)?;
    let version = Version::from_u32(get::<i32>(row, "version")? as u32)?;
    let created_at: SystemTime = SystemTime::from(get::<OffsetDateTime>(row, "created_at")?);
    let updated_at: SystemTime = SystemTime::from(get::<OffsetDateTime>(row, "updated_at")?);
    let synchronized_at: Option<OffsetDateTime> = get(row, "synchronized_at")?;
    let synchronized_at = synchronized_at.map(SystemTime::from);

    Ok(KnowledgeItem::from_parts(
        KnowledgeItemId::from_uuid(id),
        category,
        title,
        body,
        Source::new(source_kind, source_reference),
        state,
        Provenance::from_parts(version, created_at, updated_at, synchronized_at),
    ))
}
