# Staging CI remediation — awaiting authorized remote execution

2026-11-02. The four existing unpublished commits are preserved. No push, workflow dispatch, Railway operation, protected database access or provider request was performed during remediation.

## Root causes, corrections and evidence limits

- **Frontend:** previous container logs show Next.js ready on `[::]:3000`, but the IPv4 loopback publish/probe received no HTTP response. This does not distinguish an IPv4 binding issue from Docker/runner networking. The production command now explicitly binds `0.0.0.0`; probes explicitly bypass environment proxies and a failed probe captures a container-internal HTTP check. The existing production build, isolated from private dotenv files and run natively with synthetic staging authentication, returned `/healthz` 200 (`status=ok`, `mode=private-staging`, signing disabled). This is native production evidence, not a corrected-container execution claim. Next.js terminated with conventional SIGTERM status 143; CI now accepts frontend 0/143, rejects SIGKILL 137 and crashes, and still requires Rust processes to exit 0.
- **PostgreSQL native package:** the actual CI loader error names `liburing.so.2`. The existing `liburing2` package fix remains in the workflow, followed by an `ldd` missing-library gate before compilation. Local Ubuntu has `liburing2 2.5-1build1` and the library resolves; the pinned PostgreSQL 18 binary has not been re-executed in corrected CI.
- **PostgreSQL container:** current public upstream PostgreSQL 18 declares `PGDATA=/var/lib/postgresql/18/docker`; Railway's wrapper refuses paths outside `/var/lib/postgresql/data`. Running only the wrapper's non-mutating path guard reproduces exit 1 for the default and exit 0 for `/var/lib/postgresql/data/pgdata`. CI now sets the latter and mounts a fresh owned anonymous volume at `/var/lib/postgresql/data`. This is a reproduced upstream configuration defect consistent with the container exit, not recovered startup logs from the failed image. Exact pulled-image confirmation remains pending.
- **API/worker:** ordinary private-staging startup now performs SELECT-only verification of all embedded SQLx migration versions, SHA-384 checksums and success flags. Missing, extra, failed or modified revisions fail closed. Runtime logins with superuser, CREATE ROLE/DB, BYPASSRLS, schema CREATE or migration-metadata write authority are rejected. Local non-staging behavior and protected epoch gates remain intact.
- **Separate migration administration:** new `supa-staging-schema` is a CI-only bootstrap, requiring explicit `SUPA_DISPOSABLE_SCHEMA=true`, one of two exact disposable database names, and no application epoch. It cannot target the protected primary database by name. CI invokes it separately with administrator credentials, then starts API/worker using a new non-superuser/non-DDL login. This is not a new primary migration or staging-cutover approval path.
- **Least privilege:** prepared CI grants SELECT on public tables, INSERT/UPDATE on worker-health rows and UPDATE on live sessions for interrupted-session recovery. It grants no agent balance or ledger writes, no migration DML, no trusted approval DML and no schema CREATE. Application authentication remains required. The infrastructure graph now requires separately provisioned API and worker database URL references instead of propagating the helper's administrator URL; no credentials were provisioned.

## Vulnerability investigation

The following table deduplicates the actual previous run's 105 image package/advisory occurrences into 39 shared rows. A row applies to all listed images; it is not counted three times here. Versions are the observed old images, not the proposed images.

