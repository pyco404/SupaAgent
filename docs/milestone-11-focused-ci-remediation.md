# Focused CI remediation after run 41626776560

2026-11-03. Changes are local only. No push, workflow dispatch, Railway action,
protected database connection, inference or historical accounting mutation.
The [last actual run](https://github.com/pyco404/SupaAgent/actions/runs/41626776560)
remains failed. Its frontend runtime passed; API/worker runtime checks were blocked.

## Candidate build: package selection confused with installation state

`deployment/postgres.Dockerfile` held `postgresql-18` and its client during OS
upgrades, then required `${Status}` to equal `install ok installed` before
unholding them. Holding changes that field to `hold ok installed`. The assertion
therefore fails even when the package is healthy. This explains the silent exit
1 after successful package operations; no compiler error was recorded. The Go
builder was canceled when the independent runtime stage failed, so its success
must not be inferred.

A private, synthetic dpkg database reproduces both selections using the real
`dpkg-query`. The fix checks `${db:Status-Status}=installed` and
`${db:Status-Eflag}=ok` independently. It still rejects config-files-only and
reinstallation-required packages. See the [Debian field documentation](https://manpages.debian.org/trixie/dpkg/dpkg-query.1.en.html).
No host package selection is changed by these tests.

The hold, security upgrade, pgvector purge and validation are separate build
steps, giving future failures an exact stage. Required server/client versions
and package states are printed; PostgreSQL 18's binary is executed and its
dynamic dependencies must have no missing library. The pinned upstream digest,
Go 1.27.3 builder/source checksums, gosu privilege-drop checks, XML support,
entrypoint, persistence, image scanning and strict security gates are retained.
This candidate is for a new isolated database, never an in-place extension purge
on an existing database.

## Restore comparison: truncated, colliding catalog identities

The first arm of `SNAPSHOT_SQL` returned `pg_namespace.nspname` as PostgreSQL
`name`. UNION type resolution propagated that type to all object identities.
Qualified identifiers longer than 63 bytes were silently truncated. Thus:

```text
public.application_epoch.application_epoch_historical_database_check
public.application_epoch.application_epoch_historical_database_not_null
```

both became the exact last-run mismatch key:

```text
public.application_epoch.application_epoch_historical_database_
```

PostgreSQL 18 stores named NOT NULL constraints alongside CHECK constraints in
[`pg_constraint`](https://www.postgresql.org/docs/18/catalog-pg-constraint.html).
`jsonb_object_agg` then lost one colliding value; differing catalog order could
choose different definitions before and after restore. The retained last-run
log cannot recover the overwritten definitions. The truncation/collision defect
is reproduced; actual PostgreSQL 18 closure still requires the next approved run.

The first identity is now explicitly `text`, preserving every qualified key.
There is no whitespace stripping, CHECK removal, or comparison exclusion.
Constraint snapshots additionally compare type, validation, enforcement,
deferrability, initial deferral and inheritance policy, while existing relation
ownership/ACL comparisons remain. PostgreSQL 16 compatibility uses the implicit
enforced default when its catalog lacks PostgreSQL 18's `conenforced` column.
The [UNION resolution rules](https://www.postgresql.org/docs/18/typeconv-union-case.html)
explain the importance of the first input's type.

Future synthetic-CI constraint failures log bounded source/restored definitions
(up to ten constraints, 2,000 characters per value), retaining complete values
in private runner evidence. Other object definitions remain private. This query
is for reviewed schema metadata, never customer records or private prompts.

## Actual local validation

- 19 offline staging/security tests pass, including real dpkg selection/error
  fixtures and unchanged scanner/auth/network/shutdown restrictions.
- Four metadata unit tests pass, including changed CHECK expressions and each
  enforcement/validation/deferral flag, object loss, changed ownership and order.
- Actual PostgreSQL **16** disposable rehearsal passes: two new databases,
  all 13 migration SQL files, synthetic long constraint identities, custom-format
  backup/restore and equality of all 11 metadata categories. No application rows
  or historical records copied. This is not a PostgreSQL 18 runtime result or a
  new complete SQLx integration-suite result.
- Real SQL probes reproduce `name:63` versus `text:100` and prove the former
  query retains one colliding constraint while the fixed query retains both.
  A changed CHECK expression, disabled trigger and revoked function/schema/
  database rights are detected in the restored disposable database. Only the
  newly created databases are dropped afterward.
- Evidence: ignored `artifacts/milestone-11-metadata/ae41b556a83f/`:
  `rehearsal.json`, `schema-comparison.json`, `constraint-change.json`.
- Ten infrastructure graph tests, Python syntax checks, Rust formatting and
  fifteen secret-scanner self-tests pass. No Rust/frontend implementation changed.

The PostgreSQL 18 harness now explicitly checks both original long identities,
changes the CHECK only in its disposable restore, proves exactly that object is
detected, restores its original definition and rechecks full equality before
running the existing Rust PostgreSQL suite. Existing restricted native/container
API checks, paused worker, zero cycles, restart, crash recovery, authentication,
shutdown and migration-checksum gates are preserved.

No Docker is installed; free disk is approximately 146 MiB. Actual candidate
build, initialization, gosu/compiler execution, fresh scans and full container
API/worker validation require CI. The separate native runtime-profile rehearsal
requires 250 MiB, so it was not rerun. No large Rust/container rebuild or disk
cleanup was attempted, and no passing remote result is claimed.

## Security review: nine advisories, 44 package occurrences per app image

The three application scans share the same 44 HIGH package/version rows, rather
than 132 different vulnerabilities. There are nine advisory IDs. The last actual
scans contain no CRITICAL findings or detected secrets in those app images.
No new scan has run. Exact binary-package occurrences are retained in the local
last-run evidence report; current official status is summarized below.

| Advisory | Installed package family/version | Current upstream/unstable fix information | Exposure conditions |
| --- | --- | --- | --- |
| [CVE-2026-76642](https://security-tracker.debian.org/tracker/CVE-2026-76642) | util-linux 2.41.5-0+deb13u1; login's package version is 1:4.16.0-2+really2.41.5-0+deb13u1 | Debian unstable first fixed 2.42.3-1; stable vulnerable | Privileged mount helper/post-mount hook handling |
| [CVE-2026-78408](https://security-tracker.debian.org/tracker/CVE-2026-78408) | same util-linux family | Unstable 2.42.4-1 listed fixed; stable vulnerable | Privileged nsenter with --join-cgroup; CI uses host nsenter with --net only, not this image's command |
| [CVE-2026-78409](https://security-tracker.debian.org/tracker/CVE-2026-78409) | same util-linux family | Unstable 2.42.4-1 listed fixed; stable vulnerable | Authorized X-mount.subdir entry and symlink traversal |
| [CVE-2026-78410](https://security-tracker.debian.org/tracker/CVE-2026-78410) | same util-linux family | Unstable 2.42.4-1 listed fixed; stable vulnerable | Local attacker redirects restricted privileged bind mount |
| [CVE-2026-54369](https://security-tracker.debian.org/tracker/CVE-2026-54369) | libacl1 2.3.2-2+b1 | acl 2.4.0-1 unstable; stable vulnerable | Attacker-controlled path components passed to privileged ACL operations |
| [CVE-2026-119373](https://security-tracker.debian.org/tracker/CVE-2026-119373) | libsqlite3-0 3.46.1-7+deb13u2 | Upstream 3.54.0 / unstable 3.54.1-1; stable vulnerable | Crafted FTS5 data and prefix MATCH query |
| [CVE-2026-16742](https://security-tracker.debian.org/tracker/CVE-2026-16742) | libsystemd0/libudev1 257.13-1~deb13u1 | Unstable first fixed 261.2-1; stable vulnerable | systemd-homed-managed logged-in user/group manipulation |
| [CVE-2025-69720](https://security-tracker.debian.org/tracker/CVE-2025-69720) | libtinfo6/ncurses-base/ncurses-bin 6.5+20250216-2 | Upstream ncurses 6.5-20251213 / unstable 6.6+20251231-1; stable vulnerable | infocmp CLI parses malicious input |
| [CVE-2026-9538](https://security-tracker.debian.org/tracker/CVE-2026-9538) | perl-base 5.40.1-6+deb13u1 | Archive::Tar 3.10 / Debian perl 5.42.3-1 unstable; stable vulnerable | Perl Archive::Tar loads unbounded attacker-supplied archive size |

No explicit application use of these affected mount/homed/ACL/FTS5/Perl/infocmp
operations was identified in the inspected source. That is not proof of
unreachability: dependency calls, executable/module presence, SUID paths and
future command execution need actual-image inspection. Non-root startup alone
does not make local privilege escalation harmless. Findings remain blocking;
no ignore files, severity changes or false-positive exceptions were introduced.

The runtime Dockerfiles already update packages from their configured stable
repositories. Current Debian records show no compatible stable fixed versions
for these installed families. Substituting unstable libraries, force-removing
essential packages or patching binaries without ABI/runtime validation would
not be a justified compatible fix; none was done.

PostgreSQL's last upstream scan remains historical: 89 HIGH, two CRITICAL.
[CVE-2026-6653](https://security-tracker.debian.org/tracker/CVE-2026-6653) affects
libxml2 `2.12.7+dfsg+really2.9.14-2.1+deb13u3`, required by PostgreSQL XML support.
Stable remains vulnerable; unstable fixes start at 2.14.5+dfsg-0.1. No compatible
stable backport/replacement was verified. Removing XML linkage or substituting
an incompatible library is not implemented.

The historical gosu Go stdlib `v1.24.6` critical CVE-2025-68121 has scanner fixed
versions 1.24.13, 1.25.7 and 1.26.0-rc.3. Its affected behavior is TLS session
resumption with changed certificate authorities; gosu is a startup privilege
dropper, but no runtime reachability exception is taken. Debian's
[advisory](https://security-tracker.debian.org/tracker/CVE-2025-68121) and the
scanner use differing fixed-version descriptions; preserve both evidence
sources rather than silently overriding one. The pinned Go 1.27.3 rebuild and
unused pgvector removal remain prepared compatible remediations, pending an
actual successful candidate build and scan. No reduction in scan counts is claimed.

## Approval boundary and remaining blockers

One corrected manual CI run is justified by reproducible fixes to both failed
checks. After separate approval for the exact local commits/HEAD, push main
without rewriting history, then dispatch exactly once:

```sh
gh workflow run staging-validation.yml --repo pyco404/SupaAgent --ref main
```

No dispatch or push is authorized now. The next run must demonstrate the
candidate build/gosu/libraries/initialization, PostgreSQL 18 full schema equality
and tamper regression, API/worker restricted runtime/auth/recovery/shutdown,
full Rust PostgreSQL suite, and fresh strict vulnerability scans/digests. Known
unfixed advisories can still fail the unchanged security gate. No staging
deployment readiness is claimed. The Railway graph's upstream image also must
not be confused with a separately validated candidate. Backup/identity and
resource/deployment approvals remain separate. The primary stayed stopped;
both historical holds remain untouched.
