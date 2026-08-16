//! P4inz infrastructure subsystem.
//!
//! External integrations, kept behind the ports domain/application/
//! knowledge define — nothing here is depended on by those crates
//! (`docs/architecture/dependency-rules.md`: "Infrastructure implements
//! contracts required by application/domain").
//!
//! Currently: [`github::GitHubSourceAdapter`], a `p4inz_knowledge::SourceAdapter`
//! implementation for GitHub-sourced knowledge (Milestone 19).

pub mod github;
