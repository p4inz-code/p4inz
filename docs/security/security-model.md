# Security Model

P4inz follows defense-in-depth and zero-trust principles.

## Mandatory Controls

- Authentication
- Resource-level authorization
- RBAC
- Input validation
- Rate limiting
- Secret isolation
- Secure sessions
- Webhook verification
- Audit logging
- Data minimization
- Dependency auditing
- Backup and recovery
- AI prompt-injection isolation

### Threat-model closure notes (Milestone 69)

- **Webhook verification** has no applicable surface today — nothing in
  this architecture receives an incoming webhook. GitHub synchronization
  is pull-based (scheduled/manual, Milestone 35), and Discord interaction
  delivery is the gateway WebSocket (Milestone 11), not an HTTP webhook.
  `p4inz_security::constant_time_eq` (built for exactly this control —
  comparing an attacker-controlled signature against a secret without a
  timing side-channel) exists and is tested, ready for whenever a real
  webhook receiver is added; it has no callers today because nothing
  needs it yet. Session-token signature verification (`p4inz_api::auth::
  session::verify_session`) is already timing-safe via a different,
  equally correct mechanism — the `hmac` crate's own `verify_slice`.
- **Secure sessions**: session-signing-secret minimum length and
  production HTTPS enforcement for the OAuth redirect/CORS origins
  closed in Milestone 56; the audit trail's fail-closed guarantee
  (a broken audit sink denies access rather than silently granting it)
  verified in Milestone 64.
- **Security response headers** (`Content-Security-Policy`,
  `X-Frame-Options`, `Strict-Transport-Security`, etc.) were not set
  anywhere before this milestone — added to `website/static/_headers`
  (Cloudflare Pages' convention) and `infra/deployment/production/
  Caddyfile` (the API's reverse proxy).

## AI

AI cannot:
- grant permissions
- access arbitrary private data
- execute arbitrary shell commands
- execute arbitrary SQL
- perform high-impact administrative actions autonomously

Unknown or unsupported information must not be fabricated.
