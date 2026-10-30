# Milestone 6 — Railway staging readiness

Validation date: 2026-10-30. This is preparation, not a deployment approval.
No Railway resources, domains, volumes, secrets or deployments were created or
changed. No project was linked. No push was performed. The prepared local epoch
was neither migrated nor activated and the primary PostgreSQL cluster stayed
stopped. The approved frontend is preserved except for the requested status-label
contrast correction.

## Container readiness

| Target | Verified now | Still required before deployment |
| --- | --- | --- |
| Frontend | Production Next.js build; non-root `node` runtime declaration; `exec` startup; IPv6 wildcard and port 3000; `/healthz`; no known credentials in 26 browser chunks | Actual OCI build, runtime startup and vulnerability scan |
| API | Locked Cargo metadata with the exact six-crate Docker source subset; native Rust tests and startup; non-root UID 10001 and exec-form entrypoint; port 8080 and `/ready` | Release container build, image scan, image-level runtime test |
| Worker | Same manifest/source validation; native paused-worker tests; graceful SIGTERM; non-root UID 10001; port 8081 and `/ready`; one disabled replica | Release container build, image scan, image-level runtime test |
| PostgreSQL | SDK graph declares `ghcr.io/railwayapp-templates/postgres-ssl:18` with persistent mount `/var/lib/postgresql/data` | Actual image, PostgreSQL 18 migrations/restore and Railway volume validation |

There is no Docker, Podman, Buildah, BuildKit, RootlessKit, runc or crun installed,
no Docker/Podman socket, and only about 2 GB of disk free. No image build was run;
source checks and native execution do not establish container compatibility.
Do not remove historical evidence or databases to make space.

