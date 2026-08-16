use std::fmt;

use thiserror::Error;

/// A knowledge item's position in its lifecycle.
///
/// `docs/development/implementation_plan.md` section 6: `Draft -> Review ->
/// Published -> Archived`. This type only represents the state; the
/// transition rules between states (which moves are valid) are Knowledge
/// Workflow's concern (Milestone 18), not this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PublicationState {
    Draft,
    Review,
    Published,
    Archived,
}

impl PublicationState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Published => "published",
            Self::Archived => "archived",
        }
    }

    /// Parses the [`as_str`](Self::as_str) representation back into a
    /// [`PublicationState`] — the inverse used when reconstructing an
    /// item loaded from storage (Knowledge Search, Milestone 21).
    pub fn parse(raw: &str) -> Result<Self, PublicationStateError> {
        match raw {
            "draft" => Ok(Self::Draft),
            "review" => Ok(Self::Review),
            "published" => Ok(Self::Published),
            "archived" => Ok(Self::Archived),
            other => Err(PublicationStateError::Unknown { value: other.to_string() }),
        }
    }
}

impl fmt::Display for PublicationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PublicationStateError {
    #[error("unknown publication state '{value}'")]
    Unknown { value: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_is_stable() {
        assert_eq!(PublicationState::Draft.as_str(), "draft");
        assert_eq!(PublicationState::Archived.as_str(), "archived");
    }

    #[test]
    fn parse_round_trips_with_as_str() {
        for state in [
            PublicationState::Draft,
            PublicationState::Review,
            PublicationState::Published,
            PublicationState::Archived,
        ] {
            assert_eq!(PublicationState::parse(state.as_str()), Ok(state));
        }
    }

    #[test]
    fn parse_rejects_unknown_value() {
        assert_eq!(
            PublicationState::parse("bogus"),
            Err(PublicationStateError::Unknown { value: "bogus".to_string() })
        );
    }
}
