# Docker/Compose Deployment

Reproducible deployment via containers (Milestone 53) — optional, per
`docs/development/implementation_plan.md` section 17: "Docker/Compose may
be used where it improves reproducibility." `infra/deployment/` (plain
systemd services, Milestone 52) and `infra/local/` (no containers at all)
are equally valid alternatives; nothing else in this repository requires
Docker.

## What's here

- `Dockerfile` — multi-stage build producing two minimal runtime images
  (`api` and `worker` targets) from one shared compile stage.
- `docker-compose.yml` — runs `api`, `worker`, and a `postgres` service
  together.

## Usage

```bash
cp ../../.env.example .env   # from infra/docker/ — fills in infra/docker/.env
echo "POSTGRES_PASSWORD=$(openssl rand -hex 32)" >> .env
# edit .env: fill in any other values you need (DISCORD_TOKEN,
# GITHUB_TOKEN, AI_*, etc.) — leave DATABASE_URL alone, docker-compose.yml
# overrides it to point at the `postgres` service.
#
# POSTGRES_PASSWORD is a Docker-Compose-only addition, not part of the
# shared .env.example template — the plain-process deployment
# (infra/deployment/) and local dev (infra/local/) point DATABASE_URL at
# an already-running PostgreSQL instance instead, so they never need a
# separate password variable.

docker compose up --build -d
docker compose ps
curl http://localhost:8080/health
curl http://localhost:8080/ready
docker compose logs -f api worker
```

Stop with `docker compose down` (add `-v` to also discard the PostgreSQL
volume — never do this against real data without a verified backup; see
Milestone 54/55). Both `api` and `worker` run their binary directly as
container PID 1 (`ENTRYPOINT` in exec form, no shell wrapper), so
`docker compose stop`'s `SIGTERM` reaches
`p4inz_observability::shutdown::wait_for_shutdown_signal` (Milestone 51/
52) the same way it does under systemd — graceful shutdown, not a kill.

## Environment limitations

Docker is not available in the environment this milestone was implemented
in (`docker --version` fails: command not found). The `Dockerfile` and
`docker-compose.yml` above were authored and statically reviewed —
syntax, stage references, health checks, and variable substitution were
checked by hand — but **not build-verified or run-verified** against a
real Docker daemon. Verify with `docker compose up --build` in an
environment with Docker installed before relying on this in production.

## Known gap

Same as `infra/deployment/production/README.md`: `apps/p4inz/src/main.rs`
does not yet start the Discord gateway (per `docs/architecture/
runtime.md`, it's documented as part of the same process as the HTTP
API), so the `api` container serves HTTP only.
