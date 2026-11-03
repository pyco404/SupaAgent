# Focused container runtime and PostgreSQL corrections

2026-11-03. Baseline: [CI 41657190938](https://github.com/pyco404/SupaAgent/actions/runs/41657190938), commit `bb1f38ba386de9f2d353365e6e9984ba8bce7bba`. No subsequent run, push, deployment or primary database operation was performed for this correction.

## Exact root causes

1. **Process inspection:** Docker filters host `ps` output by container PID. `docker top -eo uid` omits that required field and failed after the frontend's internal and runner-side HTTP 200 health checks. The error was `Couldn't find PID field in ps output`. [Docker accepts host ps options](https://docs.docker.com/reference/cli/docker/container/top/); the corrected command is `docker top <owned-container> -eo pid,uid`.
2. **Package pin conflict, reproduced with the actual vendor solver:** the hash-verified pinned base already has `glibc-2.44=2.44-r13`. The requested `posix-libc-utils=2.43-r13` and its utility binary dependency require `glibc=2.43-r13`. The full first error, reproduced with exit code 2, is:

   ```text
   ERROR: unable to select packages:
     glibc-2.44-2.44-r13:
       breaks: posix-libc-utils-bin-2.43-r13[glibc=2.43-r13]
               posix-libc-utils-2.43-r13[glibc=2.43-r13]
       satisfies: world[glibc-2.44=2.44-r13]
   ```

   The old diagnostic kept only 40 trailing lines and discarded this first error. This was not an architecture mismatch or musl/Debian package mixture. The actual base's configured repository is `https://apk.cgr.dev/chainguard`; it was retained without alteration, additional repositories or trust bypasses. The official [glibc ABI guidance](https://support.chainguard.dev/hc/en-us/articles/46374554648859-Glibc-2-44-Release-Impact-on-apk-add-and-Chainguard-Images) also requires aligning base and package libc versions. A separate public Wolfi index comparison found identical checksums for all 19 installed base packages; package-name assumptions alone were not used as compatibility proof.
3. **Additional dependency trap discovered during reproduction:** correcting only glibc makes the solver succeed, but its generic LLVM provider selects `libLLVM 19.1.7-r8`, which depends on `libxml2.so.2` and selects `libxml2 2.13.6-r3`. The existing build-time legacy XML rejection would correctly stop that build. The vendor's `libLLVM-19 19.1.7-r31` uses `libxml2.so.16`; pinning it avoids the legacy copy while preserving PostgreSQL JIT functionality.

## Implemented changes

- `scripts/ci-staging-images.py`: one shared PID/UID inspection command for applications and PostgreSQL. Strict parsing rejects missing/invalid PIDs, malformed inventories, empty inventories and any root UID. Non-root verification is not replaced by a configured USER assertion.
- Build failures now retain sanitized output in numbered 6,000-character chunks, including the first solver error. Total evidence is bounded to 2 MiB; oversized logs explicitly flag truncation and preserve their beginning and end. Runtime diagnostic bounds remain unchanged. Redaction occurs before splitting; no raw private environment or inspect dump is emitted.
- PostgreSQL scanning now happens immediately after a successful candidate build, before a later runtime failure could erase the opportunity to collect security evidence. All severities, installed/fixed versions, digest and secret counts remain visible. The strict HIGH/CRITICAL/secret gate and independent XML fix checks remain.
- `deployment/postgres-wolfi.Dockerfile`: matches `glibc-2.44=2.44-r13`, `glibc-2.44-locales=2.44-r13` and `posix-libc-utils-2.44=2.44-r13`; pins `libLLVM-19=19.1.7-r31`. PostgreSQL remains 18.6-r7, XML remains `libxml2-16=2.15.4-r2`, OpenSSL remains 4.0.3-r8. Runtime inventory prints the additional ABI-critical packages.
- `scripts/test-staging-validation.py`: regressions cover the exact PID-bearing command, root/malformed process rejection, complete first-error retention, redaction/chunking/truncation, consistent package pins and retention of LOW/MEDIUM/HIGH/CRITICAL findings without relaxing the gate.

No application code, frontend assets, permissions, financial rules, SQL migrations or TLS wrapper behavior changed. Existing authentication, non-root, restart, shutdown, zero-worker-cycle, XML/XPath, TLS, migration-checksum, backup and full schema/data restoration checks remain in CI.

## Actual local evidence

The pinned public base's OCI manifest/layer hashes were checked before using its APK executable and libraries in a private temporary filesystem. Normal vendor signature verification accepted the repository index. Only index update and `apk --simulate add` were performed; no package install scripts or PostgreSQL services were executed. The old pin set returned exit 2. Matching libc alone returned 0 but selected legacy XML. The final exact Dockerfile pin set returned 0, selected `libLLVM-19 19.1.7-r31` and `libxml2-16 2.15.4-r2`, and selected no legacy XML package. Before/after hashes of the temporary installed-package database and world file match.

The simulated 304 MiB installed-size plan is not an installation or image build. The intermediate diagnostic experiment using `--no-network` also reported masked uncached packages; the definitive reproduction and corrected checks omit that flag and use the verified configured repository. Complete solver transcripts, exact pins, hashes and results are retained locally in ignored/private `artifacts/milestone-11-security-remediation/`.

| Check | Result |
|---|---|
| Focused CI security/process/evidence regressions | 34 passed |
| Synthetic PostgreSQL TLS wrapper tests | 15 passed; fake OpenSSL, no real keys |
| Metadata comparisons | 4 passed |
| Independent backup guards | 5 passed |
| Railway/container authoring contracts | 10 passed |
| Secret-scanner self-tests | 15 passed |
| Python compilation, bash syntax, diff whitespace | Passed |
| Original vendor APK resolution | Failed with the exact glibc conflict, as expected |
| Corrected exact Dockerfile resolution | Passed; no package installation; no legacy XML selected |

Docker is unavailable locally. About 750 MiB was available at the start; the temporary extracted base filesystem was removed after recording hashes, recovering about 25 MiB. Public index and diagnostic evidence remains private/local; about 728 MiB remains available. No large image, Rust or frontend build was attempted. A native/container PostgreSQL rerun would add little evidence without the assembled candidate, so the existing 200-test baseline is not misrepresented as a new candidate result.

## Security status and next authorized validation

The last actual application scans remain zero HIGH/CRITICAL and zero secrets: frontend has 41 MEDIUM/8 LOW, API and worker each 36 MEDIUM/8 LOW. No findings are suppressed. PostgreSQL's new assembled image has **not** been built, executed or scanned; its installed XML version, CVE-2026-6653 remediation, other findings and image digest remain unverified. Selecting a patched package in a successful solver simulation is not proof of remediation in a running image.

**GO for one separately approved corrected CI run; NO-GO for Railway deployment.** Request approval for the exact new HEAD before pushing/dispatching. That run must build/scan the actual candidate, check installed package versions and dynamic linkage, exercise XML/TLS and all migrations/restoration, verify actual non-root processes/authentication/restart/shutdown and worker zero cycles, and complete the 200-test native suite. Any new failure must be reported without automatic rerun or gate bypass.

Independent encrypted recovery remains pending verified public fingerprints, protected independent recovery copies and an approved real encryption/upload/download/disposable-restore drill. No credentials, recovery keys, resources or backups were accessed/generated/uploaded here. Primary PostgreSQL stays stopped; historical $0.004048 and $0.003536 holds remain unchanged.

Four paths changed, each committed separately on main. No history rewrite, push or CI dispatch is authorized by this report.
