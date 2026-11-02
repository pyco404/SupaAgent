# First Claude evaluation — read-only reconciliation investigation

Investigated **2026-11-02**, Africa/Lagos. Database inspection completed at approximately **05:15 WAT**. No new provider request, generation attempt, database startup, accounting mutation, retry, credential change or deployment occurred. This report supplements, and does not overwrite, the original attempt/evidence.

## Conclusion

**The provider outcome remains indeterminate.** The retained records establish an authorized application attempt, successful model/workspace metadata validation, and conservative accounting of an uncertain result. They cannot establish whether Anthropic received or completed generation, rejected it before inference, or charged it. **$0.003536 is an unresolved internal hold, not a confirmed provider charge.** No authoritative per-request Anthropic usage/cost evidence has been supplied for comparison during this investigation.

## Verified records and timing

The already-running disposable database was queried with `default_transaction_read_only=on`, a five-second statement timeout and the exact private socket/database binding. No cluster was started or restarted. Database identity/system identifier matched the reviewed fixture. Its full saved session matches the original runner journal as decoded JSON; separate file hashes verify that the original evidence files were not modified.

| Evidence | Verified value |
| --- | --- |
| Application session / fixed nonce | `f80246de-acbe-416f-b6b3-98ecbf15e286` |
| Model / workspace | `claude-haiku-5-5` / `wrkspc_01FargC8PH6NDA25MHMRgz3F` |
| Session creation | 2026-11-01 **20:21:03.297616369 WAT** |
| Session finish | 2026-11-01 **20:21:05.147692103 WAT** |
| Recorded elapsed time | Approximately **1.850076 seconds** |
| Immutable application audit | `authorize` at 20:21:03.292475 WAT; original `settle` at 20:21:05.145909 WAT |
| Metadata / generation stage counters | **1 / 1** |
| Project/model metadata validation | Passed (`project_verified=true`) |
| Session status / safe failure code | `unknown` / `provider_charge_requires_review` |
| Token usage / accepted report / tool results | All absent |
| Provider HTTP status / request ID / message ID | Not retained |
| Independent budget | `3bae1525-7e83-4220-9682-d30d06cc06f2`: ceiling **$0.005**, committed hold **$0.003536** |

The original audit operation named `settle` records the application's November 1 transition into an unknown state. It is not proof of provider billing and was not created or changed by this investigation. There are still exactly two audit entries and one live session. The separate routine-provider `inference_calls` table has zero rows; that does not mean zero controlled-provider requests, which use `live_sessions`.

The application nonce is not an Anthropic request identifier. Successful metadata validation precedes generation and cannot establish the later POST's result. The generation stage is recorded **before** invoking the provider work method. That method can fail locally before dispatch, on HTTP status/transport, or on response/usage parsing; a counter alone cannot distinguish them.

The observed 1.85-second interval does not establish that the configured 25-second client or 26-second runtime timeout fired, nor does a short failure prove absence of a charge. The log contains a later `worker_heartbeat_fenced` / `stale_cycle` event at **2026-11-02 05:10:14.527691 WAT**, many hours after the attempt. It is API-process health evidence, not evidence of the original generation cause or an agent execution. No worker-health change was made here.

## Reproducible observability defects

Source tracing and three targeted existing tests establish diagnostic loss without recreating the paid attempt:

1. `crates/runtime/src/live_work.rs` records the generation stage before calling `provider.work`. `crates/providers/src/anthropic.rs` can reject a body over 16,000 bytes without dispatch. Thus this counter denotes dispatch intent, not confirmed network receipt. The existing local-mock test demonstrates pre-dispatch rejection and exactly one attempt on timeout.
2. `Anthropic::transport` returns a sanitized `provider_http_<status>` on non-success, but does not retain provider request identifiers or selected response metadata. The existing local-mock test reproduces distinct 401, 403, 429 and 500 errors with absent usage. The original attempt's HTTP status cannot be recovered from these synthetic tests.
3. `settle_live_session_bound` computes `uncertain` when generation has started without bounded priced usage, then replaces the underlying error with `provider_charge_requires_review`. Distinct provider HTTP/transport/parsing errors therefore collapse into the same stored financial-review code. Original cause is lost even when the adapter had a safe diagnostic.
4. The strict usage parser deliberately fails closed for missing/unknown usage or pricing dimensions. The existing pure unit test reproduces these cases. That safeguard is appropriate; discarding the distinction between usage uncertainty and report failure makes later recovery harder. It does not establish a particular parser defect caused this real attempt.

