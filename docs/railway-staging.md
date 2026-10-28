# Private Railway staging preparation

Prepared for review; no Railway project, domain, database, volume, deployment or
secret was created. This is a fresh isolated staging environment, not a migration
of the local evaluation database. The approved `reference-web` experience is
served by Next.js without replacing its layout, logo or animations.

## Services and network boundary

| Service | Image definition | Listen / readiness | Exposure and storage |
| --- | --- | --- | --- |
| `supa-frontend` | `deployment/frontend.Dockerfile` | `[::]:$PORT` (default 3000), `/healthz` | Only service eligible for an HTTPS staging domain; whole-site credentials required. Ephemeral filesystem. |
| `supa-api` | `deployment/api.Dockerfile` | `[::]:8080`, `/ready` | Private networking only; no public domain or TCP proxy. Ephemeral filesystem. |
| `supa-worker` | `deployment/worker.Dockerfile` | `[::]:8081`, `/ready` | Private networking only, **one replica**, disabled by default. Ephemeral filesystem. |
| `Postgres` | New Railway PostgreSQL service | Private PostgreSQL port | Separate staging database and persistent PostgreSQL volume; no public TCP proxy. |

Use repository root `/` as the build context for all three application services.
The Dockerfiles use explicit source copies and a default-deny `.dockerignore`.
They exclude local dotenv files, Git history, evaluation artifacts, databases,
wallet envelopes, keys, build output and dependency caches. No credentials are
needed during image builds. Node 24 supports the existing server-only TypeScript
imports; Rust 1.93 matches the workspace minimum. Runtime processes use non-root
users. Images are version-tagged rather than digest-pinned; resolve and pin
reviewed digests before a production release.

Railway private DNS is `<service>.railway.internal`; use the actual service port,
not the public frontend port. IPv6 wildcard listeners support Railway private
network addressing. Explicitly set `PORT=8080` and `PORT=8081` on API and worker so
Railway readiness probes target their configured listeners. Do not change a port
without updating its bind address and dependent backend URL.

## Current Railway Infrastructure as Code

`.railway/railway.ts` is the current project-level TypeScript authoring file.
It declares a new PostgreSQL database and three services from the official GitHub
repository on `main`, using the three Dockerfiles, one replica each, readiness
routes and three failure restarts. Every service starts without public domains or
TCP proxies. A staging-only environment guard rejects production, development and
missing context. No generated domain or database dump is imported.

The official `railway` IaC SDK is pinned to **3.12.0** in `.railway/package.json`
and its lockfile. Install it using `npm ci --prefix .railway --ignore-scripts`.
The SDK supports Node 22+; local authoring validation uses Node 24. The actual IaC
engine requires Railway CLI **5.42.1 or newer**. No CLI login, link, plan, apply,
push or deployment has been run. Legacy per-service JSON manifests are not used:
current official documentation says new services cannot opt into that deprecated
format and legacy support ends on 2026-12-01.

Secrets and the exact future site origin reference pre-existing Railway **shared
variables**, with no literal secret values and no local dotenv loading. Create
these new staging shared variables through an approved account workflow:
`SUPA_SITE_ORIGIN`, `SUPA_STAGING_USERNAME`, `SUPA_STAGING_PASSWORD`,
`SUPA_WEB_USERNAME`, `SUPA_WEB_PASSWORD`, `SUPA_API_TOKEN`. SDK references do not
create their values. After account setup is authorized, verify the references
resolve before approving any plan. Do not use CLI value-printing options.

Offline checks, without any Railway credentials or network requests:

```sh
npm --prefix .railway run check
node web/node_modules/typescript/bin/tsc --noEmit --module nodenext --target es2022 --skipLibCheck --types node --typeRoots web/node_modules/@types .railway/railway.ts
```

Optional local format/presence preflight (no dotenv loading, no network, no values
printed): `npm --prefix .railway run preflight`. It exits nonzero with missing
variable names when none are provided. Passing local preflight does not prove
that Railway account shared variables exist. It also checks separate passwords,
minimum password/token lengths and an exact HTTPS origin. Do not paste credentials
into shell command arguments or commit them.

After explicit staging account authorization, link **a new isolated project** and
select its `staging` environment. The authoring file targets `Supa Agent Staging`;
review the account/project/environment IDs before a read-only
`railway config plan`. No `apply` is authorized here. Review that plan for exactly
four fresh resources, no deletions, no public domains/proxies, correct Dockerfiles,
database private references, one disabled worker and no evaluation data. Configure
the reviewed origin and visitor gate before requesting a frontend domain. Adding
a domain and applying the plan each require approval.

## Required runtime variables

Set values through Railway's service variables after approval. Generate new
staging credentials locally in a password manager; do not reuse local evaluation
keys, account passwords, bearer tokens, wallets or databases. No variable may use
a `NEXT_PUBLIC_` prefix for sensitive configuration.

