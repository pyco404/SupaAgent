# CI functional remediation after run 41630933509

2026-11-03. Local work only; no push or workflow dispatch. The approved website,
Rust application authentication/accounting implementations, migrations, primary
database, historical holds and deployment configuration are unchanged.

## API: service authentication is not principal authentication

The failed CI assertion used `SUPA_API_TOKEN` against `/agents` and expected 200.
That credential authenticates a public service; it does not bind an owner,
operator or viewer principal. The middleware explicitly permits service GETs
only for `/health`, `/directory`, `/autonomy` and `/services`. Private agent lists
require a named principal. The middleware returns **403 permission_denied** for
this valid service token, before entering the handler or executing its SQL.
The original runner did not retain the status; the exact behavior has now been
reproduced against the real router and a genuine disposable database.

No token-expiry, SQL privilege, epoch or accounting defect was found on that
request path. `/agents` reads `accessible_agents(actor.id)` and remains private.
The CI fixture already had an independently configured viewer principal with a
random scoped token and a 30-minute server-side deadline, but used the public
token instead. No grants, financial permissions or credential scopes are added.

Corrected CI checks:

| Request | Expected result |
| --- | --- |
| `/agents`, missing token | 401 |
| `/directory`, valid public service token | 200 |
| `/agents`, valid public service token | 403 |
| `/agents`, configured named viewer token | 200 |
| `/agents`, invalid token | 401 |

Each check now emits its stage, path, actual/expected HTTP status and outcome.
Only the exact error strings `permission_denied` and `unauthorized` are admitted
from bounded error bodies. Arbitrary body text, headers, tokens and SQL errors
are never printed. Failures still stop validation. The application middleware,
database role and frontend contracts are unchanged.

## Nine Rust failures: obsolete disposable-target assertion

Every failing test uses `live_work.rs::setup`. It required database
`supa_m10_test` on port 55680. The PostgreSQL 18 runner supplies
`supa_pg18_suite_test`, port 55448, on its newly created private Unix socket.
The assertion failed before connecting, migrating, creating jobs or exercising
the individual test. This is a fixture assumption, not a demonstrated
PostgreSQL 18 transaction, constraint or accounting difference.

The test-only `support/disposable.rs` preserves the legacy exact target and
adds the exact CI database/port/socket profile. The CI socket must be directly
under `/tmp/supa-pg18-validation-*/socket`; the parent must canonicalize to
itself, and a read-only server check must confirm its exact data directory and
empty `listen_addresses` before any writes. Other databases, ports, remote
hosts and primary-storage paths are rejected. Each work test still creates its
own UUID-named database from `template0` and runs real migrations. No production
permission, provider, accounting or lifecycle code was changed.

Actual PostgreSQL **16** results, running the compiled integration-test binary
with `--include-ignored --test-threads=2`, genuine new socket-only cluster,
clean environment and mocked providers:

| Previously failing test | Result |
| --- | --- |
| ambiguous_charges_concurrency_and_scope_fail_closed | Pass |
| claude_work_accounting_is_audited_and_budget_cannot_switch_models | Pass |
| interrupted_reserved_diagnostics_survive_recovery_without_refund_or_replay | Pass |
| customer_style_matrix_preserves_evidence_scope_and_accounting | Pass |
| rejected_work_diagnostics_are_private_precise_bounded_and_restart_safe | Pass |
| missing_usage_metadata_failure_and_timeout_do_not_replay_paid_work | Pass |
| structured_work_fixture_jobs_cover_creation_evaluation_review_and_terminal_closure | Pass |
| revocation_expired_leases_and_restart_retain_charge_evidence | Pass |
| uncertain_diagnostics_survive_reconnect_and_preserve_replay_fences | Pass |

The same target also tests wrong-target rejection and real router behavior for
public versus named identities, owner/operator/viewer reads, viewer/operator
creation denial, expired credentials and invalid tokens. No provider calls are
made by the authentication regression. These results do not certify PostgreSQL
18 execution; that remains a remote validation requirement.

Future Rust failures retain failed test names, relative source locations,
SQLSTATEs and compiler codes. Panic values, URLs, prompts and customer prose
remain private. No test has been newly disabled or removed from the CI command.

## Validation and disk protection

- Focused Rust work target compiled offline with locked dependencies, two build
  jobs, no debug symbols and no incremental output. Build monitor enforced a
  650 MiB free-space reserve; no existing artifact was deleted.
