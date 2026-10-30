# Milestone 6A — staging blocker validation

Validated locally on 2026-10-30. **Not yet ready for private Railway deployment.**
PostgreSQL 18 engine compatibility now has real passing evidence. Application
image builds, image execution and container vulnerability scans still require the
prepared remote CI run. No Docker was installed locally, no workflow dispatched,
no push made, and no Railway resource, deployment or credential created.

## Actual results versus prepared validation

| Area | Actual result | Boundary |
| --- | --- | --- |
| Frontend/API/worker Dockerfiles | Existing definitions reviewed; 8 Railway/declaration tests pass | No OCI image built or executed locally; no app image digest exists for this milestone |
| PostgreSQL 18 engine | Real **18.3**, official PGDG Ubuntu packages extracted into `/tmp` | Not the Railway PostgreSQL container; no claim about its exact patch, SSL setup or persistent mount |
| SQLx migrations | All **1–10** applied to a new disposable database; all SHA-384 checksums match | No real database migration, owner grant or epoch activation |
| Rust + PostgreSQL 18 | **121 tests passed**, including opt-in database, authorization, lease, concurrency, accounting and recovery tests | Paid providers are mocked; no actual GPT request |
| Native migration administrator | **3 passed**, 1 specialized Stage-1/port-55444 test skipped | No production/disposable-clone override was enabled |
| Database crash and restart | API and paused worker readiness **200 → 503 → 200**, followed by SIGTERM exit **0** | Native processes, not container PID/signal handling |
| Backup and restore | Private custom-format dump restored into a different database; full source/restore fingerprints and functions/triggers/privileges match | No private database upload; primary untouched |
| Dependency audits | npm full and production-only audits: **zero findings** in frontend and IaC lockfiles; OSV queried **267** Rust lockfile packages | Version-advisory queries are not container scans |
| Rust advisory | `rsa 0.9.10`: `RUSTSEC-2023-0071`, MEDIUM; **not in the active Linux build graph** | Not silently ignored; no patched version is currently listed |
| Trivy local scan | **Failed**, insufficient disk for expanded advisory DB; failed temporary cache removed | No clean Trivy or container-scan result is claimed |
| CI workflow | actionlint **1.7.12** passed; **7 security regression tests passed** | Workflow has not run remotely |
| Browser/design | No frontend/logo/source appearance changes | REA was not used: backend/CI-only changes add no material browser validation beyond Milestone 6 |

No image manifest digest is invented. The future CI harness reports the real
BuildKit manifest digest separately from Docker's config image ID, plus the
pulled PostgreSQL repository digest, only after actual build/runtime checks.

## PostgreSQL 18 evidence and reproducibility

`scripts/validate-postgres18.py` accepts **binaries**, never an existing data
directory, database URL, prepared epoch or production database. It verifies the
binary major version before creating a fresh private temporary cluster. The
cluster listens on a private Unix socket only, uses peer authentication for the
current test user, rejects host connections, and is stopped in cleanup.

Local binaries were extracted without system installation from signed official
PGDG package metadata. The repository signature was verified against fingerprint
`B97B0AFCAA1A47F044F244A07FCC7D46ACCC4CF8`; the signed package-index SHA-256 and each
package SHA-256 were verified. Server, client and libpq packages were all
`18.3-1.pgdg24.04+1`. The exact checksums are pinned in the CI preparation step and
retained in local `pg18-packages.json`. This is the patch actually tested, not a
claim that 18.3 is the newest available PostgreSQL release.

The real test validates:

- All ten migrations and SQLx checksums, with no source SQL modifications.
- Exact bigint roundtrip **9,007,199,254,740,993**, negative-balance rejection,
  foreign-key enforcement, and two concurrent identical ledger writes yielding
  exactly **one** successful insert.
- Critical partial/lookup indexes; complete constraints/indexes in the restored
  schema fingerprint; functions, triggers and table privileges after restoration.
- Native API/worker startup, database crash recovery, unchanged zero execution
  count while worker/scheduling gates are false, and clean SIGTERM shutdown.
- Empty epoch, authority, executor and approval tables in the isolated compatibility
  database; no historical budget imported into it.
- All existing Rust database tests against a separate new suite database, covering
  lifecycle, authorization renewal, worker fencing, duplicate protection, financial
  reservations/precision, interrupted execution, wallet observations and quarantine.

Backup/source/restore fingerprint:

`584ab9dcc7dc786213dc8b068e34e81a969e6c6b45d36c285904610f8449a5e1`

Private dump SHA-256:

`142a388ee7a0a4bd4a9d06a532d717c20cd8a40e268a7ecea7ce1827c11d81d5`

