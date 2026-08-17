#!/usr/bin/env bash
set -euo pipefail

# Restores a P4inz backup into a NEW, separate database for verification
# (`docs/development/implementation_plan.md` Milestone 55: Restore —
# verified recovery; section 18: "Restore procedure", "Restore
# verification").
#
# Deliberately never restores into an existing database, let alone the
# live production one — "Database Safety: never silently drop/destroy/
# reset/rewrite; STOP if irreversible and unspecified" (this run's
# operating rules). This script always creates a fresh, uniquely named
# database and restores into that, so running it can never overwrite or
# destroy anything. Promoting a verified restore to replace a live
# database is a separate, explicit, manual step — see
# infra/backup/README.md's "Restore procedure" section; this script does
# not perform it.
#
# Usage: restore.sh <dump-file> <admin-database-url>
#   <dump-file>          A .dump file produced by backup.sh.
#   <admin-database-url> A DATABASE_URL with CREATEDB privilege — normally
#                        the "postgres" maintenance database, not the
#                        `p4inz` application database itself, e.g.
#                        postgres://user:pass@host:5432/postgres
#
#                        Assumed to be a plain connection URI whose final
#                        path segment is the database name (no query
#                        string) — this script derives the new database's
#                        URL by replacing that segment.

dump_file="${1:?usage: restore.sh <dump-file> <admin-database-url>}"
admin_url="${2:?usage: restore.sh <dump-file> <admin-database-url>}"

if [ ! -f "$dump_file" ]; then
    echo "error: dump file not found: $dump_file" >&2
    exit 1
fi

restore_db="p4inz_restore_$(date -u +%Y%m%dT%H%M%SZ)"
restore_url="${admin_url%/*}/$restore_db"

echo "creating verification database: $restore_db"
psql "$admin_url" -v ON_ERROR_STOP=1 -c "CREATE DATABASE \"$restore_db\";"

echo "restoring $dump_file into $restore_db"
pg_restore --dbname="$restore_url" --no-owner --no-privileges "$dump_file"

echo "verifying restored data"
# Every table a fresh, fully migrated P4inz database must have
# (migrations/0001_knowledge_items.sql, migrations/0002_jobs.sql;
# _sqlx_migrations is sqlx's own migration-tracking table — see
# crates/database/src/migrate.rs) — presence of all three is evidence the
# restore actually captured a complete, migrated schema, not just "some
# tables exist."
missing=0
for table in knowledge_items jobs _sqlx_migrations; do
    exists=$(psql "$restore_url" -t -A -c \
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '$table');")
    if [ "$exists" != "t" ]; then
        echo "error: expected table '$table' is missing from the restored database" >&2
        missing=1
    fi
done

if [ "$missing" -ne 0 ]; then
    echo "restore verification FAILED — see errors above." >&2
    echo "verification database left in place for inspection: $restore_db" >&2
    exit 1
fi

knowledge_count=$(psql "$restore_url" -t -A -c "SELECT count(*) FROM knowledge_items;")
job_count=$(psql "$restore_url" -t -A -c "SELECT count(*) FROM jobs;")

echo "restore verification passed."
echo "  knowledge_items rows: $knowledge_count"
echo "  jobs rows:            $job_count"
echo
echo "Verification database: $restore_db"
echo "Connect with: psql \"$restore_url\""
echo
echo "This script never touches the live/production database. To promote"
echo "this restore, see infra/backup/README.md's 'Restore procedure'."
echo "Once you're done inspecting it, drop the verification database with:"
echo "  psql \"$admin_url\" -c 'DROP DATABASE \"$restore_db\";'"