| Service | Variable | Required value / purpose |
| --- | --- | --- |
| Frontend | `SUPA_SITE_MODE` | `staging` (also present during the frontend build; contains no secret) |
| Frontend | `SUPA_SITE_ORIGIN` | Exact `https://` staging origin, no alternate or wildcard hosts |
| Frontend | `SUPA_STAGING_USERNAME`, `SUPA_STAGING_PASSWORD` | Separate whole-site viewer gate; password at least 24 characters |
| Frontend | `SUPA_WEB_USERNAME`, `SUPA_WEB_PASSWORD` | Separate operator credentials; password at least 24 characters. Viewer access does not grant private records. |
| Frontend | `SUPA_BACKEND_URL` | `http://supa-api.railway.internal:8080` |
| Frontend + API | `SUPA_API_TOKEN` | Same newly generated server-only API bearer token, at least 32 printable ASCII characters |
| API + worker | `DATABASE_URL` | IaC reference to `supa-staging-postgres` private `DATABASE_URL`; verify it resolves to the private host |
| API + worker | `SUPA_PRIVATE_STAGING` | `true` |
| API + worker | `SUPA_PROVIDER` | `offline`; paid providers rejected by the staging runtime |
| API | `SUPA_BIND`, `PORT` | `[::]:8080`, `8080` |
| API | `SUPA_API_SCHEDULER_ENABLED` | `false`; worker is the only future scheduler |
| API | `SUPA_WALLET_ENABLED` | `false`; no vault, master-key file or signing material is provisioned |
| Worker | `SUPA_WORKER_BIND`, `PORT` | `[::]:8081`, `8081` |
| Worker | `SUPA_WORKER_ENABLED` | **`false`**; keep paused until a separate explicit authorization |
| Worker | `SUPA_WORKER_POLL_SECONDS` | `5`; supported range 1–3600 seconds |

Do not configure `OPENAI_API_KEY`, `SUPA_EVAL_DATABASE_URL`, evaluation budget
identifiers, wallet master secrets, local file paths or production URLs. The
staging database starts with schema only: no copied agents, simulated treasuries,
inference reservations, evaluation drafts, memories or credentials. An empty
directory is expected until new staging identities are explicitly provisioned.

For approved persisted wallet reads, optionally configure API
`SUPA_WALLET_READ_OPERATOR_ID` with the existing wallet owner's UUID. It scopes
GET identity, balances, treasury, payments and payment checks without enabling a
vault or any wallet writes. The UUID is not a secret; omit it on the fresh empty
database until a staging identity is approved. Results retain their observation
timestamps and can be stale. Live RPC monitoring remains a separate approval and
configuration step; combined wallet services require a vault/master-key file,
which this packaging does not import. Autonomous signing and Mainnet activity
remain disabled.

## Database and recovery

API and worker apply embedded SQLx migrations `0001`–`0004` at startup. SQLx tracks
applied checksums and serializes its migration operations; readiness is unavailable
until initialization succeeds. Bring PostgreSQL up first, then API, frontend and
the disabled worker. Back up the staging database before subsequent migrations;
inspect forward migrations before approving them. A rollback of an application
image does not automatically roll back its schema.

Agent identities, lifecycle history, cycles, actions, lessons, social drafts,
simulated ledgers and Devnet observations live in PostgreSQL. Container filesystems
are disposable. Enable database backups and test restore into a **different**
staging database before declaring recovery proven. Do not restore local or
production database dumps. Lifecycle pauses/suspensions/termination persist;
recovery must not reactivate them. In-flight cycles are reconciled on startup.

`SUPA_WORKER_ENABLED=false` is a process-wide launch gate independent of individual
agent lifecycle. Changing it to `true` is a separate authorized operation and does
not authorize paid inference. Worker startup never bypasses agent permissions,
financial policies or lifecycle eligibility. Keep API scheduling disabled to
avoid competing schedulers. Multiple worker replicas are not approved by this
staging plan.

## Health, validation and release gate

The frontend `/healthz` reveals only process readiness and bypasses the viewer
challenge for Railway. API/worker `/ready` checks database access without exposing
records or credentials; it is reachable only within the configured private
network. API `/health` remains bearer-protected. Railway uses readiness when
activating a deployment; it does not continuously monitor the endpoint afterward.
Add external monitoring and alerts through a separately approved workflow.

Local image build commands, once a Docker daemon is available:

```sh
docker build -f deployment/frontend.Dockerfile -t supa-frontend:staging .
docker build -f deployment/api.Dockerfile -t supa-api:staging .
docker build -f deployment/worker.Dockerfile -t supa-worker:staging .
```

Before any approved deployment:

1. Run Rust formatting, Clippy, unit/integration/restart tests; frontend lint,
   type checking, production build and browser regressions.
2. Build all three images, scan them and verify secret-free build contexts. Image
   compilation has **not** been verified when Docker is unavailable locally.
3. Validate the IaC plan against a new staging environment; create shared variables
   without printing them. Verify private-only API/worker/Postgres networking,
   explicit ports and one worker replica.
4. Verify viewer/operator gates are independent, exact-host/origin checks work,
   private requests fail closed, browser bundles contain no credentials and
   backend errors redact sensitive values.
5. On the fresh staging database, verify readiness, empty states and persisted
   lifecycle/receipt behavior across process restarts. Worker must report disabled
   and create no reasoning cycles. No local evidence or treasury data is imported.
6. Repeat responsive REA validation of the staged experience after approval;
   local browser validation is not evidence of Railway network behavior.
7. Run `python3 scripts/scan-secrets.py` and
   `python3 scripts/git-policy.py audit`. Obtain explicit approval for a Git push
   and, separately, deployment. Neither is authorized by this document.

Remaining account-side prerequisites are private staging authorization, current
Railway account setup, new shared secrets, a new PostgreSQL database, image build
validation, backup/restore exercise and account resource limits. No public
application availability, paid inference, real transaction, X publication,
trading, token launch or agent creation is implied by staging preparation.

## Official references

- [Current Railway Infrastructure as Code](https://docs.railway.com/infrastructure-as-code)
- [TypeScript IaC authoring reference](https://docs.railway.com/infrastructure-as-code/reference)
- [Private networking and internal service DNS](https://docs.railway.com/networking/private-networking)
- [Readiness behavior and `PORT`](https://docs.railway.com/deployments/healthchecks)
- [Volumes and separate service setup](https://docs.railway.com/guides/docker-compose)
