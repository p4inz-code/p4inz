# Operations Runbook

The single entry point for running P4inz day-to-day. Detailed how-tos
already live under `infra/` and are linked from here rather than
duplicated — this document is where an operator starts, not the only
place they'll need.

## System overview

Two long-running processes plus PostgreSQL (`docs/architecture/
runtime.md`):

- **P4inz** (`apps/p4inz`) — the HTTP API. Also documented as responsible
  for the Discord gateway; see "Known limitations" below.
- **P4inz Worker** (`apps/p4inz-worker`) — background jobs (currently
  GitHub synchronization; Milestone 35).
- **PostgreSQL** — the only persistent data store. Everything either
  process needs to recover lives here.

## Deploying

Three supported paths, pick one:

- [`infra/local/README.md`](../../infra/local/README.md) — run both
  binaries directly, no containers, for local development.
- [`infra/deployment/production/README.md`](../../infra/deployment/production/README.md)
  (and `staging/`) — systemd services on an owned or self-hosted server.
- [`infra/docker/README.md`](../../infra/docker/README.md) — Docker
  Compose, for reproducibility.

Configuration is `.env.example` (repository root) in every case — copy
it, fill in real values. `P4INZ_ENV=production` activates stricter
validation (Milestone 56 — see "Configuration hardening" below).

## Health & observability

- `GET /health` — liveness. Always `200` once the process is up; does not
  check dependencies.
- `GET /ready` — readiness. `200` only if PostgreSQL is reachable, `503`
  otherwise.
- `GET /metrics` — Prometheus plain text (Milestone 51/57). Any
  self-hosted Prometheus can scrape this; no paid metrics backend
  required. Currently exposes:
  - `p4inz_http_requests_total`, `p4inz_http_request_errors_total`,
    `p4inz_http_request_duration_ms_{sum,count}` — every request, via
    `p4inz_api::request_tracing`.
  - `p4inz_ai_requests_total`, `p4inz_ai_request_errors_total`,
    `p4inz_ai_request_duration_ms_{sum,count}` — AI provider calls.
  - `p4inz_database_pool_connections`,
    `p4inz_database_pool_idle_connections` — live PostgreSQL pool state.
  - `p4inz_jobs_pending`, `p4inz_jobs_running`, `p4inz_jobs_succeeded`,
    `p4inz_jobs_failed` — job counts by status (Milestone 36's stated
    purpose for `p4inz_jobs::job_stats`, completed here). Omitted from
    the response (rather than reported as zero) if the underlying query
    fails or takes longer than 2 seconds — a struggling database
    shouldn't also make the endpoint that helps diagnose it unavailable.
  - In production, restrict `/metrics` to your own network — see the
    `Caddyfile` in `infra/deployment/production/`.
- **Logs**: structured JSON to stdout (`journalctl -u p4inz-api -f`,
  `docker compose logs -f`, or just watch stdout locally). Verbosity via
  `RUST_LOG` (`.env.example`). Every HTTP request and Discord interaction
  is tagged with a correlation id (`request_id`/`interaction_id`/
  `message_id` — Milestone 51) that ties every log line for that request
  together; the API also echoes it back as an `X-Request-Id` response
  header.
- Security/audit events are logged at the distinct `audit` tracing
  target, separate from ordinary application logs — filter for
  `"target":"audit"` in the JSON output.

## Backup & restore

[`infra/backup/README.md`](../../infra/backup/README.md) — scheduled
daily backups, retention policy, and the restore procedure (which always
restores into a new, separate database and verifies it — never in place;
see that document for why and how to promote a verified restore).

## Performance & resource baseline

[`performance-baseline.md`](./performance-baseline.md) — release binary
size, build times, website bundle size, and what would need a real
deployment to measure (request latency, throughput, memory/CPU under
load).

## Dependency audit

[`dependency-audit.md`](./dependency-audit.md) — `cargo audit`/`npm
audit` findings and why each one is currently unfixable from this repo
(all transitive, all blocked on an upstream release). Re-run
periodically — a fix may land upstream without anything here needing to
change.

## Configuration hardening

