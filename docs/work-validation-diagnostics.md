# Rejected work diagnostics

Implemented and validated on 2 November 2026. No external inference, primary startup/migration, paid retry, financial transfer, deployment or push occurred.

## What the original evidence establishes

The consumed scope `0453528bc228c83e6137eabd53ebeb4a7b98a198e7e6937e397ece240a51a6f0` produced one generation with 614 input and 469 output tokens. Its failed session recorded `invalid_input`, cost 296 microUSD ($0.000296), no accepted report and no revenue. The job remains created/unpaid at revision 0.

The original OpenAI transport parsed structured output into a `WorkProposal`; the runtime then checked its fields, evidence, permissions and local tools. JSON/schema parsing errors were mapped to `provider_failed`, while multiple downstream checks returned `invalid_input`. The retained code therefore points to downstream domain validation, most plausibly a `WorkProposal::validate` rule, rather than identifying a particular bad citation or claim. There is no retained output with which to prove the exact rule.

`run_work` passed only a successfully validated report to settlement. Settlement retained reports only for `awaiting_approval`, so rejected candidates were discarded. Neither the journal nor the session contains the original text, generated citations, provider request ID or a specific field failure. They cannot be recovered from these records. No provider retrieval or new generation was attempted.

## Implemented design

Validation errors now carry an allowlisted stage, stable rejection code and a schema-relative JSON pointer. The existing checks are preserved: bounded text, exact approved tools, verified-memory membership, supported/incomplete/abstained status rules, literal source-line citations, verbatim observations, checked integer totals, and collection limits. Valid abstention and incomplete reports remain valid; diagnosing a rejection does not grant execution authority or revenue.

Codes distinguish:

- `work_invalid_json` and `work_invalid_structured_output`.
- `work_schema_mismatch`.
- `work_missing_evidence`, `work_unsupported_claim`, `work_invalid_citation` and `work_invalid_memory_reference`.
- `work_unauthorized_tool`.
- `work_incomplete`, `work_invalid_status`, `work_bounds_exceeded`, `work_invalid_text` and `work_invalid_total`.
- `work_permission_changed` and `work_domain_failure` for other execution/domain failures.

Serde schema failures use the already-cached, pinned `serde_path_to_error` 0.1.20 dependency. Error messages and unexpected property names are never copied into diagnostics. JSON pointer components are restricted to known schema fields and bounded numeric indices. Both OpenAI and Anthropic use the same work-proposal decoder; provider usage remains available when decoding fails.

The OpenAI Responses transport now also preserves safe generation HTTP status, recognized `req_` request identifiers, completion/stage timestamps and bounded failure classification through the existing `ProviderDiagnostic` abstraction. It does not retain headers, request bodies, raw response prose or hidden reasoning, and adds no retry.

### Private diagnostic record

The new record contains provider/model, agent, job/evaluation and request UUIDs, input hash, response schema/version, retention policy, rejection stage/code/path, optional structural output snapshot, usage, usage-based estimated cost (or conservative accounting when uncertain), outcome, recognized provider request ID, and creation/expiry timestamps.

It is stored as a private JSON extension inside the existing `live_sessions.data` row, in the same transaction as failed-session settlement and cost reconciliation. No migration is required. Ordinary `LiveSession` projections omit the extension. Subsequent session saves preserve it; recovery does not erase failed records. Existing uncertain-charge fencing and duplicate protection are unchanged.

Owner-only retrieval:

```text
GET /agents/{agent_uuid}/live-work/diagnostics/{request_uuid}
```

The API requires a server-authenticated owner role plus current ownership of the exact agent. Persistence rechecks ownership in SQL and binds both agent and request IDs. Public credentials, viewers, operators and unrelated owners are denied. Responses have `Cache-Control: no-store`. Nothing adds this resource to public frontend projections or LLM tools.

### Retention and redaction policy

No policy approving retention of model prose/private input exists. The enforced `structure_only_v1` policy therefore retains **no text values**, source excerpts, customer data, prompts, memory contents, arbitrary unknown field names, credentials or hidden reasoning. It records at most 32 structural entries (allowlisted field paths, JSON types and lengths), with an 8 KiB maximum complete diagnostic record. An invalid-JSON response has a specific code but no retained raw bytes.

