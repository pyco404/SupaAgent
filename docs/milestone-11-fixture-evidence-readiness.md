# Portable database fixtures and complete CI evidence

2026-11-03. Reviewed [run 41636146565](https://github.com/pyco404/SupaAgent/actions/runs/41636146565).
No push, CI dispatch, deployment, protected database access or paid inference.
The website, production Rust authorization/accounting implementation and all
migrations remain unchanged.

## Exact wallet regression and correction

`select_only_job_reads_preserve_wallet_locks_and_owner_boundaries` required
localhost port 55680 and `supa_m10_test`. CI provides private Unix sockets under
`/tmp/supa-pg18-validation-*/socket`, port 55448, database
`supa_pg18_suite_test`. Reusing the actual SQLx library in a tiny isolated
reproduction produces assertion exit 101 with left 55448/right 55680 before any
connection. Evidence: ignored `artifacts/milestone-11-fixtures/legacy-guard.json`.

The test now uses the existing strict disposable-target verifier. The exact
database, socket profile, server data directory and socket-only listener are
verified before creating its UUID-named database. This is not arbitrary target
acceptance or access to the protected primary.

Rehearsal exposed two additional assumptions: its reader and execution logins
were passwordless, while CI deliberately uses SCRAM for non-admin local
connections. Both temporary logins now receive distinct random fixture
passwords passed only through SQLx connection options. No real credential is
loaded or provisioned. The reader still receives SELECT only and has
`default_transaction_read_only=on`; execution column grants remain unchanged.
All original wallet row-lock, owner isolation, paused/unpaid state, accounting
and no-provider-dispatch assertions are preserved and pass.

## Other specialized fixtures are now reproducible

`work_review` previously cloned a private `supa_review_seed_test` with historical
identifiers and report hash. It now creates a fresh UUID database, applies the
authentic SQLx migrations through version 12 and seeds an explicitly synthetic
agent, unpaid simulated job, budget and completed mock session awaiting review.
Its fixed test identifiers and canonical report digest are independent of the
retained evaluation. The report has a supplied-evidence citation and passes the
actual work validator before insertion. The fixture calls no provider.

Migration-13 rollback/checksum tests still run. All existing review tests still
verify immutable output/citations/usage/accounting, owner-only decisions,
forged/stale/duplicate/concurrent requests, revocation, epoch rejection and no
job completion/revenue from acceptance. No historical evaluation is read,
reassigned, reviewed or modified.

`writable_epoch` keeps its separately supplied legacy restored-backup profile.
For the exact verified CI profile it instead creates a new synthetic prepared
epoch and accounting namespace after authentic migrations. It never opens the
private preflight dump in that branch. The same operational permit, role,
accounting, expiry, revocation, restart and lease-fencing assertions run on
these independent UUIDs. A custom `CARGO_TARGET_DIR` is respected when locating
the disposable worker binary; the standard CI target remains unchanged.

These are test identities/permissions in new disposable databases, not primary
owner/custodian grants or new live spending authorizations.

## Evidence retention and independent checks

The PostgreSQL harness still runs the full workspace test command, with the
existing migration-admin exclusion and `--include-ignored`. It adds
`--no-fail-fast`, so a failed test target no longer prevents later targets from
executing. No test is newly ignored, removed or filtered.

Every parsed individual outcome is saved to `rust-results.json` and emitted as
a `rust_test_result` record in retained Actions logs, including source target,
binary, test name and outcome. This covers integration and unit test output.
Failure codes, source locations and SQLSTATEs remain sanitized. Panic parsing
now supports Rust's thread PID format, the reason prior source locations were
missing. Partial completed results are emitted if the suite times out. Raw
panic values, connection strings, prompts and customer data are not published.

Each API container receives an additional independent named viewer credential
whose server-side deadline is already expired. CI explicitly requests private
`/agents` with that token and requires 401. Valid viewer 200, public service
403, missing/invalid token 401 and public-directory 200 checks remain.
The expired token is included in the private-value redaction set and never logged.

Authentication failures are collected without hiding the remaining auth,
restart/shutdown or vulnerability evidence, then fail the job. Authentication
outcomes are explicitly included in the final report. Safety-critical startup,
permissions and worker-disabled assertions still fail closed. The matrix
already uses `fail-fast: false` across services.

Full **public advisory metadata** for application images is now printed, as it
already was for the PostgreSQL candidate and dependencies. This retains all
severity levels, installed/fixed versions and image digests without serializing
secret-scanner matches. Strict HIGH/CRITICAL/secret gates are unchanged; all
collected contract/security failures still produce a failing job. No registry
push or artifact containing private raw logs is added.

## Actual local results

Using new socket-only PostgreSQL **16** clusters with peer admin/SCRAM test
roles, clean environments and mocked providers:

| Target | Passed | Failed / ignored |
| --- | ---: | ---: |
| wallet_read_permissions | 2 | 0 / 0 |
| work_review | 9 | 0 / 0 |
| writable_epoch | 8 | 0 / 0 |

All 19 tests executed, including real disposable API crash/restart and finite
offline worker/replay tests. Child processes and the newly created cluster are
stopped; only their own temporary database directories are removed. The primary
stayed stopped, and historical holds were not accessed or changed.

Evidence: ignored `artifacts/milestone-11-fixtures/final/results.json` and the
three individual target logs. The first rehearsal's passwordless-executor
failure is retained separately under `local/`, not presented as a final pass.

Also passed: 24 staging/security/evidence tests, four metadata tests, ten
infrastructure graph tests, fifteen secret-scanner self-tests, Python syntax,
Rust formatting, and focused locked/offline Rust compilation. Cached low-debug
builds were reused; no unrestricted clean, Docker installation or full local
workspace rebuild was attempted. Remaining free storage is about 848 MiB.

**Not executed:** a new full PostgreSQL 18 workspace run, production-container
expiry probe, new image builds or scans. Those require the next approved CI
run. Local PostgreSQL 16 results are not PostgreSQL 18 certification.

## Residual security risk and compatible remediation assessment

Last actual scans remain 44 HIGH/zero CRITICAL per application image and
63 HIGH/one CRITICAL for the API-job PostgreSQL candidate. No fresh image scan
was run during this task. Exact package/advisory/version records remain in
`artifacts/milestone-11-principal-validation/validation-report.md`; the nine
shared application advisories and exposure prerequisites are documented in
[the earlier review](milestone-11-focused-ci-remediation.md).

No additional compatible stable HIGH/CRITICAL fix was established. Current
Debian records still show the installed stable
[util-linux](https://security-tracker.debian.org/tracker/CVE-2026-76642),
[SQLite](https://security-tracker.debian.org/tracker/CVE-2026-119373),
[ACL](https://security-tracker.debian.org/tracker/CVE-2026-54369),
[Perl](https://security-tracker.debian.org/tracker/CVE-2026-9538),
[systemd](https://security-tracker.debian.org/tracker/CVE-2026-16742) and
[ncurses](https://security-tracker.debian.org/tracker/CVE-2025-69720) families as
vulnerable. Fixed versions in unstable/upstream are not proof of a safe stable
ABI substitution. The current Dockerfiles already perform stable security upgrades.

The remaining critical is CVE-2026-6653 in libxml2
`2.12.7+dfsg+really2.9.14-2.1+deb13u3`.
[Debian's current package tracker](https://security-tracker.debian.org/tracker/source-package/libxml2)
still marks trixie vulnerable and forky/sid fixed with 2.15.4+dfsg-1. Required
PostgreSQL XML functionality links this library. Malicious XML reaching the
affected parser can cause a use-after-free/denial of service. A read-only SQL
login does not inherently prevent calls to XML functions. No public XML API
path was identified, but actual unreachability is not proven and no exception
is taken. Other application advisories have specific local privilege, filesystem,
CLI-parser or FTS5 prerequisites; non-root operation does not prove harmlessness.

Reviewed alternatives, **not validated replacements**:

- Minimal glibc/non-root Rust and Node runtime images can remove unnecessary
  utilities/packages. [Distroless](https://github.com/GoogleContainerTools/distroless)
  provides supported runtime families, but copied/shared libraries still need
  scanning, and shell-dependent startup/CI checks must be adapted and tested.
- An official PostgreSQL 18 Alpine variant is available. The
  [reviewed upstream Dockerfile](https://github.com/docker-library/postgres/blob/e00e1bd34ec5c8a8e7ad89b273b3d42efaf6d5bc/18/alpine3.24/Dockerfile)
  enables libxml and OpenSSL, but it changes libc, entrypoint/SSL setup and
  PostgreSQL 18 volume layout. It cannot replace the Railway SSL helper without
  compatibility, persistence, XML/TLS and vulnerability tests. Alpine 3.23's
  [libxml2 package](https://pkgs.alpinelinux.org/package/v3.23/main/x86_64/libxml2)
  is 2.13.9-r1, not evidence of a completely patched image; 3.24 package details
  were unavailable from the package endpoint during review.
- A vendor-supported backport or PostgreSQL 18 rebuild linked against fixed
  libxml2 is the targeted XML-preserving route. PostgreSQL documents
  [explicit XML compiler/linker configuration](https://www.postgresql.org/docs/18/install-make.html).
  Mixing an incompatible unstable library into the existing image is not implemented.

There is no suppression, severity downgrade, risk acceptance or deployment
bypass. **Deployment stays blocked** by the existing strict gates until actual
compatible remediations pass, or an explicit security decision and separately
reviewed policy change are authorized. No such decision is inferred here.

## Next approval boundary

The known generic-CI legacy fixture prerequisites are resolved in code and pass
locally. One corrected remote run is justified to execute the complete applicable
PostgreSQL 18 suite and actual container-expiry probe, retain every test outcome,
and rebuild/rescan all images. It may still fail the known strict security gates;
no passing staging/deployment readiness is claimed.

Push and dispatch require separate approval for the exact resulting commits/HEAD.
After approval, the existing manual workflow command remains:

```sh
gh workflow run staging-validation.yml --repo pyco404/SupaAgent --ref main
```

No Railway action, real owner provisioning, primary migration, paid inference,
persistent agent activation, financial transaction, trading or publishing is
part of this work. Historical accounting, including both unresolved holds,
remains unchanged.
