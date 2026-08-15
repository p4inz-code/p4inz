# Dependency Rules

## Direction

Domain is the most independent layer.

Application depends on domain.

Adapters such as API and Discord depend on application contracts.

Infrastructure implements contracts required by application/domain.

Database is an infrastructure implementation.

## Forbidden

- Domain importing Discord
- Domain importing HTTP frameworks
- Domain importing PostgreSQL implementations
- AI deciding authorization
- Client-provided permissions being trusted
- Random cross-layer database access
- Business logic duplicated inside Discord handlers
- Secrets inside source code

Architecture boundary violations must be fixed rather than ignored.
