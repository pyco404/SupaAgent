# Anthropic error-handling remediation

Implemented 2 November 2026. No authenticated provider requests, paid inference, primary database operations, accounting reconciliation, frontend changes, deployment or push were performed.

## Findings and fixes

The previous adapter sent application JSON Schemas containing numeric bounds and array limits directly to Anthropic. Anthropic documents that these constraints are unsupported by structured outputs and can cause HTTP 400 responses. A new provider-only schema adapter removes unsupported wire constraints while describing them in field descriptions; existing Rust validation still enforces the original bounds. Mock tests verify both the transmitted schema and rejection of invalid output. This is a reproducible defect capable of causing the original failure, **not proof of its cause**: the original HTTP response was not retained. [Anthropic structured outputs](https://platform.claude.com/docs/en/build-with-claude/structured-outputs)

The existing Sonnet configuration also used `thinking: disabled`; the adapter now uses its required between-tools mode and low effort. Haiku retains disabled thinking. This does not authorize another model, request or budget. Thinking text is never persisted. [Anthropic error guidance](https://platform.claude.com/docs/en/api/errors)

Previously, settlement collapsed technical failure into `provider_charge_requires_review`. Live sessions now preserve optional, typed diagnostic history separately from that financial classification. Available evidence includes observation/header timestamps, HTTP status, bounded request/message IDs, allowlisted provider error types, model, response completeness, failure stage and numeric reported token counts. Validated priced usage remains separate from unverified reported counts. Raw error prose, response bodies, credentials and authentication headers are excluded. Anthropic identifies requests using the request-id header and request_id error field. [Anthropic errors](https://platform.claude.com/docs/en/api/errors)

Transport performs one attempt, with an eight-second connect timeout, ten-second read-stall timeout and 25-second request deadline. Runtime adds a 26-second outer deadline. Bounded response reads retain header evidence through timeouts, premature EOF and oversized bodies. Connection, timeout, HTTP, JSON, model, usage and output failures remain distinguishable. Complete responses with independently valid usage retain it even when output fails application validation. Zero-cost nullable metadata is accepted; unpriced cache durations and hosted tools still fail closed.

Diagnostics are written before settlement using existing session JSON, with no schema migration or historical backfill. A bounded, locked append rejects completed sessions and quarantined budgets. Metadata verification exposes observations without making an extra call. Shared invocation wrappers preserve diagnostics and existing offline/OpenAI compatibility.

## Accounting and recovery

There are no automatic provider retries. Unknown outcomes retain conservative reservations and block same-ID and fresh-ID replay. Provider errors and internal holds are not treated as confirmed charges. Existing accounting policy was preserved; this change does not add a reconciliation, release or refund operation.

The original $0.003536 reservation and historical $0.004048 hold were not modified. The reviewed scope, original journal and pre/postflight records remain byte-identical. Twenty-six protected primary, backup and historical evidence hashes match the baseline, and the primary cluster remains stopped. A final read-only connection to the original disposable evaluation socket was unavailable; it was not started. Therefore current database-row values were not independently re-read during this final check. Earlier records remain evidence, not a new confirmation of provider billing.

Only newly created test databases were written. Two PostgreSQL integration suites exercised controlled inference and live work, diagnostics across reconnect, interrupted execution, lease recovery, duplicate prevention and archived-budget rejection. Immediate shutdown/restart of the disposable cluster preserved session/budget fingerprints across nine test databases. That cluster was stopped afterward. The existing evaluation API was not restarted or repointed to this implementation.

## Validation

| Check | Result |
| --- | --- |
| Rust workspace, all targets, locked/offline | 91 passed, 0 failed, 98 opt-in tests ignored |
| Provider suite including loopback mock HTTP responses | 44 passed, 0 failed |
| Explicit disposable controlled/live-work integration | 13 passed, 0 failed |
| Explicit disposable accounting-quarantine integration | 1 passed, 0 failed |
| Disposable database crash/restart | 9 session/budget fingerprints preserved |
| Local Claude credential-script tests | 6 passed |
| Live-work evaluation-script tests | 9 passed |
| Secret-scanner self-tests | 15 passed |
| Formatting and Clippy, all targets with warnings denied | Passed |

The default workspace run does not execute every opt-in infrastructure suite. Relevant provider and accounting suites were explicitly executed as listed; remaining ignored suites are not claimed as passed. All model responses in this validation were synthetic. Evidence is retained locally under ignored `artifacts/anthropic-error-handling/`; artifacts and private database data are not committed.

## Changed areas

- Domain: bounded diagnostics and backwards-compatible optional live-session fields.
- Providers: Anthropic transport, schema normalization, parsing, observed metadata verification, invocation propagation and mock regressions.
- Persistence/runtime: durable diagnostic appends, separate technical failure code, controlled inference/work execution and conservative recovery.
- API/runtime fixtures: additive invocation/session fields and disposable recovery/accounting tests.

## Remaining uncertainty and approval boundaries

The original request's HTTP status, request ID, response and actual charge cannot be reconstructed from this fix. It remains unknown pending sufficient request-level provider evidence and separately authorized reconciliation. Neither dashboard totals nor reservations prove a charge or its absence.

A process crash before the adapter returns can still lose in-memory response headers or token evidence; durable intent, reservations and replay fencing remain. Invalid or incomplete usage cannot be priced reliably and remains subject to conservative review. No automatic retry is permissible for an ambiguous paid attempt.

Any future evaluation requires a separately reviewed scope and paid authorization; this remediation neither renews the original job nor changes its expiration. Primary database startup, migration, credential activation, agent activation, deployment and pushing remain separately protected.
