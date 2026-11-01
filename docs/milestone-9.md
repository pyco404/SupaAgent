# Milestone 9 — Bounded autonomous work and intelligence

Validated locally on 2026-11-01. This milestone implements offline work, not primary activation, external research, live-model intelligence or real economic independence.

## Implemented workflow

The existing authenticated services/jobs framework now produces `bounded_work_v1` results inside the backward-compatible `structured_report_v1` envelope. Existing service kinds and routes remain intact; no migration or new scheduler was introduced.

| Service | New application-owned analysis | Limits |
| --- | --- | --- |
| Summary / research report | Objective-ranked supplied-text extracts, exact source line references, literal requirement coverage | Supplied assertions remain unverified; no internet retrieval or semantic truth certification |
| Code analysis | Line-specific TODO, unwrap, expect, unsafe and eval text findings | Static patterns only; no execution, compilation or security certification |
| API/data report | Supplied JSON structure and checked integer-array totals | No API calls; fractional or overflowing totals are explicitly not exact |
| Delegation simulation | Three bounded proposed tasks and completion rules | Planning deliverable only; no child agents or external execution |

Plain supplied text still works. An optional structured input gives a specific objective and literal evidence requirements:

```json
{
  "objective": "Analyze storage resilience",
  "material": "Storage uses replicated volumes.\nRestore drills run weekly.",
  "requirements": ["replicated volumes", "Restore drills"],
  "require_external_research": false
}
```

An envelope requires both `objective` and `material`; ordinary API-report JSON containing only an `objective` business field remains data. Unknown envelope fields and unverified authority assertions are rejected. The existing 8,000-byte input ceiling remains, with at most eight requirements. Requirements are literal text checks, not semantic judgments. The response identifies a supporting line for each match.

## Planning, permissions and recovery

- Reuses the existing `Reasoner` abstraction and `Plan` proposal contract. Offline planning selects the task-specific application allowlist. Normal agent-cycle capabilities and tool registry are unchanged.
- Three read-only task steps, or four when evidence-backed memory is read. Agent-configured step limits are checked before claiming execution.
- At most two zero-cost provider requests per execution, further bounded by the agent's configured maximum. Only an incomplete tool plan gets one corrective attempt. Duplicate/unknown tools, injected lessons/evaluations and unauthorized actions are rejected without retry.
- Live policies, nonzero provider rates and provider usage metadata are rejected. No paid request or live provider experiment occurred.
- Existing operator scope is required before claiming and before every tool step. Active lifecycle and `evaluate_goal` capability are rechecked; memory additionally requires `read_memory`. The settlement transaction fences revoked/paused agents, stale execution tokens and expired leases.
- Ten-second execution timeout, thirty-second lease, bounded job attempts and existing explicit recovery controls remain. An interrupted lease is failed through existing recovery rather than blindly replayed. Step traces settle with the result; a process crash may leave only the claim/assessment until recovery, not a complete step-by-step trace.
- Application code computes findings and citations. Provider claims cannot establish delivery, grant permissions or change accounts.

## Memory and outcome reuse

Relevant verified observations are admitted only when their source events exist for the same agent. Hypotheses and irrelevant/unsupported observations are excluded. Retrieval is bounded lexical matching, not a semantic memory index.

A relevant verified observation or recent same-customer, same-service-kind failure moves requirement checks before analysis. A previous successful task can supply a tool sequence only after that sequence passes the current allowlist again. Old answers are never reused as evidence for new input. Up to eight recent outcomes inform this policy.

Actual memory IDs and source event IDs persist in the job assessment. Store projections expose this provenance only to scoped owners/operators. Customer job reads/lists and API mutation replies remove it. Customer reports contain usage counts, not private lessons or private references. Intelligence and Economy display operator references; Memory displays the corresponding memory IDs and source events.

This is explicit retrieval and policy adaptation, not model training or proof of general learning.

## Economic and accounting behavior

Reports record offered revenue, configured estimated cost, estimated margin, an unreserved operating-capital snapshot, observed failure factors and a nonbinding consider/review/reject recommendation. No probability of completion or real runway is fabricated. Capital-based recommendations are conservative snapshots and are not authorization decisions.