[Rootless BuildKit](https://github.com/moby/buildkit/blob/master/docs/rootless.md)
is a supported alternative, with RootlessKit and an OCI runtime. A user-namespace
probe succeeded, but the builder/runtime prerequisites are absent. Provision a
reviewed local builder with sufficient disk, or an explicitly authorized external
runner. No paid remote builder was provisioned. Actual image builds, image
scanning and PostgreSQL 18 remain release blockers. The local PostgreSQL tests
below used version 16; they do not certify the selected version-18 image.

Once a reviewed Docker engine and adequate storage exist, run from repository root:

```sh
docker build --pull -f deployment/frontend.Dockerfile -t supa-frontend:staging .
docker build --pull -f deployment/api.Dockerfile -t supa-api:staging .
docker build --pull -f deployment/worker.Dockerfile -t supa-worker:staging .
```

Then run all three images with newly generated disposable test credentials and an
isolated PostgreSQL 18 container/volume; bind any host ports to loopback only.
Check health, unauthorized requests, public/operator separation, zero worker
cycles, restart persistence, database outage/recovery and SIGTERM. Exercise a
backup/restore into a second disposable database. Never mount the real local
cluster, evaluation dotenv files or wallet directory. Resolve tested base-image
digests and retain the image scan results before approving the release.

## Railway account and configuration

The official CLI **5.71.0** was downloaded into `/tmp` only. Read-only `whoami
--json` succeeded. Read-only `list --json` returned two accessible projects; names
and IDs were not copied into reports. `status --json` confirmed this repository
is unlinked. CLI account credentials already existed in a mode-0600 local file;
no login or credential installation was performed. Authentication does not prove
project ownership, billing eligibility, limits or authorization to spend.

The official IaC SDK is pinned to **3.12.0**. Eight offline SDK/source tests and
TypeScript checking passed. Exactly four fresh resources are declared, scoped to
`staging`; all start without public domains or TCP proxies. API/worker/Postgres
remain private. The frontend references `http://supa-api.railway.internal:8080`.
Runtime private-DNS resolution and connectivity still need a real staging test.
See [Railway private networking](https://docs.railway.com/networking/private-networking/how-it-works).

A project/environment, billing cap, service capacity/region and real staging
operator identities have not been selected or provisioned. No account-backed
`config plan` was run. The authored database mount is not proof of an allocated,
persistent Railway volume.

Required secrets/settings contain no values in Git:

| Scope | Names / purpose |
| --- | --- |
| Shared, frontend + API | `SUPA_API_TOKEN`: newly generated public-service bearer credential, distinct from every operator |
| API only | `SUPA_OPERATOR_CREDENTIALS`: JSON `{id,role,token}` entries bound to approved real principals; no implicit ownership grants |
| Frontend only | `SUPA_WEB_OPERATORS`: JSON `{username,password,api_token}` entries mapped server-side to the API credentials |
| Frontend visitor gate | `SUPA_STAGING_USERNAME`, `SUPA_STAGING_PASSWORD`: separate from operator login/password |
| Frontend origin | `SUPA_SITE_ORIGIN`: exact approved HTTPS origin; `SUPA_SITE_MODE=staging` |
| Frontend backend | `SUPA_BACKEND_URL=http://supa-api.railway.internal:8080` |
| API + worker | Private database `DATABASE_URL` service reference; `SUPA_PRIVATE_STAGING=true`, `SUPA_PROVIDER=offline`, `SUPA_ALLOW_OPENAI=false`, `SUPA_CONTROLLED_INFERENCE=false` |
| API | `PORT=8080`, `SUPA_BIND=[::]:8080`, `SUPA_API_SCHEDULER_ENABLED=false`, `SUPA_WALLET_ENABLED=false` |
| Worker | `PORT=8081`, `SUPA_WORKER_BIND=[::]:8081`, `SUPA_WORKER_ENABLED=false`, `SUPA_WORKER_POLL_SECONDS=5` |
| Frontend port | `PORT=3000`, `[::]` listener in the image startup command |
| PostgreSQL | Newly generated database credentials, private host/port, reviewed persistent volume and backup policy |

The six shared references already appear in `.railway/railway.ts`; their real
values remain unconfigured. `preflight.mjs` checks names, formats and separation
without dotenv loading or printing values. No `OPENAI_API_KEY`, wallet secret,
evaluation URL, local owner credentials or historical budget identifier is to be
imported. All credentials remain server-side, with no secret `NEXT_PUBLIC_` values.
See [the detailed runtime contract](railway-staging.md#required-runtime-variables).

## Database and recovery evidence

A new private, Unix-socket-only PostgreSQL 16 cluster was used, separate from the
primary cluster. Test identities and funds were explicitly synthetic and offline.
Migrations **1–10** succeeded there; every stored SQLx SHA-384 checksum matches the
corresponding source migration. No migration was applied to the real database.
A fresh staging database will apply all ten embedded migrations at API/worker
startup, not just the four early milestones. Migrations do not provision owner,
custodian, executor or activation approvals.

The full-stack test verified owner-scoped reads, denied public and cross-user
spoofing, genuine persisted memory/actions, duplicate rejection and exact financial
persistence after API restart. The explicitly consented offline test agent ended
paused after three cycles, eight actions and six memories, with 3,300 simulated
microUSD expense and 6,700 simulated microUSD remaining. No paid provider ran. The test API, frontend and browser were stopped after evidence capture.

A separate native crash/recovery check forcibly stopped only the disposable
PostgreSQL cluster. Both API and worker readiness became **503**, then returned
**200** after PostgreSQL restarted. API and worker exited **0** after SIGTERM.
No additional cycles appeared while scheduling/worker gates were disabled.
This verifies native processes, not container PID/signal delivery.

With all test services stopped, a fresh mode-0600 custom-format backup was
restored into a separately named disposable database. The source before/after
and restored data/schema fingerprint matched:

`6ef8370d099c3c6c8eb973a3fdd83517a4dd285845d980031ba4fc5d43df5ac9`

Function definitions, triggers and table privileges also matched, excluding only
the deliberately different database catalog name. Backup SHA-256:

`633462fabe6ec41ce92af9b4e8720948d2c04b9b5f8f72cf3caaad19ae8a15e5`

The disposable source contains zero epoch records, trusted authorities,
executors, approvals or quarantined historical budgets. This is intentional
isolation, not historical restoration. The primary control-file, prepared intent,
pending owner/credential files and archived accounting files were checked against
13 prior fingerprints and remain unchanged. Historical budget
`ad91e475-f1da-495a-8247-139643d2d8b1`, its **$0.10 ceiling** and unresolved
**$0.004048 hold** remain quarantined; nothing was imported, refunded or reused.

The disposable cluster was stopped after testing. Independent backup storage
remains unset. Same-disk backups do not protect against disk loss; no private
backup was uploaded. Railway volume backups, retention, restore drills and any
PITR bucket/storage costs require approval. A Railway volume backup is not an
independent offsite copy. Consult [Railway's backup/restore guide](https://railway.com/guides/postgres-backups-restores).

## Accessibility and browser evidence

The only visual change removes `.entity-state`'s parent opacity of 0.7. Existing
colors, typography, dimensions, logo, branches, wheel, intro, portal, keyboard and
all ten sections are preserved. Default status-label contrast improves from
**3.51:1 to 6.07:1**. A rendered regression includes ancestor opacity and checks
all visible status labels across all three approved palettes. Desktop and tablet
labels pass 4.5:1; the mobile layout intentionally hides those status fields.

REA exercised the actual Next/Rust/Postgres test application:

- Desktop terminal navigation to Memory completed: evidence
  `ev_f181095a42b09596bcec360645bb9238effcb1700cca3abf47cc8287f981c089`.
- Mobile navigation open/Escape-close completed: evidence
  `ev_128c5a70d19e3826a45daa4d914f63722a7edca9c067d10c438429bcfa7d82f8`.
- Desktop screenshot, 1440×1000:
  `ev_7c00afcf26bb48be4e8baf9237a1c4484c0710e08f5afd1fb20f7fbec9a96cb9`.
- Mobile screenshot and viewport metadata are retained locally alongside them.
  Desktop/mobile showed no horizontal overflow; the system test also exercised
  768×1024 tablet views.

REA attached-session event coverage is incomplete and cannot recover pre-attach
errors. The owning full-stack browser recorded no page errors. Desktop/mobile
screenshots were captured passively after restoring and reloading the intended
viewport, avoiding attached-session emulation cleanup effects. These are Chromium
and emulated mobile results, not physical-device or cross-browser certification.

Impeccable's focused rendered detector reports no low-contrast status-label
finding after the fix. Four remaining warnings concern terminal outline padding
and tilted/offscreen wheel SVG/text overlaps. They were not suppressed or used to
redesign the approved layout. This is a focused accessibility correction, not a
claim of complete WCAG conformance.

## Validation ledger

Local evidence is in ignored `artifacts/milestone-6/`; screenshots, dumps, logs,
account responses and private configuration are not committed.

| Check | Result |
| --- | --- |
| Rust workspace, including ignored PostgreSQL/security/recovery tests | 121 passed, 0 failed |
| Migration administrator default suite | 3 passed; 1 specialized Stage-1 clone/port-55444 test not run |
| Rust formatting / all-target Clippy | Passed |
| Adapter / autonomy / epoch Node tests | 24 passed |
| Railway SDK / Docker declaration security tests | 8 passed |
| Railway TypeScript authoring check | Passed |
| Frontend production build / lint / TypeScript / experience syntax | Passed |
| Browser bundle known-secret/pattern scan | 26 chunks; no findings |
| Status contrast regression | 3 viewport profiles passed, each across 3 palettes |
| Existing console browser regression | 144 passed on the final run |
| Approved experience regression subset | 33 passed across desktop, tablet and mobile (including contrast) |
| Real local full-stack / scoped API / restart / duplicate checks | Passed; zero paid calls/transactions |
| Disposable database crash / backup / restore / checksums | Passed on PostgreSQL 16 |
| Actual OCI builds / container scans / Railway PostgreSQL 18 | Not run; local builder/runtime/storage blocker |
| Actual Railway networking/volume/deployment/backup | Not run; resource creation and deployment not authorized |

The first console run overlapped production build finalization: six early desktop
checks failed, while 138 passed. The final run starts after build completion.
The initial contrast-test draft attempted an intro-only theme control on workspace
pages; it was corrected before the passing run. An initial restore checker
compared database catalog names; its corrected comparison preserves all actual
privilege fields and passed. No product behavior was weakened to hide failures.

## Git and security audit

Changes are committed directly on `main`, one exact file per Conventional Commit,
through `scripts/commit-one`. Every commit runs its hooks and the secret gate.
The final full-history one-file audit and final secrets results are retained in
`artifacts/milestone-6/`. No generated artifact, dump, credential, screenshot or
log is staged. No push, history rewrite or deployment is performed. The scanner
is a targeted pattern/known-secret gate; no third-party image/secret scanner was
available, and this is not proof against every unknown secret format.

## Remaining release gates and exact approval sequence

1. **Nonbillable local gate:** provision adequate build storage and a reviewed
   Docker or rootless BuildKit runtime. Build/scan all three images and validate
   the selected PostgreSQL 18 image, recovery and volume layout. Pin reviewed
   image digests. Preserve existing data/evidence while making storage available.
2. **Source publication gate:** identify the exact tested main commit. A separate
   explicit push approval is needed before publishing necessary source changes.
   Current local tracking contains no artifacts; historical artifact cleanup and
   any previously rejected unpublished ancestry need an explicit Git decision.
   Do not rewrite history or force-push as part of deployment preparation.
3. **Account/billing gate:** operator selects the real Railway account/project,
   `staging` environment, region, service capacity and maximum acceptable spend.
   Obtain explicit permission to create a new isolated project/environment and
   four billable resources. Do not reuse either accessible project by inference.
4. **Identity/secrets gate:** approve real staging principals/roles and generate
   new public-service, operator, visitor and database credentials in a secure
   store. Install scoped shared variables only after explicit approval. No
   pending local owner/custodian credential or prepared epoch is reused. No
   ownership grant follows automatically from a login or role string.
5. **Plan gate:** after approved linking/account setup, generate a redacted pinned
   plan and review it against the tested source. The CLI supports:

   ```sh
   railway config plan --file .railway/railway.ts --environment staging --out /tmp/supa-staging.plan --source-tree "$(./scripts/git-local rev-parse HEAD:.railway)"
   ```

   The explicit source tree avoids the CLI's default ordinary-Git lookup because
   this repository uses `scripts/git-local`. Keep the plan private; never use
   `--show-values` or `--decrypt-variables`. This command has **not** been run.
   Require four new resources, no deletions, no public proxies/domains, private
   database references, correct persistent mount and a disabled worker.
6. **Apply/deploy gate:** obtain approval for that exact pinned plan, including
   fresh-database migrations **1–10**, service creation, build/deploy and costs.
   Applying the graph may start all services immediately; it is not a harmless
   configuration preview. Only then execute:

   ```sh
   railway config apply --file .railway/railway.ts --environment staging --plan /tmp/supa-staging.plan
   ```

   Verify database readiness first; SQLx migration locks serialize concurrent
   startup. All app containers must recover within readiness/restart limits.
   Stop on an unexpected target, migration checksum, credential or resource.
7. **Exposure/acceptance gate:** approve only the frontend HTTPS domain, configure
   its exact `SUPA_SITE_ORIGIN` before accepting traffic, and retain the whole-site
   gate. No API/worker/database public access. REA-check all ten sections, intro,
   terminal, keyboard/touch, permissions, disconnected/empty/error states and
   reduced motion against actual staging. Empty staging records stay empty; any
   new identity, ownership or offline authorization requires its own approval.
8. **Recovery gate:** approve a staging-only backup policy and separate restore
   target, verify restoration and restart persistence, then document observed
   recovery times. Keep external backup destinations unset until authorized.
   Readiness gates a deployment but does not replace ongoing monitoring.
9. **Rollback:** stop frontend/API/worker traffic/execution first; retain the
   database and volume. Revert application images only when schema-compatible.
   Migrations 9/10 have no automatic down migration. Never delete them, drop
   approval/audit tables, restore over the source, replay authorizations or refund
   uncertain charges. Restore a verified backup into a separate database and
   request explicit cutover approval if image rollback cannot recover service.
   The prepared local epoch and its quarantine remain untouched throughout.

Runtime DB credentials currently permit embedded migrations and application DML.
They must not be represented as read-only or as independently protected against a
DB superuser. Before broader exposure, decide whether separate migration/runtime
DB roles are required; that needs a reviewed startup change and grants, not an
unannounced architecture rewrite. Accounts and secrets remain a trust boundary.

No live GPT, worker enablement, Mainnet, autonomous spending, trading, launch,
financial transaction, X publishing, child-agent creation, production migration,
local epoch activation or deployment is approved by this report.
