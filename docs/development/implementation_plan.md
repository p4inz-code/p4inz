# P4inz — Implementation Plan

**Status:** IMPLEMENTATION-READY  
**Specification:** PRE-FREEZE / FINAL AUDIT REQUIRED  
**Planning target:** ~99% implementation readiness  
**Implementation model:** One focused milestone = one implementation session  
**Audit time:** Separate from implementation-session count  
**Primary priority:** Correctness > Security > Reliability > Maintainability > Speed

---

## 1. Implementation Contract

P4inz is a serious, long-term Northbyte Studios product intended to remain maintainable for years without requiring a fundamental rewrite.

Implementation MUST:

- Follow all locked specifications.
- Preserve the established repository architecture.
- Prefer mature, maintained and well-supported technologies.
- Avoid unnecessary dependencies.
- Avoid mandatory paid infrastructure.
- Keep local/self-hosted operation possible.
- Treat security as a first-class architectural requirement.
- Keep deterministic functionality independent from AI availability.
- Never silently change a locked decision.
- Never remove tests to make CI pass.
- Never weaken security for convenience.
- Never introduce destructive migrations without explicit justification.
- Never commit secrets.
- Never push unless explicitly instructed.
- Never perform unrelated refactors during a milestone.
- Never introduce speculative infrastructure without a concrete requirement.
- Never sacrifice correctness to satisfy the one-hour session target.

If implementation conflicts with a locked specification:

