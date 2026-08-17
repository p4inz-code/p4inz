# Local Development Deployment

Runs both P4inz processes directly on your machine — no containers
required (`docs/development/implementation_plan.md` section 17:
"Local development machine" is a supported deployment model on its own;
Docker/Compose is optional reproducibility tooling covered separately by
Milestone 53, under `infra/docker/`).

## Prerequisites

- The Rust toolchain pinned by this workspace (`rust-version = "1.85"` in
  the root `Cargo.toml`; `rustup show` picks it up automatically if a
  `rust-toolchain.toml` is present, otherwise install 1.85+).
- A local or self-hosted PostgreSQL instance (`docs/architecture/
  zero-cost.md`: "self-hosted PostgreSQL" is the preferred option — no
  managed/paid database is required for local development).

## Steps

1. Copy the environment template and fill in real values:

   ```bash
   cp .env.example .env
   ```

   At minimum, `DATABASE_URL` must point at a running PostgreSQL instance.
   Every other variable is optional — unset optional groups (Discord,
   GitHub, AI, web/admin auth) simply disable the feature they configure;
   see the comments in `.env.example` for which variables travel together.

2. Load `.env` into your shell (any `.env`-loading tool works; `p4inz`/
   `p4inz-worker` read configuration from the process environment, not
   from the file directly — `AppConfig::from_env`, `crates/config/src/
   app_config.rs`).

3. Run the API process:

   ```bash
   cargo run -p p4inz
   ```

   This connects to PostgreSQL, runs pending migrations automatically
   (`p4inz_database::run_migrations`, called at startup — there is no
   separate manual migration step), and serves the HTTP API on
   `API_PORT` (default `8080`). Confirm it's up:

   ```bash
   curl http://localhost:8080/health
   curl http://localhost:8080/ready
   curl http://localhost:8080/metrics
   ```

4. Run the worker process, in a separate terminal (same `.env`):

   ```bash
   cargo run -p p4inz-worker
   ```

   Handles background jobs — currently scheduled/manual GitHub
   synchronization (Milestone 35) — on the poll interval and schedule
   documented in `apps/p4inz-worker/src/main.rs`.

5. Stop either process with Ctrl-C. Both shut down gracefully: the API
   stops accepting new connections and lets in-flight requests finish; the
   worker finishes any job it has already claimed before exiting
   (`p4inz_jobs::run_worker_with`'s documented shutdown-safety guarantee)
   — see `p4inz_observability::shutdown` (Milestone 51/52).

## Logs

Both processes emit structured JSON logs to stdout (Milestone 51,
`p4inz_observability::logging::init`). Control verbosity with `RUST_LOG`
(`.env.example`; standard `tracing`/`env_logger` syntax, e.g. `debug`,
`p4inz_api=debug,info`).

## Open gap

`apps/p4inz/src/main.rs` currently starts only the HTTP API. Per
`docs/architecture/runtime.md`, the "P4inz" process is also responsible
for the Discord gateway; that wiring does not exist yet (Milestone 11
delivered a tested, working `p4inz_discord::client::build`/`run`, but no
binary calls it). This is a pre-existing gap outside this milestone's
scope (see the Milestone 52 report) — flagged here for anyone following
this guide who expects Discord functionality to be running.
