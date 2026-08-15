# P4inz Roadmap

P4inz is Northbyte Studios' open-source Discord intelligence and community platform.

## Project Status

- Planning: COMPLETE
- Architecture: LOCKED
- Repository foundation: IN PROGRESS
- Implementation: NOT STARTED
- Production: NOT STARTED

## Implementation Order

### Phase 1 — Repository Foundation
- Workspace
- Configuration
- Error system
- Logging/tracing
- CI foundation

### Phase 2 — Domain & Database
- Domain models
- PostgreSQL
- Migrations
- Repository abstractions
- Transactions

### Phase 3 — Application Layer
- Services
- Permissions
- Business rules
- Audit events

### Phase 4 — Discord
- Gateway
- Slash commands
- Components
- Modals
- Permissions
- Error handling

### Phase 5 — Knowledge
- Sources
- Ingestion
- Normalization
- Provenance
- Verification
- Versioning
- Freshness
- Conflict detection

### Phase 6 — Search
- Structured search
- Full-text search
- Semantic search
- Hybrid ranking

### Phase 7 — AI
- Provider abstraction
- Retrieval-augmented responses
- Grounding
- Output validation
- Safety controls
- Local-model support

### Phase 8 — Community Features
- Tickets
- Applications
- Feedback
- Partnerships
- Notices
- Project/release information

### Phase 9 — Security Hardening
- Threat-model validation
- Authorization audit
- Abuse protection
- Secret handling
- Webhook security
- Data isolation

### Phase 10 — Production
- Deployment
- Backups
- Monitoring
- CI/CD
- Recovery testing
- Production readiness audit

## Rules

Architecture changes require an ADR.
Security-critical changes require additional testing.
No paid service may be a mandatory dependency.
AI is never the source of truth.
Unknown information must remain unknown rather than being invented.
