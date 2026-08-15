# P4inz Architecture

## Architecture Style

P4inz uses a Rust Cargo workspace with a modular-monolith architecture.

The system is initially deployed as:
- P4inz application
- P4inz worker
- PostgreSQL

Internal crate boundaries are explicit so components can be extracted later if justified.

## Core Boundaries

- `domain` — business entities and rules
- `application` — application use cases
- `api` — HTTP interface
- `discord` — Discord adapter
- `knowledge` — authoritative knowledge lifecycle
- `search` — retrieval and ranking
- `ai` — AI orchestration/provider abstraction
- `security` — authentication, authorization, validation, secrets
- `database` — PostgreSQL implementation
- `jobs` — asynchronous work
- `audit` — security/business audit events
- `observability` — logs, metrics, tracing, health
- `errors` — shared error taxonomy
- `infrastructure` — external implementations
- `config` — validated configuration
- `common` — minimal generic primitives

## Principles

1. Domain code does not depend on infrastructure.
2. AI does not define truth.
3. Permissions are evaluated before sensitive context reaches AI.
4. Retrieved content is untrusted data.
5. External providers are replaceable.
6. Paid infrastructure is optional.
7. Security boundaries must be explicit.
8. Production data is never treated as disposable.