The backup has mode **0600**. The source fingerprint before and after backup and
the restored fingerprint match. Separate schema comparison excludes only the
intentionally different database catalog name. No PostgreSQL 18 incompatibility
was observed in the tested operations. The exact Railway image, its SSL startup,
volume layout and deployed networking remain unverified until CI/staging tests.

Re-run with reviewed PostgreSQL 18 binaries and existing native test binaries:

```sh
cargo build --locked -p supa-api --bin supa-api -p supa-runtime --bin supa-runtime
python3 scripts/validate-postgres18.py --bin-dir /path/to/postgresql18/bin --library-dir /path/to/extracted/libs --evidence-dir /tmp/new-pg18-evidence --offline
```

Omit `--library-dir` for normally installed binaries. Omit `--offline` on a cold
CI dependency cache. The evidence directory must not already exist. Local paths
and package downloads are never copied into Railway or committed.

## Isolated CI implementation

`.github/workflows/staging-validation.yml` is **manual-only** (`workflow_dispatch`)
and restricted to official repository `pyco404/SupaAgent`, `refs/heads/main`,
standard GitHub-hosted Ubuntu 24.04 runners. The image harness additionally refuses
local, fork, pull-request, other-branch and self-hosted contexts before Docker or
network operations. That is a fail-safe context check, not a substitute for
GitHub's dispatch permissions.

Security properties:

- Only `contents: read`; no OIDC, package publishing, deployment or write permission.
- Checkout pinned to immutable commit `3d3c42e5aac5ba805825da76410c181273ba90b1`
  (verified official v7.0.1); checkout credentials are not persisted.
- No `secrets` references, environments, production credentials, Railway CLI,
  registry login, image push, artifact upload or remote cache upload.
- Trivy **0.76.1** is checksum-pinned, not installed from a floating action/script.
- Three independent application-image matrix jobs, at most two image jobs at once;
  a separate PostgreSQL 18 job uses freshly extracted checksum-pinned packages.
- Image jobs allow 70 minutes each; PostgreSQL allows 45: maximum configured total
  **255 runner-minutes**. Runs are serialized by concurrency group. No larger or
  self-hosted runner is configured. A minimum **12 GiB free** is required before
  building; insufficient capacity fails rather than pruning unrelated files.
- Build context copies only reviewed source/manifests/assets. Local dotenv,
  databases, archives, credentials, dependencies, Git history and evidence are
  excluded. Synthetic exclusion canaries test ignored context paths and image
  layers; they are not real credentials.
- Ephemeral test credentials are generated after image builds and passed through
  mode-0600 environment files. No host source or Docker socket is mounted into
  application containers. Their runtime network is internal, with health ports
  published on loopback only. Database volumes are new and disposable.

`scripts/ci-staging-images.py` will build using the existing Dockerfiles, load
without publishing, record actual digests, inspect and execute non-root users,
check health/authentication, scan image layers for context canaries, verify the
frontend visitor gate and API bearer gate, restart services/databases, and require
SIGTERM exit 0. Worker tests inherit the image's disabled default and require zero
cycles across restart. Frontend image validation uses a disconnected backend;
this does not replace Milestone 6's native full-stack test or future Railway
network validation.

Trivy scans all three application images. The API job also scans the selected
Railway PostgreSQL image and isolated Rust/frontend/IaC dependency manifests.
Scanner/download/parser failure fails closed. **All HIGH/CRITICAL vulnerabilities,
including unfixed ones, and any secret finding fail the job**. No ignore list or
advisory suppression is installed. Other severity counts and advisory IDs are
recorded for review. Job logs/summaries retain sanitized status/digest/advisory
information; raw environment files, dumps, image archives, source artifacts and
private database files are not uploaded.

The workflow itself is linted, and the helper's context gates, malformed-scan
rejection, vulnerability/secret rejection, exact source allowlist and cross-chunk
canary detection have unit tests. These tests do **not** prove Docker execution.
The actual remote build/runtime/scan path remains unexecuted.

