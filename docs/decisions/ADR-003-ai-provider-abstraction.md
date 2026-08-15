# ADR-003 — AI Provider Abstraction

## Decision

AI providers are accessed through an internal abstraction.

## Reason

P4inz must not depend on one vendor, paid API, or model.

Local models and future providers must be interchangeable without changing domain logic.
