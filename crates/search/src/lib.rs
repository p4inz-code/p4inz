//! P4inz search subsystem.
//!
//! Retrieval and ranking (`docs/architecture/overview.md`).
//! - [`PgKnowledgeRepository`]: the first concrete
//!   `p4inz_knowledge::KnowledgeRepository` implementation, backed by the
//!   `knowledge_items` table (`migrations/0001_knowledge_items.sql`,
//!   Milestone 21).
//! - [`search_published`]: full-text search over published items, ranked
//!   by [`rank::combined_score`] — text relevance, source authority and
//!   freshness together (Milestone 22).
//!
//! SQLx/PostgreSQL types stay behind this crate's boundary the same as
//! `p4inz-database` — `domain`, `application` and `knowledge` must not
//! depend on them (`docs/architecture/dependency-rules.md`).

pub mod rank;
mod repository;
mod row;
mod search;

pub use repository::PgKnowledgeRepository;
pub use row::{RowMappingError, row_to_item};
pub use search::search_published;
