//! P4inz knowledge subsystem.
//!
//! The authoritative knowledge lifecycle (`docs/architecture/overview.md`,
//! ADR-004). [`KnowledgeItem`] is the core entity: unstructured content
//! (title/body) tagged with a [`KnowledgeCategory`], a [`Source`], a
//! [`PublicationState`], and [`Provenance`] (version/timestamp tracking).
//! [`KnowledgeItem::transition_to`] enforces the workflow's legal state
//! transitions. [`SourceAdapter`] is the port external ingestion (e.g.
//! GitHub) implements; [`KnowledgeRepository`] is the persistence port;
//! [`synchronize_from_source`] ties them together (Milestone 20).
//! [`KnowledgeItem::from_parts`]/[`Provenance::from_parts`] reconstruct an
//! item loaded from storage (Knowledge Search, Milestone 21). This crate
//! must stay independent of Discord, Axum, SQLx/PostgreSQL and AI
//! providers, the same as `p4inz-domain`
//! (`docs/architecture/dependency-rules.md`).

mod category;
mod content;
mod knowledge_item;
mod provenance;
mod publication_state;
mod repository;
mod source;
mod source_adapter;
mod synchronize;
mod version;
mod workflow;

pub use category::{KnowledgeCategory, KnowledgeCategoryError};
pub use content::{BODY_MAX_LEN, Body, BodyError, TITLE_MAX_LEN, Title, TitleError};
pub use knowledge_item::{KnowledgeItem, KnowledgeItemId};
pub use provenance::Provenance;
pub use publication_state::{PublicationState, PublicationStateError};
pub use repository::KnowledgeRepository;
pub use source::{Source, SourceKind, SourceKindError};
pub use source_adapter::{RawDocument, SourceAdapter};
pub use synchronize::{SyncOutcome, plan_sync, synchronize_from_source};
pub use version::{Version, VersionError};
pub use workflow::WorkflowError;
