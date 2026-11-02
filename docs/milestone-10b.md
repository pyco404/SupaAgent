# Milestone 10B — Structured work compatibility

## Outcome

The provider schema, Rust decoder and application validator now have a shared
deterministic compatibility matrix. No paid request was made. No completed
evaluation, consumed allowance, primary database or historical hold was changed.
The frontend and API contracts are unchanged.

## Findings and corrections

The nine required root fields and status values already agreed with Rust:
`interpretation`, `summary`, `status`, `tools`, `findings`, `totals`,
`proposed_steps`, `missing_information`, and `memory_ids`. Valid statuses remain
`supported`, `incomplete`, and `abstained`; `completed` is rejected.

There was a genuine context conflict: the live prompt required citation source
`material`, while application-generated deterministic findings used the offline
alias `supplied_material`. The provider now canonicalizes only that trusted
context copy. Original inputs and offline reports retain their existing values;
model proposals using the alias are still rejected. This is a plausible cause
of rejection, not a diagnosis of the original lost response.

The live schema now enumerates permitted citation sources, verified-memory IDs,
and application-checked numeric-total fields. It constrains positive line numbers,
tool counts, integer totals, and array counts. Empty memory/total collections use
zero-length array limits rather than invalid empty enums. Tool names represent
approved application capabilities, never provider-hosted tool execution.

Rust text limits count UTF-8 bytes, while earlier instructions said characters.
Instructions now specify bytes and explain that each observation must be a
contiguous verbatim span from a cited quote. Separate claims use separate findings;
synthesis belongs in interpretation. Trusted offline extracts are clipped at a
valid UTF-8 boundary to the live 300-byte quote limit without changing originals.

Nested missing fields previously identified their containing object. The decoder
now adds the missing, allowlisted schema field to the diagnostic pointer, for
example `/findings/0/evidence/0/quote`. It never retains free-form decoder messages
or arbitrary customer-supplied field names.

Strict objects, required properties and supported schema constraints follow
[OpenAI Structured Outputs documentation](https://developers.openai.com/api/docs/guides/structured-outputs).
[GPT-6 Luna documentation](https://developers.openai.com/api/docs/models/gpt-6-luna)
lists Structured Outputs support. These documentation and mock checks do not prove
that a live provider accepted this exact revised schema.

Application checks remain authoritative for source-specific line bounds, exact
quotes, supported observations, tool uniqueness/permissions, exact numeric totals,
memory eligibility, and conditional completeness/abstention rules. Schema-valid
prose is not proof of truth or a semantic quality score. No rejection rule was
relaxed and no financial permission was added.

## Deterministic scenarios

`config/work-structure-fixtures.json` supplies 29 synthetic cases, replayed both
through the mocked OpenAI wire/decoder/validator and through the Rust API/runtime
with disposable PostgreSQL state.

| Outcome | Cases |
| --- | --- |
| Accepted for review (4) | Three independently cited supported claims; explicit research abstention; incomplete supplied information; exact UTF-8 byte boundary |
| Invalid citation | Offline alias, unknown source, zero line, quote on wrong line |
| Unsupported claim | Observation not present in cited quote |
| Unauthorized tools | Forbidden, missing, or duplicate capability |
| Schema mismatch | Null/missing summary, missing nested quote, malformed memory UUID, negative line, non-string tool, fractional total, null findings |
| Other precise failures | Invalid status, absent evidence, invented total, unprovided memory, synthetic sensitive marker, blank/overlong text, unsupported empty report, incomplete report without missing information |

All 25 rejected cases assert their stable code and exact field path. Additional
tests cover verified memory versus hypotheses, checked integer total 12 versus
incorrect 13, 480-byte offline quote clipped to 300 bytes only in live context,
and 1,000-byte accepted versus 1,002-byte rejected summaries. The byte-boundary
case measures mechanical acceptance, not usefulness of repetitive text.

The API rehearsal creates each job through its normal owner-scoped endpoint,
executes one mocked evaluation, reads review/failed status, blocks duplicate
dispatch, and closes the unpaid job through the normal customer cancellation
command. All 29 jobs reach `cancelled`; none receives payment or earns revenue.
Four valid reports await review before cancellation; acceptance is not commercial
completion. The three-claim report verifies three exact citations. All agents stay
paused. All 29 sessions survive reconnect/recovery (the history projection remains
bounded to 20). Mock accounting totals 1,015 microUSD, with no provider charges.
The existing simulated economy lifecycle test separately verifies contract
completion and conserved liabilities.

Private diagnostic tests verify owner-only access, seven-day retention metadata,
8-KiB bounds, structural snapshots, safe known field paths, no customer wording,
private prompts, secret markers, or raw rejected prose. Viewer, operator,
cross-owner and public-service access is denied; responses use `no-store`.
Failed proposals retain accurate mock usage/cost and cannot earn revenue or retry
the same request. Unknown charges remain reserved across recovery.

## Actual validation

All Cargo commands used the existing `target/claude-validation` output with
incremental compilation disabled, locked dependencies and offline mode.

| Check | Result |
| --- | --- |
| `cargo test --workspace --all-targets --locked --offline` | 100 passed; 103 opt-in tests ignored |
| Provider library, `--include-ignored` | 48 passed, including loopback HTTP fixtures |
| API authorization, controlled inference, economy, live work and wallet-read permission suites | 25 passed |
| Runtime wallet and economic payment suites | 9 passed |
| `cargo fmt --all -- --check` | Passed |
| Workspace/all-target Clippy, `-D warnings` | Passed |
| Evaluation runner / disposable API launcher unit tests | 10 / 8 passed |
| Secrets scanner self-tests | 15 passed |

Opt-in integration tests ran against a newly initialized socket-only PostgreSQL 16
cluster on port 55680, with distinct child databases. The initial disposable
`initdb` attempt timed out during filesystem synchronization. It was left intact;
another fresh disposable fixture used `initdb --no-sync`. Runtime fsync remained
enabled. This tests application recovery, not hardware power-loss durability.
The new test cluster was stopped afterward. No primary startup or migration ran.

No UI changed, so no browser/design claim is made. Test logs and preservation
evidence remain local under ignored artifacts or `/tmp`; they are not committed.

## Preserved state and limitations

Read-only fingerprints of all 38 original evaluation tables match the pre-test
baseline, as do the original request journal, paid result and protected file
hashes. The completed evaluation remains failed/unpaid with one request consumed
and estimated cost $0.000296 against its original $0.005 budget. Its response was
not retained and cannot be reconstructed. The $0.003536 Claude reservation and
historical $0.004048 hold remain unchanged. Primary PostgreSQL remains stopped.

Free disk space after testing was approximately 700 MiB. More builds/restorations
need a capacity review; nothing was deleted for this milestone. No live provider
schema acceptance, model quality improvement or new paid result is claimed.

A fresh live evaluation can be prepared with new job/request identifiers, a new
independent budget, fresh expiration and a separately reviewed scope digest.
The consumed scope must not be reused. Actual paid execution still requires
explicit approval. No push, deployment, live scheduling or financial activity
was performed.

## Files and Git procedure

Changes are limited to this report, the shared synthetic fixture file,
`crates/providers/src/work.rs`, `crates/domain/src/work_diagnostics.rs`, and
`crates/api/tests/live_work.rs`. Each receives its own Conventional Commit on
`main`, with the secret gate and full one-file history audit. Existing unrelated
logo-file deletions remain unstaged.
