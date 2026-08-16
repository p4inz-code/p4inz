use std::fmt;

use p4inz_domain::Link;
use thiserror::Error;

/// The kind of authoritative source a [`Source`] originates from.
///
/// Matches the five source kinds `docs/PROJECT_SPEC.md` section 5 ("Source
/// of Truth") names, in the order given there. `Other` covers "Other
/// explicitly trusted sources" — an intentionally open-ended catch-all in
/// the specification itself, not something this type invents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    /// Official repository/project documentation.
    Repository,
    /// Official Northbyte Studios documentation.
    OfficialDocumentation,
    /// Explicit administrator-maintained records.
    Administrator,
    /// Official announcements.
    Announcement,
    /// Other explicitly trusted sources.
    Other,
}

impl SourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Repository => "repository",
            Self::OfficialDocumentation => "official_documentation",
            Self::Administrator => "administrator",
            Self::Announcement => "announcement",
            Self::Other => "other",
        }
    }

    /// Parses the [`as_str`](Self::as_str) representation back into a
    /// [`SourceKind`] — the inverse used when reconstructing an item
    /// loaded from storage (Knowledge Search, Milestone 21).
    pub fn parse(raw: &str) -> Result<Self, SourceKindError> {
        match raw {
            "repository" => Ok(Self::Repository),
            "official_documentation" => Ok(Self::OfficialDocumentation),
            "administrator" => Ok(Self::Administrator),
            "announcement" => Ok(Self::Announcement),
            "other" => Ok(Self::Other),
            other => Err(SourceKindError::Unknown { value: other.to_string() }),
        }
    }
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SourceKindError {
    #[error("unknown source kind '{value}'")]
    Unknown { value: String },
}

/// Where a [`crate::KnowledgeItem`] originated.
///
/// `reference` is optional: an administrator-authored record may have no
/// external link, while GitHub- or documentation-sourced content typically
/// does.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Source {
    kind: SourceKind,
    reference: Option<Link>,
}

impl Source {
    pub fn new(kind: SourceKind, reference: Option<Link>) -> Self {
        Self { kind, reference }
    }

    pub fn kind(&self) -> SourceKind {
        self.kind
    }

    pub fn reference(&self) -> Option<&Link> {
        self.reference.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_without_reference_round_trips() {
        let source = Source::new(SourceKind::Administrator, None);
        assert_eq!(source.kind(), SourceKind::Administrator);
        assert!(source.reference().is_none());
    }

    #[test]
    fn source_with_reference_round_trips() {
        let link = Link::parse("https://github.com/p4inz-code/p4inz").unwrap();
        let source = Source::new(SourceKind::Repository, Some(link.clone()));
        assert_eq!(source.reference(), Some(&link));
    }

    #[test]
    fn parse_round_trips_with_as_str() {
        for kind in [
            SourceKind::Repository,
            SourceKind::OfficialDocumentation,
            SourceKind::Administrator,
            SourceKind::Announcement,
            SourceKind::Other,
        ] {
            assert_eq!(SourceKind::parse(kind.as_str()), Ok(kind));
        }
    }

    #[test]
    fn parse_rejects_unknown_value() {
        assert_eq!(
            SourceKind::parse("bogus"),
            Err(SourceKindError::Unknown { value: "bogus".to_string() })
        );
    }
}
