use std::time::{Duration, SystemTime};

use crate::version::Version;

/// Version and timestamp tracking for a [`crate::KnowledgeItem`]
/// (`docs/PROJECT_SPEC.md` section 6: "Provenance", "Versioning",
/// "Freshness").
///
/// Timestamps are `SystemTime` (stdlib only, no new dependency) rather
/// than a calendar-aware type like `time`/`chrono` — this crate stays
/// infrastructure-independent; converting to a concrete timestamp
/// representation for storage is the repository implementation's job
/// (Knowledge Search, Milestone 21).
///
/// All methods that need "now" take it as a parameter rather than calling
/// `SystemTime::now()` internally, so callers (and tests) control time
/// deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provenance {
    version: Version,
    created_at: SystemTime,
    updated_at: SystemTime,
    synchronized_at: Option<SystemTime>,
}

impl Provenance {
    /// Provenance for a newly created item: version 1, `created_at` and
    /// `updated_at` both set to `now`, never synchronized.
    pub fn new(now: SystemTime) -> Self {
        Self {
            version: Version::initial(),
            created_at: now,
            updated_at: now,
            synchronized_at: None,
        }
    }

    /// Reconstructs a [`Provenance`] from already-valid stored values.
    ///
    /// For loading a previously persisted item back (Knowledge Search,
    /// Milestone 21) — the values were already validated when originally
    /// written via [`new`](Self::new)/[`touched`](Self::touched), so this
    /// intentionally does not re-derive them.
    pub fn from_parts(
        version: Version,
        created_at: SystemTime,
        updated_at: SystemTime,
        synchronized_at: Option<SystemTime>,
    ) -> Self {
        Self { version, created_at, updated_at, synchronized_at }
    }

    /// Records a content change: bumps the version and `updated_at`,
    /// preserving `created_at` and any prior `synchronized_at`.
    #[must_use]
    pub fn touched(self, now: SystemTime) -> Self {
        Self { version: self.version.next(), updated_at: now, ..self }
    }

    /// Records a successful synchronization from the item's source,
    /// implying a content change (per [`touched`](Self::touched)).
    #[must_use]
    pub fn synchronized(self, now: SystemTime) -> Self {
        Self { synchronized_at: Some(now), ..self.touched(now) }
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }

    pub fn updated_at(&self) -> SystemTime {
        self.updated_at
    }

    pub fn synchronized_at(&self) -> Option<SystemTime> {
        self.synchronized_at
    }

    /// Whether this item hasn't been updated within `max_age` of `now`
    /// (`docs/PROJECT_SPEC.md` section 6: "Stale or conflicting
    /// information must not silently appear authoritative").
    ///
    /// Returns `false` (not stale) if `now` is earlier than `updated_at` —
    /// a clock going backwards is not evidence of staleness.
    pub fn is_stale(&self, now: SystemTime, max_age: Duration) -> bool {
        now.duration_since(self.updated_at).is_ok_and(|age| age > max_age)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_at_version_one_and_never_synchronized() {
        let now = SystemTime::now();
        let provenance = Provenance::new(now);

        assert_eq!(provenance.version(), Version::initial());
        assert_eq!(provenance.created_at(), now);
        assert_eq!(provenance.updated_at(), now);
        assert!(provenance.synchronized_at().is_none());
    }

    #[test]
    fn touched_bumps_version_and_updated_at_but_preserves_created_at() {
        let created = SystemTime::now();
        let later = created + Duration::from_secs(60);

        let provenance = Provenance::new(created).touched(later);

        assert_eq!(provenance.version(), Version::initial().next());
        assert_eq!(provenance.created_at(), created);
        assert_eq!(provenance.updated_at(), later);
    }

    #[test]
    fn synchronized_sets_synchronized_at_and_touches() {
        let created = SystemTime::now();
        let synced_at = created + Duration::from_secs(60);

        let provenance = Provenance::new(created).synchronized(synced_at);

        assert_eq!(provenance.synchronized_at(), Some(synced_at));
        assert_eq!(provenance.updated_at(), synced_at);
        assert_eq!(provenance.version(), Version::initial().next());
    }

    #[test]
    fn is_stale_true_past_max_age() {
        let created = SystemTime::now();
        let provenance = Provenance::new(created);
        let much_later = created + Duration::from_secs(3600);

        assert!(provenance.is_stale(much_later, Duration::from_secs(60)));
    }

    #[test]
    fn is_stale_false_within_max_age() {
        let created = SystemTime::now();
        let provenance = Provenance::new(created);
        let soon_after = created + Duration::from_secs(10);

        assert!(!provenance.is_stale(soon_after, Duration::from_secs(60)));
    }

    #[test]
    fn is_stale_false_when_now_precedes_updated_at() {
        let created = SystemTime::now();
        let provenance = Provenance::new(created);
        let earlier = created - Duration::from_secs(10);

        assert!(!provenance.is_stale(earlier, Duration::from_secs(1)));
    }
}