| CVE | Package | Severity | Observed installed version | Scanner fixed versions | Images |
| --- | --- | --- | --- | --- | --- |
| CVE-2023-45853 | zlib1g | CRITICAL | 1:1.2.13.dfsg-1 | none reported | api, frontend, worker |
| CVE-2025-69720 | libtinfo6 | HIGH | 6.4-4 | none reported | api, frontend, worker |
| CVE-2025-69720 | ncurses-base | HIGH | 6.4-4 | none reported | api, frontend, worker |
| CVE-2025-69720 | ncurses-bin | HIGH | 6.4-4 | none reported | api, frontend, worker |
| CVE-2026-102276 | brace-expansion | HIGH | 5.0.9 | 5.0.10, 3.0.7, 2.1.5, 1.1.19 | frontend |
| CVE-2026-102278 | brace-expansion | HIGH | 5.0.9 | 5.0.11, 3.0.8, 2.1.6, 1.1.20 | frontend |
| CVE-2026-109383 | undici | HIGH | 6.28.0 | 6.29.1, 7.30.1, 8.12.2 | frontend |
| CVE-2026-109869 | undici | HIGH | 6.28.0 | 6.29.1, 7.30.1, 8.12.2 | frontend |
| CVE-2026-16742 | libsystemd0 | HIGH | 252.39-1~deb12u2 | none reported | api, frontend, worker |
| CVE-2026-16742 | libudev1 | HIGH | 252.39-1~deb12u2 | none reported | api, frontend, worker |
| CVE-2026-19534 | undici | HIGH | 6.28.0 | 6.28.1, 7.29.1, 8.10.2 | frontend |
| CVE-2026-41992 | gzip | HIGH | 1.12-1 | none reported | api, frontend, worker |
| CVE-2026-53613 | bsdutils | HIGH | 1:2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53613 | libblkid1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53613 | libmount1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53613 | libsmartcols1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53613 | libuuid1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53613 | mount | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53613 | util-linux | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53613 | util-linux-extra | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53615 | bsdutils | HIGH | 1:2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53615 | libblkid1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53615 | libmount1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53615 | libsmartcols1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53615 | libuuid1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53615 | mount | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53615 | util-linux | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-53615 | util-linux-extra | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-54369 | libacl1 | HIGH | 2.3.1-3 | none reported | api, frontend, worker |
| CVE-2026-78410 | bsdutils | HIGH | 1:2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-78410 | libblkid1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-78410 | libmount1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-78410 | libsmartcols1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-78410 | libuuid1 | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-78410 | mount | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-78410 | util-linux | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-78410 | util-linux-extra | HIGH | 2.38.1-5+deb12u3 | none reported | api, frontend, worker |
| CVE-2026-87018 | undici | HIGH | 6.28.0 | 6.29.1, 7.30.1, 8.12.2 | frontend |
| CVE-2026-9538 | perl-base | HIGH | 5.36.0-7+deb12u4 | none reported | api, frontend, worker |

The shared base findings concern Debian utilities/libraries (`util-linux` family, gzip, ACL, systemd/udev, ncurses, Perl and zlib). Installed presence alone does not establish an exploitable application path. Their scanner-recorded fixed version is absent; runtime reachability and vendor disposition need security review. The scan did not retain target paths for the frontend npm findings, so whether each comes from global package-manager tooling or application dependencies is not established.

The Debian security tracker specifically notes that the affected MiniZip code for `CVE-2023-45853` was not built in bookworm's zlib binary packages; trixie's source version is listed fixed. This is vendor context, not an approved exception, and the scanner gate is unchanged. See [Debian's advisory](https://security-tracker.debian.org/tracker/CVE-2023-45853).

Prepared remediation uses Debian 13 (`trixie-slim`) runtime bases, runs normal distribution security upgrades, keeps Node major 24, and removes unused global npm/Yarn from the frontend runtime while preserving application dependencies. Rust binaries remain built on bookworm and run on newer glibc; actual linking/startup must be confirmed in CI. Node's official [supported variants](https://github.com/nodejs/docker-node/blob/main/versions.json) include 24/trixie-slim. No global or per-CVE suppression was added. No claim is made that new image findings are zero before actual scans. New evidence includes target paths, installed/fixed versions and all dependency/PostgreSQL findings before runtime probes, without printing matched secret text.

## Validation and the single prepared run

Passed locally: 14 offline CI/security tests; 10 infrastructure graph tests; Python syntax checks; Rust formatting; offline locked Rust compile checks for API/worker/bootstrap binaries and the new `runtime_schema` integration test. The integration test is compiled, not executed: it is restricted to the disposable PostgreSQL 18 suite and tests non-DDL startup, metadata tampering, unauthorized writes and excessive-role rejection.

The prepared manual workflow builds the new schema helper; validates the pinned native PostgreSQL 18 runtime; bootstraps all 13 migrations with separate credentials; starts API/worker under restricted roles; tests forbidden schema/metadata/balance/approval writes; checks readiness, authentication, disabled worker/no cycles, restart/SIGTERM behavior, accounting precision/constraints/concurrency and backup restoration; and runs the existing ignored Rust database suite plus the new schema test. Image scans remain strict for HIGH/CRITICAL and secrets. All image digests/checkpoints are retained in sanitized CI logs; no registry push or artifact upload is added.

Docker/Podman/Buildah are unavailable locally. Disk space was approximately 598 MiB initially and approximately 187 MiB after compile-check metadata. No Docker was installed, no full Rust link/test build or PostgreSQL 18 cluster was attempted, and no protected build executable/database/backup/evidence was removed. No actual corrected-container, PostgreSQL 18 migration/recovery or fresh vulnerability scan result is claimed. The definition of done therefore remains **pending remote validation**, plus review/remediation of any residual vulnerabilities.

After separate approval to publish the final reviewed commits, verify remote HEAD, then dispatch exactly one run:

```sh
gh workflow run staging-validation.yml --repo pyco404/SupaAgent --ref main
```

Do not run this command under the current authorization. Preserve failed-run evidence and the previous digests as historical, not as new-image evidence. No Railway deployment is authorized even if the future run passes. Backup key enrollment and separately reviewed staging database/identity provisioning remain independent operational requirements.
