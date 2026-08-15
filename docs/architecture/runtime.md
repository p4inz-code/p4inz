# Runtime Architecture

## Processes

### P4inz

Responsible for:
- HTTP API
- Discord gateway
- request orchestration
- lightweight application work

### P4inz Worker

Responsible for:
- synchronization
- indexing
- embeddings
- scheduled tasks
- retries
- reconciliation

### PostgreSQL

Primary persistent data store.

## Deployment

The default deployment must support self-hosting at zero recurring infrastructure cost.

Cloudflare Pages may host static content.

Future paid infrastructure must not require rewriting the application.
