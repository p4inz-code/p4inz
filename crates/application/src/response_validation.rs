use p4inz_ai::CompletionResponse;
use p4inz_errors::AppError;

/// Errors a [`CompletionResponse`] can fail validation with.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ResponseValidationError {
    #[error("the AI provider returned an empty response")]
    Empty,
    #[error("the response references source {found}, but only {available} source(s) were provided")]
    UnknownSourceReference { found: usize, available: usize },
}

impl From<ResponseValidationError> for AppError {
    fn from(error: ResponseValidationError) -> Self {
        AppError::new(p4inz_errors::ErrorKind::Internal, error.to_string())
    }
}

/// Validates a completed response before it is shown to a user
/// (`docs/development/implementation_plan.md` section 7: "AI Response
/// Validation").
///
/// Checks:
/// - The response is not empty/whitespace-only — a provider returning
///   nothing usable is a failure, not a valid (if terse) answer.
/// - The response does not cite a `Source N` number higher than
///   `evidence_count` — an out-of-range citation is a concrete,
///   mechanically detectable sign of a hallucinated reference, distinct
///   from (and a cheaper check than) validating the citation's content is
///   actually accurate, which would require re-asking a model and isn't
///   attempted here.
pub fn validate_response(
    response: &CompletionResponse,
    evidence_count: usize,
) -> Result<(), ResponseValidationError> {
    if response.text.trim().is_empty() {
        return Err(ResponseValidationError::Empty);
    }

    if let Some(found) = highest_source_reference(&response.text) {
        if found == 0 || found > evidence_count {
            return Err(ResponseValidationError::UnknownSourceReference {
                found,
                available: evidence_count,
            });
        }
    }

    Ok(())
}

/// Scans `text` for `"Source N"` mentions and returns the highest `N`
/// found, if any. Deliberately simple substring/digit scanning rather
/// than a regex crate — the pattern is fixed and this is the only place
/// that needs it.
fn highest_source_reference(text: &str) -> Option<usize> {
    const MARKER: &str = "Source ";

    let mut highest = None;
    let mut rest = text;

    while let Some(pos) = rest.find(MARKER) {
        let after = &rest[pos + MARKER.len()..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();

        if let Ok(number) = digits.parse::<usize>() {
            highest = Some(highest.map_or(number, |current: usize| current.max(number)));
        }

        rest = &after[digits.len()..];
    }

    highest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(text: &str) -> CompletionResponse {
        CompletionResponse { text: text.to_string() }
    }

    #[test]
    fn rejects_empty_response() {
        assert_eq!(validate_response(&response(""), 3), Err(ResponseValidationError::Empty));
        assert_eq!(validate_response(&response("   "), 3), Err(ResponseValidationError::Empty));
    }

    #[test]
    fn accepts_response_with_no_source_references() {
        assert!(validate_response(&response("P4inz is a Discord bot."), 3).is_ok());
    }

    #[test]
    fn accepts_response_citing_a_provided_source() {
        assert!(validate_response(&response("According to Source 2, P4inz is a bot."), 3).is_ok());
    }

    #[test]
    fn rejects_response_citing_an_unprovided_source() {
        let err =
            validate_response(&response("According to Source 7, P4inz is a bot."), 3).unwrap_err();
        assert_eq!(err, ResponseValidationError::UnknownSourceReference { found: 7, available: 3 });
    }

    #[test]
    fn rejects_response_citing_source_zero() {
        let err = validate_response(&response("See Source 0 for details."), 3).unwrap_err();
        assert_eq!(err, ResponseValidationError::UnknownSourceReference { found: 0, available: 3 });
    }

    #[test]
    fn uses_the_highest_referenced_source_number() {
        let err =
            validate_response(&response("Source 1 and Source 9 both say so."), 3).unwrap_err();
        assert_eq!(err, ResponseValidationError::UnknownSourceReference { found: 9, available: 3 });
    }
}