**What is not reproduced:** the real Anthropic response. There is no captured envelope, HTTP status or provider request ID. No specific request-contract, model-alias, billing, schema, timeout or usage-parser cause can be attributed to this attempt. Intentional refusal to guess prices must not be weakened merely to make a test pass.

Targeted validation (no database writes or external provider endpoints):

- `anthropic::tests::unknown_pricing_missing_usage_and_one_hour_cache_fail_closed`: passed.
- `anthropic::tests::timeout_is_single_attempt_and_oversized_input_never_dispatches`: passed with a local mock server.
- `anthropic::tests::metadata_and_http_errors_have_no_generation_or_secret_leak`: passed with local mocks.

No production implementation was modified. Proposed follow-up, separately reviewed before another live evaluation:

- Persist a small allowlisted diagnostic envelope: stage, safe error class, HTTP status, provider request/message identifiers when available, response-received time, exact returned model, stop reason and available usage. Exclude keys, headers carrying credentials, arbitrary error bodies and private prompt content.
- Store technical failure separately from `accounting_status=requires_review`; preserve the original safe error rather than replacing it with the financial-review code.
- Distinguish local validation, dispatch intent, response receipt, output validation and usage reconciliation. Crashes between send and receipt remain ambiguous; do not claim a receipt flag proves billing or add automatic retry.
- Preserve authoritative usage independently of content validation and retain bounded private diagnostic evidence for unknown fields, without accepting unverified pricing or model aliases. Review any response retention for privacy before implementation.
- Add mock-provider plus disposable-database regressions showing that HTTP rejection, pre-dispatch failure, ambiguous disconnect and malformed success remain distinguishable after restart, while holds and replay fences remain conservative.
- Use an explicit evidence-backed reconciliation procedure with separately authorized accounting transitions. No guessed refunds or blanket error-based release.

## Anthropic evidence needed and comparison rules

No account credentials, new admin requests or Console access were used. Earlier operator confirmation that billing capacity exists is not per-request cost evidence.

For workspace **`wrkspc_01FargC8PH6NDA25MHMRgz3F`**, inspect the provider's request/usage records around **2026-11-01 19:21:03–19:21:05 UTC (20:21:03–20:21:05 WAT)**. Supply only sanitized provider request/message ID, request timestamp, model, status/error category, input/output/cache token counts, and actual cost/currency or explicit billing disposition. Include the reporting interval/timezone and whether costs are delayed or rounded. Never send keys, authentication headers, private prompts or payment details.

Comparison must establish attribution to this exact workspace/model/time window and request, then reconcile usage with the original pricing and accounting evidence. Because no provider identifier was retained locally, timestamp/model agreement or an organization-wide balance change alone may be insufficient for unique attribution. A rounded $0.00 display, missing aggregate row or absence of a draft does not prove the attempt was free. A verified pre-inference rejection, verified completed response/usage, or otherwise attributable provider record could support a stronger conclusion; any hold adjustment still requires separate approval.

## Preserved scope, state and future authorization

Original scope remains **`b11a1cb723e32742e23988b897504dd524e026390e027a0def5398ae44eb931b`**. Full plan, journal, preflight and postflight file hashes are unchanged. All **26 protected primary/backup/accounting hashes** match both this investigation's baseline and the original paid-attempt baseline. Primary PostgreSQL remains stopped and unqueried; the historical **$0.004048 hold** is untouched and distinct from the new **$0.003536 hold**. Neither was released, refunded, reconciled, increased or otherwise altered.

The test agent remains paused. The original job remains created, revision zero, unpaid, with unchanged input, expiration and zero received/earned amounts. Cycles, actions, scheduling authorizations and economic entries remain zero. The job was still unexpired at 05:15 WAT, with about **8 hours 15 minutes** remaining until **2026-11-02 13:30:58.948063 WAT**. Its later expiration must not be silently extended.

**A new paid evaluation requires a separately reviewed scope and fresh explicit approval**, even if the old job remains valid. The prior one-attempt consent is exhausted, its two-minute execution window has elapsed, the immutable nonce/journal cannot be reused and the unknown session blocks its budget. A future proposal needs a new request identity, reviewed digest/limits and an independently approved budget or evidence-backed resolution of the existing hold; no such proposal or authorization was created here. Preserve the original attempt permanently. Any changed task/job expiry, adapter behavior or model configuration belongs in that new review.

The sole tracked change is this investigation report, committed individually with secrets/history audits. Ignored sanitized analysis remains local under `artifacts/claude-reconciliation/`. No paid call, inference retry, financial transaction, X publication, deployment or GitHub push occurred.
