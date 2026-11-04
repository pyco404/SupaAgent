# Normal Railway staging — current plan, 4 November 2026

## Staging TLS repair verified — 4 November 2026

**TLS blocker resolved; bootstrap remains unexecuted.** The explicitly authorized
repair ran only on staging service `77352ca9-435e-4870-b092-58365310c0f3`, database
`supa_staging`, cluster `7704432314739101731`. PostgreSQL deployment
`db546a74-4038-4536-b450-d93e78362808` and its approved image digest remain unchanged.
The repair used a configuration reload, not a restart or new deployment. Postmaster
start time remains 4 November 2026, 21:32:16.818595 UTC.

A separate RSA-3072/SHA-256 CA and server identity were generated **inside the
staging volume**. The CA has critical `CA:TRUE,pathlen:0` and signing-only key usage;
the leaf has critical `CA:FALSE`, serverAuth EKU, digitalSignature/keyEncipherment,
and SANs `supa-postgres.railway.internal` and `supa-postgres`. The server presents
leaf plus CA. Original certificate/key and HBA/auto-configuration copies are
retained in the owner-only volume directory
`/var/lib/postgresql/data/supa-tls-repair-XXT6ELfp/original`; their integrity checks
pass. Both CA and server private keys have mode 0600; TLS/recovery directories have
mode 0700. No private key or password was exported, printed or committed.

| Public certificate | SHA-256 fingerprint | Expires UTC |
| --- | --- | --- |
| Server leaf | `F18DC26652BB3823E4C48355DF66C7349D051735F8E692C0D73D790B9BB012DD` | 2 February 2027, 22:27:52 |
| CA | `4CE9C08B0D0EC90BCDDB7AA990EB47133F73195453FE1AEFB9FAB37DEB4DE2C3` | 4 November 2027, 22:27:52 |

Both libpq `sslmode=verify-full` and the actual Rust SQLx/rustls read-only client
pass with `/var/lib/postgresql/data/supa-tls/ca.crt` as the trust anchor. The live
presented leaf fingerprint matches the installed candidate; strict chain, hostname
and server-purpose verification pass. TLS minimum is now **TLSv1.3**. TCP HBA rules
require TLS with SCRAM-SHA-256, with explicit non-TLS rejection; Unix-socket rules
are unchanged. Non-TLS, TLS 1.2 and incorrect-hostname connections were rejected.
All HBA rules parse without error and the sole login has a SCRAM password verifier.
There are still zero application tables, one original administrator login, no
bootstrap mutation marker, no added roles/grants and no seed records.

Seven Rust tests, Clippy, formatting, shell syntax, synthetic certificate-chain /
wrong-host / wrong-chain tests and key-permission tests pass. Full private execution
and verification receipts are in Git-ignored `artifacts/railway-staging-phase1/`.
The repair script is single-use and exact-target guarded; on failure after install
it restores the preserved certificate/key, HBA and auto-configuration, reloads and
checks the old verified connection. That recovery branch was not needed live.

**Maintenance boundary:** the unchanged pinned image still contains legacy
self-signed renewal logic when fewer than 30 days remain on the leaf. Perform a
reviewed CA-signed renewal **before 3 January 2027, 22:27:52 UTC**, or separately
approve a corrected image build/scan/publication before that boundary. This repair
did not modify the image. Independent encrypted recovery remains mandatory before
sensitive persistent state. API CA trust installation belongs to the later API
configuration phase; the API and frontend remain undeployed, worker absent, public
endpoints absent, resource limits unchanged. Protected local PostgreSQL and
historical accounting were untouched. Wait for instruction to resume the previously
authorized roles/migrations/synthetic bootstrap; this TLS task does not execute it.

## Database bootstrap stopped at Rust TLS verification — 4 November 2026

The owner authorized staging-only roles, the 13 frozen migrations and synthetic
bootstrap. The execution stopped **before any database mutation**. Authenticated
read-only libpq inspection still finds zero application tables, one login
(`supa_staging_admin`), and cluster identifier `7704432314739101731` in
`supa_staging`. No migration/runtime roles, application epoch, grants or synthetic
records were created. PostgreSQL deployment remains
`db546a74-4038-4536-b450-d93e78362808`; frontend/API remain undeployed and no worker
exists. The approved image digest and frozen application revision are unchanged.

The volume-local certificate has critical basic constraint `CA:TRUE`. libpq accepts
it as configured, but the frozen Rust SQLx/rustls stack rejects its use as the server
leaf with `CaUsedAsEndEntity`. A read-only diagnostic reproduced the specific error
`tls_ca_used_as_server_certificate`; a second libpq check confirmed the database
remained empty and the bootstrap mutation marker absent. TLS verification was not
weakened and migration SQL was not applied through another client.

The existing CI bootstrap accepts only CI database names. The separate
`crates/migration-admin/src/bin/supa-railway-staging-admin.rs` entry point prepares
an owner-executed fresh-staging bootstrap without modifying that CI guard, protected
primary migration tools, API revision or migration files. It binds the exact
project/environment/service/database/cluster, 15-minute invocation scope, and all
13 frozen SHA-384 migration checksums; runtime operations cannot invoke it. It
uses verified TLS and stops before mutations on a connection failure. Its read-only
TLS mode uses the same connection stack. Seven network-free tests, formatting and
Clippy pass; live role/privilege/migration/seed tests remain **blocked, not passed**.
The existing runtime grant plan remains SELECT plus worker-health INSERT/UPDATE
and live-session UPDATE, without schema/history/accounting writes or owner grants.

Next approval: a narrowly scoped volume-local TLS repair with a proper `CA:FALSE`
server certificate, private-hostname SAN, trusted CA chain, protected private key
and certificate backup/rollback, followed by TLS reload and both Rust and libpq
verify-full checks. Do not generate or replace certificate/key material without
that approval. Resume the already authorized database bootstrap only after those
checks pass; no application deployment is authorized. The public CA can subsequently
be installed through `PGSSLROOTCERT` (SQLx supports inline PEM in this environment
variable), without distributing its private key or weakening `sslmode=verify-full`.
Do not put inline PEM in SQLx's URL `sslrootcert` option: that setter treats it as
a file path.

Private Git-ignored evidence and the uninstalled, zero-balance paused-agent fixture
are under `artifacts/railway-staging-phase1/`. Workspace usage refreshed to
$18.78793353 against unchanged shared $60 Compute limit ($41.21206647 current-use
headroom). This is not forecast headroom or reserved staging credit. No full-stack
monthly cost has been demonstrated; the $10–$20 planning allowance is not a cap.
Existing daily native backups remain planned protection; independent encrypted
recovery is still required before sensitive persistent state. Frontend/API image
availability, finite session validity and authenticated origin remain deployment
prerequisites after bootstrap. Protected local PostgreSQL was not started or written.

## Owner-accepted residual risk — 4 November 2026

The owner explicitly accepted the following unsuppressed findings for restricted,
authenticated, synthetic-data-only **normal staging**, superseding the older
proposed/not-accepted security disposition below. This is not production acceptance,
a deployment grant, a permanent security exception or permission to change images.

