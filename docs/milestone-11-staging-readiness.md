# Milestone 11 — actual staging validation and approval boundary

2026-11-02. See the corrected-run results at the end of this report for the latest GitHub evidence. **Not cleared for provisioning or deployment.** Actual remote image builds were exercised; the validation run failed. Local corrections are implemented, tested and committed individually, but need separate GitHub publication approval before they can run remotely. No Railway service, volume, domain, credential or deployment was created.

## Verified target and unchanged infrastructure

After the operator refreshed Railway login, official CLI **5.71.0** successfully queried the exact project and workspace. The earlier direct GraphQL probe returned HTTP 403 and was not treated as successful authentication. CLI metadata is the successful verification evidence.

| Resource | Verified identity / state |
| --- | --- |
| Workspace | Daniel 's Projects — `e319f68a-9897-49ed-b119-743f1f846aa0` |
| Existing project | Supa Agent — `99648d2a-69a1-4d8c-98d0-7f2a26543c9c` |
| Accessible sole environment | staging — `ff0b2092-e42b-4a89-ae25-441ab95d92d8` |
| Existing bucket | supa-agent-backups — `76923b39-b3a7-4390-8a0b-783d8a8113e9`, Amsterdam `ams` |
| Bucket contents | 0 objects, 0 bytes |
| Services / staging volumes | 0 / 0, reconfirmed after preparation |

The bucket remains private under Railway's private-bucket model. No S3 credentials, anonymous object-access test or upload was requested. Project privacy was established by earlier provisioning evidence; the current CLI status does not return `isPublic`, so this milestone does not claim a fresh API verification of that flag.

Only a private local CLI context was linked under ignored `artifacts/milestone-11/railway-context`; the repository itself was not linked. No unrelated project was modified. Account-specific usage evidence remains private rather than entering this public report.

## Actual remote container and PostgreSQL results

Dispatched the existing manual-only, read-only-permission workflow on the verified public repository HEAD:

