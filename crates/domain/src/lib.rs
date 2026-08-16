//! P4inz domain subsystem.
//!
//! Core business entities, identifiers and invariants shared by the rest of
//! the workspace. This crate must stay independent of Discord, HTTP
//! frameworks, database drivers and AI providers — see
//! `docs/architecture/dependency-rules.md`.

mod id;
mod link;
mod project;
mod value_objects;

pub use id::Id;
pub use link::{LINK_MAX_LEN, Link, LinkError};
pub use project::{Project, ProjectError, ProjectId};
pub use value_objects::{
    PROJECT_DESCRIPTION_MAX_LEN, PROJECT_NAME_MAX_LEN, PROJECT_STATUS_MAX_LEN, ProjectDescription,
    ProjectDescriptionError, ProjectName, ProjectNameError, ProjectStatus, ProjectStatusError,
    TECHNOLOGY_NAME_MAX_LEN, TechnologyName, TechnologyNameError,
};
