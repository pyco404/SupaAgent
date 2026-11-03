# Final functional fixture correction and staging security decision

2026-11-03. Evidence baseline: [CI 41641935483](https://github.com/pyco404/SupaAgent/actions/runs/41641935483), HEAD `74913593a0641f2e559abad3ab76a9121b90e18f`. This is a focused correction and assessment, not a new deployment milestone.

## Fixture correction

`crates/runtime/tests/work.rs` failed before connecting: it required TCP localhost/127.0.0.1 and `supa_m9_test`; CI supplies a socket-only `supa_pg18_suite_test` parent.

The test now accepts only either:

- The documented legacy loopback `supa_m9_test` on port 55680, without a socket override. Pinning the documented port tightens the old unrestricted-port guard.
- The exact CI database on port 55448, under `/tmp/supa-pg18-validation-*/socket`, verified by the existing shared disposable-test helper. A canonical parent, matching server data directory and empty `listen_addresses` are required.

Before creating the UUID child database, server metadata must also confirm the database, authenticated principal, port, socket/TCP transport and an existing LOGIN administrator with CREATEDB or SUPERUSER. This checks existing test authority; it grants nothing. A plausible socket alias forwarding to a different data directory is actively rejected. Wrong database, remote TCP, primary paths, missing socket, wrong port and sibling test profiles fail closed.

All original bounded work, grounding, customer/owner privacy, unsupported/forbidden abstention, limited repair, duplicate execution, simulated accounting and restart assertions are preserved. The test remains opt-in and was actually executed with `--include-ignored`; it was not filtered or disabled. No production runtime, API, migrations, credentials, permissions, frontend or financial implementation changed.

## Actual validation

New PostgreSQL **16** socket-only cluster, peer admin/SCRAM non-admin authentication, exact CI connection profile, clean process environment and cached locked/offline Rust builds:

| Target | Passed | Failed | Ignored |
|---|---:|---:|---:|
| Runtime work, including exact-profile and forwarded-socket rejection | 3 | 0 | 0 |
| API wallet SELECT-only/locking/owner regression | 2 | 0 | 0 |
| API mocked live work/accounting/recovery/diagnostics | 11 | 0 | 0 |

Also passed: Rust formatting, 24 CI security/evidence tests, four schema metadata tests, five independent-backup guard tests and 15 secrets-scanner self-tests. Individual Rust outcomes and private logs are in ignored `artifacts/milestone-11-final-focused/`. The newly created cluster was stopped and its own temporary files removed. No existing database or backup was opened. Free space after temporary cluster removal is about 805 MiB; no broad build, Docker installation or unrelated deletion was attempted.

The last actual PostgreSQL 18 CI run recorded 197 passed/one failed, 13 matching migrations/checksums, restored data and complete schema metadata equality, accounting constraints, restricted roles and crash recovery. Actual frontend/API/worker startup, health, authentication, restart/shutdown passed; expired credentials returned 401; the worker remained disabled with zero cycles. Work review 9/9 and writable epoch 8/8 passed there. Those are prior remote results, not reruns of this new commit. The corrected full PostgreSQL 18 suite and fresh images/scans still require separate push/CI approval.

## Independent verdicts

**A — Functional: GO for one final CI confirmation; complete PostgreSQL 18 functional sign-off remains pending that run.** The exact failure is corrected and passes locally, with related permission/accounting regressions. It is not accurate to claim a fully green remote suite before the corrected code runs there.

**B — Security: NO-GO for persistent private Railway staging.** Existing strict security gates still fail, no advisories are suppressed/accepted, and independent encrypted recovery is not ready. Passing runtime or functional checks does not override this verdict. The next CI can confirm functional closure but is expected to remain security-red without further remediation. Do not spend repeated image runs expecting unchanged Dockerfiles to remove these findings.

## Actual scan inventory and current fixes

The last run scanned all three application images (44 HIGH, zero CRITICAL each) and the API job's exact PostgreSQL candidate (63 HIGH, one CRITICAL). Zero secrets were found and the source dependency scan had no findings. The API final gate aggregates its application plus PostgreSQL counts to 107 HIGH/one CRITICAL; this is not 107 application-image findings. Deduplication gives **64 CVE/package/version rows across 19 advisory IDs**. The independently built worker PostgreSQL candidate has a different digest and was not separately scanned. All digests/full public scan metadata remain in ignored `artifacts/milestone-11-complete-validation/` and retained GitHub logs.

On this date, every relevant installed Debian trixie source-package version is still marked vulnerable by its official tracker. The scans give no fixed version for these installed distribution packages. Newer upstream/testing/unstable versions do exist; that is different from a compatible stable update being available. The runtime Dockerfiles already run stable `apt-get upgrade`; no new same-distribution fix was established. No images were rebuilt or scanned locally during this task.

The critical [CVE-2026-6653](https://security-tracker.debian.org/tracker/CVE-2026-6653) affects the installed libxml2 `2.12.7+dfsg+really2.9.14-2.1+deb13u3`: the `really2.9.14` component matters. The nominal prefix is not proof of an upstream 2.12 codebase. Debian lists trixie vulnerable, forky/sid `2.15.4+dfsg-1` fixed, with first fixed unstable version `2.14.5+dfsg-0.1`. The tracker describes a parser use-after-free/DoS and labels distribution urgency differently from Trivy's CRITICAL; this assessment preserves Trivy severity and the gate. No exact exploit was attempted.

### Compatibility options, not certified replacements

1. A vendor-supported backport into the current libxml2 ABI is the smallest XML-preserving route. Alternatively rebuild PostgreSQL against a patched libxml2, with the [documented XML include/linker configuration](https://www.postgresql.org/docs/18/install-make.html). All eight XML advisories require coverage, not just the critical one. No backport/rebuild was established safe or applied here; injecting sid libraries into the stable image is not a validated shortcut.
2. [Official PostgreSQL 18 Alpine 3.24](https://github.com/docker-library/postgres/blob/master/18/alpine3.24/Dockerfile) exists with XML/OpenSSL support, but is not a drop-in replacement for the Railway SSL helper: musl, TLS entrypoint and PGDATA/volume layout differ. The Alpine 3.24 package endpoint did not yield verifiable package metadata during this review. No claim is made that its current image is clean. It needs pinned digest, actual scans and complete XML/TLS/persistence/restore/role tests before selection.
3. [Distroless Debian 13 nonroot CC and Node 24 runtimes](https://github.com/GoogleContainerTools/distroless) are supported candidate families for removing unnecessary application utilities. They have no default shell: current frontend shell-based command and shell-based validation checks need focused adaptation, plus native dependency/certificate/Next.js testing. Copied libraries can retain findings. No unbuilt alternative is declared secure.
4. Removing unused GnuPG/sudo/Perl/utilities after dependency and entrypoint review may reduce exposure, but dependency removal must be verified against actual contents and runtime tests. Source-package findings do not prove every named binary/library contains vulnerable code. None are suppressed on this basis.

Current safeguards (non-root apps, authenticated endpoints, restricted DB roles, no arbitrary execution/SQL tools, disabled workers/providers, internal CI network) reduce some prerequisites. They do not prove production reachability absent, eliminate parser bugs or override the security gate. No final Railway network/capability/SQL/XML restriction has been deployed or validated.

### Reachability, mitigation and residual risk by advisory

The following are conservative source/advisory assessments, not exploit reproductions or package exceptions. No matching direct application call was found by source search for SQLite, homed, Archive::Tar, XML/XInclude/XPointer, sudo, ACL and nsenter in `crates/*/src` and `web/src`. This does not cover every native library, extension, Node dependency or initialization script. Full metadata inventory and per-package binary/symbol reachability are still needed before any risk acceptance. Each unresolved row continues to block deployment under the existing gate.

| Advisory | Runtime exposure / reachability | Mitigation candidate | Residual risk |
|---|---|---|---|
| [CVE-2025-69720](https://security-tracker.debian.org/tracker/CVE-2025-69720) | An attacker-controlled terminfo input must reach infocmp. No application call found; installed CLI presence/invocation not certified absent. | Avoid untrusted terminfo/CLI execution; remove unneeded utilities in a tested minimal image. | CLI crash/overflow risk remains; package-source attribution is not proof each listed library contains the affected tool. |
| [CVE-2026-119373](https://security-tracker.debian.org/tracker/CVE-2026-119373) | Requires SQLite FTS5 detail=none tables, crafted rowids and prefix queries. Supa uses PostgreSQL; no SQLite API found in reviewed source. Transitive native reachability not exhaustively audited. | Do not expose SQLite/FTS5 or allow arbitrary SQL/tools; test an image without this unused dependency. | Conditional memory-corruption risk; absence of a source call is not proof of unreachability. |
| [CVE-2026-16742](https://security-tracker.debian.org/tracker/CVE-2026-16742) | Requires systemd-homed and a logged-in homed-managed user. Supa launches a process, not a systemd init; no homed path found. Scanner lists libraries from the source package. | Keep non-root application identities, no homed/user administration or host sockets; verify/remove unused components. | Lower apparent exposure; source-package finding is retained pending file/entrypoint proof. |
| [CVE-2026-24882](https://security-tracker.debian.org/tracker/CVE-2026-24882) | Requires TPM-backed keys and tpm2daemon PKDECRYPT. No Supa runtime TPM integration or exposed TPM device was found in the reviewed source/CI contract. Actual binary/device inventory is not exhaustive. | Do not mount TPM devices or use TPM-backed keys; remove unneeded GnuPG tooling from a tested runtime. | Prerequisite-dependent overflow; package attribution alone cannot establish binary reachability. |
| [CVE-2026-54369](https://security-tracker.debian.org/tracker/CVE-2026-54369) | A privileged pathname-based ACL caller must process attacker-controlled symlink components. No direct application ACL call found. PG volume initialization is privileged before dropping privileges. | Non-root applications, controlled private volume paths, no untrusted ACL tooling; a new ACL ABI needs compatibility tests. | Local privilege escalation remains conditional; do not claim fixed by non-root alone. |
| [CVE-2026-6653](https://security-tracker.debian.org/tracker/CVE-2026-6653) | PostgreSQL links libxml2 and CI successfully invokes SQL/XML. A read-only SQL login can call XML functions. No public arbitrary-XML-to-SQL endpoint found; an exact exploit through current HTTP routes was not demonstrated. | Restrict DB network/credentials and tool allowlists; do not admit untrusted XML/SQL. Obtain a supported backport or rebuild PG against a fixed library, then rescan/test. | Parser use-after-free/DoS remains plausible and blocks deployment; SQL/XML presence is genuine, not proof that this exact malicious DTD passes PG parser options. |
| [CVE-2026-74860](https://security-tracker.debian.org/tracker/CVE-2026-74860) | Requires Python libxml2 SAX bindings handling crafted DTD attribute declarations. PG SQL/XML alone does not prove the Python-binding path reachable. No application binding use found. | No Python SAX processing or arbitrary scripting; verify installed binding files and remove unused bindings. | Python-binding double-free exposure unproven; finding remains, with no exception. |
| [CVE-2026-76642](https://security-tracker.debian.org/tracker/CVE-2026-76642) | Requires privileged mount helpers/post-mount hooks and permitted attacker-triggered mount operations. Supa does not offer mount tools. | No privileged containers, no CAP_SYS_ADMIN/host filesystem mounts/fstab-triggered helpers; minimize utilities. | Local privilege risk is prerequisite-dependent; final Railway capabilities/fstab are not certified by CI. |
| [CVE-2026-78408](https://security-tracker.debian.org/tracker/CVE-2026-78408) | Requires privileged nsenter --join-cgroup targeting attacker-controlled state. CI uses the runner host nsenter for a different network-only probe; this is not invocation of the image binary with --join-cgroup. | Never grant this helper path or cgroup/host access; remove unneeded image utilities. | Privileged helper descriptor abuse remains a package finding; host and image tool inventories are distinct. |
| [CVE-2026-78409](https://security-tracker.debian.org/tracker/CVE-2026-78409) | Requires fstab-authorized X-mount.subdir, symlink control and suitable kernel behavior. No application mount path found. | No custom fstab mount privileges, CAP_SYS_ADMIN or attacker-writable mount sources. | Restricted-mount escape is conditional; no deployment-level proof of all missing prerequisites. |
| [CVE-2026-78410](https://security-tracker.debian.org/tracker/CVE-2026-78410) | Requires privileged restricted bind mounts and replaceable source path components. No such application tool is allowlisted. | Private immutable mount sources, no setuid helper access/capability or customer-controlled mount paths. | Path-redirection privilege escalation remains conditional and unaccepted. |
| [CVE-2026-86138](https://security-tracker.debian.org/tracker/CVE-2026-86138) | PG links the XML dictionary implementation; crafted XML QName/dictionary processing is a potential path. Exact trigger not exercised. | Restrict SQL/XML input and resource usage; fixed libxml2 2.15.4 series or reviewed backport required. | Integer/heap overflow remains unproven reachable through HTTP, not dismissed. |
| [CVE-2026-86139](https://security-tracker.debian.org/tracker/CVE-2026-86139) | URI escaping in libxml2 may be reached by XML consumers; exact call path in this PostgreSQL build not proven. | No arbitrary XML/URI processing; use a fixed library with compatibility tests. | Integer overflow remains unresolved; version-specific affected-code review still needed. |
| [CVE-2026-86140](https://security-tracker.debian.org/tracker/CVE-2026-86140) | XML validity/error formatting may traverse libxml2 element representations. PG linkage is proven; vulnerable validation trigger is not. | Restrict untrusted DTD/validation paths and use a patched library. | Stack overflow remains unresolved; no PoC or waiver. |
| [CVE-2026-86142](https://security-tracker.debian.org/tracker/CVE-2026-86142) | Requires libxml2 XPointer processing. Supa has no XPointer tool; actual transitive PostgreSQL/extension paths are not exhaustively proved absent. | Disallow arbitrary XML/XPointer tools and unreviewed extensions; update library. | Heap overflow remains conditional and unresolved. |
| [CVE-2026-86143](https://security-tracker.debian.org/tracker/CVE-2026-86143) | Requires XML output callbacks and overflow-sized lengths. PG links libxml2; callback implementation and practical size constraints need exact analysis. | Bound input/output sizes and use patched library; resource bounds alone are not a fix. | Callback/length-corruption risk remains unresolved. |
| [CVE-2026-86144](https://security-tracker.debian.org/tracker/CVE-2026-86144) | Requires XInclude processing with flags/resource loaders. No Supa XInclude API found; network-isolated CI is not proof of production network isolation. | No XInclude/resource-loading tools; restrict egress and XML inputs; patch the library. | Possible SSRF/DoS remains conditional; egress restriction does not eliminate local resource paths. |
| [CVE-2026-9538](https://security-tracker.debian.org/tracker/CVE-2026-9538) | Requires Perl Archive::Tar reading crafted size fields. No such application input handler found; PG image contains Perl packages. | Do not run Perl archive utilities on untrusted data; bound resources/remove unused tooling after dependency review. | Memory exhaustion is conditional; package remains vulnerable. |
| [CVE-2026-96512](https://security-tracker.debian.org/tracker/CVE-2026-96512) | Requires sudoers NOTBEFORE/NOTAFTER timestamps without Z plus attacker-controlled TZ and authenticated sudo access. No application sudo invocation found. | No sudo privilege grants; explicit Z in any required timed rules; remove unnecessary sudo after entrypoint review. | Local authorization-window bypass remains conditional; actual sudoers inventory not retained in scan evidence. |

### Official fixed versions outside the current stable image

These are first fixed unstable source-package versions from the current official tracker. They are not installed versions or proof of ABI-compatible stable upgrades. The private JSON evidence also retains current stable/testing/unstable rows and upstream notes. Every advisory below links to its primary distribution source.

| Advisory | Source package | First fixed unstable version |
|---|---|---|
| [CVE-2025-69720](https://security-tracker.debian.org/tracker/CVE-2025-69720) | ncurses | 6.6+20251231-1 |
| [CVE-2026-119373](https://security-tracker.debian.org/tracker/CVE-2026-119373) | sqlite3 | 3.54.1-1 |
| [CVE-2026-16742](https://security-tracker.debian.org/tracker/CVE-2026-16742) | systemd | 261.2-1 |
| [CVE-2026-24882](https://security-tracker.debian.org/tracker/CVE-2026-24882) | gnupg2 | 2.4.9-4 |
| [CVE-2026-54369](https://security-tracker.debian.org/tracker/CVE-2026-54369) | acl | 2.4.0-1 |
| [CVE-2026-6653](https://security-tracker.debian.org/tracker/CVE-2026-6653) | libxml2 | 2.14.5+dfsg-0.1 |
| [CVE-2026-74860](https://security-tracker.debian.org/tracker/CVE-2026-74860) | libxml2 | 2.15.3+dfsg-1 |
| [CVE-2026-76642](https://security-tracker.debian.org/tracker/CVE-2026-76642) | util-linux | 2.42.3-1 |
| [CVE-2026-78408](https://security-tracker.debian.org/tracker/CVE-2026-78408) | util-linux | 2.42.4-1 |
| [CVE-2026-78409](https://security-tracker.debian.org/tracker/CVE-2026-78409) | util-linux | 2.42.3-1 |
| [CVE-2026-78410](https://security-tracker.debian.org/tracker/CVE-2026-78410) | util-linux | 2.42.3-1 |
| [CVE-2026-86138](https://security-tracker.debian.org/tracker/CVE-2026-86138) | libxml2 | 2.15.4+dfsg-1 |
| [CVE-2026-86139](https://security-tracker.debian.org/tracker/CVE-2026-86139) | libxml2 | 2.15.4+dfsg-1 |
| [CVE-2026-86140](https://security-tracker.debian.org/tracker/CVE-2026-86140) | libxml2 | 2.15.4+dfsg-1 |
| [CVE-2026-86142](https://security-tracker.debian.org/tracker/CVE-2026-86142) | libxml2 | 2.15.4+dfsg-1 |
| [CVE-2026-86143](https://security-tracker.debian.org/tracker/CVE-2026-86143) | libxml2 | 2.15.4+dfsg-1 |
| [CVE-2026-86144](https://security-tracker.debian.org/tracker/CVE-2026-86144) | libxml2 | 2.15.4+dfsg-1 |
| [CVE-2026-9538](https://security-tracker.debian.org/tracker/CVE-2026-9538) | perl | 5.42.3-1 |
| [CVE-2026-96512](https://security-tracker.debian.org/tracker/CVE-2026-96512) | sudo | 1.9.17p2-9 |

### Exact affected package versions from the actual scan

All scanner fixed-version fields below are empty. No findings have been removed; source-package duplication is explicit. This table contains public package metadata only.

| Advisory | Severity | Package | Installed version | Image(s) |
|---|---|---|---|---|
| CVE-2026-6653 | CRITICAL | libxml2 | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | postgres-candidate |
| CVE-2025-69720 | HIGH | libncursesw6 | 6.5+20250216-2 | postgres-candidate |
| CVE-2025-69720 | HIGH | libtinfo6 | 6.5+20250216-2 | frontend, api, postgres-candidate, worker |
| CVE-2025-69720 | HIGH | ncurses-base | 6.5+20250216-2 | frontend, api, postgres-candidate, worker |
| CVE-2025-69720 | HIGH | ncurses-bin | 6.5+20250216-2 | frontend, api, postgres-candidate, worker |
| CVE-2026-119373 | HIGH | libsqlite3-0 | 3.46.1-7+deb13u2 | frontend, api, postgres-candidate, worker |
| CVE-2026-16742 | HIGH | libsystemd0 | 257.13-1~deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-16742 | HIGH | libudev1 | 257.13-1~deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-24882 | HIGH | dirmngr | 2.4.7-21+deb13u1+b5 | postgres-candidate |
| CVE-2026-24882 | HIGH | gnupg | 2.4.7-21+deb13u1 | postgres-candidate |
| CVE-2026-24882 | HIGH | gnupg-l10n | 2.4.7-21+deb13u1 | postgres-candidate |
| CVE-2026-24882 | HIGH | gpg | 2.4.7-21+deb13u1+b5 | postgres-candidate |
| CVE-2026-24882 | HIGH | gpg-agent | 2.4.7-21+deb13u1+b5 | postgres-candidate |
| CVE-2026-24882 | HIGH | gpgconf | 2.4.7-21+deb13u1+b5 | postgres-candidate |
| CVE-2026-24882 | HIGH | gpgsm | 2.4.7-21+deb13u1+b5 | postgres-candidate |
| CVE-2026-54369 | HIGH | libacl1 | 2.3.2-2+b1 | frontend, api, postgres-candidate, worker |
| CVE-2026-74860 | HIGH | libxml2 | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | postgres-candidate |
| CVE-2026-76642 | HIGH | bsdutils | 1:2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-76642 | HIGH | libblkid1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-76642 | HIGH | liblastlog2-2 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-76642 | HIGH | libmount1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-76642 | HIGH | libsmartcols1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-76642 | HIGH | libuuid1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-76642 | HIGH | login | 1:4.16.0-2+really2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-76642 | HIGH | mount | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-76642 | HIGH | util-linux | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78408 | HIGH | bsdutils | 1:2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78408 | HIGH | libblkid1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78408 | HIGH | liblastlog2-2 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78408 | HIGH | libmount1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78408 | HIGH | libsmartcols1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78408 | HIGH | libuuid1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78408 | HIGH | login | 1:4.16.0-2+really2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78408 | HIGH | mount | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78408 | HIGH | util-linux | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78409 | HIGH | bsdutils | 1:2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78409 | HIGH | libblkid1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78409 | HIGH | liblastlog2-2 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78409 | HIGH | libmount1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78409 | HIGH | libsmartcols1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78409 | HIGH | libuuid1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78409 | HIGH | login | 1:4.16.0-2+really2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78409 | HIGH | mount | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78409 | HIGH | util-linux | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78410 | HIGH | bsdutils | 1:2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78410 | HIGH | libblkid1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78410 | HIGH | liblastlog2-2 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78410 | HIGH | libmount1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78410 | HIGH | libsmartcols1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78410 | HIGH | libuuid1 | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78410 | HIGH | login | 1:4.16.0-2+really2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78410 | HIGH | mount | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-78410 | HIGH | util-linux | 2.41.5-0+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-86138 | HIGH | libxml2 | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | postgres-candidate |
| CVE-2026-86139 | HIGH | libxml2 | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | postgres-candidate |
| CVE-2026-86140 | HIGH | libxml2 | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | postgres-candidate |
| CVE-2026-86142 | HIGH | libxml2 | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | postgres-candidate |
| CVE-2026-86143 | HIGH | libxml2 | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | postgres-candidate |
| CVE-2026-86144 | HIGH | libxml2 | 2.12.7+dfsg+really2.9.14-2.1+deb13u3 | postgres-candidate |
| CVE-2026-9538 | HIGH | libperl5.40 | 5.40.1-6+deb13u1 | postgres-candidate |
| CVE-2026-9538 | HIGH | perl | 5.40.1-6+deb13u1 | postgres-candidate |
| CVE-2026-9538 | HIGH | perl-base | 5.40.1-6+deb13u1 | frontend, api, postgres-candidate, worker |
| CVE-2026-9538 | HIGH | perl-modules-5.40 | 5.40.1-6+deb13u1 | postgres-candidate |
| CVE-2026-96512 | HIGH | sudo | 1.9.16p2-3+deb13u2 | postgres-candidate |

## Backup and recovery limitations

Local config and the [8H enrollment handoff](milestone-8h-key-enrollment.md) still show four null public fingerprints, unverified independent recovery-key custody, and real backup execution disabled. The Railway bucket is provisioned according to the reviewed metadata, but no actual encrypted backup upload/download/decryption/restore drill has been verified. No Railway credentials were retrieved or network operations performed here. Bucket metadata is not a recoverable backup.

Same-disk restore rehearsals prove archive consistency, not survival of primary-disk loss. The planned bucket has no object versioning/object lock; 23 immutable-name snapshots and separately authorized deletion are a policy, not enforced WORM retention. Bucket-wide credentials would allow destructive access unless a narrower supported scope is verified. The $1 monitoring ceiling is not a verified provider hard cap.

Before persistent private data: appoint the independent public-fingerprint verifier; arrange independent protected recovery copies; verify distinct encryption/signing identities and four fingerprints; separately approve credential retrieval and encrypted export/upload; verify object integrity, download and disposable restore with the actual independent recovery material. Do not generate/approve keys, upload data, start the primary or grant these operations as a side effect of CI approval. A strictly disposable synthetic-only deployment would require its own explicit scope/risk decision and still cannot bypass current vulnerability gates.

## Approval boundary and preserved state

Two individual Conventional Commits are intended: this exact runtime test file and this assessment file. Full singleton-history audit and targeted secrets scan must pass; evidence directories remain ignored/local. No bundled commit, history rewrite, push or CI dispatch is authorized by this task.

One corrected CI run is justified to verify the fixture on PostgreSQL 18, but full deployment remains NO-GO. Next approvals are separate: exact-HEAD push/one CI run; any security-policy decision (none assumed); independent recovery-key enrollment/backup actions; eventual Railway service creation/deployment. No production activation is included.

The protected primary stays stopped and unchanged; historical $0.004048 and $0.003536 reservations, completed evaluations and pending review ownership are untouched. No real principal, ownership grant, persistent worker, paid request, transaction, publishing or cloud resource is created. The approved frontend is unchanged.