- All nine work tests passed; the final target contains eleven tests including
  the authentication and wrong-target regressions, executed without filtering.
- 22 Python staging/security/diagnostic tests pass.
- Rust formatting, Python syntax, existing metadata/infrastructure tests and
  secret-scanner self-tests are checked separately. Full history/secret audits
  accompany the individual commits.
- Evidence remains ignored/local in `artifacts/milestone-11-functional/`, with
  final test output under `final/`. The new local cluster is stopped and only
  its own newly created temporary directory is removed after the run. No
  existing cluster, backup, credential, executable or accounting evidence is removed.

The full PostgreSQL 18 workspace suite has **not** been rerun locally or remotely.
It contains additional specialized legacy fixtures beyond these nine tests:
`work_review` expects `supa_review_seed_test`, `wallet_read_permissions` expects
the legacy work database, and `writable_epoch` requires a separately restored
historical preflight fixture. Those prerequisites are absent from generic CI.
They must be provisioned with approved synthetic equivalents or separately
validated disposable restoration profiles before claiming the entire suite
works. This change does not conceal those tests, remove `--include-ignored`,
copy protected state to CI or promise the next complete workspace run is green.

## Vulnerability review and safer-runtime options

Last actual scans remain: 44 HIGH/zero CRITICAL per application image;
PostgreSQL candidate 63 HIGH/one CRITICAL. No new image scan was authorized or
performed. The 44 application occurrences are nine shared advisory IDs, not
132 different vulnerabilities. Exact packages/versions and exposure conditions
are in [the focused security review](milestone-11-focused-ci-remediation.md) and
the ignored last-run report. All findings and strict security gates remain.

Current Debian records still list the stable installed
[util-linux](https://security-tracker.debian.org/tracker/CVE-2026-76642),
[SQLite](https://security-tracker.debian.org/tracker/CVE-2026-119373) and
[libxml2](https://security-tracker.debian.org/tracker/CVE-2026-6653) packages as
vulnerable. No compatible stable update was verified. In particular, the
required libxml2 package `2.12.7+dfsg+really2.9.14-2.1+deb13u3` remains affected
by CVE-2026-6653. Debian unstable lists fixed libxml2 2.15.4+dfsg-1, with fixes
first tracked at 2.14.5+dfsg-0.1; it is not a drop-in stable ABI substitution.
The absence of a scanner fix is not evidence of harmlessness.

Exposure constraints remain advisory-specific: util-linux requires affected
privileged mount/namespace paths; SQLite requires affected FTS5 queries; ACL
requires privileged operations on attacker-controlled paths; ncurses requires
the affected CLI parser; Perl requires affected archive loading; systemd-homed
requires its managed-user behavior. Direct affected application calls were not
identified, but transitive use and actual-image reachability are not ruled out.
The PostgreSQL library is used for XML functionality; affected-parser exposure
requires further testing. Non-root execution does not justify an exception.

Options needing their own actual build/scan/runtime validation:

1. A minimal non-root glibc runtime for Rust with only its verified dynamic
   dependencies and CA roots; compare the supported
   [Distroless runtime offerings](https://github.com/GoogleContainerTools/distroless).
   Do not assume a smaller image fixes copied vulnerable libraries.
2. A compatible minimal Node runtime, preserving entrypoint, signal forwarding,
   Next.js behavior, health checks and authentication; it cannot depend on a
   shell entrypoint if the selected image has no shell.
3. A separately built PostgreSQL 18 candidate linked against a fixed supported
   libxml2, preserving XML, SSL, gosu, initialization, volume/restore behavior
   and existing PostgreSQL checks. A vendor-supported compatible backport is
   preferable to mixing unstable shared libraries into the current image.

No speculative library removal, mixed-distribution upgrade, ABI replacement,
scanner suppression or unvalidated runtime-image switch was implemented.

## Next remote validation boundary

One approved run is useful to verify the corrected API contract and nine work
tests on PostgreSQL 18, followed by restart/shutdown and strict scanning. The
specialized fixture prerequisites above and known vulnerabilities remain
blockers to a completely passing staging-validation run. No claim of full
staging readiness is made. Push and workflow dispatch still require separate
approval; no Railway resource, deployment, paid inference, real credential,
agent activation, financial action or historical hold change is authorized here.