| Component | Accepted findings | Evidence |
| --- | --- | --- |
| Frontend | 41 MEDIUM, 8 LOW; zero HIGH/CRITICAL and secrets | [41892549731](https://github.com/pyco404/supa-agent-infra/actions/runs/41892549731) |
| Restricted API | 36 MEDIUM, 8 LOW; zero HIGH/CRITICAL and secrets | Same frozen-source scan run |
| PostgreSQL | One UNKNOWN: CVE-2026-39824, golang.org/x/sys v0.1.0 in gosu, scanner fixed version 0.44.0; zero other severities and secrets | [41904471442](https://github.com/pyco404/supa-agent-infra/actions/runs/41904471442) |

The PostgreSQL scan succeeded at infrastructure revision
`d28101ad6fe4177935862ecdbed2f868705ade16`. Owner receipt association proof,
live explicit Private visibility, anonymous HTTP 401 before/after retrieval,
manifest/configuration and all 16 compressed layer hashes/sizes were verified.
The job-token repository field was missing; it was not treated as proof. Full
unsuppressed reports, receipt and integrity evidence were retained privately.
GitHub artifacts have seven-day retention. Access-controlled local copies of all
three scan reports and a SHA-256 manifest are retained under Git-ignored
`artifacts/security/staging-acceptance-2026-11-04/`; no credentials were copied. This record preserves counts and
provenance, not raw private scan contents or secrets.

Acceptance conditions remain mandatory: authenticated frontend, private API and
PostgreSQL without public TCP exposure, zero worker replicas in every region,
synthetic disposable identities/data only, no paid inference or financial execution,
no wallet signing, trading, token launches or X publishing. Preserve the exact
PostgreSQL digest below. Scope drift, exposed secrets, new HIGH/CRITICAL findings
or changed image contents require review; no scanner suppression is authorized.
No protected local database/evaluation records or historical holds are imported.
Independent encrypted recovery remains mandatory before sensitive persistent
identities, memories, customer or financial data can be introduced. Native volume
backups alone do not satisfy that independent recovery requirement.

### Minimum remaining execution approvals

1. **Credentials:** authorize an expiry-bound classic GHCR PAT with only
   `read:packages` for an identity already permitted to read the approved private
   package. Install privately in PostgreSQL Source → Registry Credentials → GitHub
   Access Token, once the service exists. Do not reuse the broader CLI token.
   Separately authorize fresh staging-only database administrator/runtime logins,
   visitor authentication and finite named synthetic viewer/operator credentials;
   keep all secrets out of Git, logs and chat. Frontend gets no database/admin
   credentials, API gets only its restricted database URL, and worker gets none.
2. **Additive provisioning/costs:** authorize only frontend/API/PostgreSQL in existing
   project `99648d2a-69a1-4d8c-98d0-7f2a26543c9c`, environment
   `ff0b2092-e42b-4a89-ae25-441ab95d92d8`, Amsterdam, plus the reviewed PostgreSQL
   volume (initial 1000 MB request) and daily native backups. Keep worker absent
   or zero; preserve the existing backup bucket. Approve $10–$20/month additional
   usage after refreshing headroom under the unchanged shared $60 Compute limit.
   Confirm accepted CPU/RAM/storage values and estimate before billable deployment;
   previously proposed small caps remain unverified. Disable automatic deployments
   and image updates. Stop if creation inseparably starts workloads without consent.
3. **Artifact availability:** the scanner saved frontend/API archives only in an
   ephemeral runner work directory; its upload step retained reports, not those
   archives. No deployable frontend/API registry reference or recoverable retained
   image is established. Recover and verify the exact scanned images first, with
   separately approved publication if needed. Otherwise request fresh build/scan
   authorization; do not silently rebuild via the current source-based Railway IaC.
   Frozen application revision remains
   `bfc29f500af08e0e2cc4676b8950b5b17f26007a`.
4. **Bootstrap/deployment:** once artifacts and preceding approvals are resolved,
   authorize starting the new PostgreSQL only, applying all 13 migrations with
   separate migration administration, verifying checksums and restricted grants,
   and seeding rebuildable synthetic fixtures. Configure the exact HTTPS frontend
   origin and mandatory authentication before allowing access; deploy API privately,
   then frontend. Use a Railway-generated hostname, with no custom DNS changes.
   Verify /healthz, private /ready, login/role denials, stopped worker, disabled
   capabilities, source/image identity, restart/manual stop and synthetic backup/
   disposable restoration. No primary database startup or migration is involved.

Official credential instructions:
[Railway private registries](https://docs.railway.com/builds/private-registries),
[GitHub Container Registry authentication](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry).

**Security disposition: GO within the accepted staging boundaries. Execution is
not authorized yet.** No credentials, resources, deployment or image rebuilds were
performed while recording this decision. Older planning sections below are
historical where this acceptance supersedes them.


This section supersedes all $1, ten-minute and six-hour rehearsal planning below.
**Planning only: NO-GO to provision/deploy until budget and protected operations
are separately approved.** No deadline workflow installation, credential creation,
resource provisioning, billing-limit change or application deployment is authorized.

## Authorized Compute-limit update — verified 4 November 2026

The owner separately authorized only the workspace Compute hard-limit change.
Verified exact workspace `e319f68a-9897-49ed-b119-743f1f846aa0`, displayed as
**Daniel 's Projects**. Before mutation: Compute hard **$40**, soft alert **$20**,
Agent hard **$20**. One `railway usage limit update --hard 60 --workspace
 e319f68a-9897-49ed-b119-743f1f846aa0 --json` invocation succeeded, followed by an
independent `usage limit status` read confirming Compute **$60**, unchanged soft
**$20** and unchanged Agent hard **$20** (Agent soft setting also unchanged).
No resource allocation, service, project, credential, DNS or other billing setting
was changed. No provisioning, deployment, additional paid service or inference is
implied. The older proposed-limit assessments below are retained historical snapshots.

After-change usage read: **$18.55633452980352**, forecast **$33.57361016554333**;
headroom against current usage **$41.44366547019648** and against forecast
**$26.42638983445667**. Adding the planned $10–$20 additional monthly staging usage
would give approximately **$43.57–$53.57**, leaving **$6.43–$16.43** against $60,
subject to actual consumption and other workloads. The limit remains shared; it
is not a reserved per-project staging allowance. No workload availability guarantee
follows from forecast arithmetic. Existing soft alerts were deliberately preserved.

### Current container security scan scope — prepared, not executed

Separate approval is still required for the scanning operation and any necessary
read-only registry authentication. No CI was dispatched and no fresh scan results
are claimed. The retained MEDIUM/LOW/UNKNOWN findings below remain visible pending
current-database scans of the exact intended artifacts:

| Component | Artifact identity to scan | Preparation boundary |
| --- | --- | --- |
| PostgreSQL 18 | `ghcr.io/pyco404/supa-agent-postgres-infra-98c1006fcb1d@sha256:9409009a40c0929052306d068c14bef92f3ecc67662c453540e2c9b95ae18fc8` | Pull/read only after separate approval; do not rebuild, republish or substitute |
| Frontend | Retained validated image/archive built from `bfc29f500af08e0e2cc4676b8950b5b17f26007a`, if available and integrity verified | Record actual manifest/config/layer identities; if archive cannot be recovered, separately approve a build at the frozen SHA and scan that exact resulting image |
| Restricted API | Same frozen-source requirement as frontend | No primary database, financial records, real credentials or deployment needed for scanning |
| Worker | Zero replicas, disabled | No worker deployment; retain prior worker findings without claiming a fresh scan |

Prepared scanner settings: Trivy vulnerability **and secret** scanning, current
vulnerability database with recorded update time/version, every severity including
UNKNOWN, no ignore file (`--ignorefile /dev/null`), complete JSON reports and
sanitized package/CVE/installed/fixed-version summaries. Use the existing reviewed
checksum-pinned scanner installation unless separately updated/reviewed. Preserve
all findings and fail closed on scanner/identity errors. Secrets findings must not
be printed verbatim. Keep detailed evidence private/access-controlled.

For an already loaded approved archive, the scan command shape is:

```sh
trivy image --input /PRIVATE/APPROVED-IMAGE.tar \
  --scanners vuln,secret --ignorefile /dev/null \
  --severity UNKNOWN,LOW,MEDIUM,HIGH,CRITICAL \
  --format json --output /PRIVATE/IMAGE-scan.json
```

Placeholder paths are deliberate; actual artifact identities/archive hashes and
execution environment must be reviewed before running. The current
`staging-validation.yml` is full runtime validation or PostgreSQL-candidate build;
it is **not** an exact-image scan-only workflow and would rebuild PostgreSQL.
Do not dispatch it blindly for this scope. An approved scan-only execution must
reuse the verified PostgreSQL artifact, with no image publication or Railway
operations. There is no need to repeat all 200 PostgreSQL functional tests simply
to refresh the vulnerability database. Comparing frozen/current source confirmed
no changes in Rust crates, frontend/reference frontend or the application and
PostgreSQL Dockerfiles; only the two inert workflow templates differ under
`deployment/`. This supports source continuity, not proof of identical new builds.

Review current HIGH/CRITICAL, MEDIUM/LOW and UNKNOWN results before the new
normal-staging risk decision. The prior short-rehearsal acceptance is not renewed
by this spending-limit change. If findings or image content change, stop for review;
no blanket exception, automatic remediation rebuild or silent digest replacement.

Next separate approvals remain scan execution/read-only access as needed,
normal-staging risk disposition, secure staging credentials, additive provisioning,
and synthetic bootstrap/deployment/backup verification. Source documentation
synchronization needs its own push approval. Frozen application revision, private
PostgreSQL digest, zero worker, stopped protected primary PostgreSQL and historical
accounting holds remain unchanged.

## Proposed $60 limit and final approval handoff

Fresh read-only usage verification on 4 November 2026: current usage
**$18.540155185361854**, forecast **$33.57538163245182**, unchanged hard limit **$40**
and soft alert **$20**. Raising only the workspace Compute hard limit to **$60**
would leave **$26.42461836754818** forecast headroom. At an additional $10–$20/month,
planning totals are **$43.57538163–$53.57538163**, leaving **$6.42461837–$16.42461837**.
This supports the requested budget under current forecast assumptions; it is not
reserved credit or a guarantee of future consumption. The present forecast covers
the billing period ending 18 November, not a verified next-month baseline.

[Railway's shared hard-limit behavior](https://docs.railway.com/pricing/cost-control)
is material: raising it does not itself change replicas, restart services or reduce
other projects' allocations. It increases maximum permitted workspace exposure.
All projects still share the Compute ceiling; hitting $60 could take all workloads
offline. No per-project reserved $20 allowance or guarantee of other-project
availability is established. Keep the existing Agent limit and soft-alert settings
unchanged unless separately approved. No limit was changed during this assessment.

### Normal-staging security disposition — proposed, not accepted

Retained scan evidence: frontend **41 MEDIUM / 8 LOW**, API and worker each
**36 MEDIUM / 8 LOW**, all with zero reported HIGH/CRITICAL at those scan dates.
PostgreSQL: zero HIGH/CRITICAL/MEDIUM/LOW and **one UNKNOWN**. These are retained
reports, not fresh scans. Preserve all findings; repeated package associations do
not imply distinct CVEs. The exact package/version/CVE table remains in
[`staging-six-hour-rehearsal.md`](staging-six-hour-rehearsal.md#residual-security-disposition-limited-operator-acceptance).

- PostgreSQL Gosu 1.20 embeds `golang.org/x/sys v0.1.0`:
  **CVE-2026-39824 / GO-2026-5024**, reported fixed at v0.44.0. The
  [official Go advisory](https://pkg.go.dev/vuln/GO-2026-5024) names
  `golang.org/x/sys/windows.NewNTUnicodeString`; the verified image is linux/amd64.
  This supports low reachability of that specific Windows function in this
  artifact, not removal of the UNKNOWN scanner result or a guarantee of Gosu
  safety. Protect its root-startup/privilege-dropping paths, immutable binaries,
  user/group configuration and volume permissions. Do not rebuild or replace the
  approved image merely to remove a metadata finding.
- Application GCC-family libraries `14.2.0-19`, glibc
  `2.41-12+deb13u4` and zlib `1:1.3.dfsg+really1.3.1-1+b1` remain the documented
  MEDIUM/LOW exposure. Runtime libraries are genuinely used; specific vulnerable
  symbol reachability is not established. Non-root operation, authenticated
  frontend, private API/database, bounded inputs, no arbitrary native plugins or
  archive-processing jobs, offline inference and resource monitoring reduce but
  do not eliminate corruption/denial-of-service risk. No compatible fixed version
  was supplied by the retained scans; that is not a fresh remediation search or
  proof no fix exists today.

Recommended disposition: conditional acceptance for authenticated, synthetic-only
normal staging, after current scans of the exact deployment artifacts and explicit
operator acceptance of lower-severity findings and remaining recovery limitations.
New HIGH/CRITICAL findings, secret leakage or scope drift require remediation and
review; no gate bypass or automatic acceptance. The old six-hour risk decision
cannot authorize this longer-lived staging. Fresh scan/build execution, if needed,
requires its own approved scope; avoid another full functional-CI campaign merely
to duplicate previously passed evidence. Do not claim application image identity
from a Git SHA alone: record actual deployed image identities and source SHA.

### Minimum credential and database configuration — prepare only

| Boundary | Proposed minimum | Explicitly excluded |
| --- | --- | --- |
| PostgreSQL image pull | Expiring classic PAT with `read:packages`, owner already permitted to read the private package; install only in Railway registry credential settings | `write:packages`, deletion scopes, broad local CLI token, public image fallback |
| Migration/bootstrap | Separate administrator login for only the new synthetic staging database; run the existing migration administrator once with disposable-schema guard and verify all 13 checksums | Primary database URL, automatic migrations on API startup, historical imports |
| API database runtime | Dedicated LOGIN with NOINHERIT, NOSUPERUSER, NOCREATEDB, NOCREATEROLE, NOREPLICATION, NOBYPASSRLS; schema USAGE/approved SELECT and only the CI-tested maintenance writes | Schema CREATE, migration-table UPDATE, simulated-balance UPDATE, epoch-approval deletion and general treasury writes |
| Frontend visitor | Separate username/password (24+ characters), exact HTTPS origin, server-side authentication | Public unauthenticated agent records, exposed secret values or NEXT_PUBLIC credentials |
| Named synthetic reader/operator | Distinct 32–256-character service and principal tokens, approved synthetic UUID/role mapping and matching frontend binding; finite session deadlines, at most 24 hours under current preflight | Real owner grants, shared visitor/operator credentials, silently permanent sessions |

The CI-tested maintenance grants are INSERT/UPDATE on `worker_health` and UPDATE
on `live_sessions`, separate from SELECT; they are not general agent/treasury write
permission. Actual grants must match the chosen supported staging endpoints, and
negative permission tests must pass before exposure. Set credentials in appropriate
private/sealed service settings after approval. Frontend receives only its private
backend URL/service and account mappings, never a database/admin credential.
Database URLs must use the private hostname and reviewed TLS configuration. Verify
connection behavior against the pinned image; do not weaken TLS merely to connect.
Worker remains zero and needs no live worker credential installation initially.
No OpenAI/Anthropic, wallet, X, shutdown-runner or backup-storage credentials are
needed for this read-only synthetic launch. Do not copy local evaluation secrets.

### Short path to online staging — separate approvals

1. **Source synchronization:** approve the application repository's pending
   one-file IaC/test/documentation commits at its final reported HEAD. Do not push
   the two superseded smoke workflows in the private infrastructure repository.
2. **Budget:** explicitly approve changing only this workspace's Compute hard limit
   **$40 → $60**, keeping other limits and projects unchanged, and approve
   $10–$20/month additional staging usage with monitoring. This is not authorized yet.
3. **Security and credentials:** approve normal-staging residual-risk disposition
   after reviewing current exact-artifact scans; separately approve creation and
   installation of the minimum staging/GHCR credentials and synthetic principals.
   No real ownership or agent activation is implied.
4. **Provisioning:** approve additive frontend/API/PostgreSQL services, initial
   1000-MB volume request and daily native backups in the existing Amsterdam staging
   target. Preserve the backup bucket and unrelated projects. Keep worker zero,
   APIs private, automatic deployment/image updates disabled and exact source/image
   pinning. Review the final graph/diff; prevent creation from auto-deploying before
   deployment consent, or stop and review the inseparable operation explicitly.
5. **Bootstrap/deployment:** approve synthetic-only migrations/seed, application
   deployment at `bfc29f500af08e0e2cc4676b8950b5b17f26007a`, a Railway-generated
   authenticated HTTPS frontend endpoint (no custom DNS), and native backup/isolated
   restoration validation. Verify health/authentication/role and DB permission
   checks, actual source/image identity, worker zero and disabled paid/financial
   switches; then record actual costs. Test manual Stop/Cancel and restart only
   within that approved staging scope, without touching unrelated workloads.

Native backups are not an independent encrypted recovery copy. Continue only with
rebuildable synthetic fixtures and explicit data-loss tolerance until encryption
and manifest-signing identities, independent key custody, encrypted upload and a
successful restoration drill are complete. Actual limits, storage sizing and TLS
connection behavior must be checked at approved provisioning/deployment time.

**Verdict:** proposed $60 limit is financially sufficient for the stated forecast
plus planned $10–$20 staging, conditionally on measured usage; no availability
isolation is promised. **NO-GO to execute** until the separate approvals above.
No service, credential, billing limit, DNS, primary database or historical accounting
was modified. No new runtime component or smoke-test automation is required.

## Minimal staging architecture

Use existing Supa Agent project `99648d2a-69a1-4d8c-98d0-7f2a26543c9c`, staging
`ff0b2092-e42b-4a89-ae25-441ab95d92d8`, Amsterdam. One Next.js frontend, one restricted
Rust API and one PostgreSQL 18 instance; worker stays at zero global/regional
replicas and disabled. No extra running infrastructure. The two deadline workflows
remain only in the private infrastructure checkout's two unpublished commits;
do not push or dispatch them for normal staging. Preserve their history and leave
application-side templates inert. No new watchdog token is needed for this plan.

The local Railway authoring now proposes normal staging, a **$10–$20 monthly
additional-usage planning range**, no mandatory deadline automation, and daily
volume backups. Its GitHub source explicitly includes frozen application SHA
`bfc29f500af08e0e2cc4676b8950b5b17f26007a`. This is not proof of the remote source
selection: disable automatic source/image updates and require the actual deployed
SHA before accepting the deployment. Application functionality is unchanged.
PostgreSQL remains pinned to:

`ghcr.io/pyco404/supa-agent-postgres-infra-98c1006fcb1d@sha256:9409009a40c0929052306d068c14bef92f3ecc67662c453540e2c9b95ae18fc8`

Public access is only via the existing authenticated frontend flow on a separately
approved Railway-generated HTTPS address; no custom DNS changes. Set its exact
`SUPA_SITE_ORIGIN`. API and PostgreSQL use private networking and have no public
TCP proxies or generated domains. Keep all model keys absent and all paid inference,
worker/scheduler, wallet-signing, trading, launches and X publishing disabled.

Health checks: frontend `/healthz`, API `/ready`; verify PostgreSQL with authenticated
`pg_isready`/SQL and its actual restart state. Proposed ON_FAILURE application/database
restart policies allow three retries; confirm their actual settings at provisioning. Preserve 60-second app and 120-second database drain allowances.
Monitor CPU, RSS, restarts, disk usage, backup success and costs in Railway. The
owner checks usage after startup and daily until baseline consumption is measured,
then weekly; alerts are supplementary. Manual shutdown cancels pending builds and
stops frontend/API before PostgreSQL, verifies terminal/stopped state and leaves
worker at zero. Storage/backups remain billable; deletion needs separate approval.
No second administrator or hard per-project spending ceiling is assumed.

## Cost decision

Fresh read-only workspace snapshot: Daniel 's Projects
`e319f68a-9897-49ed-b119-743f1f846aa0`, current usage **$18.54015519**, forecast
**$33.57494066**, shared hard limit **$40**, forecast headroom **$6.42505934**.
The current billing period ends 18 November 2026, 16:34:07 UTC. The forecast is not
reserved capacity or a guarantee of next period's baseline.

Official [Railway pricing](https://docs.railway.com/pricing/plans): actual RAM
$10/decimal GB-month, CPU $20/vCPU-month, egress $0.05/GB and used volume/backup
storage $0.15/GB-month. Builds are free; existing services still consume resources
while builds run. Existing Pro plan fees are excluded from incremental estimates.

Illustrative full-month aggregate averages across all three running services:

| Scenario | RAM | CPU | Volume | Incremental backup data | Egress | Approximate monthly usage |
| --- | --- | --- | --- | --- | --- | --- |
| Lean | 0.75 GB | 0.05 vCPU | 1 GB | 1 GB | 10 GB | $9.30 |
| Baseline | 1 GB | 0.10 vCPU | 1 GB | 2 GB | 10 GB | $12.95 |
| More headroom | 1.25 GB | 0.15 vCPU | 1 GB | 2 GB | 20 GB | $16.95 |
| Higher usage | 1.5 GB | 0.20 vCPU | 1 GB | 2 GB | 20 GB | $20.45 |

These are workload assumptions, not verified application measurements or installed
resource ceilings. The three-service stack may require more memory/CPU during
startup, migrations or load. Small per-service caps remain unverified; no silent
limit changes or guarantees. Private GHCR transfer/storage charges, if any, are
separate from Railway. Volume and retained backups continue charging while compute
is stopped; private networking avoids public inter-service egress.

**Always-on staging at an additional $10–$20 does not fit the existing forecast
headroom.** Planning against the same baseline gives about $43.57–$53.57 combined
monthly usage, above $40. Do not raise the limit or stop other workloads. Partial
current-period costs might be lower but are not proof the recurring budget fits.
Before deployment choose explicitly: authorize an appropriate workspace limit with
margin after reviewing all workloads; approve a smaller measured usage envelope;
or approve part-time manual staging. For example, 8 hours/day on 20 days/month
(160/720 hours) scales assumed compute to about $2–$4.22 plus always-retained storage,
backups and actual egress, roughly **$3–$6/month** under these assumptions. This
option requires dependable manual Stop/Cancel and does not promise a hard cap.
No operating schedule or budget-limit change is selected automatically.

## Credentials, storage and recovery

After separate approval: expiry-bound GHCR classic PAT with `read:packages` and
existing private-package read access, installed only in PostgreSQL Source → Registry
Credentials; a separate migration/admin database credential and restricted runtime
login; synthetic viewer/operator accounts with explicit principal bindings; distinct
frontend visitor authentication; private service tokens and exact HTTPS origin.
Use Railway sealed/private settings where supported. Do not reuse local evaluation
credentials or the owner's broad interactive GitHub token. No watchdog credential
is needed. Preserve existing finite operator-session deadlines (maximum 24 hours);
renew through the authorized process without making permissions permanent.

Propose the existing 1000-MB volume request at `/var/lib/postgresql/data` and daily
Railway-native backups (six-day retention). Confirm actual capacity/permissions
before starting; monitor free space. Keep migrations separate from normal API
startup and verify all 13 checksums. Do not import the protected local database or
historical accounting. Seed only reviewed synthetic records through a separately
approved migration/bootstrap action. Runtime credentials never get schema-admin
or blanket financial write permissions.

[Native backups](https://docs.railway.com/volumes/backups) are incremental, billed
like volumes and limited to restoration in the same project/environment. Wiping
the volume deletes its backups; manual backups have a documented 50%-of-volume
size limit. Confirm backup success and demonstrate restoration using approved
isolated synthetic state; a configured schedule is not recovery evidence.
Independent encrypted recovery remains incomplete: verified encryption/signing
fingerprints, independent recovery-key custody, encrypted upload and restoration
rehearsal are still required before sensitive or irreplaceable state. A private
bucket alone is not a backup. Synthetic-only staging must remain reproducible and
explicitly tolerate data loss; no weakened production recovery requirement.

Prior residual-vulnerability acceptance applied only to the short rehearsal.
Obtain a **new normal-staging security decision** after reviewing current scan age,
MEDIUM/LOW/UNKNOWN findings (including the recorded Gosu UNKNOWN advisory), private
exposure, package credential scope and recovery limitations. Do not reuse the
six-hour exception or claim the old zero HIGH/CRITICAL scan is a fresh scan.
Do not change the verified PostgreSQL digest without separate review.

## Actionable deployment sequence and separate approvals

1. Approve the operating schedule and a recurring budget that fits the shared limit;
   any limit increase is a separate explicit decision. Current financial verdict:
   **NO-GO for always-on $10–$20 additional usage under the unchanged $40 limit**.
2. Review this small IaC/test/doc change and separately approve synchronization.
   Reconfirm repository/project/environment, frozen SHA, exact image, authenticated
   frontend/private backend boundaries and residual-risk/recovery decision.
3. Separately approve secure credential provisioning/installation, then additive
   creation of three services and the volume/backup schedule. Preserve the existing
   bucket, zero worker and unrelated resources. Review the exact graph/diff first.
   Do not apply while any destructive change, image substitution or source drift
   appears. Service creation can auto-deploy: explicitly prevent that until the
   next approval. If it cannot be prevented, stop and review the exact combined
   operation rather than treating deployment as incidental provisioning.
4. Separately approve synthetic migrations/bootstrap, deployment at the frozen
   SHA, a generated authenticated HTTPS frontend address, and backup/restore drill.
   Verify health, authentication/role boundaries, restricted DB access, zero worker,
   XML/TLS, restart and source/image identity. No full rebuild/test campaign is
   needed merely to re-prove prior functional CI, but actual Railway operation must
   be checked. Do not deploy on the assumption that IaC preview proves runtime.
5. Record actual resource costs and backup success; use manual Stop/Cancel on unsafe
   behavior or excessive spend. Any credential revocation, resource deletion or
   subsequent source/image change follows its separately approved scope.

Local validation: 14 focused Railway SDK/IaC tests pass, including the normal
planning policy, immutable application-source field, daily backups, private image,
restricted credentials and zero worker. No application/backend code was changed.
No credentials, resources, DNS, billing limits, deployments or database records
were changed. The historical smoke-test sections below are retained as evidence,
not active operating instructions.

---

# Historical rehearsal planning — superseded

# Railway staging integration review — 4 November 2026

Preparation only. **NO-GO to execute now; recommend a separately approved ten-minute
smoke test instead of the six-hour rehearsal.** This support-free proposal and its
approvals below supersede the earlier six-hour decision. No Railway resource,
variable, credential, domain or billing limit was
changed. The protected primary PostgreSQL remains stopped and historical accounting
holds remain untouched.

## Final operational checklist — 5 November 2026 window

**NO-GO until the separate approvals below are satisfied.** GitHub notification
receipt remains **UNVERIFIED**, but is supplementary and no longer a startup
prerequisite. Do not implement or execute another notification test. Shutdown has
no notification-delivery or acknowledgement dependency.

Read-only billing refresh on 4 November: current $18.517429869918455; forecast
$33.574423859042895; unchanged workspace hard limit $40 and soft limit $20;
forecast headroom **$6.425576140957105**. The separate Railway Agent spending setting
is not a staging-service spending cap. Recheck immediately before the session.
Conservative ten/fifteen-minute totals including the prepared GitHub runner bounds,
1-GB egress allowance and interval volume usage remain approximately **$0.69/$0.94**.
The $1 incremental monitoring policy is not enforced; extra egress, private-registry
transfer, retained storage, delayed cancellation or surviving workloads can exceed
it. Other workspace services can consume the shared headroom independently.

1. **Before the window — obtain separate permissions.** Approve synchronization
   and installation of the inert runner templates into the private infrastructure
   repository; project/environment-scoped watchdog authentication; GHCR read-only
   pull authentication; synthetic runtime/migration/visitor credentials; additive
   service provisioning; the exact shutdown scope; smoke deployment/spending; and
   any resource deletion. No such action is authorized by this checklist. Do not
   expand personal GitHub scopes or reuse broad interactive credentials.
2. **Prepare exact inventory without starting billable resources.** Target project
   `99648d2a-69a1-4d8c-98d0-7f2a26543c9c`, staging
   `ff0b2092-e42b-4a89-ae25-441ab95d92d8`. Allow only frontend/API/PostgreSQL service
   UUIDs, plus an explicitly declared dormant worker UUID if that service exists;
   worker replicas must be zero. IDs are deliberately unassigned until separately
   approved provisioning. Do not invent IDs or claim an executable scope digest.
   Preserve the existing backup bucket. If services cannot be created without
   automatic deployment or billable allocation, stop: revise the bootstrap order
   and obtain approval before proceeding. Volume allocation must not precede
   verified runner readiness.
3. **Freeze the actual deployment.** Require application commit
   `bfc29f500af08e0e2cc4676b8950b5b17f26007a`, one frontend, one restricted API and
   one PostgreSQL replica, zero worker replicas, Amsterdam, a proposed 1000-MB
   volume, authenticated visitor access and private API/database networking. Keep
   offline provider and all inference, wallet, publishing and scheduler switches
   disabled. Migration administration is separate from the restricted runtime.
   Do not blindly apply `.railway/railway.ts`: it remains authoring material with
   a legacy six-hour policy label and a `main` source. Verify the actual deployment
   source SHA, disable automatic GitHub/image redeployment and review the exact
   ten-minute plan before applying anything. Small CPU/RAM ceilings remain
   unverified. No DNS changes or generic PostgreSQL substitution.
4. **Private image pull — prepare only.** Use exactly
   `ghcr.io/pyco404/supa-agent-postgres-infra-98c1006fcb1d@sha256:9409009a40c0929052306d068c14bef92f3ecc67662c453540e2c9b95ae18fc8`.
   After separate approval, the owner supplies an expiry-bound classic PAT with
   `read:packages` and existing package-read access through Railway PostgreSQL
   Settings → Source → Registry Credentials, never ordinary variables/chat/Git.
   Railway Pro supports this flow and handles the GHCR username. Classic scopes
   can reach other packages accessible to that owner; they are not per-package.
   Do not retrieve or install a credential now.
5. **Exact shutdown authorization.** The prepared runner is independent of the
   local Ubuntu host and uses only the separately installed staging project token.
   Scope binds the IDs above, frozen application/image, synthetic-only state,
   zero worker replicas and raw-byte SHA-256. For the proposed 19:00 WAT T0:
   first allocation at 18:00 UTC, stop at 18:05 UTC, verification deadline
   18:10 UTC, emergency authority expires at **18:15 UTC / 19:15 WAT**.
   Install/start only with explicit approval. The controller enforces expiry on
   every polling iteration; this does not establish a native token expiration.
   Verify provider-supported credential expiry separately, or explicitly approve
   manual revocation after the window. The project token is broader than stop-only.
   The owner must authorize the exact final scope; no silent time renewal. Run the
   offline scope validator, then the approved runner and wait for its inventory-
   checked, durably audited `armed` event before any billable allocation. Abort on
   queue delay, stale scope, token failure or inventory mismatch.
6. **19:00–19:05 WAT — approved synthetic smoke only.** Confirm owner attendance
   through 19:15, refreshed costs, runner health and exact deployed source/digest.
   If readiness takes the whole window, abort rather than extending it. Check
   frontend authentication/health, API authorization, migrations and database
   health using synthetic records only. No historical records or real identities.
7. **19:05 WAT — shutdown begins automatically.** Cancel pending/replacement
   deployments, stop frontend/API, observe them stopped, then stop PostgreSQL.
   Operator checks progress directly; GitHub alerts require no acknowledgement and
   never gate stop/cancel. An ambiguous mutation is not automatically retried.
8. **19:10 WAT — verify compute stopped.** Require repeated terminal deployment
   state with `deploymentStopped: true`, no queued builds/replacements and worker
   zero. A stop acknowledgement or stopped watchdog process is not evidence that
   workloads stopped. Record sanitized state and audit evidence.
9. **Through 19:15 WAT — emergency human fallback.** If shutdown is unconfirmed,
   the attending workspace owner uses Railway Stop/Cancel and directly verifies
   state without waiting for an alert. No second administrator is available.
   A GitHub outage, Railway outage, lost runner or revoked token cannot be fully
   mitigated automatically. Minute 15 ends controller authority, not billing:
   surviving resources require human intervention and fresh authority if necessary.
10. **Afterward — approved cleanup and costs.** Separately approved deletion may
    remove only the rehearsal volume/services. Stopping compute does not delete
    storage or guarantee zero charges. Preserve the backup bucket and unrelated
    services, revoke approved temporary credentials, inspect residual resource
    inventory and delayed charges. Do not introduce sensitive persistent state
    until independent encrypted recovery and a restoration drill are complete.

The primary operator's minimum oversight is verified runner readiness/startup,
direct minute-five and minute-ten checks, and emergency availability through minute
fifteen. No repeated manual notification checks are required. Local workflow tests
remain passing (four); the watchdog's previously passing 32 offline cases do not
prove live project-token permission or remote shutdown. No runner was installed,
credential created, service/volume provisioned, watchdog armed, deployment started,
DNS changed or protected database/accounting record touched during this preparation.

## Smoke-test safety preparation and approved synchronization

The five approved commits were pushed without rewriting history; `origin/main`
was verified at `82095682e1da58a432245cfe9311af43fcee5d95`.
The frozen application revision and private PostgreSQL digest remain unchanged.
No CI was dispatched. The additional safety changes described here are local only.

**Primary operator:** the authenticated workspace owner confirmed availability on
Thursday **5 November 2026, 19:00–19:15 WAT (18:00–18:15 UTC)** and will acknowledge
failure alerts. This is availability, not deployment or shutdown-arming approval.
There is no separately verified backup administrator.

**Human alert test:** one non-sensitive test issue mentioning `@pyco404` was
successfully created in the verified private infrastructure repository:
https://github.com/pyco404/supa-agent-infra/issues/1 . Human notification receipt is
pending confirmation; issue creation does not prove notification delivery.
No secrets, database records or private operational output were included.

**Independent runner preparation:** the inert templates
`deployment/railway-smoke-runner.yml` and
`deployment/railway-smoke-failure-monitor.yml` are for the private infrastructure
repository only. They are not installed under `.github/workflows`, scheduled or
armed. The manual runner downloads exact hash-checked controller/test bytes from
an explicitly selected immutable application commit, runs offline regressions,
and verifies the exact separately approved ten-minute scope before accessing the
future project-scoped shutdown token. The application commit containing these new
bytes must be separately approved for synchronization before the runner can use it.
It checks current Railway inventory and writes durable audit evidence before
emitting its `armed` readiness event. No billable resource may begin before that
event. Provisioning IDs, scope timestamps and authorization must be reviewed
separately; do not silently set the scope's runner-verification claim to true.

The completion listener runs independently of the shutdown job and posts one
private issue on failure, cancellation, timeout or runner startup failure. It uses
only job-scoped `issues: write` and `contents: read`, never Railway credentials,
and rejects foreign/public/untrusted events. It does not retry issue creation.
GitHub notification preferences and an attending human remain essential: a
GitHub-wide outage could prevent both runner and monitor. Runner-loss scenarios
are tested using offline event fixtures, not an installed cloud runner.
Private job logs retain bounded sanitized audit output when the runner survives;
runner loss can prevent final flushing. No private environment reviewer protection
is assumed available on this account.

**Timing and manual fallback:** if separately authorized to start at 19:00 WAT,
start shutdown by **19:05**, verify stopped compute and canceled pending builds by
**19:10**, and keep the owner available for emergency verification through
**19:15**. The scope's T0 must cover the first billable allocation, including the
volume; time is not extended for build queues or cold startup. Do not provision
before the runner is ready. If it cannot become ready within the approved window,
abort. A stopped deployment is verified from Railway state, not a successful
mutation acknowledgement. Check frontend/API first, then PostgreSQL, every pending
build/replacement and the zero-replica worker. If the runner fails, the attending
owner uses Railway's deployment Stop/Cancel operations and verifies terminal
states manually. Do not alter unrelated workspace services. Retained volume and
other storage continue to need separate cleanup approval; this runner never
deletes resources. Obtain that approval before starting if immediate deletion is
needed for the spending policy. No automatic deletion or live cleanup was tested.

**Refreshed read-only cost snapshot:** current usage $18.49482742, forecast
$33.57587601, shared hard limit $40, forecast headroom **$6.42412399**. The workspace
contains one verified administrator. These are a point-in-time snapshot and do not
reserve headroom against other workloads.

Using three 24-GB/24-vCPU default ceilings as a conservative compute scenario:
ten minutes costs $0.499680 compute; fifteen minutes costs $0.749520 compute.
Adding 1 GB used volume for the same interval and a **1-GB egress allowance** gives
**$0.549715 / $0.799572** Railway estimates. Proposed private Linux runner bounds
add up to $0.120 for twenty runner minutes and $0.018 for three notifier minutes
at $0.006/minute, giving approximately **$0.687715 / $0.937572** across providers.
No free GitHub allowance is assumed. Actual usage, billing rounding, registry
transfers, egress beyond 1 GB, retained storage, or workloads surviving minute 15
can increase charges. GitHub runs are not deployed/charged by this preparation.
The small proposed CPU/RAM limits remain unverified; do not base a guaranteed
estimate on them. The $1 policy is monitoring and approval, not an enforced cap.
There is insufficient margin to promise a hard $1 outcome. Recheck usage before
arming; abort if the conservative bound plus known extra costs exceeds approval.

**Local evidence:** 32 synthetic shutdown tests pass, including minute-five stop,
restart deadlines, minute-fifteen expiry, SIGTERM, failed mutation and no automatic
mutation retry. Four inert-workflow tests pass, including failure/cancellation/
timeout events, malformed/foreign-event rejection, restrictive file permissions,
immutable source hashes and Python/Bash syntax. These do not establish real
credential validity or live shutdown/notification reliability.

**Decision: NO-GO to start now.** Still required separately: human notification
acknowledgement; approval to push/install these additional templates in the private
infrastructure repository; scoped shutdown-token and GHCR read-only credential
provisioning/installation; exact service/volume provisioning approval; exact
shutdown-scope authorization and verified armed runner; synthetic smoke deployment
and spending approval; and retained-resource deletion approval if desired. Refresh
headroom and confirm owner attendance immediately beforehand. No primary database,
historical accounting hold, application source, frontend, Railway setting, DNS,
credential or live workload was changed. Independent encrypted recovery remains
mandatory before introducing sensitive or irreplaceable persistent data.

## Current decision — support-free smoke test

Do not contact Railway Support. The frozen application remains
`bfc29f500af08e0e2cc4676b8950b5b17f26007a`; this is infrastructure preparation only.
One frontend, one restricted API and one PostgreSQL replica remain the proposed
running stack; the worker is zero globally and in Amsterdam. Keep the verified
private PostgreSQL digest below, synthetic data, private API/database networking,
authenticated frontend and all paid/financial/publishing operations disabled.

### What can be validated without provisioning

The installed CLI 5.71.0's documented
[`railway config plan`](https://docs.railway.com/cli/config) previews changes without
applying them. A temporary proposal with exact binary-byte RAM limits and CPU
ceilings (frontend 536870912/0.5, API 268435456/0.25, PostgreSQL 536870912/0.5)
passed the read-only graph/diff preview with no diagnostics. It did not provision
anything, call the resource-update mutation or prove runtime enforcement/stability.
No documented resource-free server-side acceptance/enforcement test was established.
The exact small maxima and minimum-field semantics remain unverified, but no longer
need to block proposing a short test at the confirmed default ceilings.

The initial preview revealed a real safety defect: the whole-project graph omitted
the existing bucket and proposed deleting it. Nothing was applied. The IaC now
includes the existing `supa-agent-backups` bucket in `ams`; preservation has a focused
regression. The corrected read-only preview succeeded with zero destructive changes
and no diagnostics, preserving the bucket, 1000-MB volume request and zero worker
replicas in the rendered graph. Do not use the earlier destructive preview as an
apply artifact. Temporary proposal files were removed; original variables were not
decrypted and
no CLI link, credential or remote setting was changed. The SDK initially mistook
the parent Python executable for the CLI via `_`; the preview was corrected by
pointing that process-local executable hint at the genuine installed CLI, not by
spoofing a version or bypassing its compatibility check.

### Proposed single attempt and cost scenarios

Prepare a **10-minute** smoke scope, starting at the first billable resource
(including volume allocation), not the first successful health check:
begin shutdown at minute 5, target verified stopped compute by minute 10, and allow
only emergency cleanup through minute 15. The emergency authority is not permission
to operate for 15 minutes normally. If startup/builds are incomplete at minute 5,
cancel them and retain diagnostics; do not extend or automatically retry the test.
Cold builds may consume the useful window, so this is a technical startup probe,
not a promised full browser journey. Railway builds have no separate build charge;
any already running database/application is still billed during other builds.
Approve no automatic source/image updates, PR previews or overlapping rollout.

Use the confirmed Pro default 24-GB/24-vCPU per-replica ceilings for conservative
arithmetic rather than assuming the small settings are enforceable:

| Scenario: three replicas, worker zero | Compute | With 1 GB used storage + 1 GB egress |
| --- | ---: | ---: |
| 10 minutes at proposed small caps, conditional | $0.008888 | $0.058923 |
| 10 minutes at confirmed default ceilings | $0.499680 | $0.549715 |
| 15 minutes at confirmed default ceilings, emergency scenario | $0.749520 | $0.799572 |
| Six hours at confirmed default ceilings | $17.988480 | $18.039730 |

These are rate scenarios, **not an enforced $1 cap or measured usage predictions**.
The default-ceiling calculation assumes exactly three replicas, no overlap and
shutdown within the stated time. Egress and stored-byte allowances are not platform
caps. Keep traffic controlled and record actual volume allocation/used bytes before
approval; never silently substitute a larger retained volume. Independent runner/CI
charges need review and must not be assumed free. API outages or a lost watchdog can
extend billing. Storage survives compute shutdown and needs a separately approved
retention/deletion decision. The existing shared $40 limit is not changed.

One candidate independent host is a manually dispatched job in the existing private
infrastructure repository, on a standard Linux 2-core runner with a proposed
20-minute job timeout. GitHub's [current runner pricing](https://docs.github.com/en/billing/reference/actions-runner-pricing)
is $0.006/minute: budget $0.12 for 20 billable minutes without assuming any remaining
included allowance. Rounding, setup/cleanup overhead, evidence storage, notifier
hosting and any private-registry transfer charges still need review. Such a runner
must be running and armed before T0; queued/cron-only execution is not adequate.
No workflow was created, pushed or dispatched. This proposes an independently hosted
shutdown controller, not a move of Supa Agent off Railway.

Latest read-only workspace usage: **$18.483313516963086**, forecast
**$33.57650728003991**, forecast headroom **$6.42349271996009** below $40.
Recheck immediately before any paid resource approval. The $1 amount stays an
incremental monitoring/approval policy, not a project hard limit.

### Shutdown and failure-alert verification

`scripts/railway-shutdown.py` now accepts a separate **version 2** manifest with
`profile: "smoke-10m"`, T0+300 shutdown and T0+600 deadline. Version 1 six-hour
scopes retain their original rules; no existing approved scope is renewed or
converted. A smoke scope gets its own exact-byte SHA-256, service inventory and
human approval. No real scope timestamps/service IDs have been fabricated.

The protected runner now requires a **distinct** owner-only `--failure-alert-file`
in addition to its private audit file. On failure it writes a durable, sanitized
operator-alert record, emits structured stderr, emits an error annotation under
GitHub Actions and exits nonzero. The host must connect that interface to an
approved notifier; no email, webhook, external recipient or schedule is configured
or contacted here. An isolated local child process was actually terminated with
SIGTERM and produced the expected record/annotation/nonzero exit. Tests also cover
interruptions, API/auth failures, missing/broken alert storage, disk errors, secret
exclusion and absence of false alerts on verified success. All **31** synthetic
watchdog tests and **14** IaC tests passed, with the focused TypeScript check.

SIGKILL, runner loss and loss of all notification channels cannot be handled by the
dying process. The independently hosted runner therefore needs an external job-loss
monitor, tested human alert delivery and an available primary operator before launch.
No independent runner has been installed or armed. Cleanup never deletes the backup
bucket or volume automatically; no financial/provider operations are implemented.

### One deployment decision and separate approvals

**NO-GO to provision or deploy now.** The recommended next review is one ten-minute,
synthetic-only smoke attempt using known default ceilings, without waiting for support.
Remaining approvals are:

1. Exact three-service/zero-worker smoke plan and one attempt, acceptance of monitored
   spending/egress/shutdown risk, and the volume's confirmed allocation/retention.
2. An independent trusted runner, its assessed costs, scoped shutdown credentials,
   failure notifier/job-loss monitor and tested primary-operator alert coverage.
3. Private GHCR pull credentials and separate synthetic runtime/migration accounts;
   an authenticated temporary Railway hostname, with no official-domain DNS change.
4. Additive provisioning, exact frozen-source builds, the actual service IDs/start
   time and smoke-scope digest, deployment and arming; then separately scoped cleanup.
5. Any source push or CI dispatch. Neither is authorized or performed in this review.

## Verified target and image

- Workspace: Daniel 's Projects, `e319f68a-9897-49ed-b119-743f1f846aa0`.
- Existing Supa Agent project: `99648d2a-69a1-4d8c-98d0-7f2a26543c9c`.
- Staging: `ff0b2092-e42b-4a89-ae25-441ab95d92d8`.
- Live workspace plan: **PRO**, independently read from the workspace API.
- No application services were present at review. Preserve the existing Amsterdam
  `supa-agent-backups` bucket; it is not rehearsal cleanup inventory.
- PostgreSQL: `ghcr.io/pyco404/supa-agent-postgres-infra-98c1006fcb1d@sha256:9409009a40c0929052306d068c14bef92f3ecc67662c453540e2c9b95ae18fc8`.

`config/railway-staging-image.json` records image acceptance separately from deployment
authorization. The authoring graph accepts only this exact image, verified private
repository association, anonymous denial, Linux/amd64 platform, 16 verified layers,
configuration digest and a hash of the retained independent verification report.
No fallback to the two public packages, a mutable tag or a generic PostgreSQL image
is permitted. The previous publication workflow remains failed; subsequent independent
read-only verification established image integrity and private access.

The image's recorded UNKNOWN Gosu advisory, CVE-2026-39824 in golang.org/x/sys v0.1.0,
remains visible. This integration is not a fresh vulnerability scan. Its use remains
limited to the accepted synthetic-only six-hour rehearsal.

## Registry authentication — prepare, do not create or install

[Railway private registries](https://docs.railway.com/builds/private-registries)
require Pro, which the selected workspace already has. For GHCR, Railway accepts
a GitHub access token and handles the username. Use a separately approved, expiry-bound
**personal access token (classic), read:packages only**, whose owner already has read
access to the specific private package. Package scopes on classic PATs are not
per-package restrictions; owner access can cover other packages. Do not claim narrower
scope than actually granted. No write:packages, delete:packages or new repo scope is
needed for this granular package. A dedicated read-only identity would require its
own approved account/package access grant.

Do not install the existing interactive GitHub CLI credential in Railway, copy its
other scopes, reuse the expired Actions GITHUB_TOKEN, or reuse S3/model credentials.
The owner must provision the approved credential through a private channel. Never
put it in chat, Git, ordinary service variables, image layers or logs. After separate
approval, install it in the PostgreSQL service's Source → Registry Credentials only,
verify a private by-digest pull, and record expiry/revocation responsibility. No token
has been generated, rotated, retrieved or installed for this Railway integration.

## Proposed graph and resource review

Four compute services, one replica each, Amsterdam `europe-west4-drams3a`:

| Service | Port / readiness | Required boundary |
| --- | --- | --- |
| supa-web | 3000 / /healthz | visitor/operator authentication; separately approved HTTPS access |
| supa-api | 8080 / /ready | private network; distinct restricted runtime login; no runtime DDL |
| supa-worker | 8081 / /ready | private network; offline, disabled, zero autonomous cycles |
| supa-postgres | private 5432 | exact image above; TLS; separately scoped migration administrator |

The graph additionally declares one `supa-postgres-data` volume, proposed **1000 MB**,
mounted at `/var/lib/postgresql/data`, with no automatic backup schedule for disposable
fixtures. The live plan's unspecified-volume default is **50,000 MB**; do not accept
that default silently. Confirm the explicit size in the eventual additive provider
plan. Storage and persistence do not establish independent recovery.

Application shutdown grace is 60 seconds; PostgreSQL receives 120 seconds. These are
supported SDK `deploy.drainingSeconds` fields. The installed SDK also exposes
`deploy.limitOverride.containers.{cpu,memoryBytes}` and `volume.config.sizeMB`.
Its schema accepting numbers does not prove provider acceptance of every value.

| Service | Earlier RAM target | Earlier CPU target |
| --- | --- | --- |
| Frontend | 512 MiB | 0.5 vCPU |
| API | 256 MiB | 0.25 vCPU |
| Disabled worker | 128 MiB | 0.125 vCPU |
| PostgreSQL | 512 MiB | 0.5 vCPU |

The live PRO entitlement reports `containers.minReplicaCpu = 4` and
`containers.minReplicaMemoryBytes = 4000000000`, with maximums 24 vCPU and
24,000,000,000 bytes. The public documentation does not resolve the precise effect
of those minimum fields on small custom limits. **Earlier small caps are unverified**;
they are not installed or represented as enforced in the authoring graph. Obtain
provider confirmation/read-only planning evidence before accepting them. Do not
silently round up or represent these ceilings as minimum charged allocations.
Startup and workload stability at the small targets have not been measured.

No domain/TCP proxy is requested by the graph. Do not attach trysupaagent.xyz, alter
DNS, expose API/PostgreSQL, or generate a frontend domain by implication. The exact
authenticated access mechanism and origin remain a separate approval requirement.
The three application services currently reference GitHub main; freeze the separately
approved application revision and disable automatic deployments before launch. Their
Railway rebuilds are not asserted byte-identical to retained CI application images.

## Refreshed cost decision

Read-only CLI usage and live workspace-plan evidence were retained locally.
Billing period ends 18 November 2026, 16:34:07 UTC.

| Workspace measure | USD |
| --- | ---: |
| Current usage | 18.407073 |
| Forecast usage | 33.581035 |
| Shared hard limit, unchanged | 40.00 |
| Current headroom | 21.592927 |
| Forecast headroom | 6.418965 |

[Current Railway container pricing](https://docs.railway.com/pricing/plans):
RAM $0.000231/GB/minute, CPU $0.000463/vCPU/minute, volume
$0.000003472222222/GB/minute, egress $0.05/GB. Service builds and image storage/pulls
are not separately charged by Railway under the documented build policy. GitHub
registry/Actions billing is separate and must not be silently assumed covered.

- Earlier assumed average usage (aggregate 0.75–1.5 decimal GB RAM, 0.05–0.2 vCPU)
  gives $0.07–0.16 compute for six hours; this is an assumption, not a staging metric.
- If all earlier small caps were accepted and sustained, six-hour compute would be
  about **$0.352**. A full 1 GB volume for six hours adds **$0.00125**; 1 GB of egress
  adds **$0.05**, totaling about **$0.403** under those conditional assumptions.
- A sustained four-service scenario at 4 GB and 4 vCPU each is about **$3.997** compute.
  This is a usage scenario, not a minimum bill or an installed limit.
- With no confirmed custom caps, four services at the reported 24 GB/24 vCPU maxima
  would be about **$23.985** compute in six hours, exceeding forecast headroom.
- Retaining 1 GB after shutdown costs up to roughly $0.15 per 30-day month; a volume,
  snapshot or backup can continue billing when compute is stopped. Retention needs
  an approved deadline or separately approved deletion. The earlier 5 GB option is
  not the current proposal.

The $1 value is an **operator monitoring/approval policy**, not an enforced cap.
The shared $40 hard limit can affect unrelated workloads and must not be changed.
Costs below $1 are plausible at low consumption, but not bounded sufficiently by
the current supported-limit evidence. Therefore deployment is **NO-GO** under the
current envelope. Recheck usage immediately before any later approval. Do not
upgrade the plan, enlarge the envelope, provision a probe service, or test resource
limits through a real deployment to resolve this preparation blocker.

## Six-hour startup and shutdown procedure — for separate approval

These control-plane command signatures were checked against installed CLI 5.71.0
and live read-only schema introspection. There are no rehearsal services to stop
now; end-to-end live shutdown remains a rehearsal acceptance test, not a claimed
completed operation.

1. Resolve resource/cost limits. Name a launch/shutdown operator and fallback;
   assign an independent deadline reminder before launch. Capture exact project,
   environment, four service IDs, deployment IDs and one volume ID in a session
   inventory. Exclude the existing bucket and unrelated projects/resources.
2. Approve credentials, additive provisioning, bootstrap and deployment separately.
   If provisioning implicitly launches workloads, deployment approval must already
   cover that operation. Start the clock with the first billable rehearsal workload,
   not after health checks. Record T0 and an absolute T0+6h deadline.
3. Bootstrap only a fresh synthetic staging database, approved schema-only migrations
   and SQLx checksums using separate administration. Supply restricted API/worker
   logins; never import the primary dump, historical ledgers, retained evaluations,
   actual owner records, wallets, model keys or private memories.
4. Start PostgreSQL, then the restricted API, disabled worker and frontend. Verify
   TLS, /ready and /healthz, unauthorized/role boundaries and zero cycles. Require
   provider/scheduler/signing/publishing/trading/launch restrictions to remain off.
   Finite synthetic agent activity requires its own authorization; none is included.
5. Check incremental usage every ten minutes and at startup/shutdown. Propose early
   warning at $0.50 and begin shutdown at $0.75, or earlier on abnormal usage. Billing
   lag prevents treating those triggers as a guaranteed $1 ceiling.
6. Begin normal shutdown by T0+5h50m: close frontend access, disable automatic source
   deployments/image updates, prevent new requests and verify no active task leases.
   Remove frontend, API and worker deployments, then PostgreSQL gracefully. Use the
   inventoried deployment IDs; cancel queued/building deployments as well. Do not
   treat disabled worker scheduling, a failed health check or an idle process as
   proof that its compute billing stopped.
7. Read all deployment states until none of the four services has Active, Deploying,
   Building, Initializing, Queued or replacement workloads. Ensure terminal
   Removed/Cancelled states, stopped replicas and no pending source-triggered run.
   Escalate to the fallback operator before T0+6h if this cannot be established.
8. Review volume, backup and service inventory for remaining charges. With separate
   explicit cleanup approval, delete only the inventoried synthetic volume and
   unused rehearsal services. Otherwise retain the volume with a cost/deletion
   deadline; do not claim all billing ended. Preserve the backup bucket and all
   unrelated resources. Revoke only approved rehearsal credentials, record final
   usage and check delayed billing afterward. Restart requires new approval.

Supported commands to prepare with exact IDs; **do not execute now**:

```sh
railway deployment list --project PROJECT_ID --environment ENVIRONMENT_ID --service SERVICE_ID --json
railway api 'mutation($id: String!) { deploymentRemove(id: $id) }' --var id=DEPLOYMENT_ID
railway api 'mutation($id: String!) { deploymentCancel(id: $id) }' --var id=QUEUED_OR_BUILDING_DEPLOYMENT_ID
railway volume --project PROJECT_ID --environment ENVIRONMENT_ID list --json
# Only after separate approval to discard this synthetic volume:
railway volume --project PROJECT_ID --environment ENVIRONMENT_ID delete --volume VOLUME_ID --yes --json
```

Do not rely solely on `railway down`: it removes only the latest successful
deployment and does not establish absence of concurrent/queued workloads or remove
volumes. See [deployment lifecycle](https://docs.railway.com/deployments/reference),
[CLI down](https://docs.railway.com/cli/down) and
[volume billing](https://docs.railway.com/pricing/plans).

## Exact remaining approvals and prerequisites

1. Provider-supported resource settings and a sufficiently bounded cost plan;
   neither the $1 policy nor the shared $40 hard limit is changed here.
2. Private-channel creation and service-specific installation of read-only GHCR
   credentials, plus fresh synthetic runtime/migration credentials and their expiry.
3. Exact additive four-service/one-volume provisioning plan, storage retention,
   schema-only bootstrap and authenticated frontend access mechanism.
4. One six-hour deployment at reviewed source/image identities, with named operators,
   monitoring, verified shutdown responsibility and separately authorized cleanup.
5. Any source push remains separately approved. No code was pushed in this review.

Synthetic test state must be disposable and reconstructible. Independent encrypted
backup upload, verified encryption/signing identities, off-machine recovery keys
and a restoration drill remain mandatory before persistent sensitive data is used.

## Focused validation

All 13 Railway authoring tests passed, including exact image identity, fail-closed
attestation checks, one-replica service boundaries, explicit volume sizing/mounting
and graceful shutdown settings. The focused TypeScript check and whitespace check
passed. No container rebuild or full application-suite repeat was needed.

An optional local runtime-profile check stopped with `disposable_profile_psql_failed`
while reading the retained disposable fixture, before its separate cluster creation
or startup. That fixture was not restarted. This is an unavailable local runtime
check, not new container-execution evidence. The prior successful CI validation is
preserved. Live Railway startup, custom resource-limit acceptance and end-to-end
shutdown remain unperformed pending separate approval.

## GO/NO-GO follow-up after authorized source synchronization

The five approved commits were pushed without rewriting history. Remote main was
independently checked at `bfc29f500af08e0e2cc4676b8950b5b17f26007a`.
The pre-push audit passed all 1,574 commits and the targeted secrets scan found no
matches. This follow-up is a separate local documentation commit, not authorization
to push additional changes or run CI.

### Resource and cost findings

Official [replica-limit documentation](https://docs.railway.com/pricing/cost-control)
describes CPU/RAM limits as maximum resources available to each replica, rather
than advisory requests or reserved billable allocations. Exceeding available
memory can crash the application. The live workspace entitlement still reports
`minReplicaCpu: 4` and `minReplicaMemoryBytes: 4000000000`; its scalar has no schema
description explaining the minimums. Neither public documentation nor read-only
schema inspection establishes acceptance of the earlier 0.125–0.5 vCPU and
128–512 MiB values. No mutation was used to test them. Their acceptance and
workload stability remain unresolved; do not approve the $0.403 estimate as a bound.

The refreshed read-only usage query succeeded after a transient DNS failure:
current usage **$18.41912976945389**, forecast **$33.57974022494188**, unchanged hard
limit **$40**, current headroom **$21.58087023054611**, forecast headroom
**$6.42025977505812**. The earlier conditional compute scenarios remain valid:
small targets approximately $0.352, four sustained 4-GB/4-vCPU replicas $3.99744,
and the four default 24-GB/24-vCPU maxima $23.98464, all for six hours.
The latter two are scenarios, not minimum charges. Costs remain insufficiently
bounded for the $1 monitoring policy, despite adequate headroom for a low-usage run.

[Volume documentation](https://docs.railway.com/volumes/reference) clarifies billing
is for used bytes, including approximately 2–3% filesystem metadata, not the whole
allocated capacity. The Pro default allocation is 50 GB; paid volumes can grow but
cannot shrink. The explicit 1000-MB authoring request remains subject to provider
plan confirmation. At most 1 GB used costs about $0.00125 for six hours or $0.15
for 30 days. A default 50-GB empty volume can already use roughly 1–1.5 GB of
metadata. Stopping compute does not delete storage; do not silently accept the
default or claim immediate elimination of all charges on deployment removal.

### Frozen application source and authenticated access

The intended application SHA is exactly
`bfc29f500af08e0e2cc4676b8950b5b17f26007a`, independently of subsequent documentation
commits. Frontend, API and worker must use that revision; automatic source/image
updates must be disabled by separate approval. The authoring graph's `main` source
is not an immutable pin. Before launch, the approved deployment procedure must
select this revision and prove the resulting deployment source matches it; stop
if the chosen Railway deployment method cannot do that. No live source setting was
changed here.

The successful full-stack CI at
[run 41679768139](https://github.com/pyco404/SupaAgent/actions/runs/41679768139)
used `d43289535d80a864a3f9f132cdb4b42be77a6f2e`. Rust crates, migrations, dependency
locks, API/worker Dockerfiles, frontend auth/proxy and application contracts have
not changed since that run. The subsequent frontend changes are official-domain
metadata and links. Thus existing runtime compatibility evidence remains relevant,
but it does not certify new image bytes. The current GitHub-source proposal needs
three fresh application builds and startup checks at the frozen revision; do not
rebuild PostgreSQL. Railway service builds have no separate build-resource charge
under [current pricing](https://docs.railway.com/pricing/plans); running workloads,
storage and egress still consume the rehearsal envelope. No build was dispatched.

The existing frontend staging proxy rejects unauthenticated pages and API requests,
enforces the configured HTTPS origin/host and maps each named operator to a distinct
server-side Rust credential. Staging visitor credentials cannot become operator
credentials. Only minimal `/healthz` is intentionally unauthenticated; static bundles
contain no private records. An eventual Railway-generated HTTPS hostname can be
used without touching trysupaagent.xyz DNS, but hostname creation and matching
`SUPA_SITE_ORIGIN`/credential installation require separate approval. Keep API and
PostgreSQL private. Ten network-free assertions against the actual auth/proxy modules
passed using synthetic credentials: missing authentication, visitor/operator
separation, cross-origin rejection and missing-origin fail-closed behavior. All 13
Railway authoring tests also passed. No new live browser/container claim is made.

### Operators, automatic safeguard and cleanup

Read-only workspace membership shows exactly one member, the authenticated current
user, with ADMIN role. Propose this workspace owner as primary shutdown operator;
explicit availability/acceptance for the launch window remains pending. No separate
authorized workspace member is currently available as backup. Do not invent a backup
or invite/grant access automatically.

An independently hosted deadline watchdog is technically feasible using Railway's
supported deployment APIs, but no built-in six-hour TTL for ordinary services was
established. A separately approved implementation would need an absolute deadline,
the exact four service IDs/project/environment allowlist, an environment-scoped
project token held by a separate trusted runner, and stop/cancel-state verification.
Project credentials are broader than a stop-only capability; disclose and approve
that scope. It must not delete volumes, touch the bucket or unrelated services,
restart workloads or infer success after an API failure. A local laptop timer is
not independent protection against loss of that laptop or its network. GitHub cron
can be delayed and is not a strict deadline guarantee. No timer, token, workflow or
schedule was created. Without a verified backup operator or separately approved,
tested independent safeguard, shutdown responsibility remains NO-GO.

Live read-only schema inspection confirms `deploymentStop(id: String!): Boolean!`
and deployment cancellation support. Prefer stopping compute explicitly and retaining
its audit history; list every service's deployments and cancel every pending build
or queued replacement as well. Do not confuse history removal with proof of stopped
compute. The [deployment API](https://docs.railway.com/integrations/api/manage-deployments)
documents stop/cancel operations. No stop or cancellation mutation was executed.
Begin by T0+5h50m and verify no running or pending deployment remains by T0+6h.

Inventory volumes/backups after compute shutdown and separately approve deletion
of only the disposable rehearsal volume/resources. Volume deletion has a documented
48-hour recovery window; check residual inventory and delayed usage rather than
promising immediate zero charges. Preserve the existing independent-backup bucket.
Its presence still does not establish encrypted recovery; sensitive state remains
prohibited until key verification, off-machine recovery storage, encrypted upload
and disposable restoration are complete.

Separate approvals remain: supported resource/cost envelope; named shutdown
responsibility or independent safeguard; read-only GHCR PAT creation/installation;
fresh synthetic identity/runtime/migration credentials; exact provisioning and
authenticated hostname/source-freeze settings; six-hour deployment/monitoring/stop;
and final volume/resource cleanup. No authorization is combined or assumed.

## Financial feasibility decision — three-service alternative

**NO-GO for the six-hour $1 policy with the evidence currently available.** This
does not establish a $4 minimum bill: consumption billing and the meaning of the
plan's minimum-cap fields are separate questions. The frozen application revision
remains `bfc29f500af08e0e2cc4676b8950b5b17f26007a`.

### What is confirmed, and what is not

The fresh read-only Pro entitlement still reports `minReplicaCpu=4` and
`minReplicaMemoryBytes=4000000000`. These are plan metadata, not measurements or
proof of reserved/charged service allocations. The API describes
`serviceInstanceLimits` as merged limits including plan defaults. Its update input
accepts Float CPU and decimal-GB memory values, but its schema supplies no lower
bounds. A Float type is not proof that every positive fraction will be accepted.
Official documentation establishes actual-consumption billing and hard maximum
resource limits; it does not explain these exact minimum fields. Inspection of
publicly served dashboard modules did not establish authoritative minimum-cap
validation. No settings mutation was used as a probe.

Therefore the proposed frontend 512 MiB/0.5 vCPU, API 256 MiB/0.25 vCPU, PostgreSQL
512 MiB/0.5 vCPU and optional worker 128 MiB/0.125 vCPU remain **unconfirmed caps**.
The 24 GB/24 vCPU per-replica default ceiling is confirmed in live metadata and the
[official right-sizing guide](https://railway.com/guides/right-size-cpu-memory).
Neither successful startup nor peak consumption at the small targets has been
measured. Do not describe any of them as a supported, stable deployed configuration.

### Worker treatment

The cheaper alternative is one frontend, one restricted API, one private PostgreSQL
instance and **zero running worker replicas**. The
[official scale command](https://docs.railway.com/cli/scale) accepts non-negative
replica counts and explicitly supports removing all replicas from a region with
`REGION=0`. This differs from a running process with `SUPA_WORKER_ENABLED=false`,
which still consumes memory and maintains health heartbeats.

Source inspection confirms frontend/API startup does not call the worker's health
endpoint. API scheduling remains false; no scheduler cycles or agent activation
are included. The operator UI must show the worker absent/stopped, not healthy.
Initial rehearsal coverage would exclude Railway worker startup/heartbeat tests;
retain their prior CI evidence without claiming they ran on Railway. Keep the
worker definition dormant with zero in **every** region, or omit its provisioning
until separately approved. No worker was provisioned or scaled. The current IaC
still declares one worker replica; it must not be applied unchanged for this
alternative. A zero-replica alternative requires its own reviewed configuration.

### Six-hour costs, with explicit assumptions

Fresh workspace usage is **$18.428253594249323**, forecast **$33.57914204794395**,
unchanged hard limit **$40**, forecast headroom **$6.42085795205605**.
The compute hard limit remains workspace-wide and includes unrelated projects.

Use `360 * (GB_RAM * 0.000231 + vCPU * 0.000463)` for six-hour compute,
plus used-volume GB-minutes and $0.05 per GB egress. The clock begins with the first
billable rehearsal workload; startup is included, not an additional free interval.

| Three running services, worker at zero | Six-hour compute | With 1 GB used volume and 1 GB egress |
| --- | ---: | ---: |
| Earlier small caps, only if accepted and stable | $0.319965 | $0.371215 |
| 4 GB/4 vCPU each, sustained usage scenario | $2.998080 | $3.049330 |
| Confirmed 24 GB/24 vCPU default ceilings, sustained | $17.988480 | $18.039730 |

The table is not a predicted consumption range or an enforced spend cap. The
first row is conditional; the second is not a minimum charge; the third bounds
compute only for three replicas over exactly six hours at those default ceilings.
Public traffic/egress is not capped by these assumptions. Private API/database
networking avoids inter-service public egress. Service builds have no Railway
build-resource charge; independent watchdog hosting/CI costs need separate review.
An allocated 1-GB volume with up to 1 GB used adds $0.00125 for six hours; its
acceptance must be confirmed before provisioning. Retained used storage continues
at approximately $0.15/GB per 30-day month after compute shutdown.

Replica maxima, zero worker replicas, private networking, limited authenticated
traffic and prompt shutdown meaningfully reduce costs. They do not enforce a
$1 session allowance. [Usage-limit documentation](https://docs.railway.com/cli/usage)
specifies whole-dollar workspace hard limits from $10, not project/session limits;
soft alerts do not stop workloads. Lowering the existing shared limit would affect
unrelated services and is not proposed. Serverless is not a fixed deadline: database
connections/polling can prevent sleep and incoming requests can wake services.

### Shutdown without a second administrator

Retain the authenticated owner as proposed primary and obtain explicit availability
for the window. No independently verified second operator is currently available.
Before reconsidering GO, require a separately approved deadline watchdog on a
trusted runner independent of the owner's Ubuntu machine and of these Railway
workloads. A manually dispatched private-infrastructure CI job is a candidate, not
an already configured safeguard. Confirm runner availability, maximum job duration
and incremental hosting/Actions charges before choosing it; delayed cron alone is
not a reliable six-hour cutoff.

The watchdog must be armed and observable **before** any billable workload starts,
bind one absolute deadline and exact project/environment/service IDs, and initiate
shutdown early (proposed T0+5h30m, leaving recovery time before six hours). Use a
separately approved environment-scoped project token; it has broader authority than
stop-only and needs explicit acceptance. Cancel queued/building deployments, stop
all active deployments, remove worker replicas and verify no replacement compute
appears. Disable automatic source/image deployments through separately approved
settings. Signal success only after read-back verifies stopped state, and alert the
primary on any failure. No token, CI job, timer or setting was created. Volume
deletion remains a separate action and is not part of watchdog authority.

### Exact approvals and missing evidence

1. Obtain authoritative Railway confirmation of the minimum allowed CPU/RAM maxima
   for this Pro workspace, whether the four small targets above are accepted, their
   units/enforcement and whether a 1000-MB volume is accepted. Ask specifically
   whether `minReplicaCpu`/`minReplicaMemoryBytes` constrain configurable ceilings.
   No support message has been sent. If a mutating probe is the only way to verify
   acceptance, it needs separate approval; it is not covered by this review.
2. Approve a revised three-running-service/zero-worker graph, supported limits,
   authenticated traffic allowance, monitoring triggers and total cost envelope.
   If the supported minimum ceilings prevent a sufficiently bounded $1 run, retain
   NO-GO or separately review a shorter session; do not silently raise the budget.
3. Approve and test independent watchdog credentials, trusted runner, costs and
   exact stop/cancel/scale authority; confirm primary operator availability.
4. Only then consider separate GHCR pull/runtime credential setup, synthetic-only
   provisioning, frozen-source builds, deployment and final storage cleanup.

No resources, credentials, DNS, limits or runtime state changed in this feasibility
review. No paid inference or primary-database operation occurred. Only read-only
queries, source/documentation inspection and local cost arithmetic were needed;
the prior functional test results are unchanged.

## Final readiness implementation — shutdown preparation, not activation

The frozen application revision is still
`bfc29f500af08e0e2cc4676b8950b5b17f26007a`. Subsequent changes are infrastructure
preparation only; frontend, Rust application code, database and historical ledgers
were not modified. There is still no authoritative support clarification accepting
the small caps. Rechecking current official pricing/scaling documentation does not
resolve their minimums; the 4-vCPU/4-GB metadata must not be guessed into a conclusion.
The owner confirmed that no support reply has arrived; limits remain unverified.

The local IaC now actually specifies one frontend, one API, one PostgreSQL and a
dormant worker definition with **zero replicas**, both globally and in its explicit
Amsterdam region. Deployment remains unauthorized and replica-cap verification
remains false. This supersedes the earlier one-worker graph and its warning above.
The source-revision authoring field is a review constraint, not proof that a future
Railway checkout selects it: the exact deployed source must still be verified.
All 13 focused graph/security tests and the TypeScript check passed.

The configured PostgreSQL image remains exactly:

```text
ghcr.io/pyco404/supa-agent-postgres-infra-98c1006fcb1d@sha256:9409009a40c0929052306d068c14bef92f3ecc67662c453540e2c9b95ae18fc8
```

Its retained independent-verification evidence hash still matches
`config/railway-staging-image.json`. This check did not retrieve registry credentials,
pull/rebuild/publish an image or claim a fresh vulnerability scan.

### Prepared watchdog and offline evidence

`scripts/railway-shutdown.py` now implements the deadline controller; its 22 synthetic
tests are in `scripts/test-railway-shutdown.py`. Run the regression suite without
credentials, API requests, scheduling or real time delays:

```sh
python3 scripts/test-railway-shutdown.py
```

The eventual manifest must contain only the strict fields accepted by `load_scope`:
version 1, exact project/environment, frozen application SHA, exact PostgreSQL image,
three service UUIDs keyed `frontend`/`api`/`postgres` and optionally a dormant `worker`,
first-billable Unix timestamp, shutdown timestamp exactly T0+19800 seconds and
deadline exactly T0+21600 seconds; worker replicas zero; synthetic-only,
independent-runner and disabled-autodeploy attestations. No service IDs or launch
time exist yet. Do not invent real inventory or set attestations true without an
operator verifying them. Hashing binds the exact manifest bytes for separate review.

Network-free validation, only once that reviewed inventory is available:

```sh
python3 scripts/railway-shutdown.py validate --scope /PRIVATE/PATH/rehearsal-scope.json
```

The protected future action, **not authorized or executed here**, is:

```sh
python3 scripts/railway-shutdown.py run \
  --scope /PRIVATE/PATH/rehearsal-scope.json \
  --scope-sha256 REVIEWED_SHA256 --authorize-shutdown \
  --audit-file /PRIVATE/PATH/shutdown-audit.jsonl \
  --failure-alert-file /PRIVATE/PATH/shutdown-alerts.jsonl
```

It reads only an explicitly installed `SUPA_SHUTDOWN_PROJECT_TOKEN`, never local
Railway CLI, GHCR or model credentials. The token must be scoped to this project's
staging environment. It is broader than stop-only; approval is still required.
The live transport is not operationally tested with that token; no such token was
created, retrieved or installed. No watchdog is armed or independently hosted yet.
The prepared environment/deployment read queries succeeded against the empty staging
environment using existing CLI authentication after a transient DNS failure. They
returned zero service instances and deployments. Project-token authentication was
skipped in that read-only query and remains covered by synthetic tests only.

The controller verifies token target and exact service inventory, paginates
deployments with bounded reads, rejects unknown states and unexpected targets,
cancels pending replacements, stops frontend/API before PostgreSQL and requires
`deploymentStopped=true` in terminal states. A stop acknowledgment alone is not
success. It watches through the original deadline and requires repeated stopped
snapshots. Restart never extends the absolute window; monotonic elapsed time
prevents a backward wall-clock change from extending it. Cleanup authority expires
30 minutes after the six-hour deadline; no scope is silently renewed.

Audit records use owner-only permissions, exclusive locking and fsync before
mutations; sensitive response bodies and credentials are not logged. API errors,
ambiguous stop outcomes, a running worker, inventory drift or audit failure produce
a nonzero exit requiring operator action. Mutations are not automatically retried.
Only stop and cancel mutations are implemented: no deletion, scaling, deployment,
database access or financial operations. No success claim covers retained storage.
Tests cover scope/target denial, pagination, private logging, clock jumps, restart,
replacement deployments, timeout/no retry, unconfirmed instance stop and audit faults.

**Operational limitation:** on API/network failure the watchdog cannot guarantee
remote shutdown; the independent runner must immediately alert the primary operator
on nonzero exit. Approve and test runner availability, alert delivery, credential
scope, early shutdown and read-back using separately approved disposable resources
before treating it as reliable coverage. It must be armed before billable startup,
on a host independent of Ubuntu and these Railway services, with approved running
costs. No notification, schedule, runner or credential installation occurred here.

### GHCR installation preparation and refreshed cost decision

After separate credential approval, the owner should:

1. Prepare an expiry-bound classic PAT with `read:packages` only, for an identity
   already allowed to read the private package. Classic PAT scopes are not
   per-package restrictions. Do not reuse the broader interactive CLI credential.
2. Enter it privately in the PostgreSQL service's Source → Registry Credentials
   → GitHub Access Token. Railway handles the GHCR username. Keep it out of ordinary
   variables, prompts, logs, Git and images. No token value belongs in this document.
3. Select the exact approved digest, verify an authenticated pull and record
   ownership/expiry/revocation. Do not install this token in API, frontend or worker.

[Railway private registries](https://docs.railway.com/builds/private-registries)
require Pro, already confirmed; no plan upgrade is proposed. These instructions
do not authorize credential creation/installation or an image pull/deployment.

Refreshed usage: **$18.471422088903395**, forecast **$33.57639385405126** and
forecast headroom **$6.42360614594874** below the unchanged $40 hard limit. For
three running replicas and zero worker, prior rate arithmetic still gives the
conditional small-cap total **$0.371215** (six-hour compute plus 1 GB used volume and
1 GB egress). The confirmed default-ceiling scenario is **$18.039730** with those
same storage/egress assumptions. Neither scenario is an enforced spending cap.
No supported $1 project/session hard limit was established, nor a zero-cost
independent runner. The final decision remains **NO-GO** until supported small caps,
stable startup, bounded traffic/costs and independently tested shutdown coverage
are established. No provisioning, credentials, DNS, paid inference, primary
database changes, protected accounting changes, push or CI dispatch occurred.
