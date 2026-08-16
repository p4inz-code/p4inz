//! P4inz common subsystem.
//!
//! Minimal, generic primitives shared across the workspace. This crate must
//! stay small and dependency-free — it is not a place for
//! subsystem-specific logic (see `docs/architecture/overview.md`).

mod secret;

pub use secret::Secret;
