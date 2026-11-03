# Remaining CI failures — implemented completion fixes

2026-11-03. No GitHub push, workflow dispatch, Railway operation, protected database operation, paid inference or historical accounting change occurred. The prior run remains historical evidence: [41526693076](https://github.com/pyco404/SupaAgent/actions/runs/41526693076).

## Exact networking defect and correction

All application containers were attached exclusively to an `--internal` bridge but the harness attempted localhost port publishing. Docker requires a default-gateway network for published ports; internal-only containers have none. The [Docker maintainer explanation](https://github.com/moby/moby/discussions/53256) matches the run's internal HTTP 200 and missing external HTTP responses. Binding and endpoints were correct; proxies were already disabled. More retries cannot create a missing mapping.

The harness now retains the internal-only bridge and probes the inspected private container IPv4 address directly from the Linux runner. No published port, public listener exposure, second egress network, host-network application mode or authentication bypass is added. It rejects extra container networks and non-RFC1918 addresses. The reviewed logical Host/origin is retained for frontend authentication. The existing unauthenticated, visitor/operator separation and invalid-token checks use this external-to-container route. Internal readiness is checked independently by entering only the owned container's network namespace with the hosted runner's `nsenter` and Python; application privileges do not change. Restart refreshes the bridge address and repeats both checks. Transport failures retain only exception type and errno, with no headers or credential prose.

This is a corrected Linux-hosted validation configuration, not a claim that the new container execution has already passed. There is no local Docker installation and no new CI run is authorized yet.

## Reproduced PostgreSQL restore defect

Created two disposable databases in the verified running local launch fixture, applied all 13 migration SQL files to the new source, dumped/restored into the new target, and dropped only those two databases afterward. No source application rows, historical budgets, credentials or ownership records were copied. The protected primary stayed stopped and was not connected to.

The old comparison was unequal solely in `triggers`. Both sides contained the same 127 trigger definitions, but 106 array positions differed because `ORDER BY tgname` leaves repeated names tied across tables. At position 14, source listed `immutable` on `epoch_guard.operational_principals`, while restore listed `immutable` on `epoch_guard.operational_claims`. Both objects existed unchanged on both sides. Function definitions and privileges matched. Restoring an already sorted schema dump before this test concealed the defect; preserving migration creation order reproduces it.

The replacement uses stable object identities, including schema/table/trigger and fully qualified function signatures. It compares complete values and emits exact changed/missing/extra objects, preserving full definitions privately in `schema-comparison.json` and bounded identities in CI logs. It also adds checks for database/schema/relation ownership, ACLs and column permissions, trigger enablement, constraints, indexes, extension version/ownership/configuration and extension membership. No schema-integrity check was disabled.

Actual PostgreSQL **16** rehearsal result (not a new PostgreSQL 18 certification):

| Category | Source / restored object counts | Result |
| --- | ---: | --- |
| Database properties / ownership / ACLs | 1 / 1 | Equal |
| Schemas | 2 / 2 | Equal |
| Functions | 22 / 22 | Equal |
| Triggers | 127 / 127 | Equal |
| Relations / ownership / ACLs | 139 / 139 | Equal |
| Constraints | 246 / 246 | Equal |
| Indexes | 83 / 83 | Equal |
| Table privileges | 350 / 350 | Equal |
| Column privileges | 1365 / 1365 | Equal |
| Extensions | 1 / 1 | Equal |
| Extension members | 4 / 4 | Equal |

Negative rehearsal detected an actually disabled `public.agents.operational_write` trigger, revoked function EXECUTE, revoked schema USAGE and revoked database CONNECT in the disposable restore. Unit scenarios also reject changed owners, definitions, missing objects and extra objects in every category. Local ignored evidence: `artifacts/milestone-11-metadata/eb5e2717ca2b/rehearsal.json` and `schema-comparison.json`. No temporary test databases remain from the rehearsal.

## Compatible security changes and remaining blockers

`deployment/postgres.Dockerfile` builds a **candidate overlay**, retaining the exact previously scanned upstream image digest and PostgreSQL/SSL entrypoint. It rebuilds the pinned gosu 1.19 source commit with Go **1.27.3**, verifies the source archive SHA-256, uses a verified immutable official Go builder index and checks real privilege-dropping behavior during the build. [Official Go releases](https://go.dev/doc/devel/release) confirm that version. Go image metadata returned HTTP 200 and includes linux/amd64; no image was downloaded locally. Dependency sums are verified and builds are read-only. PostgreSQL server/client package versions are held during ordinary OS security upgrades. The unused pgvector package is removed only in this fresh-database candidate: Supa's migrations and Rust queries contain no vector dependency. Required PostgreSQL packages are asserted still installed.

The image is built and scanned inside the prepared API/worker jobs, not published. It retains XML functionality and verifies `xml_is_well_formed` after initialization. The image digest and actual gosu/toolchain/server versions enter CI evidence. These are implemented build changes, not already verified replacement-image results.

**libxml2 remains unresolved:** [Debian's CVE-2026-6653 record](https://security-tracker.debian.org/tracker/CVE-2026-6653) lists the installed stable package as vulnerable, with no stable fixed version. PostgreSQL links this library for XML support, so deleting it is not compatible. Replacing it with a different library ABI or blindly applying the large upstream parser/structure rewrite would require a separately validated library/PostgreSQL rebuild. No unreviewed backport, CVE exclusion, severity downgrade or scanner weakening is implemented. The critical remains deployment-blocking.

The original Go stdlib critical and 25 HIGH occurrences have fixed compiler versions; candidate Go 1.27.3 is above every listed threshold. Gosu is still needed for startup privilege dropping. Vendor guidance discusses unreachable Go code, but no exception is taken. The unused pgvector HIGH occurrence is targeted by removal. Remaining OS/GnuPG/Perl/sudo/library findings had no fixed versions in the actual scan; removing these packages without runtime-dependency proof is not claimed safe. The 44 shared application-image HIGH findings remain open. Every row below stays in the baseline until an actual rescan proves a change.

The existing Railway graph still points to the upstream helper image. Selecting/publishing a validated candidate for future Railway use is a separate, unperformed approval step. Passing candidate CI alone would not validate that different graph image or authorize deployment. Encrypted-backup/key enrollment and staging identity/database-role provisioning remain independently pending.

## Deduplicated vulnerability baseline and remediation status

Last actual scan: application images each 44 HIGH/0 CRITICAL; PostgreSQL 89 HIGH/2 CRITICAL. There are 91 unique package/advisory/version rows across all images. Fixed-version columns are the scanner's actual values, not predictions. No updated scan counts are claimed.

| CVE | Package | Severity | Installed baseline | Scanner fixed versions | Images | Remediation status |
| --- | --- | --- | --- | --- | --- | --- |
| CVE-2025-61726 | stdlib | HIGH | v1.24.6 | 1.24.12, 1.25.6 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2025-61729 | stdlib | HIGH | v1.24.6 | 1.24.11, 1.25.5 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2025-68121 | stdlib | CRITICAL | v1.24.6 | 1.24.13, 1.25.7, 1.26.0-rc.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2025-69720 | libncursesw6 | HIGH | 6.5+20250216-2 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2025-69720 | libtinfo6 | HIGH | 6.5+20250216-2 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2025-69720 | ncurses-base | HIGH | 6.5+20250216-2 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2025-69720 | ncurses-bin | HIGH | 6.5+20250216-2 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-103484 | postgresql-18-pgvector | HIGH | 0.8.7-1.pgdg13+1 | none reported | postgres | Unused in Supa; removed from candidate; build/rescan pending |
| CVE-2026-111943 | stdlib | HIGH | v1.24.6 | 1.26.10, 1.27.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-119373 | libsqlite3-0 | HIGH | 3.46.1-7+deb13u2 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-16742 | libsystemd0 | HIGH | 257.13-1~deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-16742 | libudev1 | HIGH | 257.13-1~deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-24882 | dirmngr | HIGH | 2.4.7-21+deb13u1+b5 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-24882 | gnupg | HIGH | 2.4.7-21+deb13u1 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-24882 | gnupg-l10n | HIGH | 2.4.7-21+deb13u1 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-24882 | gpg | HIGH | 2.4.7-21+deb13u1+b5 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-24882 | gpg-agent | HIGH | 2.4.7-21+deb13u1+b5 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-24882 | gpgconf | HIGH | 2.4.7-21+deb13u1+b5 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-24882 | gpgsm | HIGH | 2.4.7-21+deb13u1+b5 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-25679 | stdlib | HIGH | v1.24.6 | 1.25.8, 1.26.1 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-27145 | stdlib | HIGH | v1.24.6 | 1.25.11, 1.26.4 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-32280 | stdlib | HIGH | v1.24.6 | 1.25.9, 1.26.2 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-32281 | stdlib | HIGH | v1.24.6 | 1.25.9, 1.26.2 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-32283 | stdlib | HIGH | v1.24.6 | 1.25.9, 1.26.2 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-33811 | stdlib | HIGH | v1.24.6 | 1.25.10, 1.26.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-33814 | stdlib | HIGH | v1.24.6 | 1.25.10, 1.26.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-33818 | stdlib | HIGH | v1.24.6 | 1.25.13, 1.26.6, 1.27.0-rc.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-39820 | stdlib | HIGH | v1.24.6 | 1.25.10, 1.26.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-39821 | stdlib | HIGH | v1.24.6 | 1.25.13, 1.26.6, 1.27.0-rc.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-39822 | stdlib | HIGH | v1.24.6 | 1.25.12, 1.26.5, 1.27.0-rc.2 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-39836 | stdlib | HIGH | v1.24.6 | 1.25.10, 1.26.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-42499 | stdlib | HIGH | v1.24.6 | 1.25.10, 1.26.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-42504 | stdlib | HIGH | v1.24.6 | 1.25.11, 1.26.4 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-54369 | libacl1 | HIGH | 2.3.2-2+b1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-56853 | stdlib | HIGH | v1.24.6 | 1.25.13, 1.26.6, 1.27.0-rc.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-56858 | stdlib | HIGH | v1.24.6 | 1.25.13, 1.26.6, 1.27.0-rc.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-56859 | stdlib | HIGH | v1.24.6 | 1.25.13, 1.26.6, 1.27.0-rc.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-56860 | stdlib | HIGH | v1.24.6 | 1.25.13, 1.26.6, 1.27.0-rc.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-56862 | stdlib | HIGH | v1.24.6 | 1.25.13, 1.26.6, 1.27.0-rc.3 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-6653 | libxml2 | CRITICAL | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | none reported | postgres | Open: required XML library; no compatible stable fix verified |
| CVE-2026-74860 | libxml2 | HIGH | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | none reported | postgres | Open: required XML library; no compatible stable fix verified |
| CVE-2026-76642 | bsdutils | HIGH | 1:2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-76642 | libblkid1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-76642 | liblastlog2-2 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-76642 | libmount1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-76642 | libsmartcols1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-76642 | libuuid1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-76642 | login | HIGH | 1:4.16.0-2+really2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-76642 | mount | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-76642 | util-linux | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78408 | bsdutils | HIGH | 1:2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78408 | libblkid1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78408 | liblastlog2-2 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78408 | libmount1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78408 | libsmartcols1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78408 | libuuid1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78408 | login | HIGH | 1:4.16.0-2+really2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78408 | mount | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78408 | util-linux | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78409 | bsdutils | HIGH | 1:2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78409 | libblkid1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78409 | liblastlog2-2 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78409 | libmount1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78409 | libsmartcols1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78409 | libuuid1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78409 | login | HIGH | 1:4.16.0-2+really2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78409 | mount | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78409 | util-linux | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78410 | bsdutils | HIGH | 1:2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78410 | libblkid1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78410 | liblastlog2-2 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78410 | libmount1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78410 | libsmartcols1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78410 | libuuid1 | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78410 | login | HIGH | 1:4.16.0-2+really2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78410 | mount | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78410 | util-linux | HIGH | 2.41.5-0+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-78667 | stdlib | HIGH | v1.24.6 | 1.26.9, 1.27.2 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-78669 | stdlib | HIGH | v1.24.6 | 1.26.9, 1.27.2 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |
| CVE-2026-86138 | libxml2 | HIGH | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | none reported | postgres | Open: required XML library; no compatible stable fix verified |
| CVE-2026-86139 | libxml2 | HIGH | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | none reported | postgres | Open: required XML library; no compatible stable fix verified |
| CVE-2026-86140 | libxml2 | HIGH | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | none reported | postgres | Open: required XML library; no compatible stable fix verified |
| CVE-2026-86142 | libxml2 | HIGH | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | none reported | postgres | Open: required XML library; no compatible stable fix verified |
| CVE-2026-86143 | libxml2 | HIGH | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | none reported | postgres | Open: required XML library; no compatible stable fix verified |
| CVE-2026-86144 | libxml2 | HIGH | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | none reported | postgres | Open: required XML library; no compatible stable fix verified |
| CVE-2026-9538 | libperl5.40 | HIGH | 5.40.1-6+deb13u1 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-9538 | perl | HIGH | 5.40.1-6+deb13u1 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-9538 | perl-base | HIGH | 5.40.1-6+deb13u1 | none reported | api, frontend, postgres, worker | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-9538 | perl-modules-5.40 | HIGH | 5.40.1-6+deb13u1 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-96512 | sudo | HIGH | 1.9.16p2-3+deb13u2 | none reported | postgres | Open: stable/vendor fix or validated dependency removal needed |
| CVE-2026-97031 | stdlib | HIGH | v1.24.6 | 1.26.9, 1.27.2 | postgres | Candidate rebuilt with Go 1.27.3; build/rescan pending |

## Files, validation and approval boundary

Changed files:

- `scripts/ci-staging-images.py`: internal-bridge host probes, independent internal health, safe transport diagnostics, candidate image build/scan/digests.
- `scripts/postgres-metadata.py`: complete deterministic object-level catalog comparison and mismatch evidence.
- `scripts/test-postgres-metadata.py`: offline tamper tests and explicitly scoped disposable restore reproduction.
- `scripts/validate-postgres18.py`: corrected comparison, preserved integrity gates, pre-suite runtime/restore checkpoint and sanitized failing Rust test names.
- `scripts/test-staging-validation.py`: address/network validation, auth/Host preservation, network-namespace probe and safe failure regression tests.
- `deployment/postgres.Dockerfile`: pinned compatible gosu security rebuild and unused pgvector removal for fresh candidate databases.
- `.github/workflows/staging-validation.yml`: run offline harness/security/metadata tests before native integration work; still four jobs and manual-only dispatch.
- This report.

Passed: 18 offline CI/security tests, three metadata unit tests (with all-category tamper cases), ten infrastructure graph tests, actual disposable PostgreSQL 16 schema restore and trigger/ACL negative tests, Python syntax, Rust formatting and fifteen secret-scanner self-tests. No Rust source changed and no low-disk Rust rebuild was performed. Free disk is approximately 144 MiB; no additional caches, validated executables, backups, credentials or historical evidence were deleted.

Still pending: actual corrected container execution, Go/gosu rebuild and image scans, PostgreSQL 18 normalized comparison and full Rust database suite. The workflow preserves authentication, restart/recovery, graceful shutdown, disabled-worker/zero-cycle, schema-checksum and restricted-role assertions, and unchanged HIGH/CRITICAL/secret gates. It emits native runtime/restore checkpoints before the Rust suite and only safe failed-test names if that suite fails. No passing production/staging readiness is claimed.

After separate approval to push the reviewed commits and dispatch **one** run, the manual command is:

```sh
gh workflow run staging-validation.yml --repo pyco404/SupaAgent --ref main
```

Do not run it under this milestone's authorization. Residual unfixed advisories can still cause the security gate to fail even after runtime checks pass. No repeated automatic dispatch, Railway resource creation/deployment, primary database operation, live agent activation, paid inference, financial action or publishing operation is authorized.
