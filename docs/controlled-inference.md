# Controlled GPT inference

Implemented 2026-10-29. The approved experience remains in `reference-web`; all ten destinations, visual assets, CSS, intro, portal, wheel, branches and hands are preserved. The six-crate architecture is unchanged.

## Read-only audit and official API verification

The existing provider already used Responses, strict `text.format` JSON Schema, disabled response storage, bounded bodies, a fixed HTTPS endpoint, sanitized status errors, and no transport retries. Existing persistent inference accounting separates provider budgets from simulated ledger expenses. Managed lifecycle scheduling deliberately selects the deterministic provider. Existing ownership/roles restrict private context; relevant-memory retrieval preserves hypothesis versus verified-observation classifications.

The missing piece was an explicitly authorized HTTP session independent of lifecycle scheduling, with persisted proposals and a second tool approval. Reusing a normal reasoning cycle would mix lifecycle activation and simulated expenses into a paid operator session. The new path reuses the provider transport, `InferencePolicy`, `TokenUsage`, memory retrieval, permission registry and cumulative `live_budgets` table without changing existing cycle behavior.

Official documentation checked before implementation:

- [Responses overview](https://developers.openai.com/api/reference/responses/overview)
- [Structured Outputs](https://developers.openai.com/api/docs/guides/structured-outputs)
- [Function calling](https://developers.openai.com/api/docs/guides/function-calling)
- [GPT-6 Luna](https://developers.openai.com/api/docs/models/gpt-6-luna)
- [GPT-6.1 Sol](https://developers.openai.com/api/docs/models/gpt-6.1-sol)
- [Standard pricing](https://developers.openai.com/api/docs/pricing?latest-pricing-expanded=true)

The documented identifiers support structured outputs and function calling. Supa uses structured **proposals**, not provider-hosted tool execution: local tools run only after separate review. The existing allowlist and standard short-context rates remain unchanged: Luna input/cached/write/output USD per million = 0.10/0.01/0.125/0.50; Sol = 2.00/0.10/2.50/10.00. Cached usage is counted once; reservations use the highest possible input rate and round upward. Regional/other pricing endpoints are not supported.

Documentation establishes an API contract, not this account's access. No ChatGPT subscription, configured key, model metadata response, or previous evaluation substitutes for fresh paid consent. Model metadata access does not prove permission or quota to generate. Exact response model attribution is checked for project-bound controlled requests; alias-to-snapshot changes fail closed pending review.

## Session and permission boundaries

- Scheduling and controlled inference default off. Production provider selection has no mock-provider environment switch.
- A known agent **owner**, with Owner credential ceiling, supplies fresh consent, a goal, request UUID, explicit configured model, expiry (at most ten minutes), exactly one request, input/output limits, and a real-API microUSD ceiling.
- Input and output ceilings must equal the configured provider bounds (24,192 input; configurable 256–4,096 output). The session ceiling must cover its worst-case reservation. The request body is capped at 16,000 bytes, with conservative token overhead; oversized context fails before transport.
- Created or paused agents can receive this separate manual authorization without activation. Suspended or terminated agents cannot start requests or approve tools. Owner and current memory capability are checked again at request stages and tool execution.
- Only relevant memories from that agent are sent. Memory IDs and classifications are retained. No environment, credentials, wallet secrets, or hidden model reasoning are sent or recorded as reasoning.
- One project-scoped model metadata GET precedes the single generation POST. The fixed OpenAI origin disallows redirects; both requests carry the same server-side project and key. Counters record attempted requests conservatively before transport.
- A successful response persists a concise model decision summary, cited memory references, usage and proposed actions in `awaiting_approval`. At most one `read_memory` action is allowed; an empty action list is valid abstention.
- Owner review of the exact persisted proposal is separate. Current permissions are rechecked inside an agent-locked database transaction. Approval records read memory references and a result atomically; concurrent or repeated approval cannot execute twice. Declining records zero executed actions.
- This milestone intentionally allows only the existing nonfinancial `read_memory` capability. It does not write memories, complete goals, publish drafts, sign, transfer, trade, launch tokens, create agents or enable other irreversible effects.
- Private session history is protected by existing agent-scoped Viewer/Operator/Owner access. Public discovery never receives these records. Global budget consumption from other users is not exposed through an agent's UI.

## Independent accounting and recovery

Migration `0007_controlled_inference.sql` adds session and immutable approval-audit tables only. It does not create/refill budgets, change ownership, modify lifecycle state, erase inference records, or touch simulated treasury rows.

Reservations lock the **existing** cumulative budget row, require its exact configured ceiling, and retain all prior commitment. Missing budgets fail closed. The runtime never creates another budget to bypass a limit. The historical evaluation UUID `ad91e475-f1da-495a-8247-139643d2d8b1` and $0.10 ceiling were not modified or replaced during this work.

Every session is one generation attempt, with no automatic retry at any layer. UUID reuse is rejected. Reserved or uncertain calls, including historical evaluator `inference_calls`, block another session using that budget. Concurrent processes serialize reservations through PostgreSQL; no process-local semaphore is relied on for spending protection.

Provider-reported tokens are authoritative usage evidence; monetary charges are **estimates** at recorded rates, not an invoice. Successful bounded usage releases only the confirmed unused reservation. Malformed output with valid usage still records the charge but never exposes an actionable proposal. Missing/invalid/out-of-bound usage retains the full reservation (or a larger reported estimate), marks the session unknown, and blocks further requests. Rate limits are not retried or assumed free. A failed metadata check is known to have started zero generation requests and releases that session's generation reservation.

An interrupted request is never replayed. Expired reserved sessions are shown as unknown on reads and persisted as unknown at API startup. There is no background live scheduler. A killed request may leave its conservative attempt counter at one even if it never reached the network; that is intentional. Ambiguous reservations require invoice/provider evidence and an independently reviewed reconciliation workflow before another request. This milestone supplies no automatic release or retry button.

## API and UI

- `GET /agents/{id}/live`: configured model and ceilings, minimum reservation, latest twenty scoped sessions. Reading never probes OpenAI or performs inference.
- `POST /agents/{id}/live`: strict `LiveConsent`; Owner consent required. Unknown fields (keys, endpoint overrides, extra tools) are rejected.
- `POST /agents/{id}/live/{session}/approval`: `{ "approve": true, "confirm_review": true }` or explicit decline; only the original consenting owner can approve.

Intelligence adds an expandable **Controlled GPT session** panel using existing terminal forms and details styles. It shows model, one-request limit, token ceilings, cumulative ceiling, consent controls, saved summary, retrieved/cited memory IDs, attempted metadata/generation requests, estimated costs, failures and tool outcomes. Offline cycles remain in their existing history. Changing the goal, model or ceiling clears consent. Responses are rendered as text, never HTML. Errors instruct the operator to reload saved sessions without automatic retry. The same server adapter is used by the local Node host and the prepared Next staging route.

## Server configuration and rollout

Keep all values in server-only owner-readable configuration; never put a key in a public Next variable. The frontend receives only a private Rust bearer through its existing account mapping.

Required **only after separate operator approval**:

```text
SUPA_CONTROLLED_INFERENCE=true
SUPA_ALLOW_OPENAI=true
SUPA_PROVIDER=offline
SUPA_API_SCHEDULER_ENABLED=false
OPENAI_API_KEY=<server-only OpenAI API credential>
OPENAI_PROJECT_ID=<explicit proj_ identifier>
OPENAI_MODEL=gpt-6-luna
SUPA_MAX_OUTPUT_TOKENS=1024
SUPA_LIVE_BUDGET_ID=<existing approved budget UUID>
SUPA_LIVE_BUDGET_MICROUSD=<unchanged cumulative ceiling>
```

Existing DATABASE_URL, SUPA_API_TOKEN, scoped operator credentials and frontend account mapping are also required. `.env.eval` is not loaded by this feature. Controlled inference cannot be enabled alongside the API scheduler or a globally live provider. The dedicated worker remains offline-only.

No production migration, service deployment, local-agent adoption or live account request was performed. The old local ports 3200/55432 and documented `/tmp/supa-agent-pg` were unavailable in this execution environment. Validation instead used a fresh disposable localhost PostgreSQL cluster on port 55439 and isolated `_test` schemas. Historical balances/account access cannot be reverified from that fresh cluster. Restore the original durable database and inspect its existing reservations before any real evaluation; do not recreate or reset the old budget from fixture data.

## Repeatable validation

Use a disposable local database ending `_test`, never the evaluation or production database:

```sh
TEST_DATABASE_URL=postgresql://pikoo@127.0.0.1:55439/supa_controlled_test \
  cargo test --workspace --all-targets --locked --offline -- --include-ignored
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo build -p supa-api --example controlled_fixture --locked --offline
TEST_DATABASE_URL=postgresql://pikoo@127.0.0.1:55439/supa_controlled_test \
  node scripts/controlled-smoke.mjs
npm --prefix reference-web run test:unit
npm --prefix reference-web run check
npm --prefix reference-web run build
npm --prefix reference-web test -- --workers=1 --timeout=60000
npm --prefix web run lint
npm --prefix web run typecheck
npm --prefix web run build
```

`controlled_fixture` requires an explicit fixture flag, loopback bind, a `_test` database and an isolated `controlled_browser_` schema. It cannot use the historical evaluation database. Its mock summary and agent name identify the fixture clearly; no real key is read. Fixture budget rows are isolated test data, not replacement authorizations. Browser evidence records 120 input + 60 output **mock** tokens, a 42-microUSD **mock estimate**, separately approved memory read, and no simulated-ledger changes. Actual OpenAI HTTP requests and charges during this milestone: **zero**.

See `artifacts/controlled-inference/` for system/browser evidence and final validation results. REA evidence proves rendered behavior, not actual GPT intelligence or comprehensive accessibility certification. No new visual assets, animation dependencies, CSS or branding were introduced.

## Implementation validation results

- Full Rust workspace/all-targets suite, including ignored isolated PostgreSQL and mock HTTP tests: **124 passed**.
- Final API/provider recheck after model-envelope and recovery hardening: **47 passed**.
- New controlled-inference integration cases: **6 passed**, covering consent/ownership/model bounds, cumulative costs, concurrent requests, exact-once tool approval, reconnect persistence, crash recovery, malicious proposals, authentication rejection, out-of-bound usage, rate limits and a real 26-second mocked timeout.
- Reference frontend server/contract unit tests: **18 passed**.
- Approved experience desktop/tablet/mobile browser regressions: **150 passed**, one clean run, no retries.
- Preserved Next operator console desktop/tablet/mobile regressions: **144 passed**, one clean run, no retries.
- Rust formatting and all-target Clippy with warnings denied: passed.
- Frontend syntax checks, lint, typecheck, static experience build and Next production build: passed. The sandboxed Next build's TypeScript subprocess returned empty output; the same build succeeded with normal local subprocess access. No configuration workaround or dependency change was introduced.
- Isolated end-to-end fixture: explicit goal and cost consent, one mock generation, persisted memory references, a separately approved memory read, and unchanged simulated financials. Fresh store connections retained sessions; concurrent approval produced one audit entry.
- REA independently selected the fixture agent, opened the GPT controls and reloaded saved records at **1440×1000** and **390×844**. Final screenshots and evidence hashes are in `artifacts/controlled-inference/verified/`. The initial nested-summary selector was too broad; the corrected direct-child locator passed both scenarios.
- Supplemental browser inspection confirmed ten navigation sections, no horizontal overflow, unchecked initial paid consent, a 3,536-microUSD minimum reservation for the fixture's 1,024-output-token policy, keyboard field order, reduced-motion `still`, and a 44px model control. Native gray field styling observed in review was corrected by reusing the existing `schedule-form` class, without editing CSS.
- All **12** approved visual source/asset files are byte-identical to the baseline. No new frontend dependency or continuous animation was added. This is focused keyboard/responsive validation, not a complete accessibility or performance certification.

### Remaining limitations and next step

Actual OpenAI account access, inference quality and invoice settlement remain unverified in this milestone. No paid generation or metadata probe was executed against OpenAI. Summaries are not factual proof of goal completion. The only executable proposal is a memory read; additional tool types require their own reviewed effect, idempotency and consent design.

Only the most recent twenty sessions are exposed in Intelligence; all older records remain in PostgreSQL. There is no automated billing reconciliation or replay of ambiguous requests. Unknown historical or current reservations must be resolved with provider billing evidence through an independently reviewed operator process. Exact model attribution is deliberately strict. Existing static operator credential registration and its restart requirements remain.

Recommended next milestone: restore/verify the durable historical database, review existing budget reservations and project/model configuration without printing credentials, then separately authorize **one** bounded real GPT session. Assess actual summary quality and memory provenance before considering any broader tool allowlist. Continuous live scheduling should remain disabled.
