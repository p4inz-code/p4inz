use std::fmt;

use thiserror::Error;

/// A monotonically increasing version number for a [`crate::KnowledgeItem`].
///
/// `docs/PROJECT_SPEC.md` section 6 requires versioning and that
/// "Historical versions must not be silently destroyed"
/// (`docs/development/implementation_plan.md` section 6). This type only
/// tracks the current version number; preserving prior versions'
/// content is a persistence-layer concern (an append-only history table),
/// not modeled here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(u32);

impl Version {
    pub const fn initial() -> Self {
        Self(1)
    }

    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Reconstructs a [`Version`] from an already-valid stored value
    /// (Knowledge Search, Milestone 21). Rejects `0`: every item starts
    /// at version 1 ([`initial`](Self::initial)), so `0` cannot be a
    /// legitimately stored version.
    pub fn from_u32(value: u32) -> Result<Self, VersionError> {
        if value == 0 {
            return Err(VersionError::Zero);
        }
        Ok(Self(value))
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::initial()
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VersionError {
    #[error("version must be at least 1")]
    Zero,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_version_is_one() {
        assert_eq!(Version::initial().as_u32(), 1);
    }

    #[test]
    fn next_increments() {
        assert_eq!(Version::initial().next().as_u32(), 2);
    }

    #[test]
    fn ordering_reflects_version_number() {
        assert!(Version::initial() < Version::initial().next());
    }

    #[test]
    fn from_u32_round_trips() {
        assert_eq!(Version::from_u32(5).unwrap().as_u32(), 5);
    }

    #[test]
    fn from_u32_rejects_zero() {
        assert_eq!(Version::from_u32(0), Err(VersionError::Zero));
    }
}
