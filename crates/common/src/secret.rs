use std::fmt;

/// A string value that must never be exposed via [`Debug`] or [`Display`].
///
/// Used for credentials such as API tokens so that an accidental
/// `{:?}`/log/print statement cannot leak the value (`docs/security/
/// security-model.md`: "Secret isolation"; `AGENTS.md`: "Never log
/// secrets").
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the wrapped value. Callers must not log, print or otherwise
    /// persist the returned string outside of its intended use.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(\"[REDACTED]\")")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl From<String> for Secret {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expose_secret_returns_original_value() {
        let secret = Secret::new("super-secret-token");
        assert_eq!(secret.expose_secret(), "super-secret-token");
    }

    #[test]
    fn debug_does_not_leak_value() {
        let secret = Secret::new("super-secret-token");
        let debug = format!("{secret:?}");
        assert!(!debug.contains("super-secret-token"));
        assert_eq!(debug, "Secret(\"[REDACTED]\")");
    }

    #[test]
    fn display_does_not_leak_value() {
        let secret = Secret::new("super-secret-token");
        assert_eq!(secret.to_string(), "[REDACTED]");
    }
}
