use std::fmt;

use thiserror::Error;

/// Maximum accepted length for a [`Question`], in characters.
pub const QUESTION_MAX_LEN: usize = 1000;

/// A natural-language question directed at P4inz.
///
/// `docs/PROJECT_SPEC.md` section 2/8 name "natural-language questions" as
/// a core capability without prescribing input structure beyond it being
/// free text, so this only enforces the bounds needed for safety (non-empty,
/// bounded length — resource-exhaustion protection per
/// `docs/security/security-model.md`), not any particular grammar.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Question(String);

impl Question {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, QuestionError> {
        let trimmed = raw.as_ref().trim();

        if trimmed.is_empty() {
            return Err(QuestionError::Empty);
        }
        if trimmed.chars().count() > QUESTION_MAX_LEN {
            return Err(QuestionError::TooLong { max: QUESTION_MAX_LEN });
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Question {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum QuestionError {
    #[error("question must not be empty")]
    Empty,
    #[error("question must be at most {max} characters")]
    TooLong { max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_accepts_valid_question() {
        let question = Question::parse("  What is P4inz?  ").unwrap();
        assert_eq!(question.as_str(), "What is P4inz?");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(Question::parse(""), Err(QuestionError::Empty));
        assert_eq!(Question::parse("   "), Err(QuestionError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        let too_long = "a".repeat(QUESTION_MAX_LEN + 1);
        assert_eq!(
            Question::parse(too_long),
            Err(QuestionError::TooLong { max: QUESTION_MAX_LEN })
        );
    }
}
