# Milestone 10C — Disk recovery and fresh GPT work readiness

## Result and boundaries

A fresh disposable GPT-6 Luna evaluation is prepared. No paid generation occurred.
The primary PostgreSQL cluster stayed stopped; no migration, historical accounting
change, persistent worker, external mutation, deployment or push occurred. The
approved frontend and unrelated logo deletions were left unchanged.

## Controlled disk recovery

Initial available space: **731,856,896 bytes (697.95 MiB)**. Cleanup reused the
Milestone 8D classification, manifest, Cargo lock, process inspection, exact-name
unlink journal and protected-content verification method. No unrestricted Cargo
clean or recursive deletion ran.

Only fixed regular files under `target/claude-validation/debug/deps` and
`target/claude-validation/debug/examples` were removed:

| Class | Files |
| --- | ---: |
| Cargo dependency information (`.d`) | 1,404 |
| Rust static libraries (`.rlib`) | 616 |
| Compiler metadata (`.rmeta`) | 1,219 |
| **Total** | **3,239** |

Every executable, hashed test executable and shared library was retained. No
`build/`, `.fingerprint/`, source, registry cache, Git metadata, database, backup,
private credential, report or historical evidence was deleted. Candidate checks
verified Cargo names, regular-file types, signatures, hashes, inode/device, owner,
mode, timestamps, allocated blocks and hard links. Tracked candidates were denied.

The first pass removed 603 unpaired metadata files while retaining the dependencies
needed to build/test the new helper. The second pass removed 2,636 intermediates
only after that helper was built and rehearsed. No new build is required to use the
retained pinned executable.

Manifest SHA-256 values:

- Metadata pass: `5c1004d680e5dd0fd092010b0f601a19fb3a8b809696c9a5734653f97b75ffdf`.
- Final pass: `a8ce2ce417e0345df734927db1d2df15bf25dda36e78a7d1920fa48f8a7faf4a`.

The initial guard stopped before deletion when comparing the sandbox's synthetic
read-only `.aws` directory with its host mount view (inode, device and directory
mode differ). A full content comparison established that no protected file changed.
The exact mount-view discrepancy was recorded; no permissions were altered. Final
preparation and execution used the same host view.

The metadata pass verified 328,219 protected entries. The final pass verified
325,626 protected entries before and after deletion. Final manifest preparation
reused previously verified content hashes only when complete identity/size/mode/
mtime metadata matched; deletion verification independently rehashed every protected
file before and after. All removal journals match their exact manifests. Host
process visibility is limited for inaccessible system processes; Cargo's exclusive
lock was held throughout each cleanup.

| Capacity measurement | Bytes | GiB |
| --- | ---: | ---: |
| Observed combined increase during the two unlink operations | 2,279,174,144 | 2.123 |
| Available after final verification/probes | 2,530,037,760 | 2.356 |
| Net available-space gain from the initial sample | 1,798,180,864 | 1.675 |

New private audit snapshots, the fresh database and retained new helper binaries
consume part of the reclaimed space. Filesystem activity can also affect observed
capacity; manifest block estimates and net free-space measurements are distinct.

The remaining capacity is adequate for this bounded evaluation, not the earlier
10 GiB primary-backup/preflight gate or unrestricted compilation. The new launcher
requires at least 1 GiB available before startup/dispatch. Future rebuilds require
a capacity check and must not silently refill the disk during database work.

Fourteen retained programs passed matching before/after fail-closed loader probes,
with empty configuration, no credentials, no database URL and no operational
arguments. Core API/disabled worker, migration helpers, original eight named helper
programs and the new scoped API loaded and exited with their expected guard codes.
These are startup-guard checks, not configured-service health claims. The complete
preservation snapshots also cover all other retained binaries.

## Fresh independent scope

Plan: `config/openai-work-evaluation-10c.json`.

Scope SHA-256:

`3c3e5a2f689f6af3347b78c9c19f639842ae6ec1472aea61ace5c853287471ba`

| Binding | Value |
| --- | --- |
| Provider / model | OpenAI / `gpt-6-luna` |
| Project | `proj_npsyg93Ofk7ivKheGa8IDw5q` |
| Database | `supa_m10a_8c6356653dc2_test` |
| Socket-only cluster | `/tmp/supa-m10a-pg-8c6356653dc2/data`, port 55681 |
| System identifier | `7703596362529484555` |
| API | `http://127.0.0.1:55682` |
| Paused test agent | `82c254ff-4659-4404-94c4-c497565567e6` |
| Unpaid job | `7bf3067c-ea30-4557-8c70-c06aeb1c429b` |
| Test owner | `acede749-746b-4541-b178-0085434dcd39` |
| Independent budget | `3248e1c5-b306-4806-825a-f9b0a79d8f22` |
| Fixed one-request nonce | `41316d3e-40b8-4b7d-8605-83db7aab36b8` |
| Isolated accounting label | `79c1d3b9-6ed4-4587-a818-af3242de4aba` |
| Job expiration | **3 November 2026, 15:28:20.554261 UTC / 16:28:20.554261 WAT** |
| Budget / committed | **$0.005 / $0** |
| Maximum requests / estimated request ceiling | **1 / $0.004** |
| Maximum input / output tokens | **24,192 / 1,024** |
| Conservative future reservation | **$0.003536**, not reserved or spent |

No application epoch was activated. All 12 migrations ran only in the fresh empty
disposable database. The test owner is disposable, not real owner onboarding. The
budget is not paid consent. New agent/job/owner/service/request/budget/accounting
identifiers are disjoint from the consumed evaluation. Existing scope files and
journals were preserved.

