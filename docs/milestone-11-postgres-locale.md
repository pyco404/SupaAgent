# PostgreSQL locale initialization correction

2026-11-03. Baseline: [CI 41663207784](https://github.com/pyco404/SupaAgent/actions/runs/41663207784), `5b77a345114a0dad62f76ac568407228c22d6566`. Frontend runtime and native PostgreSQL 18 validation passed; replacement database initialization failed before API/worker runtime validation.

## Root cause established before changing configuration

The exact pinned image uses glibc 2.44-r13 and sets `LANG=en_US.utf8`. The CI PostgreSQL environment supplies only database/data-directory/password settings; it adds no `LC_ALL`, `LC_CTYPE` or `LC_COLLATE` overrides. The selected base's relevant locale defaults do not override the Dockerfile LANG. Build-time package installation does not generate an English locale automatically.

The vendor splits source and generated locale data into different packages:

- `glibc-2.44-locales=2.44-r13` contains `/usr/share/i18n` sources, including the en_US source, but no generated `/usr/lib/locale/en_US.utf8/LC_*` files.
- `glibc-2.44-locale-en=2.44-r13` contains those generated English locale files, including en_US.utf8's category data.

This matches the [vendor package recipe](https://github.com/wolfi-dev/os/blob/e56b3f33a6e7ea76029947d68ce17227fd1fe841/glibc-2.44.yaml). [PostgreSQL 18 initdb](https://www.postgresql.org/docs/18/app-initdb.html) defaults to libc and derives locale/encoding from its environment; a source definition alone cannot satisfy libc setlocale.

A tiny C program calling `setlocale(LC_ALL, "")` and `nl_langinfo(CODESET)` was run with the exact candidate glibc in an isolated, read-only temporary root. No host locale files or primary/database files were mounted. Clean environment: LANG=en_US.utf8, LC_* unset. Results:

| Temporary root state | Actual result |
|---|---|
| Pinned base without generated English data | glibc=2.44, locale=UNAVAILABLE, codeset=UNAVAILABLE; exit 1 |
| Add old glibc-2.44-locales file contents | Same failure; exit 1 |
| Add matching glibc-2.44-locale-en file contents | glibc=2.44, locale=en_US.utf8, codeset=UTF-8; exit 0 |

The real vendor locale CLI also lists en_US.utf8 and reports UTF-8 with the corrected data. Invalid LC_ALL/LC_CTYPE overrides fail the all-category probe; an explicit ASCII LC_ALL=C is recognized as non-UTF-8 and rejected by that probe. No fallback locale is substituted.

The public base was OCI-hash verified. Its vendor APK index passed normal trusted-key verification. Downloaded locale archives were authenticated by matching the signed index's control-stream checksum and the control metadata's SHA-256 data checksum before extraction. Standalone `apk verify` reported these unsigned package archives as UNTRUSTED; that result was not waived with `--allow-untrusted`. A direct exact-version `apk fetch` attempt also failed selection, so it is not presented as a successful verification. The authenticated index/control/data chain and real isolated test are the successful evidence. Future Docker builds retain normal APK trust checks.

## Implemented correction

1. `deployment/postgres-wolfi.Dockerfile` selects `glibc-2.44-locale-en=2.44-r13` instead of the source-data package. The base, glibc, PostgreSQL, XML, LLVM and OpenSSL pins remain unchanged.
2. LANG remains **en_US.utf8**. No persistent LC_ALL override, C/C.UTF-8 substitution, ICU switch, encoding change, initdb collation flag or existing-database conversion is introduced.
3. Build-time checks require the generated locale in `locale -a` and require UTF-8 from `LC_ALL=en_US.utf8 locale charmap`. This command's LC_ALL applies only to that check; it does not change runtime ENV.
4. `scripts/ci-staging-images.py` checks actual `pg_database` metadata after startup: encoding UTF8, datcollate/datctype en_US.utf8 and libc provider `c`. It records the collation version and candidate digest. A mismatch fails closed rather than silently accepting another collation.
5. `scripts/test-staging-validation.py` covers missing metadata and changes to encoding, collation, character classification or provider, plus the exact generated-locale/build-check contract.

No Rust, frontend, SQL migration, permission, accounting, TLS entrypoint or worker control changed. The existing 13-checksum inventory, XML/XPath and TLS tests, non-root checks, authentication boundaries, worker zero-cycle assertion, restart/shutdown and full data/schema restoration checks remain intact.

## Actual validation and limits

| Local check | Result |
|---|---|
| Focused staging/security/locale regressions | 35 passed |
| Synthetic TLS startup/recovery tests | 15 passed; fake OpenSSL only |
| Schema metadata regressions | 4 passed |
| Independent backup guards | 5 passed |
| Infrastructure/container contracts | 10 passed |
| Secret-scanner self-tests | 15 passed |
| Isolated candidate libc/CLI tests | 6 expected outcomes passed, plus the three-state reproduction above |
| Exact Dockerfile package resolution | Passed simulated vendor solve; installed database/world hashes unchanged; no legacy XML selected |
| Python compilation, shell syntax, diff whitespace | Passed |

These are 84 focused automated checks plus native libc/CLI evidence, not a new container or PostgreSQL initialization run. Docker is unavailable locally and disk space is limited. No large build was attempted. The temporary public root and probe created for this investigation were inventoried and removed; about 755 MiB remains available. Complete results/hashes remain ignored/private in `artifacts/milestone-11-security-remediation/locale*`.

A separately approved CI run is still required to build/scan the changed candidate and complete actual initdb, XML/TLS SQL execution, migration checksums, API/worker authentication/restricted access/non-root operation, zero worker cycles, restart/shutdown/recovery and restoration. The prior native PostgreSQL 18 result remains 200 passing Rust tests; it was not rerun locally or relabeled as a replacement-image result. No new image digest or scan is fabricated.

## Remaining gosu advisories

The gosu source/toolchain/dependencies are unchanged by this locale correction. The last actual PostgreSQL scan remains zero HIGH/CRITICAL, one MEDIUM and one UNKNOWN, with zero secrets. Findings are not suppressed or reclassified:

| Finding | Last installed dependency | Available fix / verified scope |
|---|---|---|
| CVE-2026-61801, MEDIUM | github.com/moby/sys/user v0.1.0 | [Upstream fix v0.4.1](https://github.com/moby/sys/security/advisories/GHSA-mjcv-p78q-w5fw). Its go.mod requires Go 1.18; the current upstream gosu go.mod already uses it. DoS exposure requires influence over parsed user/group files; do not mount untrusted replacements for /etc/passwd or /etc/group. |
| CVE-2026-39824, UNKNOWN in this scanner | golang.org/x/sys v0.1.0 | [Go advisory GO-2026-5024](https://pkg.go.dev/vuln/GO-2026-5024) scopes the affected NewNTUnicodeString function to the windows package and fixes it in v0.44.0. That module requires Go 1.25. The current image is linux/amd64; this is platform-scope evidence, not permission to remove the scanner finding. |

Both fixed module versions fit the pinned Go 1.27.3 toolchain's version requirements. Compatibility of a jointly updated gosu build and scanner clearance still require a reproducible rebuild, privilege-drop regressions and fresh scan; none is claimed here. A lower severity or Windows-specific function is not a blanket harmlessness assertion or security exception.

## Decision and safety

**GO for one separately authorized corrected container validation; NO-GO for staging deployment until it passes and remaining security/recovery decisions are resolved.** Approval must name the new HEAD and cover its push and exactly one run. Do not retry failed CI automatically.

Primary PostgreSQL stays stopped. No real database was initialized, migrated or accessed. Historical $0.004048 and $0.003536 holds, retained evaluations and pending ownership records are unchanged. No credentials/keys, agents, inference, transfers, cloud resources, deployments or pushes were authorized or performed. Four changed paths receive individual Conventional Commits on main; existing history is preserved.
