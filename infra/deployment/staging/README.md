# Staging Deployment

Staging uses the exact same procedure as production
(`infra/deployment/production/README.md`) — same systemd units, same
`Caddyfile` shape — with two differences:

1. **`P4INZ_ENV=staging`** in `/opt/p4inz/.env`, not `production`.
2. **A separate domain** in the `Caddyfile` (e.g.
   `staging.api.p4inz.example.com`), and its own PostgreSQL database —
   never point a staging environment at the production database.

Everything else (build steps, systemd unit files, service user,
directory layout, verification commands, graceful shutdown behavior) is
identical to `infra/deployment/production/README.md`; refer to it
directly rather than duplicating the same instructions here.
