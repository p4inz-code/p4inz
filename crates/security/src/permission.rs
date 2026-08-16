use std::fmt;

use thiserror::Error;

/// Maximum accepted length for a [`Permission`], in characters.
pub const PERMISSION_MAX_LEN: usize = 100;

/// A capability identifier (e.g. `"project:register"`).
///
/// P4inz's specification does not fix a concrete permission vocabulary
/// (`docs/PROJECT_SPEC.md` names "Administrative functions must be
/// permission-controlled" but no specific capability list), so this stays
/// an open, validated identifier rather than a closed enum — mirroring how
/// `p4inz_domain::ProjectStatus` handles the same kind of gap. Concrete
/// permission names are introduced by the milestones that need them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Permission(String);

impl Permission {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, PermissionError> {
        let trimmed = raw.as_ref().trim();

        if trimmed.is_empty() {
            return Err(PermissionError::Empty);
        }
        if trimmed.chars().count() > PERMISSION_MAX_LEN {
            return Err(PermissionError::TooLong { max: PERMISSION_MAX_LEN });
        }
        if !trimmed.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == ':' || c == '_' || c == '-'
        }) {
            return Err(PermissionError::InvalidCharacters);
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PermissionError {
    #[error("permission must not be empty")]
    Empty,
    #[error("permission must be at most {max} characters")]
    TooLong { max: usize },
    #[error("permission must contain only lowercase ascii letters, digits, ':', '_' or '-'")]
    InvalidCharacters,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_permission() {
        let permission = Permission::parse("project:register").unwrap();
        assert_eq!(permission.as_str(), "project:register");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let permission = Permission::parse("  admin  ").unwrap();
        assert_eq!(permission.as_str(), "admin");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(Permission::parse(""), Err(PermissionError::Empty));
        assert_eq!(Permission::parse("   "), Err(PermissionError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        let too_long = "a".repeat(PERMISSION_MAX_LEN + 1);
        assert_eq!(
            Permission::parse(too_long),
            Err(PermissionError::TooLong { max: PERMISSION_MAX_LEN })
        );
    }

    #[test]
    fn rejects_uppercase_and_unexpected_characters() {
        assert_eq!(Permission::parse("Project:Register"), Err(PermissionError::InvalidCharacters));
        assert_eq!(Permission::parse("project register"), Err(PermissionError::InvalidCharacters));
        assert_eq!(Permission::parse("project.register"), Err(PermissionError::InvalidCharacters));
    }
}