Existing application limits still enforce acceptance, reserves, spending ceilings, permissions and settlement. Configured simulated inference/infrastructure amounts are estimates, not provider invoices. Only a completed work contract earns revenue and posts those simulated delivery expenses. Incomplete or abstained reports remain inspectable as failed jobs with zero earned revenue; customer funds remain liabilities. No refund or real transfer is performed.

Historical legacy reports remain readable. Mainnet, paid inference, autonomous spending, trading, launches, X publishing and persistent worker activation were not enabled.

## Controlled demonstrations and measured evaluations

The new disposable PostgreSQL work test exercised eight jobs:

- Four completed deliveries; incomplete, abstained and forbidden work did not earn revenue.
- One incomplete plan repaired in exactly two requests; a forbidden proposal rejected after one.
- Concurrent duplicate execution produced one settlement; stale replay and cross-user execution failed.
- A real disposable completed-cycle memory changed preflight behavior without disclosing its lesson.
- A past successful code-review sequence was reused, with a fresh answer computed from supplied input.
- Customer job reads/lists hid private provenance; operator reads retained it.
- A reconnected store retained output hashes and all eight job records.

Final fixture accounting: **20,000 simulated revenue atoms, 4,400 simulated expense atoms, 20,000 customer-liability atoms**. Devnet earned revenue, real revenue, transfers and paid provider requests: **zero**. These are synthetic test records, not application achievements.

The reproducible domain matrix passed **8/8 scenarios**: objective ranking, missing evidence, retrieval abstention, exact totals, overflow refusal, static review, bounded planning and untrusted source instructions. Every retained quote was checked against its source line. On one explicit ranking fixture, the relevant fourth line was recovered while the legacy first-three-lines baseline missed it. This single fixture is not a general quality score.

Archived live-provider reports cover different contracts and do not provide a comparable `bounded_work_v1` benchmark. No like-for-like live-model advantage, accuracy score or new provider usage is claimed. A future comparison needs fresh separately approved inference and the same frozen fixtures.

## Validation evidence

Database tests used PostgreSQL 16.15 in a new loopback-only disposable cluster and fresh databases migrated through schema 12. They did not import historical primary records. This milestone does not add new PostgreSQL 18 validation evidence.

| Check | Result |
| --- | --- |
| Workspace Rust, all targets | 81 passed; 78 opt-in tests ignored in the default run |
| New disposable work rehearsal | 1 passed, eight jobs and privacy/accounting assertions |
| Economic API regression | 1 passed, including customer command-reply redaction |
| Mocked payment reconciliation | 1 passed; local RPC fixtures only |
| Runtime PostgreSQL regressions | 26 passed |
| API authorization regressions | 8 passed |
| Opt-in provider HTTP mocks | 7 passed; no real providers |
| Frontend lint/type check/build | Passed |
| Existing browser regressions | 144 passed across desktop/tablet/mobile projects |
| Actual Rust/PostgreSQL/Next.js browser workflow | Passed, all ten sections, three viewports, zero page errors, API restart persistence |
| Rust formatting, JavaScript syntax, whitespace | Passed |

The default ignored count includes the opt-in suites listed separately; do not add them twice. The other specialized opt-in epoch/worker/controlled-inference fixtures were not all rerun. No primary-state test was performed.

The initial sandboxed frontend build failed while parsing TypeScript subprocess output. The permitted local production build subsequently passed; lint/type checks also passed. No workaround changed application configuration.

Ignored local evidence:

- `artifacts/milestone-9/validated/economic-system-check.json`
- `artifacts/milestone-9/validated/economic-{desktop,tablet,mobile}.png`
- `artifacts/milestone-9/final/rea-evidence.json`
- `artifacts/milestone-9/final/rea-{desktop,mobile}.png`

REA exercised picker selection and report expansion, including keyboard activation on mobile. Successful scenarios: `ev_93d2e8520ab31bc7d108bf20eb27832efd1e01a41c50b8c9393873908a75c36b` and `ev_72da1881a40ac8ac9ab1b4244816645fefcad223d78fcf735c4f91dabb648a82`. Attachment event coverage was incomplete; action steps completed. Chromium viewports are emulated, not physical-device evidence. An initial scenario chose an earlier fixture owned by another identity and correctly could not load its private records; subsequent scenarios selected the current owner's fixture.

