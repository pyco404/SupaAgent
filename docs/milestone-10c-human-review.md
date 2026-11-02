# Milestone 10C — Semantic assessment and pending human closure

## Status

The retained GPT-6 Luna proposal is suitable for **evaluation-only acceptance**
according to the coding agent's assessment below. This is not a human decision,
owner attestation, customer delivery, commercial completion or earned revenue.

**No review decision was recorded.** Inspection found that the existing approval
workflow cannot review work evaluations. Human approval and a dedicated
authenticated recording mechanism are still required. No owner was impersonated,
credentials activated, provider called, database mutated or primary service started.

## Exact retained evidence

The unchanged decoded model report is retained locally in:

`artifacts/milestone-10c/evaluation/gpt-6-luna-work-result.json`.

Its original session/journal and usage evidence remain in:

`artifacts/milestone-10/live-requests/3c3e5a2f689f6af3347b78c9c19f639842ae6ec1472aea61ace5c853287471ba.jsonl`.

This is the retained structured proposal, not a reconstruction of raw provider
response bytes or hidden model reasoning.

Exact model summary:

> Supplied documents describe different recovery trade-offs: replication reduces downtime but copies accidental deletion; offline snapshots preserve earlier state but need a tested restore. Neither provides measured recovery times.

Exact top-level model interpretation:

> The supplied comparison presents replication as reducing downtime but carrying deletion risk, while offline snapshots preserve earlier state but require a tested restore. It provides no measured recovery times, so comparative recovery performance cannot be quantified.

All three citations refer to the exact approved `material` input:

| Line | Exact supplied quote | Semantic assessment |
| --- | --- | --- |
| 1 | Document A: Replication reduces downtime but copies accidental deletion. | The observation is verbatim; interpreting deletion copying as propagation preserves the supplied meaning. |
| 2 | Document B: Offline snapshots preserve earlier state but need a tested restore. | The observation and interpretation preserve both earlier-state retention and the restore-testing caveat. |
| 3 | Neither document contains measured recovery times. | The report correctly avoids numerical recovery-time comparisons. No such measurements appear in the supplied text. |

The report uses only `inspect_input`, `extract_evidence` and `check_requirements`.
Memory references, totals, proposed steps and missing-information arrays are empty.
The complete report, including every finding interpretation, is in the unchanged
retained JSON and the private proposed-review packet.

## Semantic assessment — recommendation only

- Accuracy: faithful to the supplied statements; no invented measurements or
  independent verification claims were identified.
- Completeness: covers all three statements and the requested comparison. Choosing
  a winning strategy or estimating recovery times was not required. Missing
  measurements are disclosed in the summary and interpretation.
- Relevance: focuses on downtime, deletion propagation, retained earlier state and
  tested restoration. No unrelated or financial activity is proposed.
- Citation support: all three quotes exactly match their numbered source lines;
  each observation is a contiguous verbatim span within its quote.

This assessment establishes fidelity to a synthetic supplied-document task. It
does not establish the empirical truth of those documents, validate an actual
backup system or demonstrate commercial success. Quantitative strategy selection
would require additional evidence. Human semantic acceptance remains pending.

## Existing authenticated workflow findings

The existing owner-only `GET /agents/{id}/live-work` endpoint and terminal work
panel can display saved reports, evidence, usage and costs. The work panel contains
no semantic acceptance/rejection action. The main website was not repointed to the
isolated evaluation database, and no authenticated browser presentation is claimed.

`POST /agents/{id}/live/{session}/approval` is a separate **memory-tool approval**
workflow. The API requires an authenticated owner and `confirm_review`; the store
then explicitly rejects any session where `s.work.is_some()`. Its audit operations
are `approve_tools`/`decline_tools`, not human work acceptance. It cannot be used
to close this evaluation without changing the meaning of the existing safeguard.

Additionally, the original paid consent expired at **2 November 2026,
16:04:37.313950 UTC**. Renewing it is neither needed nor appropriate for a
historical semantic review. The pinned one-shot evaluation API deliberately refuses
startup after a session exists or its budget has been consumed. It must not be
restarted as if this were a fresh inference session.

Consequently there is **no valid executable semantic-review command today**.
No approval POST, direct database update or tool-approval workaround was attempted.
Read-only administrative inspection is not presented as an authenticated human
owner workflow.

Relevant implementation:

- `crates/api/src/live_work.rs`: owner-only work presentation and diagnostics.
- `reference-web/app.js`: report/evidence presentation without work review controls.
- `crates/api/src/controlled.rs`: memory-tool approval route and principal checks.
- `crates/persistence/src/controlled.rs`: explicit work-session and expiry rejection.
- `crates/api/examples/openai_work_10c_api.rs`: consumed-session startup fence.

