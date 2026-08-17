-- Retry System (Milestone 34): adds bounded-retry/backoff bookkeeping to
-- the jobs table introduced in 0002_jobs.sql. Forward-only, as required by
-- migrations/README.md — existing rows default to zero attempts and the
-- job system's default retry cap (p4inz_jobs::DEFAULT_MAX_ATTEMPTS), which
-- is correct for the rows already present (0002 predates any retry
-- concept, so nothing has been attempted yet).

ALTER TABLE jobs
    ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN max_attempts INTEGER NOT NULL DEFAULT 5,
    ADD COLUMN last_error TEXT;
