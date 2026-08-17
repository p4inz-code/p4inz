use std::time::Duration;

/// The delay before the first retry.
const BASE_DELAY: Duration = Duration::from_secs(30);

/// The maximum delay between retries, regardless of how many attempts
/// have already failed — unbounded exponential growth would eventually
/// schedule a retry so far out it's effectively abandoned, and risks
/// integer overflow computing it
/// (`docs/development/implementation_plan.md` section 15: "Bounded
/// retries", "Exponential backoff"; this run's job-safety requirements:
/// "avoid retry storms", "avoid infinite retry loops").
const MAX_DELAY: Duration = Duration::from_secs(30 * 60);

/// Computes how long to wait before retrying a job that has already
/// failed `attempts` times (`attempts` = 0 for the delay before the
/// *first* retry, i.e. after the first failure).
///
/// Doubles with each attempt (`BASE_DELAY * 2^attempts`), capped at
/// [`MAX_DELAY`]. The exponent is clamped before the `pow`/`mul` so this
/// can never panic or silently wrap regardless of how large `attempts`
/// gets — irrelevant in practice since [`crate::job::Job::max_attempts`]
/// bounds real attempt counts to something far smaller, but this function
/// has no way to know that from its own signature.
pub fn backoff_delay(attempts: u32) -> Duration {
    let exponent = attempts.min(20);
    let multiplier = 2u32.saturating_pow(exponent);
    BASE_DELAY.saturating_mul(multiplier).min(MAX_DELAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_retry_uses_the_base_delay() {
        assert_eq!(backoff_delay(0), BASE_DELAY);
    }

    #[test]
    fn doubles_with_each_attempt() {
        assert_eq!(backoff_delay(1), BASE_DELAY * 2);
        assert_eq!(backoff_delay(2), BASE_DELAY * 4);
        assert_eq!(backoff_delay(3), BASE_DELAY * 8);
    }

    #[test]
    fn is_capped_at_the_maximum_delay() {
        assert_eq!(backoff_delay(10), MAX_DELAY);
        assert_eq!(backoff_delay(1000), MAX_DELAY);
    }

    #[test]
    fn never_panics_for_any_attempt_count() {
        for attempts in [0, 1, 5, 20, u32::MAX] {
            let _ = backoff_delay(attempts);
        }
    }

    #[test]
    fn is_monotonically_nondecreasing() {
        let mut previous = Duration::ZERO;
        for attempts in 0..15 {
            let delay = backoff_delay(attempts);
            assert!(delay >= previous);
            previous = delay;
        }
    }
}