`P4INZ_ENV=production` (Milestone 56) requires:

- `AUTH_SESSION_SECRET` at least 32 characters, in **every** environment
  — it signs session tokens via HMAC-SHA256.
- `AUTH_REDIRECT_URI` must be `https://` in production.
- Every entry in `API_ALLOWED_ORIGINS` must be `https://` in production.

`AppConfig::from_env` fails fast (refuses to start) if any of these are
violated — see `crates/config/src/app_config.rs`.

## Common tasks

**Deploying an update** — see the "Deploying an update" section of
`infra/deployment/production/README.md` (systemd) or rebuild+restart the
relevant `docker compose` service. Database migrations run automatically
at process startup; there is no separate manual migration step.

**Restarting** — `systemctl restart p4inz-api p4inz-worker` (or
`docker compose restart`). Both processes handle `SIGTERM` gracefully
(Milestone 52): the API stops accepting new connections and lets
in-flight requests finish; the worker finishes any job it has already
claimed before exiting.

**Rotating a secret** (`DISCORD_TOKEN`, `GITHUB_TOKEN`, `AI_API_KEY`,
`DISCORD_CLIENT_SECRET`): generate the new credential with the relevant
provider, update it in `.env`, then restart the affected process(es).
None of these are cached anywhere beyond the running process's memory,
so a restart is the entire procedure.

**Rotating `AUTH_SESSION_SECRET`** is different: every session token
signed with the old secret stops validating the instant you switch it —
every logged-in web/admin user is signed out simultaneously (re-running
"Sign in with Discord" is all that's needed to recover; nothing else is
affected). There's no dual-key transition mechanism, so treat this as a
deliberate, briefly-disruptive action, not a routine one — rotate it if
you suspect it's been exposed, not on an arbitrary schedule.

**Checking job health** — `p4inz_jobs_failed`/`p4inz_jobs_pending` in
`/metrics` for the aggregate picture; structured logs (search for
`"job failed permanently"` or `"job failed; scheduling retry"`, both
include `job_id`/`kind`) for a specific job. There is no admin UI or API
to list individual failed jobs yet — for ad-hoc inspection, query
PostgreSQL directly:

```sql
SELECT id, kind, attempts, max_attempts, last_error, updated_at
FROM jobs
WHERE status = 'failed'
ORDER BY updated_at DESC
LIMIT 20;
```

## Troubleshooting

| Symptom | Check |
|---|---|
| `/ready` returns `503` | PostgreSQL is unreachable — check `DATABASE_URL`, network, and that PostgreSQL is actually running. |
| Elevated `p4inz_http_request_errors_total` | `journalctl -u p4inz-api`, filter for `"request failed"`; each line's `request_id` also appears in the client's `X-Request-Id` response header if you have it. |
| `p4inz_jobs_failed` climbing | See "Checking job health" above. |
| Discord bot not responding at all | See "Known limitations" below — this is expected today, not a bug to chase. |
| A dependency (GitHub, AI provider) is down | Deterministic fallback / bounded retries are by design (`docs/PROJECT_SPEC.md` section 7, Milestone 34) — check `p4inz_ai_request_errors_total` and job retry logs before assuming an outage on P4inz's own side. |

## Known limitations

- **The Discord gateway is not started by any binary.** Per
  `docs/architecture/runtime.md`, the "P4inz" process is documented as
  responsible for both the HTTP API and the Discord gateway;
  `apps/p4inz/src/main.rs` currently starts only the HTTP API. The
  gateway client itself (`p4inz_discord::client::build`/`run`) is built
  and tested, just not wired into a running binary. Flagged consistently
  across `infra/local/`, `infra/deployment/production/`, and
  `infra/docker/`'s READMEs — this is the single source of truth for it.
- **Docker/Compose (`infra/docker/`) was authored and statically
  reviewed but not build-verified** — Docker is unavailable in the
  environment these milestones were implemented in. Verify with
  `docker compose up --build` before relying on it in production.
- **No admin surface for individual job inspection** — only aggregate
  counts (`/metrics`) and structured logs; see "Checking job health."
