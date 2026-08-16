-- Knowledge Search (Milestone 21): the knowledge_items table and its
-- full-text search index. Enum-like columns (category, source_kind,
-- publication_state) are stored as TEXT rather than a Postgres ENUM type,
-- so adding a new value later doesn't require an ALTER TYPE migration —
-- validity is enforced in application code (p4inz_knowledge's typed enums).

CREATE TABLE knowledge_items (
    id UUID PRIMARY KEY,
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    source_reference TEXT,
    publication_state TEXT NOT NULL,
    version INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    synchronized_at TIMESTAMPTZ,
    search_vector TSVECTOR GENERATED ALWAYS AS (
        setweight(to_tsvector('english', title), 'A') ||
        setweight(to_tsvector('english', body), 'B')
    ) STORED
);

-- A source can be synchronized into at most one knowledge item
-- (p4inz_knowledge::synchronize::synchronize_from_source correlates by
-- this column). Multiple items with no source_reference are fine
-- (administrator-authored content need not be source-linked).
CREATE UNIQUE INDEX knowledge_items_source_reference_idx
    ON knowledge_items (source_reference)
    WHERE source_reference IS NOT NULL;

CREATE INDEX knowledge_items_search_idx ON knowledge_items USING GIN (search_vector);
