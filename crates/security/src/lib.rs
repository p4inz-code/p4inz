//! P4inz security subsystem.
//!
//! Authentication, authorization, validation and secret-handling
//! primitives (`docs/architecture/overview.md`).
//!
//! - [`Permission`], [`Role`] and [`PermissionSet`]: the role-based
//!   permission model (Milestone 07).
//! - [`authorize`]: the audited authorization decision boundary, composing
//!   [`PermissionSet`] with `p4inz_audit` (Milestone 09).
//! - [`constant_time_eq`]: timing-safe byte comparison, for future
//!   webhook/token verification (Milestone 09).
//! - [`RateLimiter`]: transport-agnostic per-key rate limiting (Milestone 10).
//!
//! Deliberately not yet included: session/token issuance, webhook HMAC
//! verification wiring, and a general input-validation framework. Each
//! depends on a concrete consumer (Authentication, Discord/GitHub webhook
//! integration, API Security) that does not exist yet — adding them now
//! would be speculative.

mod authorize;
mod constant_time;
mod permission;
mod permission_set;
mod rate_limit;
mod role;

pub use authorize::authorize;
pub use constant_time::constant_time_eq;
pub use permission::{PERMISSION_MAX_LEN, Permission, PermissionError};
pub use permission_set::PermissionSet;
pub use rate_limit::{RateLimiter, RateLimiterConfig};
pub use role::{ROLE_NAME_MAX_LEN, Role, RoleName, RoleNameError};
