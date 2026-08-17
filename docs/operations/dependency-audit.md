# Dependency Audit

`docs/development/implementation_plan.md` Milestone 68: "Supply-chain
review." Run with `cargo audit` (Rust) and `npm audit` (website), both
against the actual resolved lockfiles in this repository.

## Rust (`cargo audit`)

314 crate dependencies scanned. **5 advisories, 1 unmaintained warning** —
all in transitive dependencies, none in this workspace's own crates, and
none currently fixable without an upstream release this project doesn't
control:

| Crate | Advisory | Via | Fixable here? |
|---|---|---|---|
| `rsa` 0.9.10 | [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071) — Marvin Attack timing side-channel, medium (5.9) | `sqlx-mysql` (a dependency of `sqlx-macros-core` regardless of which backend feature is enabled — this workspace only ever enables sqlx's `postgres` feature; the MySQL backend, and therefore any code path that would use `rsa`, is never actually compiled into anything this project executes) | No — the advisory itself states "No fixed upgrade is available." |
| `rustls-webpki` 0.102.8 | [RUSTSEC-2026-0049](https://rustsec.org/advisories/RUSTSEC-2026-0049), [-0098](https://rustsec.org/advisories/RUSTSEC-2026-0098), [-0099](https://rustsec.org/advisories/RUSTSEC-2026-0099), [-0104](https://rustsec.org/advisories/RUSTSEC-2026-0104) — certificate-validation logic bugs | `serenity` 0.12.5 (the pinned, latest available version) → `tokio-tungstenite` 0.21.0 → `rustls` 0.22.4 → `rustls-webpki` 0.102.8 | No — 0.12.5 is the latest published `serenity`; the fix requires `serenity` itself to move to a `tokio-tungstenite`/`rustls` combination using `rustls-webpki` >=0.103.10, which is an upstream `serenity` change, not something adjustable from this repo's `Cargo.toml`. |
| `paste` 1.0.15 | Unmaintained ([RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436)) — not a vulnerability | `utoipa-axum` → `p4inz-api` | No direct replacement without dropping generated OpenAPI docs (Milestone 38), a deliberate architectural choice, not something to abandon over an unmaintained transitive proc-macro helper. |

**Verified**: `cargo tree -i` confirms each path above; none of these
three crates are reachable from code this workspace actually calls
(`rsa`/`sqlx-mysql` because only the `postgres` sqlx feature is enabled
anywhere in this workspace; `rustls-webpki`'s vulnerable certificate
paths are exercised only by whatever TLS connections `serenity`'s
gateway/HTTP client make, which is an accepted, documented dependency of
using Discord's own infrastructure at all).

**A separate, non-security observation**: `time` 0.3.55 (a `sqlx`
dependency) declares it requires Rust 1.88, while this workspace's own
`Cargo.toml` declares `rust-version = "1.85"`. The actual toolchain
building this workspace is 1.97.1, so nothing is broken today, but the
declared MSRV is not strictly accurate against the currently locked
dependency tree. Not fixed here — changing the declared MSRV is a
locked-architecture-adjacent decision (`Cargo.toml`'s `rust-version`)
outside a dependency audit's scope; flagged for whoever owns that
decision.

## Website (`npm audit`)

**1 low-severity advisory**, in a transitive dependency: `cookie` <0.7.0
([GHSA-pxg6-pf52-xh8x](https://github.com/advisories/GHSA-pxg6-pf52-xh8x)
— accepts out-of-bounds characters in cookie name/path/domain), via
`@sveltejs/kit` 2.70.2 (this project's exact installed version — also
the latest published stable release; the fix only exists in `3.0.0-next.*`
prereleases, not appropriate to adopt for a production dependency).
`npm audit fix --force` would downgrade `@sveltejs/kit` to a very old
0.0.x prerelease to "resolve" this — a strictly worse outcome, not
applied.

**Not a vulnerability, just staleness** — `npm outdated` shows three
packages with newer major versions available (`@types/node` 24→26,
`typescript` 6→7, `vitest-browser-svelte` 2→3). Left alone here: major
version bumps carry real breaking-change risk and aren't a supply-chain
security concern, so upgrading them belongs to whoever next touches that
tooling, evaluated on its own, not folded into a dependency audit.

## Summary

No exploitable, fixable vulnerability was found in either dependency
tree. Every finding above is either genuinely unreachable from this
project's own code, or blocked on an upstream release this repository
doesn't control. Re-run `cargo audit` and `npm audit` periodically (both
are free, no paid service required) — a fix may become available
upstream (a new `serenity` release, a new `@sveltejs/kit` release) without
this project needing to change anything itself.
