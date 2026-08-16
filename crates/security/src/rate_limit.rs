use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use p4inz_errors::{AppError, AppResult};

/// Token-bucket tuning.
///
/// `docs/PROJECT_SPEC.md` section 8 / `docs/development/
/// implementation_plan.md` section 9 require per-user, per-command and
/// global rate limiting. This type is transport-agnostic — the caller
/// decides what a "key" means (e.g. `"user:123"`, `"command:ping"`,
/// `"global"`), so the same limiter serves Discord, the API, or anything
/// else without this crate depending on either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimiterConfig {
    /// Maximum tokens (i.e. allowed actions) a single key can accumulate.
    pub max_tokens: u32,
    /// Time to refill exactly one token.
    pub refill_interval: Duration,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self { max_tokens: 5, refill_interval: Duration::from_secs(1) }
    }
}

struct Bucket {
    tokens: u32,
    last_refill: Instant,
}

/// A per-key token-bucket rate limiter.
///
/// Each distinct `key` passed to [`check`](Self::check) gets its own
/// independent bucket, created on first use with a full allowance.
pub struct RateLimiter {
    config: RateLimiterConfig,
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl RateLimiter {
    pub fn new(config: RateLimiterConfig) -> Self {
        Self { config, buckets: Mutex::new(HashMap::new()) }
    }

    /// Consumes one token for `key` if available, using the current time.
    /// Returns `Err(ErrorKind::RateLimited)` if the bucket is empty.
    pub fn check(&self, key: &str) -> AppResult<()> {
        self.check_at(key, Instant::now())
    }

    /// Same as [`check`](Self::check), but with an explicit `now` — used
    /// by tests to advance time deterministically without real sleeping.
    pub fn check_at(&self, key: &str, now: Instant) -> AppResult<()> {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| Bucket { tokens: self.config.max_tokens, last_refill: now });

        let elapsed = now.saturating_duration_since(bucket.last_refill);
        let refill_interval_nanos = self.config.refill_interval.as_nanos().max(1);
        let refilled_tokens = (elapsed.as_nanos() / refill_interval_nanos) as u64;

        if refilled_tokens > 0 {
            bucket.tokens =
                bucket.tokens.saturating_add(refilled_tokens as u32).min(self.config.max_tokens);
            bucket.last_refill += self.config.refill_interval * refilled_tokens as u32;
        }

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            Ok(())
        } else {
            Err(AppError::rate_limited(format!("rate limit exceeded for '{key}'")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use p4inz_errors::ErrorKind;

    #[test]
    fn allows_up_to_max_tokens_then_rejects() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_tokens: 3,
            refill_interval: Duration::from_secs(60),
        });
        let now = Instant::now();

        assert!(limiter.check_at("user:1", now).is_ok());
        assert!(limiter.check_at("user:1", now).is_ok());
        assert!(limiter.check_at("user:1", now).is_ok());

        let err = limiter.check_at("user:1", now).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::RateLimited);
    }

    #[test]
    fn refills_tokens_after_the_interval_elapses() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_tokens: 1,
            refill_interval: Duration::from_secs(10),
        });
        let start = Instant::now();

        assert!(limiter.check_at("user:1", start).is_ok());
        assert!(limiter.check_at("user:1", start).is_err());

        let after_refill = start + Duration::from_secs(10);
        assert!(limiter.check_at("user:1", after_refill).is_ok());
    }

    #[test]
    fn refill_never_exceeds_max_tokens() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_tokens: 2,
            refill_interval: Duration::from_secs(1),
        });
        let start = Instant::now();
        limiter.check_at("user:1", start).unwrap();

        let much_later = start + Duration::from_secs(1000);
        assert!(limiter.check_at("user:1", much_later).is_ok());
        assert!(limiter.check_at("user:1", much_later).is_ok());
        assert!(limiter.check_at("user:1", much_later).is_err());
    }

    #[test]
    fn keys_are_independent() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_tokens: 1,
            refill_interval: Duration::from_secs(60),
        });
        let now = Instant::now();

        assert!(limiter.check_at("user:1", now).is_ok());
        assert!(limiter.check_at("user:2", now).is_ok());
        assert!(limiter.check_at("user:1", now).is_err());
    }
}
