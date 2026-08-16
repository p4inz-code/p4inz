//! P4inz shared error taxonomy.
//!
//! This crate provides the common vocabulary ([`ErrorKind`], [`AppError`])
//! that domain, application, infrastructure and API code use to describe
//! failures consistently, without forcing every layer to know about every
//! other layer's concrete error types. It intentionally stays
//! transport-agnostic: HTTP status mapping, Discord response formatting and
//! similar concerns belong to the adapters that consume an [`AppError`], not
//! to this crate.
//!
//! Existing fine-grained domain errors (e.g. in `p4inz-domain`) are left as
//! they are — precise, typed, and useful on their own. [`IntoAppError`]
//! shows how such an error converts into the shared taxonomy at a boundary,
//! without requiring `p4inz-domain` itself to depend on this crate.

mod error;
mod kind;

pub use error::{AppError, AppResult, IntoAppError};
pub use kind::ErrorKind;
