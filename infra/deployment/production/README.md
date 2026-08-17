# Self-Hosted Production Deployment

A plain-process deployment on an owned or self-hosted Linux server
(`docs/development/implementation_plan.md` section 17: "Owned server",
"Self-hosted server" are both supported deployment models; no mandatory
paid SaaS dependency). Container-based deployment is optional
reproducibility tooling covered separately by Milestone 53, under
`infra/docker/` — nothing here depends on it.

## Layout this assumes

```
/opt/p4inz/
├── bin/
│   ├── p4inz            # release binary, from `cargo build --release -p p4inz`
│   └── p4inz-worker      # release binary, from `cargo build --release -p p4inz-worker`
└── .env                  # real production values — see .env.example; chmod 600, owned by the service user
```

A dedicated, unprivileged Linux user runs both processes — never root:

```bash
useradd --system --home /opt/p4inz --shell /usr/sbin/nologin p4inz
mkdir -p /opt/p4inz/bin
chown -R p4inz:p4inz /opt/p4inz
```

## Steps

1. **Build release binaries** (on the server, or elsewhere on the same
   architecture/OS and copy them over):

   ```bash
   cargo build --release --workspace
   install -m 755 target/release/p4inz /opt/p4inz/bin/p4inz
   install -m 755 target/release/p4inz-worker /opt/p4inz/bin/p4inz-worker
   ```

2. **Configure environment**: copy `.env.example` to `/opt/p4inz/.env`,
   fill in real values, and set `P4INZ_ENV=production`. Restrict
   permissions — this file holds secrets:

   ```bash
   chown p4inz:p4inz /opt/p4inz/.env
   chmod 600 /opt/p4inz/.env
   ```

3. **PostgreSQL**: point `DATABASE_URL` at a self-hosted PostgreSQL
   instance (same host or another host on the private network — this
   repository does not run PostgreSQL itself). Migrations run
   automatically at process startup (`p4inz_database::run_migrations`) —
   no separate manual step.

4. **Install the systemd units**: copy `p4inz-api.service` and
   `p4inz-worker.service` from this directory to
   `/etc/systemd/system/`, then:

   ```bash
   systemctl daemon-reload
   systemctl enable --now p4inz-api p4inz-worker
   ```

   Both units run as the `p4inz` user, load `/opt/p4inz/.env`, restart on
   failure, and shut down gracefully on `systemctl stop`/`restart`
   (`SIGTERM`, handled via `p4inz_observability::shutdown` — Milestone 51/
   52; see the comments in each unit file for why this matters).

5. **Reverse proxy + TLS**: install [Caddy](https://caddyserver.com/docs/install)
   and use `Caddyfile` from this directory (replace the placeholder
   domain first). Caddy provisions and renews a Let's Encrypt certificate
   automatically — no manual certificate management.

6. **Verify**:

   ```bash
   systemctl status p4inz-api p4inz-worker
   curl https://<your-domain>/health   # liveness
   curl https://<your-domain>/ready    # readiness — checks the database
   curl https://<your-domain>/metrics  # Prometheus text (Milestone 51); restricted to the private network by the Caddyfile above
   journalctl -u p4inz-api -f          # structured JSON logs (Milestone 51)
   ```

7. **Backups**: install `infra/backup/`'s scheduled backup timer — see
   `infra/backup/README.md`. Not optional for a real production
   deployment: "Production data is never treated as disposable"
   (`docs/architecture/overview.md`).

## Deploying an update

```bash
cargo build --release --workspace
install -m 755 target/release/p4inz /opt/p4inz/bin/p4inz
install -m 755 target/release/p4inz-worker /opt/p4inz/bin/p4inz-worker
systemctl restart p4inz-api p4inz-worker
```

Both units restart gracefully — see the shutdown behavior noted above.
There is no separate migration step to run: the new binary runs pending
migrations itself on startup.

## Known gap

`apps/p4inz/src/main.rs` starts only the HTTP API. Per
`docs/architecture/runtime.md`, the "P4inz" process is also documented as
responsible for the Discord gateway; wiring that in does not exist yet
(see `infra/local/README.md`'s note and the Milestone 52 completion
report). Deploying via this guide today gives you a working HTTP API and
worker, not a connected Discord bot.
