use std::fmt;

use crate::error::ConfigError;

/// The deployment environment P4inz is running in.
///
/// The three variants match what already exists under `infra/deployment/`
/// (`staging`, `production`) plus local `development`, which is the default
/// in `.env.example`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Environment {
    #[default]
    Development,
    Staging,
    Production,
}

impl Environment {
    pub const ENV_KEY: &'static str = "P4INZ_ENV";

    pub fn parse(raw: &str) -> Result<Self, ConfigError> {
        match raw.trim().to_lowercase().as_str() {
            "development" => Ok(Self::Development),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            other => Err(ConfigError::Invalid {
                key: Self::ENV_KEY,
                reason: format!("'{other}' is not one of: development, staging, production"),
            }),
        }
    }

    pub fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Staging => "staging",
            Self::Production => "production",
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_values_case_insensitively() {
        assert_eq!(Environment::parse("development"), Ok(Environment::Development));
        assert_eq!(Environment::parse("Staging"), Ok(Environment::Staging));
        assert_eq!(Environment::parse("PRODUCTION"), Ok(Environment::Production));
        assert_eq!(Environment::parse("  production  "), Ok(Environment::Production));
    }

    #[test]
    fn rejects_unknown_value() {
        assert_eq!(
            Environment::parse("prod"),
            Err(ConfigError::Invalid {
                key: Environment::ENV_KEY,
                reason: "'prod' is not one of: development, staging, production".to_string(),
            })
        );
    }

    #[test]
    fn default_is_development() {
        assert_eq!(Environment::default(), Environment::Development);
    }

    #[test]
    fn is_production_only_true_for_production() {
        assert!(!Environment::Development.is_production());
        assert!(!Environment::Staging.is_production());
        assert!(Environment::Production.is_production());
    }
}
