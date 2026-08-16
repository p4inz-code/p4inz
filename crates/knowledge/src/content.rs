use std::fmt;

use thiserror::Error;

enum Bound {
    Empty,
    TooLong,
}

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

/// Maximum accepted length for a [`Title`], in characters.
pub const TITLE_MAX_LEN: usize = 200;

/// A knowledge item's title.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Title(String);

impl Title {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, TitleError> {
        trimmed_non_empty(raw.as_ref(), TITLE_MAX_LEN).map(Self).map_err(|b| match b {
            Bound::Empty => TitleError::Empty,
            Bound::TooLong => TitleError::TooLong { max: TITLE_MAX_LEN },
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TitleError {
    #[error("title must not be empty")]
    Empty,
    #[error("title must be at most {max} characters")]
    TooLong { max: usize },
}

/// Maximum accepted length for a [`Body`], in characters.
pub const BODY_MAX_LEN: usize = 20_000;

/// A knowledge item's content body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Body(String);

impl Body {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, BodyError> {
        trimmed_non_empty(raw.as_ref(), BODY_MAX_LEN).map(Self).map_err(|b| match b {
            Bound::Empty => BodyError::Empty,
            Bound::TooLong => BodyError::TooLong { max: BODY_MAX_LEN },
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BodyError {
    #[error("body must not be empty")]
    Empty,
    #[error("body must be at most {max} characters")]
    TooLong { max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_trims_and_accepts_valid_value() {
        assert_eq!(Title::parse("  P4inz  ").unwrap().as_str(), "P4inz");
    }

    #[test]
    fn title_rejects_empty() {
        assert_eq!(Title::parse(""), Err(TitleError::Empty));
    }

    #[test]
    fn title_rejects_too_long() {
        let too_long = "a".repeat(TITLE_MAX_LEN + 1);
        assert_eq!(Title::parse(too_long), Err(TitleError::TooLong { max: TITLE_MAX_LEN }));
    }

    #[test]
    fn body_trims_and_accepts_valid_value() {
        assert_eq!(Body::parse("  hello world  ").unwrap().as_str(), "hello world");
    }

    #[test]
    fn body_rejects_empty() {
        assert_eq!(Body::parse(""), Err(BodyError::Empty));
    }

    #[test]
    fn body_rejects_too_long() {
        let too_long = "a".repeat(BODY_MAX_LEN + 1);
        assert_eq!(Body::parse(too_long), Err(BodyError::TooLong { max: BODY_MAX_LEN }));
    }
}