This intentionally does not provide a verbatim rejected response for human review. Any future prose-retention policy needs separate review and approval; it cannot be enabled by model output, frontend parameters or an environment flag.

Each diagnostic expires after seven days and becomes unreadable immediately at expiry. Expired extensions are purged on subsequent failed-work settlements for that agent, or through the explicit owner-scoped `Store::purge_work_diagnostics` maintenance method. Purging removes only the diagnostic extension, never sessions, audit records, budgets, costs or reservations. **No always-on janitor was activated:** on an idle/stopped application, physical erasure requires maintenance; database backups can retain older copies under their own approved retention policy. This limitation must be resolved operationally before claiming guaranteed physical deletion at a deadline.

## Actual validation

| Check | Result |
| --- | --- |
| Rust workspace, all targets, locked/offline | 98 passed, 0 failed; 101 opt-in ignored |
| Provider suite, including ignored loopback mocks | 45 passed, 0 failed |
| Selected disposable PostgreSQL integration suites | 33 passed, 0 failed |
| Evaluation runner / scoped launcher regressions | 10 / 8 passed |
| Secrets scanner self-tests | 15 passed |
| Formatting and workspace Clippy, warnings denied | Passed |
| Frontend lint and type checking | Passed; no frontend source changes |

The unit fixture matrix checks fourteen representative field failures plus missing-information/external-research requirements, valid abstention, schema decoding, unknown-key redaction, sensitive-value removal and snapshot bounds. Local HTTP mocks cover malformed output JSON, schema rejection, usage retention and request-ID preservation without contacting a provider.

The new PostgreSQL/API rehearsal runs eight synthetic failed sessions: unsupported observation, forbidden tool, missing evidence, invalid citation line, sensitive output, wrong schema type, invalid JSON and insufficient work. Each records the expected code/path, safe correlation metadata and 35 microUSD of **mock usage accounting**. All remain failed; their jobs remain unpaid and earn nothing. These eight fixtures total 280 synthetic microUSD in a separate disposable budget, not the completed evaluation's budget.

The test denies private diagnostics to public, invalid, viewer, operator and unrelated-owner credentials; checks no-store responses and omission from ordinary session projections; reconnects/recoveries without losing failure state; refuses duplicate invocations; and tests diagnostic expiry/purge without changing cost accounting. Existing suites cover concurrent budgets, unknown charges, leases, duplicate receipts, financial limits, owner boundaries and wallet locking.

One initial disposable setup approval review timed out; the permitted single retry succeeded. No protected action was performed by the timed-out request. During implementation, compilation caught a moved fixture string and a test-only call to a private helper; both were corrected before the final passing runs.

## Protected state and limitations

Read-only before/after fingerprints of **all 38 public tables** in the completed evaluation database match. The original request journal and saved paid-result file hashes match. Primary and historical accounting evidence hashes match. The completed $0.000296 cost, $0.005 ceiling, failed session, consumed one-request allowance and unpaid job are unchanged. The historical $0.004048 hold and Claude $0.003536 unknown reservation were not settled, released or modified.

Only a fresh, socket-only disposable cluster and synthetic identities/budgets were used for integration testing; that cluster was stopped afterward. The primary PostgreSQL cluster remains stopped. No real owner credentials or agents were activated. No consumed scope was replayed.

Private sanitized evidence remains ignored under `artifacts/work-validation-diagnostics/`. The code change does not backfill a diagnostic for the original attempt, reconstruct lost text, certify model interpretations as true, or authorize another paid request. A process crash before response validation/settlement can still leave an unknown reserved session without a candidate snapshot; existing lease and replay fences conservatively protect its accounting.

The Rust dependency rebuild reduced free storage to approximately 1 GiB. No protected files, earlier artifacts or databases were deleted. Further large builds need capacity review or separately approved cleanup.

All changed files receive separate Conventional Commits on `main`, with hook-enforced staging checks and secrets scanning. The complete main-history and reachable-history secrets audits are run after those commits. No push is authorized.
