use std::fmt;

use thiserror::Error;

/// The information category a [`crate::KnowledgeItem`] belongs to.
///
/// Matches `docs/PROJECT_SPEC.md` section 4 ("Information Model") exactly
/// — its four top-level categories (Northbyte Studios, People, Projects,
/// Community) are the complete, closed set the specification defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnowledgeCategory {
    NorthbyteStudios,
    People,
    Projects,
    Community,
}

impl KnowledgeCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NorthbyteStudios => "northbyte_studios",
            Self::People => "people",
            Self::Projects => "projects",
            Self::Community => "community",
        }
    }

    /// Parses the [`as_str`](Self::as_str) representation back into a
    /// [`KnowledgeCategory`] — the inverse used when reconstructing an
    /// item loaded from storage (Knowledge Search, Milestone 21).
    pub fn parse(raw: &str) -> Result<Self, KnowledgeCategoryError> {
        match raw {
            "northbyte_studios" => Ok(Self::NorthbyteStudios),
            "people" => Ok(Self::People),
            "projects" => Ok(Self::Projects),
            "community" => Ok(Self::Community),
            other => Err(KnowledgeCategoryError::Unknown { value: other.to_string() }),
        }
    }
}

impl fmt::Display for KnowledgeCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeCategoryError {
    #[error("unknown knowledge category '{value}'")]
    Unknown { value: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_is_stable() {
        assert_eq!(KnowledgeCategory::NorthbyteStudios.as_str(), "northbyte_studios");
        assert_eq!(KnowledgeCategory::Projects.as_str(), "projects");
    }

    #[test]
    fn parse_round_trips_with_as_str() {
        for category in [
            KnowledgeCategory::NorthbyteStudios,
            KnowledgeCategory::People,
            KnowledgeCategory::Projects,
            KnowledgeCategory::Community,
        ] {
            assert_eq!(KnowledgeCategory::parse(category.as_str()), Ok(category));
        }
    }

    #[test]
    fn parse_rejects_unknown_value() {
        assert_eq!(
            KnowledgeCategory::parse("bogus"),
            Err(KnowledgeCategoryError::Unknown { value: "bogus".to_string() })
        );
    }
}
