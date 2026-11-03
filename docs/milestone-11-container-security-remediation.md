# Container security remediation candidates

2026-11-03. Baseline: [functional CI 41645972685](https://github.com/pyco404/SupaAgent/actions/runs/41645972685), `c23d0f6b51e80d6a2f7aceeb36c13ddbf6892ecd`. PostgreSQL 18 had 200 passing Rust tests; all 13 migrations and actual application runtime/authentication/restart/recovery checks passed. Only the vulnerability gates failed.

**Private Railway staging: NO-GO until the new images are built, executed and scanned, and independent recovery is verified.** Source changes and limited native checks are not image certification. No push, CI dispatch, image publication or deployment was performed in this task.

## Implemented application remediation

API and worker now use the official pinned `gcr.io/distroless/cc-debian13:nonroot` runtime, with UID/GID 65532. These binaries require glibc, libm and libgcc; cached binaries successfully load against the actual candidate libraries and fail closed on deliberately invalid disposable configuration. No real database connection or provider call occurs. Their existing offline/disabled defaults and exact entrypoints/ports remain.

Frontend uses pinned official Node **26.13.0 LTS** for build and distroless runtime. The current [official Node release index](https://nodejs.org/dist/index.json) lists it as LTS; [Next 16.4.3's engine contract](https://github.com/vercel/next.js/blob/v16.4.3/packages/next/package.json) admits Node >=20.9.0. It runs Next directly, without a shell, while retaining PORT from the environment, 0.0.0.0 container binding, non-root ownership and the exact approved application files. No frontend JavaScript, Rust implementation, permissions or API contracts changed.

Node 24 was explicitly rejected as the final security candidate: inspecting its real `process.versions` found bundled SQLite 3.53.4. That copy remains below the fix for [CVE-2026-119373](https://security-tracker.debian.org/tracker/CVE-2026-119373), even when the Debian SQLite package is removed. The selected, hash-verified Node 26.13.0 binary reports **SQLite 3.54.0**. This fixes the known upstream issue rather than waiving an unused component. CI now checks the embedded version independently of the OS scanner and fails on missing/unsupported metadata or a vulnerable version.

The inspected CC and final Node base inventories contain none of the original application HIGH package names. That is direct base-filesystem evidence, **not a fresh scan of the assembled application image**. Exact old CVE/package/version rows remain in [the prior assessment](milestone-11-final-functional-security.md). The source dependency lockfiles remain unchanged; new/transitive/bundled advisories still require the fresh full scan.

| Shared original advisories | Original affected package family / version | Candidate treatment |
|---|---|---|
| CVE-2026-76642, -78408, -78409, -78410 | util-linux family 2.41.5-0+deb13u1; login also has the documented epoch/really version | Those package names absent from both inspected minimal application bases |
| CVE-2026-54369 | libacl1 2.3.2-2+b1 | Package absent |
| CVE-2026-119373 | libsqlite3-0 3.46.1-7+deb13u2; additionally Node 24 embeds SQLite 3.53.4 | OS package absent; real Node 26 binary embeds fixed SQLite 3.54.0 |
| CVE-2026-16742 | libsystemd0/libudev1 257.13-1~deb13u1 | Packages absent |
| CVE-2025-69720 | ncurses packages 6.5+20250216-2 | Packages absent |
| CVE-2026-9538 | perl-base 5.40.1-6+deb13u1 | Package absent |

The original 44 rows per application deduplicate to nine advisory IDs; do not add three copies as independent vulnerabilities. Package absence does not prove no other embedded copy, unknown advisory or copied dependency exists. Source/library checks and Trivy stay enabled.

## PostgreSQL: a package-compatible patched XML ABI

The installed Debian library is `2.12.7+dfsg+really2.9.14-2.1+deb13u3`; the `really2.9.14` component is important. [Debian still marks it vulnerable](https://security-tracker.debian.org/tracker/CVE-2026-6653), including the current stable security package. An ordinary stable upgrade does not establish remediation. No manual library patch, ABI symlink hack or unstable Debian library is installed.

The ordinary official Alpine PostgreSQL 18 image was assessed but not selected: its official stable index still provides libxml2 2.13.9-r2. It is beyond the critical's upstream fix, but complete coverage of seven later XML HIGH advisories was not established. This alternative must not create a false green result from incomplete distribution metadata.

`deployment/postgres-wolfi.Dockerfile` instead uses a pinned official Wolfi base and exact vendor APK versions. Availability was verified from the official public index. The [vendor PostgreSQL build recipe](https://github.com/wolfi-dev/os/blob/908da0eb48aefa270d76f90cb19557819aa72cbd/postgresql-18.yaml) and package dependencies show PostgreSQL is already compiled against **libxml2.so.16**. The [XML recipe](https://github.com/wolfi-dev/os/blob/0d1cb8462683e2c06d7591b8b4ad4d1f2797a0f7/libxml2.yaml) pins the upstream 2.15.4 source commit. These are matching vendor-built packages, not an old PostgreSQL binary with an incompatible library copied underneath it.

| Component | Exact candidate version | Purpose / evidence |
|---|---|---|
| postgresql-18-base/client-base/contrib-base/oci-entrypoint-base | 18.6-r7 | Vendor package family with XML, TLS, ICU, LDAP, GSSAPI, compression and contrib functionality; base/contrib depend on XML ABI 16 |
| libxml2-16 | 2.15.4-r2 | Real shared-library package, not the small libxml2 metapackage; advertises libxml2.so.16 |
| openssl-4.0 | 4.0.3-r8 | Matches vendor PostgreSQL's libssl/libcrypto ABI 4 |
| bash | 5.3-r14 | Supported upstream/vendor initialization script |
| glibc-locales, posix-libc-utils | 2.43-r13 | Locale data and explicit ldd diagnostics; ldd is absent from the minimal base |
| gosu | 1.19 rebuilt with Go 1.27.3 | Retains the previously validated pinned source/toolchain recipe and static privilege-drop binary |

The recipe uses the pinned vendor base's trust configuration and normal signed APK verification. No `--allow-untrusted`, repository mixing, testing/edge packages or hand-edited shared libraries. Package installation and signature validation have **not run locally**: they are enforced by the future container build. Public OCI manifest/layer content hashes were checked; publisher keyless signature verification was not performed locally, and no cryptographic publisher-attestation claim is made.

The vendor's rolling public repository has retention/lifecycle constraints and no paid support/SLA is inferred. Exact pins intentionally fail if unavailable; do not automatically select a different package, major or repository. Review updates and consider a suitable vendor support arrangement before long-lived operation.

### Known XML advisory coverage

Upstream 2.15.4 covers the baseline critical CVE-2026-6653 and the seven newer XML HIGH advisories: CVE-2026-74860, CVE-2026-86138, -86139, -86140, -86142, -86143 and -86144. Individual primary-distribution advisories/fix versions are linked in [the previous assessment](milestone-11-final-functional-security.md). CI now records the actual installed shared-library package version and requires independent upstream fix coverage, including when a distro scanner has no advisory entry. This version check does not certify other libraries or unknown/new issues; all scan findings remain gating.

Build-time checks run postgres --version and ldd, reject missing dependencies, and reject any retained legacy `libxml2.so.2*`. No patched image is claimed before those checks execute. Residual package findings, actual runtime installed versions and assembled image digests are **pending the authorized build/scan**, not zero.

### Startup, TLS and persistence contract

The fresh candidate retains `/var/lib/postgresql/data/pgdata` within the persistent `/var/lib/postgresql/data` mount and port 5432. `postgres-entrypoint.sh` drops root before OpenSSL/PostgreSQL, refuses unexpected/symlink paths, mismatched majors and unrecognized nonempty directories, and calls the signed vendor's upstream-derived entrypoint. It creates private 3072-bit RSA self-signed TLS material beside PGDATA, so initdb receives an empty data directory. Certificate/key ownership and 0600 permissions are checked; expired certificates renew using the same private key. Partial/mismatched pairs fail closed for operator review. Temporary output is private and removed, and keys/credentials are never logged. TLS defaults to 90-day certificates with renewal inside 30 days. A self-signed certificate still needs an explicit client trust arrangement for verify-full authentication; sslmode=require alone provides encryption, not a trusted server identity.

This is a new disposable/fresh-staging candidate, not authorization to attach a primary volume or perform an in-place OS/database conversion. The legacy Debian recipe is retained unchanged for baseline reproducibility and is no longer selected by CI. Railway's current authoring graph still references its old helper image: it must reference the validated replacement artifact through a separately reviewed distribution/deployment plan before staging. Optional Railway-specific upgrade/PITR/pgaudit helper behavior is not certified by this replacement; Supa's database audit triggers and actual required functionality must pass the existing tests.

## Validation implemented in CI, not yet executed remotely

The existing manual workflow, minimal permissions and no-registry/no-artifact-upload policy remain. No new run is triggered. The harness now:

- Inspects exact exported app filesystems and package inventories without requiring a shell in a hardened image. Layer canary and excluded private-file checks remain; filesystem metadata is read without extraction.
- Verifies actual non-root process UIDs using host docker top rather than installing image account tools. Existing internal/runner-bridge health and all authentication/expiry boundaries remain.
- Records real Node component versions and requires the bundled SQLite fix. Records actual XML package versions and known upstream coverage.
- Builds and scans the exact Wolfi PostgreSQL candidate separately in **both** API and worker jobs, retaining both digests and findings. No unscanned independently built worker database candidate is treated as certified.
- Verifies PostgreSQL non-root processes, TLS-required connection, key permissions, actual XML parsing/XPath, certificate identity across restart and all 13 migration checksums.
- After stopping the app to prevent heartbeat races, performs a private custom dump, isolated logical restore and complete data/catalog comparison using the existing fingerprint/metadata implementations. No SQL target other than the two fixed CI databases is accepted. No primary URL or archive is read.
- Retains all strict HIGH/CRITICAL/secret gates and public findings; added source-coverage gates fail closed without environment waivers. Native PostgreSQL 18's complete applicable Rust suite remains unchanged.

## Actual local validation

| Check | Actual result |
|---|---|
| Security/evidence regressions | 30 passed |
| TLS startup/restart/expiry/identity/privilege/error tests | 15 passed, fake OpenSSL only; no keys or database generated |
| Metadata comparison regressions | 4 passed |
| Infrastructure authoring contracts | 10 passed |
| Independent backup guards | 5 passed |
| Secrets-scanner self-tests | 15 passed |
| Python syntax, bash syntax and diff whitespace | Passed |
| Candidate CC libraries with cached Rust API/worker | Both load, then fail closed on invalid test config; no DB/provider access |
| Verified Node 26.13.0 binary, unchanged existing Next 16.4.3 production build | /healthz 200, missing authentication 401, synthetic visitor 200, PORT environment honored, SIGTERM exit 143 |
| Bundled SQLite in the actual Node candidate | process.versions.sqlite = 3.54.0 |
| New assembled containers / fresh scans / PostgreSQL candidate execution | Not run: local Docker unavailable and only about 795 MiB free; remote push/CI approval withheld |

Local Node execution used the real pinned base binary/libraries and a private copy of the cached production build, **not a container or a fresh Node 26 npm/build**. A first local test incorrectly excluded Next's already-accepted exit 143; it was corrected without changing the CI shutdown policy. A synthetic pipe mock was corrected to consume its input, avoiding a test-only BrokenPipe exit. Final tests pass; failures were not ignored or reclassified as success.

New temporary inspection copies were inventoried with hashes and removed only after tests; original executables, historical evidence, databases and backups were not removed. No large Rust/frontend rebuild or Docker installation was attempted. The 200-test functional result is the prior remote baseline, not a rerun against these new images. Evidence, exact base package inventories and hashes remain ignored/private under `artifacts/milestone-11-security-remediation/`.

## Base digests versus assembled image digests

These are verified **base** index/platform digests. The assembled application/database digests require the next approved build; they are not fabricated from these bases.

| Base | Pinned index digest | Inspected linux/amd64 digest |
|---|---|---|
| gcr.io/distroless/cc-debian13:nonroot | `sha256:08476c244888bfb425bae1ae4dd7c17f093e972aab8426883d97f3c6f280f7c1` | `sha256:65af2e9e44e71de2df28245f6814301775d3ef04436b7d563f240d86fedf24aa` |
| gcr.io/distroless/nodejs26-debian13:nonroot | `sha256:9b945e92c1b5463a4b75f11e8417515e4bea7355f6adfd79ec220fbbe905dda8` | `sha256:070587ffceddf4e981a054782b6dbb63bb439b215a01f326a25a8d38b434fde4` |
| registry-1.docker.io/library/node:26.13.0-trixie-slim | `sha256:e4db4c914bf8ad241bd24804fc25d272d89bad97e68cdc8a1414862790a02c39` | `sha256:18f03e4b532a63baf09809fe8349c2171807f1818cb6c5230fb12aecd5beda65` |
| cgr.dev/chainguard/wolfi-base | `sha256:629d9acaceda7746cd9293365eb0d8e8fabf74e91830b05470cfbee20b53f447` | `sha256:76636d8fa771f6e1397c92f98919ae588c0210b7d3b8badf1e85caa40ba8f0c5` |

## Independent recovery: minimum human actions

Reviewed the existing provisioning report/local plan, not fresh Railway state or S3 credentials. Dedicated project `99648d2a-69a1-4d8c-98d0-7f2a26543c9c`, staging `ff0b2092-e42b-4a89-ae25-441ab95d92d8`, private Amsterdam bucket `76923b39-b3a7-4390-8a0b-783d8a8113e9` (`supa-agent-backups`). Four approved public fingerprints remain null and recovery custody/execution remains unverified/disabled. An empty provisioned bucket is not a recoverable backup.

1. Arrange the separate trusted key workstation and independent encrypted recovery-copy locations, and appoint the independent technical fingerprint reviewer using an authenticated channel.
2. The owner, outside chat/model terminals, generates/imports dedicated separate encryption and manifest-signing identities, protects private material, and independently verifies all four full primary/subkey fingerprints. Rehearse using independent copies; never place the only recovery copy on Ubuntu or Railway. No real key enrollment/generation happened here.
3. Approve exact public fingerprints and the scoped backup operation separately, including private S3 credential retrieval/storage, source archive/export and upload. Starting the stopped primary for a consistent new export requires its own approval; do not infer it from bucket/CI approval.
4. Encrypt with the reviewed authenticated OpenPGP profile, sign the minimal outer manifest, upload ciphertext to unique no-overwrite names and verify download/hash/signature/decryption plus a disposable restore with the independent recovery material. Compare source identity, schema, audit records, epoch/accounting namespace and quarantined hold. Never restore over the primary.

The policy remains 23 snapshots, no automatic deletion, $1/month monitoring policy (not a verified hard cap), no bucket object versioning/lock and bucket-wide credential risk unless narrower permissions are actually supported. Same-disk backups do not cover primary-disk loss. No private keys, backup credentials or real database exports/uploads were accessed or created in this task.

## Changed files and next boundary

Application Dockerfiles (3), new PostgreSQL Wolfi recipe/TLS entrypoint (2), exact build-context allowlist, CI harness/security tests, synthetic TLS tests, workflow test invocation, infrastructure contract tests and this report. One exact path per Conventional Commit on main. The Node runtime correction is a separate follow-up commit; existing commits are preserved, never amended/rebased/squashed.

**GO for separately authorized image-security validation; NO-GO for private Railway deployment.** Next requires exact-HEAD push/one remote build-execution-scan approval. Do not call baseline advisories resolved in assembled images until that evidence exists. If package pins/compatibility/scans fail, stop and investigate; never bypass a gate. Before deployment, additionally align the reviewed IaC with the validated PostgreSQL artifact and complete independent encrypted recovery or obtain an explicit, separately scoped decision for a strictly disposable environment. No such decision is assumed.

Primary PostgreSQL remains stopped and unchanged. Both historical reservations ($0.004048 and $0.003536), retained evaluations and pending ownership/review records are untouched. No agents, owner credentials, paid inference, trading, transfers, token launches, publishing or Railway resources were activated.
