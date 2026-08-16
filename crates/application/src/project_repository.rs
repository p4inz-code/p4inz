use std::future::Future;

use p4inz_domain::{Project, ProjectId};
use p4inz_errors::AppResult;

/// The persistence contract for [`Project`], owned by the application
/// layer so that infrastructure implements it rather than the other way
/// around (`docs/architecture/dependency-rules.md`: "Infrastructure
/// implements contracts required by application/domain").
///
/// No concrete implementation lives here — this crate must not depend on
/// `sqlx`/PostgreSQL or any other infrastructure detail. A PostgreSQL-backed
/// implementation belongs to `p4inz-infrastructure` (or `p4inz-database`),
/// wired in by the composition root.
///
/// Methods return `impl Future + Send` rather than using `async fn` in the
/// trait directly, so the returned futures are usable from a
/// multi-threaded Tokio runtime (`async fn` in traits cannot express a
/// `Send` bound on stable Rust).
pub trait ProjectRepository {
    /// Persists a new project. Implementations decide how to react to an
    /// already-existing id (e.g. `ErrorKind::Conflict`).
    fn save(&self, project: &Project) -> impl Future<Output = AppResult<()>> + Send;

    /// Looks up a project by id. Returns `Ok(None)` if it does not exist —
    /// absence is not itself an error.
    fn find_by_id(&self, id: ProjectId) -> impl Future<Output = AppResult<Option<Project>>> + Send;
}
