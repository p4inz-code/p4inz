# Backup & Restore

`docs/development/implementation_plan.md` section 18: "Backup strategy",
"Backup retention policy", "Restore procedure", "Restore verification".

## What's backed up

PostgreSQL is the only persistent data store in this architecture
(`docs/architecture/runtime.md`) — no other process holds state that
outlives a request. Backing up the database is backing up everything
P4inz needs to recover.

## How

[`backup.sh`](./backup.sh) runs `pg_dump` in custom format (`-Fc`),
against whatever `DATABASE_URL` it's given, producing one timestamped,
compressed dump file per run under `BACKUP_DIR`. Custom format (rather
than a plain `.sql` dump) is chosen specifically because it supports
selective and parallel restore via `pg_restore` — see "Restore procedure"
below.

## Schedule

[`p4inz-backup.timer`](./p4inz-backup.timer) runs
[`p4inz-backup.service`](./p4inz-backup.service) (which invokes
`backup.sh`) once daily, with `Persistent=true` so a backup that was
missed while the server was off still runs on next boot — see
`infra/deployment/production/README.md` for the same systemd-based
deployment model this reuses.

Install alongside the application services:

```bash
install -m 755 infra/backup/backup.sh /opt/p4inz/bin/backup.sh
cp infra/backup/p4inz-backup.service infra/backup/p4inz-backup.timer /etc/systemd/system/
mkdir -p /opt/p4inz/backups && chown p4inz:p4inz /opt/p4inz/backups
systemctl daemon-reload
systemctl enable --now p4inz-backup.timer
```

Run it manually at any time (e.g. right before a risky migration or
deploy) with:

```bash
systemctl start p4inz-backup.service
```

or directly:

```bash
/opt/p4inz/bin/backup.sh /opt/p4inz/.env
```

## Retention policy

The default keeps the newest **14 daily backups** (`RETENTION_COUNT` in
`backup.sh`, overridable via the environment) and prunes older ones —
enough to recover from a bad deploy or data corruption discovered within
two weeks, without unbounded disk growth on a self-hosted server.

This is a local retention policy only: it protects against "yesterday's
deploy broke something," not against "the server's disk failed" or "the
server was compromised." For that, periodically copy files out of
`BACKUP_DIR` to storage on a different machine — any location you
control works (another server you own, removable media, a self-hosted
object store); this project does not mandate or wire up a specific
off-site destination, consistent with "no mandatory paid SaaS dependency"
(`docs/architecture/zero-cost.md`).

## Verifying a backup exists and looks reasonable

```bash
ls -lh /opt/p4inz/backups/
pg_restore --list /opt/p4inz/backups/p4inz-<timestamp>.dump | head
```

`pg_restore --list` reads a dump's table of contents without touching any
database — a quick sanity check that the file isn't empty or corrupt.
This is not the same as a real restore verification (below) — it can't
catch, say, a dump that completed but is missing rows.

## Restore procedure

[`restore.sh`](./restore.sh) restores a backup produced by `backup.sh`
and verifies it — "Restore verification" is a required capability
(section 18), not just "the restore command exited 0."

**It never touches an existing database, let alone the live production
one.** Every run creates a fresh, uniquely named database (e.g.
`p4inz_restore_20260117T030000Z`) and restores into that — matching this
project's database safety rule that nothing silently drops, destroys,
resets, or rewrites existing data. There is no flag to make it restore
in place.

```bash
./restore.sh /opt/p4inz/backups/p4inz-<timestamp>.dump \
    postgres://p4inz:<password>@<host>:5432/postgres
```

The second argument is a connection URL for a database the same
PostgreSQL user can run `CREATE DATABASE` against (normally the
`postgres` maintenance database) — not the `p4inz` application database
itself.

It then verifies the restore by connecting to the new database and
confirming the tables every fully migrated P4inz database must have
(`knowledge_items`, `jobs`, `_sqlx_migrations`) actually exist and
reporting row counts, failing loudly (non-zero exit, verification
database left in place for inspection) if anything is missing. Exit `0`
means the backup is a real, usable, migrated database — not just a file
that exists.

### Promoting a verified restore

Replacing a live database with a restored one is exactly the kind of
irreversible operation this run's rules require being explicit and
deliberate about — it is **not** automated here. Once you've inspected
the verification database and are confident it's what you want:

1. Stop `p4inz-api` and `p4inz-worker` (`systemctl stop p4inz-api
   p4inz-worker`) — nothing should be writing to the database during the
   swap.
2. Rename or drop the current database, then rename the verification
   database to take its place (or point `DATABASE_URL` in
   `/opt/p4inz/.env` at the verification database directly, if you'd
   rather keep the old one around under its original name for a while).
3. Restart the services.

This is a manual, operator-performed sequence by design — an automated
"restore and swap into production" script is exactly the kind of
irreversible, hard-to-reverse action that should not run unattended.
