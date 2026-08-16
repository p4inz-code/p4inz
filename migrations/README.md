# Migrations

Applied via SQLx (`sqlx::migrate!`, run by `p4inz_database::run_migrations`).
SQLx tracks applied migrations itself in a `_sqlx_migrations` table; running
migrations is idempotent and safe on every startup.

## Convention

Simple (non-reversible) migrations, one file per change:

```
migrations/
  0001_short_description.sql
  0002_another_change.sql
```

- Four-digit, zero-padded, strictly increasing sequence number, then an
  underscore, then a short `snake_case` description.
- Each file contains forward-only SQL. There is no paired `.down.sql` —
  correcting a mistake means writing a new forward migration, not editing or
  reverting a merged one (`docs/PROJECT_SPEC.md` section 6: "Historical
  versions must not be silently destroyed").
- Migrations must never be destructive (dropping/truncating tables or
  columns that may hold real data) without explicit, separate authorization
  — see `AGENTS.md` and `docs/development/implementation_plan.md` section 1.
- No product schema is defined yet; the first real migration lands with the
  milestone that needs it (e.g. the Knowledge Model).
