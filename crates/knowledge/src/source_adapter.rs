use std::future::Future;
use std::time::SystemTime;

use p4inz_errors::AppResult;

/// Content fetched from an external source, before normalization into a
/// [`crate::KnowledgeItem`].
///
/// Intentionally minimal (title/body only, plus when it was fetched) —
/// normalization, versioning and conflict handling against any existing
/// item are Knowledge Synchronization's job (Milestone 20), not this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDocument {
    pub title: String,
    pub body: String,
    pub fetched_at: SystemTime,
}

/// Fetches raw content from an external, authoritative source
/// (`docs/PROJECT_SPEC.md` section 5: "GitHub may be used for frequently
/// changing project information"; `docs/development/
/// implementation_plan.md` section 6: "Source -> Source Adapter ->
/// Ingestion -> ...").
///
/// No concrete implementation lives here — this crate must not depend on
/// `reqwest` or any other infrastructure detail. A GitHub-backed
/// implementation belongs to `p4inz-infrastructure`.
///
/// `reference` is adapter-specific (e.g. an `"owner/repo"` string for a
/// GitHub adapter) — this trait doesn't prescribe its shape.
pub trait SourceAdapter {
    fn fetch(&self, reference: &str) -> impl Future<Output = AppResult<RawDocument>> + Send;
}
