use std::fmt;

use thiserror::Error;

/// Maximum accepted length for a [`Link`], in characters.
pub const LINK_MAX_LEN: usize = 2048;

/// An absolute `http`/`https` link (e.g. a project repository or
/// documentation URL).
///
/// This performs shape validation only: scheme, a non-empty host, and the
/// absence of whitespace/control characters. Full URL parsing (percent
/// encoding, IDN, query strings) is deferred to the infrastructure layer,
/// where links are actually dereferenced and SSRF protections apply.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Link(String);

impl Link {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, LinkError> {
        let trimmed = raw.as_ref().trim();

        if trimmed.is_empty() {
            return Err(LinkError::Empty);
        }
        if trimmed.chars().count() > LINK_MAX_LEN {
            return Err(LinkError::TooLong { max: LINK_MAX_LEN });
        }
        if trimmed.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(LinkError::InvalidCharacters);
        }

        let rest = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))
            .ok_or(LinkError::UnsupportedScheme)?;

        let host = rest.split(['/', '?', '#']).next().unwrap_or("");
        if host.is_empty() {
            return Err(LinkError::MissingHost);
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LinkError {
    #[error("link must not be empty")]
    Empty,
    #[error("link must be at most {max} characters")]
    TooLong { max: usize },
    #[error("link must not contain whitespace or control characters")]
    InvalidCharacters,
    #[error("link must start with http:// or https://")]
    UnsupportedScheme,
    #[error("link is missing a host")]
    MissingHost,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_link() {
        let link = Link::parse("https://github.com/p4inz-code/p4inz").unwrap();
        assert_eq!(link.as_str(), "https://github.com/p4inz-code/p4inz");
    }

    #[test]
    fn accepts_http_link() {
        assert!(Link::parse("http://example.com").is_ok());
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let link = Link::parse("  https://example.com  ").unwrap();
        assert_eq!(link.as_str(), "https://example.com");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(Link::parse(""), Err(LinkError::Empty));
        assert_eq!(Link::parse("   "), Err(LinkError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        let too_long = format!("https://example.com/{}", "a".repeat(LINK_MAX_LEN));
        assert_eq!(Link::parse(too_long), Err(LinkError::TooLong { max: LINK_MAX_LEN }));
    }

    #[test]
    fn rejects_interior_whitespace() {
        assert_eq!(Link::parse("https://example.com/ path"), Err(LinkError::InvalidCharacters));
    }

    #[test]
    fn rejects_unsupported_scheme() {
        assert_eq!(Link::parse("ftp://example.com"), Err(LinkError::UnsupportedScheme));
        assert_eq!(Link::parse("javascript:alert(1)"), Err(LinkError::UnsupportedScheme));
    }

    #[test]
    fn rejects_missing_host() {
        assert_eq!(Link::parse("https://"), Err(LinkError::MissingHost));
        assert_eq!(Link::parse("https:///path"), Err(LinkError::MissingHost));
    }
}
