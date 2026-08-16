use thiserror::Error;

use crate::publication_state::PublicationState;

/// A state transition that isn't allowed by the knowledge lifecycle
/// (`docs/development/implementation_plan.md` section 6: `Draft -> Review
/// -> Published -> Archived`).
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("cannot move from {from} to {to}")]
pub struct WorkflowError {
    pub from: PublicationState,
    pub to: PublicationState,
}

/// Whether moving from `from` to `to` is a legal workflow transition.
///
/// Allowed: the linear happy path (`Draft -> Review -> Published ->
/// Archived`) plus `Review -> Draft` for rejecting an item back for
/// revision — an ordinary, expected part of any review process, even
/// though the specification's lifecycle diagram only draws the forward
/// path. No other backward move (e.g. un-archiving, or reverting a
/// published item) is allowed here; the specification doesn't describe
/// one, and inventing it would be a real product decision, not a
/// mechanical detail — see the Milestone 18 report for this open
/// question.
pub fn is_allowed_transition(from: PublicationState, to: PublicationState) -> bool {
    use PublicationState::{Archived, Draft, Published, Review};

    matches!(
        (from, to),
        (Draft, Review) | (Review, Draft) | (Review, Published) | (Published, Archived)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use PublicationState::{Archived, Draft, Published, Review};

    #[test]
    fn forward_path_is_allowed() {
        assert!(is_allowed_transition(Draft, Review));
        assert!(is_allowed_transition(Review, Published));
        assert!(is_allowed_transition(Published, Archived));
    }

    #[test]
    fn rejection_back_to_draft_is_allowed() {
        assert!(is_allowed_transition(Review, Draft));
    }

    #[test]
    fn skipping_review_is_not_allowed() {
        assert!(!is_allowed_transition(Draft, Published));
    }

    #[test]
    fn archived_is_terminal() {
        assert!(!is_allowed_transition(Archived, Draft));
        assert!(!is_allowed_transition(Archived, Published));
    }

    #[test]
    fn publishing_directly_from_draft_is_not_allowed() {
        assert!(!is_allowed_transition(Draft, Published));
        assert!(!is_allowed_transition(Draft, Archived));
    }

    #[test]
    fn staying_in_the_same_state_is_not_a_transition() {
        assert!(!is_allowed_transition(Draft, Draft));
        assert!(!is_allowed_transition(Published, Published));
    }
}
