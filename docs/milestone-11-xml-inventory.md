# XML installed-inventory validation correction

2026-11-03. Baseline: [CI 41671026398](https://github.com/pyco404/SupaAgent/actions/runs/41671026398), `b30c2fbe14e274b3b1d2c993503da54287fe7f31`. Replacement PostgreSQL initialized with UTF8/en_US.utf8/libc and passed the XML/TLS/non-root assertions, then the inventory validator rejected its input before migrations or API/worker startup.

## Exact mismatch and evidence limits

The old collector ran `apk info -v` with seven explicit package names. The real pinned APK 2.14.12 executable prints human-readable description/URL/size fields in this mode, prefixed by the **name without its version**:

```text
libxml2-16: ...
libxml2-16: ...
libxml2-16: 1494 KiB
```

The old regex required a line starting `libxml2-16-2.15.4-r2`, so no version matched. The generic exception did not establish that two XML libraries were installed. The [APK info manual](https://raw.githubusercontent.com/alpinelinux/apk-tools/v2.14.12/doc/apk-info.8.scd) describes a detailed human information command; its formatting is unsuitable as a complete installed identity/architecture/provider contract.

The original CI raw inventory was never printed or retained; it cannot be recovered from those logs. Reproduction used the hash-verified pinned public base's actual APK executable and reconstructed installed metadata for the exact requested package versions, including libxml2-16 2.15.4-r2 and libLLVM-19 19.1.7-r31 on x86_64. That command returned success but no version-bearing XML line, reproducing the rejection. These are explicitly metadata fixtures, not a reconstructed running PostgreSQL image or new installation.

## Focused correction

Only the CI harness, its tests and this report change. No Dockerfile/package pin, Rust/API logic, frontend, database, permissions or accounting changes.

The collector now uses the vendor base's actual BusyBox awk to project only `P:` (package), `V:` (version), `A:` (architecture) and `p:` (provides) from `/usr/lib/apk/db/installed`. It reads **all installed records**, not just a requested name or repository candidate, so an unexpected competing XML provider cannot be hidden by a narrow query. Descriptions, file lists, scripts and environment values are excluded.

Parsing requires complete valid identities, bounded fields, unique package names/fields/provider entries and a maximum 256 KiB/1,024 records. This intentionally rejects the old description format rather than inferring a version it never supplied. A CRLF-equivalent field projection is accepted; malformed or unknown formats fail closed.

XML validation requires exactly one related package/provider:

```text
P:libxml2-16
V:2.15.4-r2
A:x86_64
p:so-ver:libxml2.so.16=2.15.4-r2 so:libxml2.so.16=16
```

Its architecture must match the verified image architecture (amd64 maps to x86_64; arm64 maps to aarch64; unknown mappings fail). The package version must be a complete numeric upstream version plus APK revision, and the ABI-16/so-version declarations must match it exactly. Additional XML providers, unexpected XML-family packages, legacy ABI, duplicate/conflicting metadata and missing versions are rejected. Provider identities are never collapsed into a set of version numbers.

The existing security floor is preserved and enforced earlier: versions below 2.15.4 are rejected, including versions above the critical's 2.11 floor but below the seven later HIGH-advisory fixes. No advisory waiver, distribution-backport assumption or environment override is introduced. Build-time dynamic-linkage/legacy-library rejection, runtime XML/XPath and TLS checks, strict scans and all later authorization/migration/recovery checks remain unchanged.

## Bounded diagnostic record

Before XML validation, CI logs the candidate digest, installed package count and normalized XML and related ABI/runtime identities, including architecture and providers. Any unexpected XML provider is included. The record is limited to 32 KiB and scrubbed using the existing private-value/key redaction before output. Oversized records fail rather than silently dropping a competing provider.

If the input format itself is malformed, CI logs only a precise rejection code, byte count, SHA-256 and image digest, never the malformed prose. Coverage failures have distinct `xml_library_*` codes and an explicit `postgres_xml_inventory_rejected` stage. Thus format errors, duplicate records, provider conflicts, architecture mismatches, ABI/version mismatches and missing upstream fixes are distinguishable.

## Actual local validation

| Check | Result |
|---|---|
| Focused staging/security/XML regressions | 43 passed |
| Synthetic TLS wrapper regressions | 15 passed; fake OpenSSL only |
| Schema metadata comparisons | 4 passed |
| Backup guards | 5 passed |
| Infrastructure/container contracts | 10 passed |
| Secrets scanner self-tests | 15 passed |
| Syntax and diff whitespace | Passed |
| Actual pinned APK description command with matching metadata fixture | Reproduced version-less output and old parser failure |
| Actual vendor awk, 25-record metadata fixture | Parsed and verified the single ABI-16 XML provider |
| Actual vendor awk, complete 60-record metadata fixture | Parsed and verified; 19 actual base records plus 41 recorded solver-plan/vendor-index records |

This is 92 automated checks plus native tool-format reproduction. Regression cases cover valid metadata, unexpected/prose formatting, missing fields/versions, unknown version syntax, two providers with the same or differing versions, duplicate/conflicting records/fields, duplicate provides, vulnerable versions, legacy packages/ABI, provider-version mismatch, architecture mismatch/noarch, oversized input/diagnostics and exclusion of malformed private prose.

No packages or PostgreSQL services were installed/started during reproduction. Only an owned temporary public-base/fixture directory was written; it was inventoried and removed. About 738 MiB remains available; no large local build was attempted. Evidence remains private/ignored under `artifacts/milestone-11-security-remediation/xml-*`.

## Remaining validation and advisories

No new CI dispatch, container build, scan or image digest is claimed. The previous native PostgreSQL 18 suite passed 200 Rust tests and all 13 migrations/restoration/recovery. Replacement-container migrations, restricted API/worker startup, authentication, zero-worker-cycle assertions, restart/shutdown and restoration still need the next separately approved run after this parser correction. The existing workflow retains all those checks and fresh scans.

Latest actual image findings remain zero HIGH/CRITICAL and zero secrets. Frontend retains 41 MEDIUM/8 LOW; API and worker each 36 MEDIUM/8 LOW. PostgreSQL retains the unsuppressed gosu findings CVE-2026-61801 (MEDIUM, moby/sys/user v0.1.0, reported fix v0.4.1) and CVE-2026-39824 (UNKNOWN, x/sys v0.1.0, reported fix v0.44.0). Nothing in this fix modifies packages or claims those findings resolved.

**GO for one separately approved corrected CI run; NO-GO for Railway staging until the full run and outstanding security/recovery decisions pass.** Approval must cover the exact new HEAD's push and one dispatch. No automatic reruns or security-gate bypasses.

Primary PostgreSQL remains stopped. No financial reservations, retained evaluations, ownership records or protected state were accessed or modified. Historical $0.004048 and $0.003536 holds remain unchanged. No agents, paid inference, transfers, publishing, cloud resources or deployment. Each of the three changed files receives its own Conventional Commit on main; no push is authorized by this report.