[Docker documents build-and-load testing before image publication](https://docs.docker.com/build/ci/github-actions/test-before-push/).
[GitHub recommends read-only tokens and SHA-pinned actions](https://docs.github.com/en/actions/reference/security/secure-use).
Railway's [deployment lifecycle](https://docs.railway.com/deployments/reference)
transitions a successful build into deployment. Its build facility is therefore
not used as a no-deployment sandbox here. No `railway up`, project linking,
resource creation or build/deploy action was performed.

## Actual dependency findings and limitations

The local Trivy advisory database exceeded available disk during expansion. Only
this run's failed `/tmp/supa-m6a-dependencies-…/cache` was removed; historical
artifacts, databases and build caches were preserved. No scanner-success result
or image vulnerability result exists locally.

Lightweight npm audits succeeded with **zero** advisories for both `web` and
`.railway`, once with production dependencies and once including development
packages. OSV's official batch API was queried for all 267 registry Rust lockfile
packages. It returned only `rsa 0.9.10` / `RUSTSEC-2023-0071`.
`cargo tree --locked --offline --target x86_64-unknown-linux-gnu -i rsa` and the
same command for `sqlx-mysql` both report no active dependency. This is a lockfile
entry outside the current selected Linux feature graph; no unnecessary lockfile
or Rust architecture change was made. The advisory is MEDIUM, with no patched
version currently listed by [RustSec](https://rustsec.org/advisories/RUSTSEC-2023-0071.html).
It remains documented, and dependencies/features must be rechecked if MySQL/RSA
support is enabled. Advisory API queries cannot establish absence of runtime OS
vulnerabilities, unknown flaws or future advisories.

## Remote status, requirements and approval boundaries

Read-only GitHub checks verified the official repository is public, default branch
`main`, not archived; account repository access and Actions are enabled. There
are **zero existing remote workflows**. No settings were changed.

Remote main was `a9041fd94193929b1bd695338283ec75bfe160d5`. Before this milestone's
commits, local main was `3253b89c3d3b61ed0abf550d95102078fb839728`, **487 commits
ahead**. The local tip tracks zero artifacts, but its unpublished ancestry includes
**72 historical artifact additions**. A normal push would publish those objects
as history. This milestone does not authorize that publication or any rewrite,
rebase, squash, force-push or metadata replacement. An explicit operator decision
on this existing publication conflict is required before a source-only push can
be represented as safe. No artifact or private file is added by Milestone 6A.

Required next approvals/actions:

1. Resolve the unpublished-history/artifact publication decision without violating
   the established Git rules. Approve the precise source commit(s) to publish.
   **No push is authorized by this milestone.**
2. After reviewed publication on official `main`, separately approve one manual
   `Isolated staging validation` run at the verified main SHA. No secrets, registry
   publishing, Railway resources or deployment are needed. The command would be:

   ```sh
   gh workflow run staging-validation.yml --repo pyco404/SupaAgent --ref main
   ```

   This command has **not** been executed. Confirm remote main is still the
   approved SHA immediately beforehand and verify each run's actual head SHA.
3. Confirm standard Ubuntu runner eligibility/capacity and the 255-minute maximum.
   GitHub currently documents standard runners as free for public repositories;
   [larger runners have separate billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions).
   No larger runner, paid remote builder or increased storage allowance is
   authorized. If resource limits prevent completion, stop and report them.
4. Review actual image manifest/config/PostgreSQL digests, all startup/restart,
   permission/default-worker results, and complete vulnerability findings. Fix
   failures and rerun; a prepared workflow or successful build alone is insufficient.
5. Only then consider the separate Railway project/billing/secrets/plan/deploy
   approvals already listed in [Milestone 6](milestone-6.md#remaining-release-gates-and-exact-approval-sequence).

## Preservation, Git and readiness verdict

The approved frontend, official logo, all ten sections, Rust architecture,
Dockerfiles and Railway resource graph are unchanged. No browser regression run
was needed for this backend/CI-only milestone. All 13 protected primary/owner/
epoch/accounting fingerprints remain unchanged; the primary cluster stayed
stopped. Historical budget `ad91e475-f1da-495a-8247-139643d2d8b1`, **$0.10** ceiling
and unresolved **$0.004048** hold remain quarantined and unrecovered. The new
PostgreSQL 18 test cluster is stopped. Same-disk backups remain local and private.

Five source/report files are committed individually with descriptive Conventional
Commit messages directly on `main`. Full one-file history and secrets scans are
run before completion. Logs, reports, package downloads, scanner caches and dumps
remain untracked; no artifacts are committed or pushed. Local evidence is in
ignored `artifacts/milestone-6a/`, including `postgres18/postgres18.json`, Rust logs,
signed-package checksums, npm/OSV findings, actionlint and security-test results,
remote/preservation checks, and the unsuccessful Trivy scan log.

**Readiness:** PostgreSQL 18 engine compatibility is demonstrated for the tested
patch and operations. Container build/runtime/vulnerability evidence and the
publication/CI approvals remain outstanding. Supa Agent is **not yet cleared for
private Railway staging**. Runtime DB migration privileges and lower-severity
advisory review remain documented operational risks from the previous milestone.
Base-image tags are still mutable. A later Railway rebuild is not automatically
the same image as a CI-tested digest: pin reviewed bases or separately approve
publication/reuse of tested images, then verify the deployed digest. No registry
publication is authorized here.
Workers, live GPT, financial actions, trading, launches, Mainnet and X publishing
remain disabled; this report grants no execution or deployment permission.
