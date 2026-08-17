# syntax=docker/dockerfile:1
#
# Multi-stage build producing the two P4inz binaries as separate, minimal
# runtime images (`docs/development/implementation_plan.md` Milestone 53:
# Docker/Compose — reproducible deployment; section 17: "Docker/Compose
# may be used where it improves reproducibility" — optional, not a
# required deployment path; see infra/deployment/ (Milestone 52) for the
# plain-process alternative).
#
# Build context is the repository root (see docker-compose.yml's
# `build.context: ../..`) — the whole workspace is needed since every
# crate is a path dependency. `.dockerignore` at the repo root keeps
# `.env`/`target/`/`.git` out of that context.
#
# Dependency compilation is not layer-cached separately from source code
# here (a common optimization) — this workspace has 18 path-dependency
# crates, and stubbing each one out just for cache layering would add
# real fragility for a build that cannot even be executed in this
# environment to verify (Docker is unavailable here — see
# infra/docker/README.md's "Environment limitations"). Simplicity was
# chosen deliberately; revisit if real build times become a problem.

FROM rust:1.85-slim-bookworm AS builder
WORKDIR /build

COPY . .
RUN cargo build --release -p p4inz -p p4inz-worker

# Every dependency in this workspace uses rustls (`reqwest`'s
# "rustls-tls" feature, serenity's "rustls_backend"), never OpenSSL — see
# crates/ai/Cargo.toml, crates/infrastructure/Cargo.toml,
# crates/discord/Cargo.toml — so the runtime image needs no TLS library
# beyond what's statically linked into the binary itself.
FROM debian:bookworm-slim AS runtime-base
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --home /app --shell /usr/sbin/nologin p4inz
WORKDIR /app
USER p4inz

FROM runtime-base AS api
COPY --from=builder /build/target/release/p4inz /usr/local/bin/p4inz
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/p4inz"]

FROM runtime-base AS worker
COPY --from=builder /build/target/release/p4inz-worker /usr/local/bin/p4inz-worker
ENTRYPOINT ["/usr/local/bin/p4inz-worker"]