Impeccable context/hardening guidance informed the existing terminal details panels: progressive disclosure, wrapping report text, native summary controls and explicit limitations. No CSS, logo, palette, layout, route or animation architecture was redesigned.

## Primary safety and remaining limits

The primary PostgreSQL cluster remained stopped. Offline inspection preserved all 26 protected hashes, including reviewed backup/control/accounting evidence. Last live-verified primary state remains schema 8, prepared epoch `fd2f52e0-33fc-4c0d-893a-7b08b26ab748`, no approved activation. This is not a fresh primary SQL inspection.

The historical database remains unrecovered. Quarantined budget `ad91e475-f1da-495a-8247-139643d2d8b1`, its $0.10 ceiling and unresolved $0.004048 hold were not changed, adopted, refunded or reused.

Builds used a separate debug-stripped `/tmp/supa-m9-target`; retained validated project executables were not replaced. Free disk space at completion is approximately 9.9 GiB, below the earlier 10-GiB primary-preflight reserve. No cleanup or protected preflight was performed. Recheck capacity before future migration/backup approval.

Limitations: extractive synthesis, lexical retrieval/coverage, modest static patterns, no independent retrieval, no semantic contradiction guarantee, no arbitrary code execution, no continuous scheduling, and no live-model evaluation for this contract. Operational activation still requires the separately reviewed owner verification, independent recovery arrangements and explicit migration/provisioning/activation approvals. No real credentials or ownership grants were issued.

## Reproducing safely

Use only a separately initialized, disposable local PostgreSQL cluster. Never supply the primary connection. The work test refuses databases other than local `supa_m9_test` as its disposable parent and creates a uniquely named child database.

```sh
CARGO_TARGET_DIR=/tmp/supa-m9-target CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --workspace --all-targets --offline -j 2
# With TEST_DATABASE_URL set exclusively to the disposable local parent:
CARGO_TARGET_DIR=/tmp/supa-m9-target CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test -p supa-runtime --test work --offline -- --ignored --nocapture
# After building supa-api into that same isolated target and building web/:
SUPA_TEST_API_BINARY=/tmp/supa-m9-target/debug/supa-api node scripts/economic-smoke.mjs
```

The browser harness generates ephemeral synthetic principals/tokens in memory, keeps provider/worker/live flags off, performs only disposable simulated actions, and stops its owned services on exit. Its API-binary override avoids using an older retained executable. The primary cluster must remain stopped throughout.

## Source changes and Git

- `crates/domain/src/work.rs`: bounded input, evidence/analysis and delivery contract.
- `crates/domain/src/lib.rs`: exports the work contract.
- `crates/domain/tests/work_evaluation.rs`: reproducible eight-scenario matrix.
- `crates/providers/src/lib.rs`: offline task-specific tool proposals.
- `crates/runtime/src/economy.rs`: grounded orchestration, bounded repair, memory/outcome reuse and economics.
- `crates/runtime/tests/work.rs`: disposable work, privacy and accounting rehearsal.
- `crates/persistence/src/economy.rs`: delivery-aware settlement and scoped provenance projection.
- `crates/api/src/economy.rs`: truthful limits and scoped mutation responses.
- `crates/api/tests/economy.rs`: API provenance privacy assertions.
- `reference-web/app.js`: existing Economy panel renders work results and references.
- `reference-web/platform.js`: existing Intelligence/Memory views expose scoped task evidence.
- `scripts/economic-smoke.mjs`: clean-route/intro-aware full-stack work check and isolated binary override.
- This report.

Each file receives its own Conventional Commit directly on `main` through `scripts/commit-one`. Evidence, databases, credentials and build outputs remain untracked/ignored. Secrets scan and full one-file history audit are required after the commits. No push or deployment is authorized or performed.

Recommended next milestone: deepen the frozen supplied-input evaluation corpus and semantic grounding/revision checks, then conduct a separately authorized, strictly bounded live-model comparison. Keep operational cutover, recipient enrollment and paid execution as separate approvals.
