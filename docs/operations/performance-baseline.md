# Performance & Resource Baseline

`docs/development/implementation_plan.md` Milestone 67: "Baseline/resource
measurement." Numbers below were captured directly in this environment
(Windows, no live PostgreSQL/Discord/AI provider — see "What isn't
measured here").

## Release binary size

```
target/release/p4inz         ~8.46 MiB (8,872,448 bytes)
target/release/p4inz-worker  ~6.61 MiB (6,931,968 bytes)
```

`[profile.release] strip = true` (added this milestone, root `Cargo.toml`)
strips debug symbols from the compiled binary. Debug info on
Windows/MSVC is written to a separate `.pdb`, not embedded in the `.exe`,
so this specific measurement doesn't move much on this platform — but on
the Linux target `infra/docker/Dockerfile` and `infra/deployment/`
actually ship (`debian:bookworm-slim`), debug symbols *are* embedded in
the ELF binary by default, so `strip = true` has real effect exactly
where deployment size matters. No behavioral change either way — a pure
size optimization.

LTO is deliberately not enabled — see the comment next to `[profile.
release]` in `Cargo.toml` for why (marginal runtime benefit for a
mostly I/O-bound service, real build-time cost).

## Rust build time (this machine, cold `target/`)

```
cargo build --release -p p4inz -p p4inz-worker   ~27-32s
```

## Website production bundle (client-side JavaScript, gzipped)

From `npm run build` (`website/`):

```
Largest chunk (shared vendor code)   49.95 kB → 19.10 kB gzip
Second-largest chunk                 27.41 kB → 10.62 kB gzip
Everything else (per-route code)     combined well under 15 kB gzip
```

Total client JS transferred is roughly **~39 kB gzipped** for the entire
site — no client-side framework bloat, consistent with the
"Performance-conscious" design principle already documented in
`website/src/lib/styles/tokens.css` (system font stack, zero web-font
requests).

## Developer feedback loop (this machine, incremental)

```
cargo check --workspace --all-targets    ~1-2s
cargo test --workspace --all-features    ~2s   (dominated by two
                                                  deliberately-timed
                                                  outage tests, Milestone
                                                  66 — everything else
                                                  completes in
                                                  milliseconds)
cargo clippy --workspace --all-targets --all-features   ~1-2s
```

## What isn't measured here

This environment has no live PostgreSQL, Discord gateway, or AI
provider — so none of the following can be measured until run against a
real deployment:

- **API request latency/throughput** under concurrent load.
- **Database query performance** (`p4inz_search`'s full-text search,
  `PgJobRepository`'s `claim_next`) against a realistically sized
  dataset — the current test data (a handful of rows in ignored
  integration tests, Milestone 60) is far too small to be representative.
- **Worker job-processing throughput** under a real backlog.
- **Memory/CPU usage** of either process under sustained load.

These require a real deployment (`infra/deployment/` or `infra/docker/`)
and a load-testing tool (e.g. `k6`, `hey`, or `wrk` — any free,
self-hosted option; none is bundled with this repository, matching "no
mandatory paid SaaS dependency"). If/when a live environment is
available, capture at minimum: p50/p95/p99 latency for `/v1/knowledge/
search` under realistic concurrency, and `p4inz_jobs_pending`'s recovery
time after a burst of enqueues (both already observable via `/metrics`,
Milestone 51, once a real deployment is scraping it).
