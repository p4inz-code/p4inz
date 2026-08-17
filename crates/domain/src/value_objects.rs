use std::fmt;

use thiserror::Error;

enum Bound {
    Empty,
    TooLong,
}

/// Trims `raw` and rejects it if empty or longer than `max_len` characters.
/// Shared by the bounded-text value objects below to avoid duplicating the
/// same validation logic for each one.
fn trimmed_non_empty(raw: &str, max_len: usize) -> Result<String, Bound> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Err(Bound::Empty)
    } else if trimmed.chars().count() > max_len {
        Err(Bound::TooLong)
    } else {
        Ok(trimmed.to_string())
    }
}

/// Maximum accepted length for a [`ProjectName`], in characters.
pub const PROJECT_NAME_MAX_LEN: usize = 200;

/// A project's display name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectName(String);

impl ProjectName {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, ProjectNameError> {
        trimmed_non_empty(raw.as_ref(), PROJECT_NAME_MAX_LEN).map(Self).map_err(|b| match b {
            Bound::Empty => ProjectNameError::Empty,
            Bound::TooLong => ProjectNameError::TooLong { max: PROJECT_NAME_MAX_LEN },
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProjectNameError {
    #[error("project name must not be empty")]
    Empty,
    #[error("project name must be at most {max} characters")]
    TooLong { max: usize },
}

/// Maximum accepted length for a [`ProjectDescription`], in characters.
pub const PROJECT_DESCRIPTION_MAX_LEN: usize = 4000;

/// A project's descriptive summary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectDescription(String);

impl ProjectDescription {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, ProjectDescriptionError> {
        trimmed_non_empty(raw.as_ref(), PROJECT_DESCRIPTION_MAX_LEN).map(Self).map_err(
            |b| match b {
                Bound::Empty => ProjectDescriptionError::Empty,
                Bound::TooLong => {
                    ProjectDescriptionError::TooLong { max: PROJECT_DESCRIPTION_MAX_LEN }
                }
            },
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProjectDescriptionError {
    #[error("project description must not be empty")]
    Empty,
    #[error("project description must be at most {max} characters")]
    TooLong { max: usize },
}

/// Maximum accepted length for a [`ProjectStatus`], in characters.
pub const PROJECT_STATUS_MAX_LEN: usize = 100;

/// A project's current status.
///
/// The specification (`docs/PROJECT_SPEC.md`, section 4) names "Status" as a
/// required project field but does not define a fixed vocabulary of status
/// values. Rather than invent one, this type only enforces that a status is
/// present and reasonably bounded; a closed set of values should replace
/// this once the product defines one (see the Milestone 02 report).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectStatus(String);

impl ProjectStatus {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, ProjectStatusError> {
        trimmed_non_empty(raw.as_ref(), PROJECT_STATUS_MAX_LEN).map(Self).map_err(|b| match b {
            Bound::Empty => ProjectStatusError::Empty,
            Bound::TooLong => ProjectStatusError::TooLong { max: PROJECT_STATUS_MAX_LEN },
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProjectStatusError {
    #[error("project status must not be empty")]
    Empty,
    #[error("project status must be at most {max} characters")]
    TooLong { max: usize },
}

/// Maximum accepted length for a [`TechnologyName`], in characters.
pub const TECHNOLOGY_NAME_MAX_LEN: usize = 100;

/// The name of a technology used by a project (e.g. "Rust", "PostgreSQL").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TechnologyName(String);

impl TechnologyName {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, TechnologyNameError> {
        trimmed_non_empty(raw.as_ref(), TECHNOLOGY_NAME_MAX_LEN).map(Self).map_err(|b| match b {
            Bound::Empty => TechnologyNameError::Empty,
            Bound::TooLong => TechnologyNameError::TooLong { max: TECHNOLOGY_NAME_MAX_LEN },
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TechnologyName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TechnologyNameError {
    #[error("technology name must not be empty")]
    Empty,
    #[error("technology name must be at most {max} characters")]
    TooLong { max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_name_trims_and_accepts_valid_value() {
        let name = ProjectName::parse("  P4inz  ").unwrap();
        assert_eq!(name.as_str(), "P4inz");
    }

    #[test]
    fn project_name_rejects_empty() {
        assert_eq!(ProjectName::parse(""), Err(ProjectNameError::Empty));
        assert_eq!(ProjectName::parse("   "), Err(ProjectNameError::Empty));
    }

    #[test]
    fn project_name_rejects_too_long() {
        let too_long = "a".repeat(PROJECT_NAME_MAX_LEN + 1);
        assert_eq!(
            ProjectName::parse(too_long),
            Err(ProjectNameError::TooLong { max: PROJECT_NAME_MAX_LEN })
        );
    }

    #[test]
    fn project_name_accepts_max_length() {
        let max_len = "a".repeat(PROJECT_NAME_MAX_LEN);
        assert!(ProjectName::parse(max_len).is_ok());
    }

    #[test]
    fn project_description_rejects_empty() {
        assert_eq!(ProjectDescription::parse(""), Err(ProjectDescriptionError::Empty));
    }

    #[test]
    fn project_description_rejects_too_long() {
        let too_long = "a".repeat(PROJECT_DESCRIPTION_MAX_LEN + 1);
        assert_eq!(
            ProjectDescription::parse(too_long),
            Err(ProjectDescriptionError::TooLong { max: PROJECT_DESCRIPTION_MAX_LEN })
        );
    }

    #[test]
    fn project_description_trims_and_accepts_valid_value() {
        let description = ProjectDescription::parse("  a Discord bot  ").unwrap();
        assert_eq!(description.as_str(), "a Discord bot");
        assert_eq!(description.to_string(), "a Discord bot");
    }

    #[test]
    fn project_status_accepts_arbitrary_non_empty_value() {
        assert!(ProjectStatus::parse("active").is_ok());
        assert!(ProjectStatus::parse("anything-not-yet-defined").is_ok());
    }

    #[test]
    fn project_status_rejects_empty() {
        assert_eq!(ProjectStatus::parse(""), Err(ProjectStatusError::Empty));
    }

    #[test]
    fn project_status_rejects_too_long() {
        let too_long = "a".repeat(PROJECT_STATUS_MAX_LEN + 1);
        assert_eq!(
            ProjectStatus::parse(too_long),
            Err(ProjectStatusError::TooLong { max: PROJECT_STATUS_MAX_LEN })
        );
    }

    #[test]
    fn project_status_as_str_and_display_match() {
        let status = ProjectStatus::parse("active").unwrap();
        assert_eq!(status.as_str(), "active");
        assert_eq!(status.to_string(), "active");
    }

    #[test]
    fn technology_name_trims_and_accepts_valid_value() {
        let tech = TechnologyName::parse("  Rust ").unwrap();
        assert_eq!(tech.as_str(), "Rust");
        assert_eq!(tech.to_string(), "Rust");
    }

    #[test]
    fn technology_name_rejects_empty() {
        assert_eq!(TechnologyName::parse(""), Err(TechnologyNameError::Empty));
    }

    #[test]
    fn technology_name_rejects_too_long() {
        let too_long = "a".repeat(TECHNOLOGY_NAME_MAX_LEN + 1);
        assert_eq!(
            TechnologyName::parse(too_long),
            Err(TechnologyNameError::TooLong { max: TECHNOLOGY_NAME_MAX_LEN })
        );
    }

    #[test]
    fn project_name_display_matches_as_str() {
        let name = ProjectName::parse("P4inz").unwrap();
        assert_eq!(name.to_string(), name.as_str());
    }
}
