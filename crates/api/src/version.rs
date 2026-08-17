/// The current API version's URL prefix (`docs/development/
/// implementation_plan.md` section 10: "Versioned"; "API contracts must
/// be treated as compatibility boundaries").
///
/// Business endpoints (Public Knowledge API, Milestone 39, and later)
/// nest under this. `/health`, `/ready` and `/openapi.json` are
/// deliberately unversioned — they're infrastructure-level operational
/// concerns, not part of the API's versioned business contract, matching
/// the common REST convention of keeping health/readiness checks outside
/// any version prefix.
pub const API_V1_PREFIX: &str = "/v1";