- [Isolated staging validation, run 41508493968](https://github.com/pyco404/SupaAgent/actions/runs/41508493968)
- Exact source: `907f0331f19c231e7dcac28187416eea2ad7e809`.
- Standard GitHub-hosted Ubuntu 24.04 runners; no Railway access, production secrets, registry publication, cache upload or artifact upload.
- All four jobs completed with **failure**. No failed job is presented as container readiness.

| Job | Actual progress | Failure / missing evidence |
| --- | --- | --- |
| Frontend image | BuildKit image build/load, image user/layer checks and Trivy image invocation completed | Initial `/healthz` did not reach 200; old harness retained no runtime log or scan findings |
| Rust API image | Image built/loaded, image checks and application/dependency scans invoked; fresh Railway PostgreSQL image started | First SQL probe failed before API startup; no PostgreSQL image scan or end-to-end runtime validation passed |
| Worker image | Image built/loaded and application image scan invoked; fresh PostgreSQL image started | Same SQL-probe failure, before disabled-worker execution checks |
| PostgreSQL 18 | Pinned packages downloaded/extracted; Rust API/runtime compiled successfully with Rust 1.93.0 | Extracted `postgres --version` exited 127; old handler discarded loader stderr; no schema-13 PostgreSQL 18 suite or restoration passed |

Reaching runtime probes establishes actual image builds, not just Dockerfile review. However, the old harness emitted manifest digests and vulnerability summaries only at its successful end. They are **not available from this failed run**; no digest or clean vulnerability result is invented. Trivy exit zero alone is not a clean scan because the application enforces severity policy afterward. Neither health/restart/shutdown validation nor full container security clearance is established.

### Implemented CI corrections

- PostgreSQL probes inherit a generated `PGPASSWORD` from the mode-0600 test environment file; passwords never enter command arguments. Probes use TCP, avoiding the socket-only bootstrap server's early readiness response.
- Image migration checks compare the complete current source inventory and SHA-384 checksums, replacing the stale eleven-migration count. PostgreSQL compatibility validation also derives all current versions, presently **1–13**, rather than its historical range.
- Workflow installs only common PostgreSQL **runtime libraries** on the disposable runner, then checks unresolved libraries with `ldd` before compiling. No PostgreSQL server package/system cluster is installed. Whether this resolves the original exit 127 needs the next real CI run; the missing library was not retained in the first run.
- Loader failure now retains bounded library names and exit stage, without arbitrary stderr or local paths.
- Build digest and scan checkpoints are emitted before runtime checks. Runtime health and Docker failures retain bounded diagnostics with generated credentials, credential URLs and recognizable key formats redacted. Image scan advisory IDs/counts survive later runtime failure. No container environment dump or secret finding match text is printed.
- Scan severity gates remain unchanged: any HIGH/CRITICAL or secret finding blocks success, including unfixed advisories. No suppression/ignore exception was added.

A disposable native production-dependency-filtered frontend check reached `/healthz` **200**. It removed 336 development packages only from its temporary hardlinked runtime tree; original dependencies were not pruned. This is **not a container test** and did not reproduce the hosted frontend failure. That failure remains unresolved until the corrected CI diagnostics run.

## Database safety and restricted-runtime rehearsal

A new test helper, `scripts/test-staging-runtime-profile.py`, accepts no primary URL. It verifies the existing socket-only local launch fixture's identity, reads **schema/ACLs and SQLx metadata only**, then starts its own new private socket-only PostgreSQL 16 cluster. It imports no source agents, financial records, memories or authorizations.

Measured results:

- Restored thirteen migrations' schema and checked every installed SHA-384 checksum against repository SQL.
- Temporary runtime role had no superuser, role creation, database creation, bypass-RLS or schema-creation permission. Updates to migration metadata and access to trusted approval tables were denied.
- Fresh private custom-format backup restored into another new database. Public-table fingerprints, constraints, indexes, function definitions, triggers and table privileges matched source/restore.
- **Restricted runtime startup failed closed.** The current API unconditionally invokes SQLx's `CREATE TABLE IF NOT EXISTS _sqlx_migrations`, which requires CREATE on the schema even when migration metadata already exists. The server log identifies that statement and permission error.
- This is a reproduced deployment blocker, not a successful runtime/authentication rehearsal. No extra schema/financial privilege was granted to make the test pass.
- Temporary roles/databases were removed and the new clusters stopped. Only these newly generated physical test-cluster directories were cleaned; reports, logs and synthetic dumps remain private. Existing clusters, backups, historical evidence and build outputs were preserved.

**Required correction before deployment:** separate approved migration administration from runtime startup with a fail-closed verification of already-applied migration versions/checksums. Test that path with restricted runtime credentials before replacing the graph's current helper/admin database URL. The existing graph still references the PostgreSQL helper credential and must not be applied as a least-privilege production configuration.

Earlier PostgreSQL 18 evidence covered an older migration set; it cannot certify all current thirteen migrations or the current Railway image. No primary database was started or migrated. No downgrade/rollback claim is made for schema changes that have not been deployed.

## Prepared service configuration

The existing SDK graph now uses the requested names and Amsterdam service region, preserves private networking, references secrets rather than values, and explicitly disables both providers and controlled paid work.

| Service | Prepared configuration |
| --- | --- |
| `supa-web` | Next.js image, non-root `node`, port 3000, `/healthz`, whole-site visitor gate plus distinct operator authentication |
| `supa-api` | Rust API, UID 10001, port 8080, private only, `/ready`, offline provider, API scheduling and wallet disabled |
| `supa-worker` | Rust runtime, UID 10001, port 8081, private `/ready`, `SUPA_WORKER_ENABLED=false`, no paid provider |
| `supa-postgres` | New empty PostgreSQL 18 service, private only, reviewed data mount `/var/lib/postgresql/data`; proposed initial capacity 5 GB, to confirm in the actual resource plan |

Services target `europe-west4-drams3a`; bucket region remains `ams`. These are different provider identifiers for Amsterdam. [Railway region identifiers](https://docs.railway.com/deployments/regions).

The graph label now matches **Supa Agent**. A label is not a remote ID binding: use the exact existing project/environment IDs above and inspect an explicit additive plan. Preserve the existing bucket; reject project/environment creation, unexpected resources and all deletes. The new offline `targetPreflight` checks the captured target/resource baseline, but supplied JSON is not provider authentication or deployment authorization.

All domains and public database/API/worker proxies remain absent. No application hostname has been selected or enabled. New identities must be inactive; finite offline execution requires separate per-agent consent and an explicitly approved execution topology. The disabled default worker does not demonstrate a running scheduler.

### Required staging secrets and configuration

No real staging credentials were generated or installed.

| Input | Scope / requirement |
| --- | --- |
| PostgreSQL bootstrap/admin credential | Separately approved migration/bootstrap operation only; no browser or routine runtime exposure |
| Runtime `DATABASE_URL` | New staging database and restricted role, private host, TLS requirements reviewed; blocked by the automatic-migration dependency above |
| `SUPA_API_TOKEN` | New service credential, distinct from every operator token |
| `SUPA_OPERATOR_CREDENTIALS` | Explicit principal UUID/role/token mappings for approved staging identities; no legacy shared-owner identity |
| `SUPA_OPERATOR_SESSION_DEADLINES` | Each configured principal bound to a fresh deadline; preflight requires no more than 24 hours |
| `SUPA_WEB_OPERATORS` | Server-only Basic-account→Rust-token mapping, separate from visitor credentials; no frontend-supplied identity authority |
| `SUPA_STAGING_USERNAME`, `SUPA_STAGING_PASSWORD` | Separate visitor gate; strong unique password, not an operator credential |
| `SUPA_SITE_ORIGIN` | Exact approved HTTPS origin, assigned before enabling a domain |
| `SUPA_BACKEND_URL` | `http://supa-api.railway.internal:8080`, server-side only |
| Disabled feature flags | Offline provider; OpenAI/Anthropic, controlled inference/work, API scheduler, wallet and standalone worker false |

No OpenAI, Anthropic, wallet, X or backup private key belongs in this initial profile. Do not copy dotenv files, local evaluation credentials, simulated treasury rows, historical budgets, owner proposals or temporary runtime files to Railway.

**MVP parity gap:** the local launch wizard intentionally rejects staging and Next.js does not expose its creation POST. A reviewed staging-scoped creation flow with durable idempotency journals is still required; the local loopback/database/epoch fences must not be removed to achieve parity. This report does not claim that deploying current containers alone provides the complete cloud launch journey.

## Billing and backup readiness

Read-only CLI access verified the correct workspace's current usage and its existing soft/hard compute limits. Exact account figures are retained privately and reported to the operator in chat. Limits were not changed. Available credits and current subscription details were not returned by these queries; neither is assumed from historical PRO evidence. A workspace-wide hard limit can stop unrelated workloads, so changing it needs separate approval. [Railway cost controls](https://docs.railway.com/pricing/cost-control).

Current container usage rates: RAM **$10/GB-month**, CPU **$20/vCPU-month**, volume **$0.15/GB-month**, service egress **$0.05/GB**. An illustrative aggregate average of 0.75–2 GB RAM, 0.05–0.5 vCPU, 5 GB volume and 0–5 GB egress yields roughly **$10–32/month additional staging usage**, before taxes and account-specific included usage. This is an assumption-based envelope, not measured usage or a spending guarantee. Current documentation lists PRO at $20/month including $20 of usage; do not double-count plan fees and included usage. [Railway pricing](https://docs.railway.com/pricing/plans).

The existing **$1/month bucket-only monitoring policy** and 23-snapshot retention target remain separate from compute costs. Bucket storage is $0.015/GB-month; no snapshot exists. [Bucket billing](https://docs.railway.com/storage-buckets/billing). Current shared workspace headroom must be reviewed against the proposed incremental use before any provisioning.

**Backup is not operational.** Independent encryption/signing fingerprints, recovery copies, bucket credentials, approved export/upload and an independent download/decrypt/disposable-restore drill remain incomplete. Do not introduce sensitive or irreplaceable persistent staging records before those requirements are satisfied. Do not enable automatic bucket/WAL archiving with unapproved credentials or describe same-disk synthetic restore tests as cloud disaster recovery.

## Tests, evidence and preservation

The pre-commit scanner initially rejected a literal synthetic credential URL in the diagnostic-redaction test. The fixture now constructs the same test URL at runtime; the gate was not bypassed and no real credential was involved.

Passed locally: ten CI/security regression tests; ten Railway graph/target/credential checks; Python syntax checks; secret-scanner self-tests; production npm advisory check with **zero findings**. New tests cover current migration inventory/checksum changes, gaps/duplicates, diagnostic redaction, expired/overlong principal deadlines and wrong workspace/environment/resource/bucket baselines.

The disposable PostgreSQL 16 privilege/restore test passes its verification assertions while explicitly recording `restricted_runtime_start_and_auth=false` and `runtime_blocker=sqlx_auto_migrate_requires_schema_create`. It is not a passing full-stack staging rehearsal. No full Rust suite was rerun locally with approximately 600 MiB free. The remote Rust services compiled, but PostgreSQL 18 suite execution failed before migration validation. No Docker was installed locally.

No website/Rust source was changed; approved local browser evidence remains applicable to the local MVP. Cloud REA acceptance is not performed because deployment is not authorized. No claim about a functioning cloud UI is made.

Private, ignored evidence is under `artifacts/milestone-11/`: final CI job/run logs, exact Railway inventory/bucket/usage/limits, target preflight, native frontend result, synthetic profile dump and schema/privilege results. Nothing there is committed or uploaded. Tests preserved twenty-nine protected/original file hashes and the fresh read-only before/after fingerprint of thirty-eight retained evaluation tables. Historical review remains pending; the $0.004048 hold and $0.003536 reservation were unchanged. Primary stayed stopped. No inference, transfers, publishing, trading, launches or production agents were activated.

## Ordered approval and rollback checklist

1. **GitHub only:** review and separately approve pushing these corrective one-file commits, then one existing manual workflow run at the exact new published SHA:
   `gh workflow run staging-validation.yml --repo pyco404/SupaAgent --ref main`.
   First verify remote main equals that SHA. Standard runners for this public repository are free under [GitHub's documented policy](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). No larger runner, secret access, registry push or Railway deployment is included.
2. Resolve the captured frontend/runtime findings; obtain actual image manifest/config/PostgreSQL digests, complete vulnerability findings, health/auth/network/restart/disabled-worker/SIGTERM evidence and a passing thirteen-migration PostgreSQL 18 restore suite. Fix the runtime/migration privilege separation and cloud launch parity, with disposable tests, before declaring the MVP deployable.
3. Complete the approved encrypted backup/recovery-key arrangements. Review exact staging principals, credentials, origin, database role grants and migration bootstrap separately; no local real-owner grant or primary cutover is implied.
4. Present an exact existing-project additive resource plan: the four services, private database volume/capacity, region, image/source pin, resource limits, monthly incremental budget and shared-workspace headroom. Reject any existing-bucket change or delete. **Request fresh explicit provisioning/deployment approval.** Existing project/bucket creation approval is insufficient.
5. After that approval, provision only the fresh database/volume; apply reviewed migrations with the separate administrator; verify restricted runtime credentials; deploy API, disabled worker, then gated frontend. Add an authenticated HTTPS endpoint only under an explicit domain approval. Keep all agents inactive.
6. After a separate finite-offline-session approval, use REA to test ten routes, logo/animation, owner/operator/viewer boundaries, creation, finite cycles, memory/activity, pause/resume/stop, restart persistence and zero paid/financial actions. Perform approved isolated restoration before treating data as recoverable.

Rollback: before first deployment, discard only the unexecuted local plan. Once approved resources exist, stop application services/disable domains and revoke new runtime credentials while **preserving PostgreSQL volumes and the existing bucket**. Roll application images back only to a known compatible schema version; do not reverse migrations by restoring over live state. For database recovery, restore a verified backup into a separate new database/volume, compare invariants, then request explicit cutback approval. Deleting persistent volumes or the bucket is not an authorized rollback action. Pausing compute does not eliminate storage charges.

All changes use separate Conventional Commits on `main`; no squash, rebase, force-push or history rewrite. Complete history/secrets gates run after the final commit. No new push is authorized or performed by this milestone. **Deployment remains blocked; the immediate approval needed is GitHub synchronization of the corrective validation code, not Railway provisioning.**


## Approved synchronization and single corrected CI run

The operator authorized exactly ten existing commits. Fetched `origin/main`, verified the clean `main` tree at `54d1bdcb54654238ac26d31e413684e6d635a820`, and confirmed the remote was an ancestor exactly ten commits behind. All 1,468 main commits passed the one-file audit. The full local pattern/known-secret scan passed without findings, including unpublished blobs. This is a targeted scanner, not proof against every unknown secret format.

Pushed only those commits, without tags or history rewriting. Verified remote HEAD matched the approved SHA and the tree was clean before subsequent local corrections. No generated artifacts, credentials or database files were added.

Dispatched exactly one [corrected validation run, 41513534137](https://github.com/pyco404/SupaAgent/actions/runs/41513534137), at the approved SHA. No reruns, additional dispatches, image publication or Railway operations occurred. All four jobs finished with failure.

| Job | Actual result | HIGH / CRITICAL image findings | Secret findings |
| --- | --- | --- | --- |
| Frontend | Image built/loaded; exclusion-canary checks passed; Next.js 16.4.3 reported ready in 129 ms. Published-loopback `/healthz` never returned HTTP; configured non-root user checked. Full runtime UID/auth/restart/SIGTERM checks were not reached. | 38 / 1 | 0 |
| API | Image built/loaded/scanned; configured non-root user and canary checks passed. Disposable Railway PostgreSQL container exited before the TCP SQL version probe. API application startup/migrations/auth/recovery were not reached. | 32 / 1 | 0 |
| Worker | Image built/loaded/scanned; configured non-root user and canary checks passed. Same PostgreSQL-container exit. Worker application health/default-disabled/no-cycle/restart/shutdown assertions were not reached. | 32 / 1 | 0 |
| PostgreSQL 18 | Pinned packages downloaded and checksums verified. Early loader check identified missing `liburing.so.2`. Compilation, thirteen-migration compatibility, accounting/concurrency/crash/restore tests were skipped. | Not scanned by this job | Not applicable |

The frontend started as a real container, but reported readiness does not establish reachable HTTP health. Its network/binding failure remains undiagnosed. For API/worker, the log proves the database container stopped, not why it stopped; startup logs were not retained in this run. Their application runtimes were not successfully demonstrated. The workflow cleanup removed owned disposable containers/volumes. No PostgreSQL image digest was emitted before the failing probe; none is claimed.

### Actual BuildKit image digests

These are digests from successful local loads on ephemeral CI runners, not published registry images:

- frontend: `sha256:902d0b4b1e53af60d885768416d575852812766ff61137aee8a2ca1ba5fbef8d`
- api: `sha256:60eb0afbb452a5f60967393e54c695211dbde2d71df8ad17e7f0be48088140bb`
- worker: `sha256:d7be50fe504099baa19830fc8a551566a1fa954387433c6e374950e2fac8c05c`

### Vulnerability evidence and limitations

Counts are package/advisory occurrences from the actual application-image scans, not unique CVE counts or exploitability conclusions. All three images report CRITICAL `CVE-2023-45853` in `zlib1g`, with no fixed version listed. The strict HIGH/CRITICAL gate remains unchanged; no advisories were suppressed. Runtime failures occurred before the final combined policy/reporting step, so these image scan counts do not certify dependency/IaC/PostgreSQL scans as clean. The Rust executable scans reported no language-specific metadata; separate Cargo manifest analysis began in the API job but its full findings were not emitted before the runtime failure.

Frontend additionally reports fix-available HIGH findings for `brace-expansion` 5.0.9 and `undici` 6.28.0. Their exact layer/dependency provenance needs inspection before choosing a remediation; an earlier local npm audit with zero findings is not evidence against these container scan results. The 33 shared base-image occurrences list no fixed versions.

Unique HIGH/CRITICAL advisory IDs reported per image:

- frontend: `CVE-2023-45853`, `CVE-2025-69720`, `CVE-2026-102276`, `CVE-2026-102278`, `CVE-2026-109383`, `CVE-2026-109869`, `CVE-2026-16742`, `CVE-2026-19534`, `CVE-2026-41992`, `CVE-2026-53613`, `CVE-2026-53615`, `CVE-2026-54369`, `CVE-2026-78410`, `CVE-2026-87018`, `CVE-2026-9538`.
- api: `CVE-2023-45853`, `CVE-2025-69720`, `CVE-2026-16742`, `CVE-2026-41992`, `CVE-2026-53613`, `CVE-2026-53615`, `CVE-2026-54369`, `CVE-2026-78410`, `CVE-2026-9538`.
- worker: `CVE-2023-45853`, `CVE-2025-69720`, `CVE-2026-16742`, `CVE-2026-41992`, `CVE-2026-53613`, `CVE-2026-53615`, `CVE-2026-54369`, `CVE-2026-78410`, `CVE-2026-9538`.

### Local follow-up, not pushed or remotely executed

- `1cc6437`: install the missing `liburing2` runtime package on the disposable Ubuntu runner. This addresses the identified loader dependency but is not yet a successful PostgreSQL 18 test.
- `6dfb21b`: fail early when the owned database container exits, retain bounded credential-scrubbed startup logs, and emit the pulled PostgreSQL digest before startup probes.
- `f934e67`: mocked regression coverage for running/TCP readiness, early exit, no SQL probe after exit and diagnostic redaction.

Eleven offline CI/security tests and fifteen secret-scanner self-tests pass. No application source, frontend design, financial policy or primary database was changed. Native/container builds were not repeated locally with approximately 600 MiB free. Raw run/job logs and parsed image checkpoints remain private and ignored under `artifacts/milestone-11-corrected/`; they were not committed or uploaded.

Remaining blockers: unreachable frontend health; unexplained PostgreSQL container startup exit; incomplete PostgreSQL 18 migration/recovery certification; HIGH/CRITICAL image findings; previously documented restricted-runtime automatic-migration privilege dependency, staging launch-flow parity, and pending independent backup key/recovery arrangements. No deployment-readiness claim is made. The primary stayed stopped; no database/accounting operations were performed, preserving both historical holds by non-intervention. Worker execution and paid providers remain disabled in the prepared configuration, with no new runtime-default claim from the failed container tests.

A fresh approval is required to push the local follow-up commits and run another validation. Railway provisioning/deployment remains a separate approval boundary.