## Exact decision prepared for human consideration

Private packet: `artifacts/milestone-10c/human-review/review-proposal.json`.

It is explicitly marked `proposal_only_not_a_human_decision`, with
`authenticated_reviewer: null`, `human_decision: null`,
`human_decision_recorded: false`, and `execution_available: false`.

| Binding | Value |
| --- | --- |
| Generation scope | `3c3e5a2f689f6af3347b78c9c19f639842ae6ec1472aea61ace5c853287471ba` |
| Session | `41316d3e-40b8-4b7d-8605-83db7aab36b8` |
| Agent | `82c254ff-4659-4404-94c4-c497565567e6` |
| Job | `7bf3067c-ea30-4557-8c70-c06aeb1c429b` |
| Input hash | `613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e` |
| Canonical report SHA-256 | `01fcfc0576fb7d8cebfc09666c060eb1bff5c85a08b2682d2b17842cfe5abc42` |
| Existing disposable owner principal | `acede749-746b-4541-b178-0085434dcd39` |
| Proposed decision | `accept_evaluation_only` |

Report hashing uses UTF-8 JSON with sorted keys, comma/colon separators and ASCII
escaping. It is an integrity binding, not proof of human identity or approval.
The disposable owner UUID is not evidence that KI Daniel has authenticated as
that principal. No principal binding, grant or credential was created here.

Proposed approval wording:

> Accept session `41316d3e-40b8-4b7d-8605-83db7aab36b8`, bound to report SHA-256 `01fcfc0576fb7d8cebfc09666c060eb1bff5c85a08b2682d2b17842cfe5abc42`, as an evaluation-only result. This authorizes no customer delivery, job completion, revenue recognition, additional inference, financial mutation or agent activation.

This wording is prepared for review; it is **not an available API request or a
recorded decision**. A future recorder must derive the reviewer from authenticated
server-side identity, verify current ownership, bind the exact session/report,
reject duplicate/stale decisions, and append an immutable semantic-review event.
It must preserve original model output and accounting and keep paid consent,
customer delivery and financial transitions separate. Only then can an explicitly
approved human decision be recorded through that mechanism.

## Unchanged state and non-paid checks

Read-only SQL inspection confirmed:

- Session remains `awaiting_approval`, with one generation consumed and no new
  review event. Existing audit contains only `authorize` and `settle`.
- Usage remains **1,007 input / 391 output tokens**, no cached or cache-write tokens.
  Recorded estimated cost remains **$0.000297**.
- Independent budget remains **$0.005**, committed **$0.000297**; remaining capacity
  **$0.004703** does not grant another request allowance.
- Job remains `created`, revision 0, unpaid, received 0 and earned 0. Economic
  entries remain 0. Agent remains paused; scheduling authorizations remain 0.
- Journal, original model-result file, paid result and all current evaluation
  tables were unchanged before/after inspection. Table fingerprint:
  `06193a74141e9ebb405cb917e754b89e4d8a038cafc2bdcac0dfad233c644373`.
- The previous evaluation's 38 tables match their historical fingerprint and
  protected primary/accounting hashes match the previous baseline. The primary
  remains stopped; the $0.004048 historical hold and $0.003536 Claude reservation
  remain unchanged.

| Non-paid check | Result |
| --- | --- |
| Retained work-provider test binary, including mocked schema/evidence matrix | 5 passed |
| Scope/launcher/storage regressions | 13 passed |
| Evaluation runner/replay regressions | 10 passed |
| Retained exact-scope/locked/one-shot Rust gate tests | 3 passed |
| Actual retained report/source comparisons | Three exact citations and observations passed |
| Existing review-route exclusion | Source-inspected; not a live owner action |
| Read-only database/file preservation | Passed |

The tests used existing binaries or network-free Python fixtures. No deleted Rust
artifacts were rebuilt and no external model request was made. No new review
endpoint or authenticated browser flow was tested or implemented.

Only this sanitized report is committed, following the one-file Conventional Commit
workflow on `main`. Private packets and inspection scripts remain ignored and are
not a replacement for immutable application audit records. Unrelated logo-file
deletions remain unstaged. No GitHub push, deployment or financial action occurred.

## Remaining closure requirements

1. Implement a dedicated owner-authenticated, append-only evaluation review path;
   the existing memory-tool route must continue to reject work sessions.
2. Present this exact report through that read-only/authenticated review context,
   without re-enabling generation or repointing the primary frontend implicitly.
3. Obtain the human decision and authenticate the actual authorized reviewer.
4. Record only evaluation acceptance/rejection, preserve all source/cost records,
   and demonstrate that job/payment/revenue/lifecycle state does not change.

Until these conditions are met, human closure is blocked and the persisted state
correctly remains `awaiting_approval`.
