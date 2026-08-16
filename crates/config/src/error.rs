use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required environment variable '{key}'")]
    Missing { key: &'static str },

    #[error("invalid value for environment variable '{key}': {reason}")]
    Invalid { key: &'static str, reason: String },
}