Task: **Compare the supplied recovery strategies**, using only:

```text
Document A: Replication reduces downtime but copies accidental deletion.
Document B: Offline snapshots preserve earlier state but need a tested restore.
Neither document contains measured recovery times.
```

Canonical input hash is unchanged:
`613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`.
Approved pure local tools remain `inspect_input`, `extract_evidence`, and
`check_requirements`. Memory use and external research are disabled. Claims need
exact evidence references. Unsupported recovery-time claims are rejected; useful
model output cannot turn an unpaid job into earned revenue.

The scope binds the revised provider-schema and Rust validator source hashes. The
launcher also pins the retained executable hash and rejects changed implementation,
wrong scope, expired jobs, consumed allowances and low disk space. No automatic
retry or silent expiration extension was added. The private diagnostic route is
bound to this exact test agent/nonce and still enforces owner access in Rust.

## Model and account evidence

One generation-free metadata GET returned **HTTP 200** for `gpt-6-luna` with the
intended project header. Keys and authentication headers were not printed. This
proves metadata access, not current billing credit, generation entitlement or
available shared rate capacity. No account-admin billing evidence was available;
previous operator billing/limits reports are not a fresh balance observation.
Confirm the current project spending restrictions before paid authorization.

Official [model documentation](https://developers.openai.com/api/docs/models/gpt-6-luna)
continues to list Structured Outputs. [Standard pricing](https://developers.openai.com/api/docs/pricing)
is $0.10 input, $0.01 cached input, $0.125 30-minute cache write and $0.50 output
per million tokens, matching current accounting. The conservative bound prices all
24,192 input tokens at the cache-write rate plus 1,024 output tokens: 3,536 microUSD,
below the 4,000 microUSD request ceiling. No hosted tools, regional processing or
automatic provider fallback are enabled. Live acceptance/quality of the revised
schema remains unverified until a separately authorized generation.

## Non-paid validation and fixes

| Check | Actual result |
| --- | --- |
| New scoped Rust API build, locked/offline | Passed |
| Exact-scope/expired-consent/one-shot/locked gate Rust tests | 3 passed |
| Mock-provider tests including the 29 structured-work cases | 48 passed |
| Existing launcher regressions | 8 passed |
| Fresh-scope/launcher/storage regressions | 13 passed |
| Evaluation runner regressions | 10 passed |
| Secret-scanner self-tests | 15 passed |
| Formatting check | Passed |
| Disposable schema checksums | All 12 matched |
| Actual local HTTP/SQL rehearsal | Eight grouped checks passed |

The Rust application/schema implementation is unchanged from Milestone 10B, whose
workspace and PostgreSQL suites remain applicable. This milestone did not rebuild
the entire workspace after cleanup, which would recreate the removed output.
No frontend changed and no browser/design result is claimed.

Actual local rehearsal verified authenticated health without an application epoch,
owner-only exact unpaid-job reads, model/project/budget policy, denied financial
write privileges, public/cross-agent rejection, private diagnostic access, exact
paid consent blocked before dispatch, and locked restart persistence. Public-table
fingerprints remained unchanged; no session, reservation, generation, economic
entry or revenue was created. New API and cluster were stopped afterward.

The initial fresh API startup found a Python default argument that captured the old
database name despite a new pinned target. It failed to connect to that nonexistent
name inside the fresh cluster, before any dispatch. The launcher now resolves its
exact configured target at call time; regression tests cover this. Its old default
scope/database remain unchanged, and no arbitrary target CLI was introduced.

Private diagnostics remain structural, precise, size-bounded and owner-only; the
prior schema/grounding matrix covers rejected claims, references, tools and missing
fields. No rejected customer prose retention was enabled. The new scope has not
consumed a request or created a diagnostic record.

## Preserved financial state and remaining approval

The consumed evaluation's 38 public tables still match fingerprint
`bd2e1ac3f768170ba65e2dc2089f1adb0be013b5bba690dfc005fd2a861ae168`.
Original result/journal and protected hashes are unchanged. Its failed session,
unpaid job, one consumed request and estimated $0.000296 cost remain intact.
The historical $0.004048 quarantine hold and Claude $0.003536 unknown reservation
were not settled, released, refunded or reused. Primary PostgreSQL remains stopped.

Final scope/expiry checks, 14 executable hash/probe comparisons, original table
fingerprints and stopped-primary guards passed after cleanup. All scope, role,
provider and accounting validation evidence is local; no paid result is claimed.

Fresh explicit approval must name this new scope, model and limits, with current
project billing/spending permission confirmed. Recheck job validity, unused nonce,
private temporary credentials, zero-spent budget and disk space immediately before
execution. Test credentials can be renewed only within this unchanged job lifetime.
Loss of this disposable state or expiration requires a newly reviewed scope.

Protected command, **not executed**:

```bash
python3 scripts/openai-work-10c-api.py execute \
  --authorize-paid \
  --scope-sha256 3c3e5a2f689f6af3347b78c9c19f639842ae6ec1472aea61ace5c853287471ba
```

This command grants no standing permission. Ambiguous outcomes preserve their
reservation and stop without replay. No push, deployment or financial transaction
is authorized by preparation.

Private manifests, snapshots, probe records, credentials, database files and logs
remain ignored locally under `artifacts/milestone-10c/` or `/tmp`. Only source, the
public synthetic plan, tests and this sanitized report are committed, one file per
Conventional Commit on `main`. Existing unrelated logo deletions stay unstaged.
