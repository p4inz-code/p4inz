#!/usr/bin/env bash
set -euo pipefail

# P4inz PostgreSQL backup (`docs/development/implementation_plan.md`
# Milestone 54: Backup strategy; section 18: "Backup strategy",
# "Backup retention policy"). PostgreSQL is the only persistent data
# store (`docs/architecture/runtime.md`), so backing it up is backing up
# everything.
#
# Requires `pg_dump` (the `postgresql-client` package on most
# distributions) and a DATABASE_URL pointing at the database to back up —
# read from the environment, or from an env file passed as $1 (e.g. the
# same /opt/p4inz/.env the application services use — see
# infra/deployment/production/).
#
# Produces one timestamped, compressed, custom-format dump per run
# (`pg_dump -Fc`: supports selective and parallel restore via
# `pg_restore`, unlike a plain SQL dump) under BACKUP_DIR, then enforces
# the retention policy below. This script only produces and retains
# backups — restoring from one is Milestone 55's concern; see
# infra/backup/README.md.

BACKUP_DIR="${BACKUP_DIR:-/opt/p4inz/backups}"
# 14 daily backups is a deliberately simple default: enough to recover
# from a bad deploy or data corruption discovered within two weeks,
# without unbounded disk growth on a self-hosted server. Not a substitute
# for an off-site copy — see infra/backup/README.md.
RETENTION_COUNT="${RETENTION_COUNT:-14}"

if [ -n "${1:-}" ]; then
    # shellcheck source=/dev/null
    source "$1"
fi

if [ -z "${DATABASE_URL:-}" ]; then
    echo "error: DATABASE_URL is not set (pass an env file as \$1, or export it directly)" >&2
    exit 1
fi

mkdir -p "$BACKUP_DIR"

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
dest="$BACKUP_DIR/p4inz-$timestamp.dump"
tmp="$dest.tmp"

echo "backing up to $dest"
pg_dump "$DATABASE_URL" -Fc -f "$tmp"
# Only the finished dump ever appears under its real name — an
# interrupted or failed pg_dump leaves (at most) a stray .tmp file, never
# a truncated backup that looks complete.
mv "$tmp" "$dest"

# Retention: keep only the newest RETENTION_COUNT backups.
mapfile -t backups < <(find "$BACKUP_DIR" -maxdepth 1 -name 'p4inz-*.dump' -type f | sort -r)
if [ "${#backups[@]}" -gt "$RETENTION_COUNT" ]; then
    for old in "${backups[@]:$RETENTION_COUNT}"; do
        echo "pruning $old"
        rm -f "$old"
    done
fi

echo "backup complete: $dest"