```text
STOP
  ↓
REPORT THE CONFLICT
  ↓
IDENTIFY THE AFFECTED DECISION
  ↓
CREATE / UPDATE ADR
  ↓
UPDATE THE SPECIFICATION
  ↓
RESUME IMPLEMENTATION
2. Core Technology Stack
Layer	Locked Decision
Primary language	Rust
Async runtime	Tokio
Discord	Serenity
Backend	Axum
API style	REST
API contract	OpenAPI
Database	PostgreSQL
Database access	SQLx
Migrations	SQLx migrations
Serialization	Serde
HTTP client	Reqwest
Observability	tracing + tracing-subscriber
Error handling	thiserror + application error types
Web	TypeScript + SvelteKit
Web deployment	Static-first
Search	PostgreSQL-first hybrid architecture
AI architecture	Provider abstraction
Local AI	First-class
Online AI	Optional
Deployment	Local/self-hosted first-class
Containerization	Docker/Compose where useful
Database clarification

PostgreSQL is the canonical production database.

SQLite may be used for isolated tests, tooling or development utilities where appropriate, but P4inz does not promise production feature parity between PostgreSQL and SQLite.

3. Architectural Principles

The system follows a modular-monolith architecture with strong internal boundaries.

                         ┌──────────────────┐
                         │     Discord      │
                         │    Serenity      │
                         └────────┬─────────┘
                                  │
                         ┌────────▼─────────┐
                         │   Application    │
                         │     Layer        │
                         └───────┬─┬────────┘
                                 │ │
                 ┌───────────────┘ └───────────────┐
                 │                                 │
        ┌────────▼────────┐               ┌────────▼────────┐
        │    Knowledge    │               │       AI        │
        │    + Search     │◄─────────────►│   Read-only     │
        └────────┬────────┘               └─────────────────┘
                 │
        ┌────────▼────────┐
        │   PostgreSQL    │
        │     + SQLx      │
        └─────────────────┘


                         ┌──────────────────┐
                         │      Axum API    │
                         └────────┬─────────┘
                                  │
                         ┌────────▼─────────┐
                         │ SvelteKit Web UI │
                         │   static-first   │
                         └──────────────────┘


                         ┌──────────────────┐
                         │  Worker / Jobs   │
                         │ GitHub Sync etc. │
                         └──────────────────┘
Core rule

Discord and website layers are adapters/clients.

Business logic belongs in the domain/application layers.

No UI, Discord handler or API handler may become the source of business truth.

4. Architecture Boundaries
Domain

Owns:

Core entities.
Domain rules.
Invariants.
Domain-level values.
Domain errors.

Must not depend on:

Discord.
HTTP.
Web framework.
Database implementation.
AI provider.
Application

Owns:

Use cases.
Orchestration.
Authorization decisions.
Transaction boundaries.
Coordination between domain services and infrastructure.
Infrastructure

Owns:

PostgreSQL.
SQLx.
GitHub integrations.
AI provider implementations.
Discord integration.
External HTTP.
File/system integrations.
API

Owns:

HTTP transport.
Request validation.
Authentication boundary.
Response serialization.
OpenAPI contract.
Discord

Owns:

Gateway.
Commands.
Discord-specific interaction handling.
Discord permissions translation.
Discord response formatting.
Web

Owns:

Presentation.
Client-side interaction.
Accessibility.
API consumption.
5. Data Flow
Discord request
Discord Event
    ↓
Discord Adapter
    ↓
Identity Resolution
    ↓
Authorization
    ↓
Application Use Case
    ↓
Knowledge / Search / AI / Domain
    ↓
Response Validation
    ↓
Discord Response
    ↓
Audit where applicable
Website request
Browser
    ↓
SvelteKit
    ↓
Versioned API
    ↓
Authentication
    ↓
Authorization
    ↓
Application Use Case
    ↓
Knowledge / Search / Domain
    ↓
API Response
Knowledge ingestion
Source
    ↓
Source Adapter
    ↓
Ingestion
    ↓
Normalization
    ↓
Validation
    ↓
Versioning
    ↓
Authority / Freshness
    ↓
Draft
    ↓
Review
    ↓
Published
    ↓
Search Index
    ↓
Retrieval
AI request
User Request
    ↓
Input Validation
    ↓
Authorization
    ↓
Conversation Context
    ↓
Knowledge Retrieval
    ↓
Permission Filtering
    ↓
Evidence Validation
    ↓
AI Provider
    ↓
Output Validation
    ↓
Confidence / Evidence Check
    ↓
Response
6. Knowledge Architecture

Knowledge is a controlled source-of-truth system.

Lifecycle
Draft
  ↓
Review
  ↓
Published
  ↓
Archived

Every published knowledge item should contain, where applicable:

Source.
Source authority.
Version.
Created timestamp.
Updated timestamp.
Synchronization timestamp.
Provenance.
Publication state.
Rules
GitHub is authoritative only for explicitly assigned knowledge categories.
Admin-maintained information may have higher authority for designated categories.
Conflicting sources must be detected.
Historical versions must not be silently destroyed.
AI cannot directly publish authoritative knowledge.
Retrieval must respect permissions.
Stale information must be detectable.
Source changes must not automatically destroy valid existing knowledge.
Synchronization failures must preserve the last valid state.
7. AI Architecture

AI is an assistant, not the source of truth.

Provider model
AI Provider Interface
       │
       ├── Local Provider
       │
       └── Optional Online Provider

The core application must not depend directly on a specific AI provider.

AI rules

AI MUST:

Use retrieved evidence for factual responses when applicable.
Distinguish evidence from inference.
Respect application authorization.
Use only authorized read-only tools in V1.
Never mutate authoritative knowledge directly.
Never bypass permissions.
Never execute arbitrary OS commands.
Never become the authorization mechanism.
Handle prompt injection defensively.
Handle unavailable providers gracefully.
Never claim verification when evidence was unavailable.
Preserve source provenance in the internal response pipeline.
AI availability

P4inz must remain useful without AI.

Deterministic features must continue working when:

Local AI is unavailable.
Online AI is unavailable.
An AI provider times out.
A provider returns an error.
8. Search Architecture

V1 uses a PostgreSQL-first search architecture.

The design must allow a future dedicated search engine without rewriting the application/domain layers.

Search ranking may consider:

Text relevance.
Source authority.
Freshness.
Knowledge state.
Other explicitly defined ranking signals.

Semantic/vector search is optional.

V1 must function without an external vector database or external AI service.

Retrieval security

Authorization occurs before results are returned.

The search system must never rely on the AI to decide whether a user may access a result.

9. Discord Architecture

Discord is a transport/interface layer.

Requirements:

Multi-server capable architecture.
Northbyte optimized initially.
Per-guild configuration isolation.
Reliable gateway reconnect.
Bounded retry/backoff.
Discord rate-limit compliance.
Application-level rate limiting.
Per-user protection.
Per-command protection.
Global protection.
Idempotency for retryable events.
Duplicate event protection.
Clear failure UX.
Discord-specific types remain inside the Discord boundary.

Business logic must never be embedded directly in command handlers.

10. API Architecture

The API is:

REST.
Versioned.
OpenAPI-described.
Authentication-aware.
Authorization-aware.
Rate-limited.
Validation-first.
Rules
Website never accesses PostgreSQL directly.
Public endpoints expose only public information.
Administrative endpoints require authentication and authorization.
API errors use consistent structures.
CORS is explicitly configured.
API requests have bounded resource usage.
API contracts must be treated as compatibility boundaries.
11. Website Architecture

The website is:

TypeScript.
SvelteKit.
Static-first.
API-driven.
Accessible.
Performance-conscious.
Deployable using zero-cost hosting options.

The website must not contain business logic that belongs to P4inz backend services.

The website must remain deployable independently of the bot process.

12. Authentication + Authorization
Discord
Discord Identity
    ↓
Guild Membership
    ↓
Discord Role Mapping
    ↓
P4inz Permission
    ↓
Application Authorization
    ↓
Action
    ↓
Audit
Web
Web Authentication
    ↓
Identity
    ↓
P4inz Permission
    ↓
Application Authorization
    ↓
Action
    ↓
Audit

Authorization MUST fail closed.

AI output must never grant authorization.

13. Security Model

P4inz follows a zero-trust-style security model.

Security requirements include:

Formal threat modeling.
Input validation.
Output validation.
Authorization before retrieval.
Authorization before mutation.
Secret isolation.
Dependency auditing.
Rate limiting.
Resource limits.
Prompt-injection defense.
Knowledge-poisoning defense.
SSRF protection where URL fetching exists.
Auditability of security-sensitive actions.
Safe failure behavior.
No arbitrary AI command execution.

External content must be treated as potentially hostile.

This includes:

Discord content.
GitHub content.
Documentation.
Imported knowledge.
URLs.
AI-generated output.
14. Privacy

Default policy:

Collect minimum
Store minimum
Expose minimum
Retain only when justified

Rules:

Normal Discord conversations are not permanently stored.
Only explicitly retained information is persisted.
User deletion must be auditable.
Raw user content should not appear in ordinary logs.
Sensitive information should be minimized.
External AI providers receive user content only when explicitly enabled by deployment configuration.
Data access follows authorization boundaries.
15. Jobs + Synchronization

Background jobs are isolated from request handlers.

Requirements:

Dedicated worker process.
Persistent/reliable jobs where required.
Idempotent operations.
Bounded retries.
Exponential backoff.
Failure state tracking.
Dead-letter/failure handling where appropriate.
Scheduled GitHub synchronization.
Manual synchronization trigger.
Incremental synchronization where possible.
Concurrency limits.
Observability.
Safe recovery.

A failed synchronization must never destroy the last valid knowledge state.

16. Observability

P4inz uses structured observability.

Required:

Structured logs.
Metrics.
Health checks.
Readiness checks.
Correlation/request IDs.
Job tracing.
API tracing.
Discord interaction tracing.
AI latency/error metrics.
Database health metrics.
Security/audit events separated from ordinary logs.

Logs must avoid sensitive content by default.

Observability must work using free/local infrastructure.

17. Infrastructure

Infrastructure must remain portable.

Supported deployment models include:

Local development machine.
Owned server.
Self-hosted server.
Cloudflare Pages.
Cloudflare free-tier capabilities where appropriate.
Vercel free-tier capabilities where appropriate.
Local/self-hosted PostgreSQL.
Local AI models.

No mandatory paid SaaS dependency is permitted.

Docker/Compose may be used where it improves reproducibility.

18. Backup + Recovery

Production data requires:

Backup strategy.
Backup retention policy.
Restore procedure.
Restore verification.
Migration compatibility.
Recovery documentation.

A backup that has never been restored/tested is not considered verified.

19. Dependency Policy

Before adding a dependency:

Confirm a real requirement.
Prefer mature and maintained projects.
Check license compatibility.
Check security status.
Check maintenance activity.
Avoid duplicate functionality.
Avoid unnecessary framework dependencies.
Consider long-term maintenance cost.

Dependency upgrades must be deliberate.

20. Performance Principles

Priority order:

Correctness
    ↓
Security
    ↓
Reliability
    ↓
Maintainability
    ↓
Resource Efficiency
    ↓
Latency

Do not prematurely optimize.

Do not introduce expensive infrastructure before measurements justify it.

Performance budgets should be measured and documented when meaningful.

21. Testing Strategy

Testing layers:

Unit
  ↓
Domain
  ↓
Application
  ↓
Database
  ↓
API
  ↓
Discord
  ↓
AI Evaluation
  ↓
Security
  ↓
End-to-End
  ↓
Failure / Recovery

Required testing areas:

Domain invariants.
Permissions.
Authorization failures.
Knowledge provenance.
Knowledge versioning.
GitHub synchronization.
Retry behavior.
Database integrity.
API contracts.
Discord commands/events.
AI evidence handling.
Prompt injection.
Knowledge poisoning.
Rate limiting.
Resource exhaustion.
Dependency failures.
Recovery behavior.

Tests must never be deleted simply to make CI pass.

22. CI / Quality Gates

CI should verify, as applicable:

Formatting.
Compilation.
Unit tests.
Integration tests.
Clippy/static analysis.
Dependency/security checks.
Database migration checks.
API contract checks.
Security tests.

A change that fails required gates is not release-ready.

23. Repository Stability

The repository structure is considered a long-term foundation.

Agents MUST NOT casually:

Move crates.
Rename architectural modules.
Replace frameworks.
Introduce microservices.
Merge unrelated layers.
Create duplicate implementations.
Rewrite working infrastructure.

Architectural changes require justification and an ADR.

The structure should support future:

Admin dashboard.
Additional AI providers.
Additional knowledge sources.
Additional clients.
Additional search implementations.
Additional jobs.

without rewriting the domain/application core.

24. Implementation Milestone Chart

Each major milestone is targeted as one focused implementation session.

Audit work is separate.

Phase 0 — Foundation
#	Milestone	Goal
01	Architecture Hardening	Verify workspace boundaries, dependency direction and core contracts
02	Domain Model	Implement core entities, identifiers and invariants
03	Error System	Establish typed application-wide errors
04	Configuration	Production-safe configuration
05	Database Foundation	PostgreSQL, migrations and repository foundation
Phase 1 — Core Application
#	Milestone	Goal
06	Application Services	Establish use-case architecture
07	Permission System	Roles, permissions and authorization
08	Audit System	Security/admin audit events
09	Security Foundation	Threat-model controls and security boundaries
10	Rate Limiting	User/command/global abuse protection
Phase 2 — Discord
#	Milestone	Goal
11	Discord Gateway	Connection/reconnect lifecycle
12	Discord Commands	Slash-command framework
13	Natural Language	Question/interaction pipeline
14	Discord Permissions	Guild-role mapping
15	Discord Error UX	Failure/recovery UX
Phase 3 — Knowledge
#	Milestone	Goal
16	Knowledge Model	Entities, sources and lifecycle
17	Provenance	Source/version/timestamp tracking
18	Knowledge Workflow	Draft → Review → Published → Archived
19	GitHub Ingestion	Source adapters
20	GitHub Synchronization	Incremental sync and safe updates
21	Knowledge Search	PostgreSQL full-text search
22	Ranking	Authority/freshness/relevance
23	Retrieval Permissions	Permission-aware retrieval
Phase 4 — AI
#	Milestone	Goal
24	AI Provider Contract	Provider abstraction
25	Local AI Provider	Local-model implementation
26	Optional Online Provider	External provider adapter
27	AI Context	Conversation + knowledge context
28	Evidence Pipeline	Evidence validation
29	AI Safety	Injection/tool/data protections
30	AI Response Validation	Confidence/evidence checks
31	AI Fallback	Deterministic fallback
Phase 5 — Worker / Jobs
#	Milestone	Goal
32	Worker Runtime	Worker lifecycle
33	Job System	Reliable jobs
34	Retry System	Backoff and failure handling
35	GitHub Jobs	Scheduled/manual synchronization
36	Job Observability	Job status and failure visibility
Phase 6 — API
#	Milestone	Goal
37	API Foundation	Axum routing/application
38	API Contracts	OpenAPI/versioning
39	Public Knowledge API	Search/browse
40	Authentication	Web/admin authentication
41	API Authorization	Fine-grained authorization
42	API Security	Validation/rate limiting/errors
Phase 7 — Website
#	Milestone	Goal
43	Website Foundation	SvelteKit foundation
44	Design System	Visual tokens/components
45	Public Home	Product identity
46	Documentation	Public docs
47	Knowledge Explorer	Search/browse
48	Provenance UI	Source/version visibility
49	API Integration	Typed API client
50	Accessibility/Performance	Production UX hardening
Phase 8 — Operations
#	Milestone	Goal
51	Observability	Logs/metrics/health
52	Deployment	Local/self-hosted deployment
53	Docker/Compose	Reproducible deployment
54	Backup	Backup strategy
55	Restore	Verified recovery
56	Production Configuration	Environment hardening
57	Operations Documentation	Operator documentation
Phase 9 — Quality
#	Milestone	Goal
58	Unit Coverage	Core/application coverage
59	Integration Tests	Cross-layer behavior
60	Database Tests	Migration/repository correctness
61	API Tests	Contract/authorization
62	Discord Tests	Commands/events/permissions
63	AI Evaluation	Deterministic evaluation
64	Security Tests	Abuse/injection/authorization
65	E2E Tests	Full-system scenarios
66	Failure Tests	Outage/retry/recovery
Phase 10 — Release
#	Milestone	Goal
67	Performance	Baseline/resource measurement
68	Dependency Audit	Supply-chain review
69	Security Hardening	Threat-model closure
70	Architecture Audit	Implementation/spec consistency
71	V1 Acceptance	Full acceptance criteria
72	Release Preparation	Version/changelog/artifacts
73	Release Candidate	Final candidate
74	Release Verification	Clean deployment/recovery
25. Milestone Execution Protocol

Every implementation session follows:

Read specification
      ↓
Inspect repository
      ↓
Inspect existing implementation
      ↓
Identify exact milestone scope
      ↓
Plan
      ↓
Implement
      ↓
Add/update tests
      ↓
Format
      ↓
Compile/check
      ↓
Run relevant tests
      ↓
Self-review
      ↓
Architecture/security review
      ↓
Report

A milestone is not complete merely because it compiles.

26. Definition of Done

A milestone is complete only when:

Intended behavior works.
Relevant tests exist.
Existing tests still pass.
Formatting passes.
Required static analysis passes.
No known security regression exists.
No architecture boundary was violated.
No unnecessary dependency was introduced.
Documentation is updated where contracts changed.
Git status has been inspected.
Implementation result is reported.
27. Audit Policy

Audits are separate from implementation-session count.

Audits may occur:

After milestones.
At phase boundaries.
Before major merges.
Before release.

Audit discoveries are handled through controlled changes.

Issue
  ↓
Classify
  ↓
Determine scope
  ↓
Fix or create follow-up
  ↓
Test
  ↓
Document if architectural

Audit time is never a reason to skip required validation.

28. Explicit V1 Scope

V1 MUST provide:

Natural-language Discord interaction.
Slash commands.
Northbyte/project knowledge.
GitHub synchronization.
Manual knowledge management.
Knowledge provenance.
Knowledge versioning.
Search.
AI retrieval.
Permission enforcement.
Audit logging.
Rate limiting.
Privacy controls.
Public P4inz website.
Public knowledge search/browse.
Versioned REST API.
OpenAPI contract.
AI-independent deterministic functionality.
Graceful failure handling.
CI/security/test gates.
Deployment documentation.
Backup/recovery documentation.
29. Explicitly Out of V1

Do not implement these unless the specification is intentionally changed:

Full community-management suite.
Full admin dashboard UI.
Autonomous AI actions.
AI-generated authoritative knowledge.
Full individual user-memory system.
Voice/music functionality.
Mandatory paid AI services.
Mandatory paid infrastructure.
Dedicated distributed search cluster.
Unnecessary microservices.
Premature multi-region infrastructure.
30. Change Control

A locked decision may only change through:

Problem discovered
      ↓
Explain conflict
      ↓
Evaluate alternatives
      ↓
Create/update ADR
      ↓
Update Decision Registry
      ↓
Update affected specifications
      ↓
Resume implementation

No silent architectural drift is permitted.

31. Implementation Session Target

Target:

One major milestone per focused session.
Approximately one hour per milestone where realistically possible.
Audits are separate.
Complex milestones may require additional sessions.
Correctness takes priority over the session target.

The milestone count is a planning target, not a reason to rush.

32. V1 Acceptance Gate

P4inz V1 is complete only when:

Product functionality       ✓
Discord                     ✓
Knowledge                   ✓
Search                      ✓
AI                          ✓
Website                     ✓
API                         ✓
Permissions                 ✓
Security                    ✓
Privacy                     ✓
Jobs                        ✓
Observability               ✓
Testing                     ✓
Deployment                  ✓
Backup                      ✓
Restore                     ✓
Documentation               ✓
Final audit                 ✓
Release verification        ✓

Additionally:

No critical security issue remains.
No known data-integrity issue remains.
No locked architectural requirement is knowingly unimplemented.
No mandatory paid service is required.
Clean deployment has been verified.
Recovery has been verified.
33. Implementation Authority

The authoritative order is:

1. Decision Registry
2. PROJECT_SPEC.md
3. Architecture specifications
4. Security specifications
5. UX specification
6. Data model
7. This Implementation Plan
8. ROADMAP.md
9. AGENTS.md
10. Existing implementation

If two authoritative documents conflict:

STOP IMPLEMENTATION and resolve the conflict.

34. Start Condition

Implementation may begin only after:

Decision Registry exists.
Final specification audit passes.
Architecture/data-flow documentation exists.
Security model exists.
UX specification exists.
Data model exists.
Development/testing specification exists.
Repository is clean.
Planning changes are committed.
Remote backup is synchronized.

Then:

P4inz Specification Freeze
          ↓
Session 01
          ↓
Architecture Hardening
35. Final Principle

P4inz is built for the long term.

The implementation process optimizes for:

Correctness
    >
Security
    >
Reliability
    >
Maintainability
    >
Performance
    >
Speed

A slower correct implementation is preferable to a fast architectural mistake.

END OF IMPLEMENTATION PLAN


