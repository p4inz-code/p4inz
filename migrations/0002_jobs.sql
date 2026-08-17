-- Job System (Milestone 33): the jobs table backing p4inz_jobs::JobRepository.
-- Status is stored as TEXT for the same reason as knowledge_items' enum-like
-- columns (0001_knowledge_items.sql) — adding a new status later doesn't
-- require an ALTER TYPE migration.
--
-- No retry/backoff columns yet (attempts, max_attempts, last_error) —
-- those land with Milestone 34 (Retry System) as a forward-only ALTER
-- TABLE, not here.

CREATE TABLE jobs (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL,
    payload TEXT NOT NULL,
    status TEXT NOT NULL,
    run_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

-- Supports p4inz_infrastructure's claim_next query: the oldest due pending
-- job. Partial (WHERE status = 'pending') so it stays small regardless of
-- how many succeeded/failed jobs accumulate.
CREATE INDEX jobs_pending_run_at_idx ON jobs (run_at) WHERE status = 'pending';
