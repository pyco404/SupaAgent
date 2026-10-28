# Bounded operator autonomy workflow

## Existing implementation

The six-crate workspace already supplies persistent identities and lifecycle transitions,
prioritized goals, relevant-memory retrieval, event-grounded goal assessments, a provider
abstraction, a deterministic offline reasoner and an opt-in OpenAI Responses adapter.
The runtime assembles curated context, obtains a structured plan, records every proposal,
checks the application capability registry, and persists tool outcomes and failures.
Personality does not grant permissions. Goal assessments are provisional; they do not
mark goals completed automatically. Memory lessons from reasoning remain hypotheses,
not established facts.

The registry permits reading memory, recording lessons, evaluating goals, conserving
simulated resources and drafting social posts. It grants no shell, deployment, trading,
wallet-signing, payment, publishing, code modification or child-agent tools. Separate
operator APIs control lifecycle, goals and simulated funding. The worker and API have
separate execution processes; staging starts the worker paused. Agents not active cannot
run cycles, and scheduler-disabled identities are never selected by the worker.

Database cycle leases fence stale writes. Pausing or emergency-stopping an agent fences
running cycles; crash recovery persists interrupted outcomes. Inference reservations and
real API ceilings remain independent from simulated agent treasuries. Existing inference
and social live evaluations require their own explicit authorization.

## New complete workflow

Authenticated `GET /autonomy` advertises the fixed offline limits. Authenticated
`POST /autonomy` accepts exactly:

```json
{"request_id":"<new UUID>","goal":"Review previous outcomes and record useful lessons","authorize_offline":true}
```

The operator explicitly authorizes a new isolated **human-created rehearsal identity**.
This is not autonomous child-agent creation. No existing identity is resumed or changed.
The UUID becomes the new agent ID and is a durable replay guard: an already used ID,
including an unrelated existing identity, returns HTTP 409 without re-funding or replay.
Unknown fields, invalid goals and absent authorization are rejected before creation.
Goals are trimmed and must contain 3–500 UTF-8 bytes.

The handler always constructs `Offline` directly; it cannot select the configured OpenAI
provider even if live credentials exist. It seeds 10,000 **simulated microUSD**, grants
only `read_memory`, `remember` and `evaluate_goal`, and executes three cycles at least
one second apart. Each cycle costs 1,000 simulated microUSD inference plus 100 operating,
for a maximum 3,300 simulated microUSD; actual API requests and charges are zero.

The API's shared execution semaphore rejects concurrent work in that process. Durable
primary keys prevent cross-process duplicate requests. Agent locking, interval enforcement,
and leases remain the runtime's authority. A fresh ID is created per explicitly authorized
run; there is no automatic retry or resume. On success the new identity is paused.
The detached task survives HTTP cancellation so disconnecting does not skip its final
pause. Execution has a fixed 20-second timeout, followed by at most five seconds for
emergency fencing; failure attempts emergency suspension
and reports whether fencing was confirmed. Error responses include a stage and agent ID,
never the submitted goal or private context. If the database is unavailable, suspension
cannot be guaranteed; scheduler-disabled identities remain excluded from worker pickup.

## What this proves and what it does not

Three deterministic cycles demonstrate persisted goal selection, memory retrieval and
writes, permission-checked tools, evidence-backed provisional progress, simulated costs
and final lifecycle control. They do not demonstrate live GPT intelligence, learning model
weights, genuine earnings, token usage or autonomous economic independence. Goal progress
is a deterministic suggestion based on completed cycles and must be presented as such.

The existing approved frontend can expose the operator action and then read ordinary
agent status, goals, memories, cycles, actions, activity, financials and evaluations.
Public identities remain separate from private operator records. Browsers receive no
backend bearer token, provider credential or wallet material. The BFF must enforce its
operator session, same-origin JSON writes, fixed upstream route and no automatic retries.

A server crash can interrupt the detached task. Its identity remains scheduler-disabled;
existing recovery fences expired cycles when explicitly resumed or stopped. No background
recovery automatically reactivates it. Further work should add operator-led lifecycle and
goal management for existing identities, controlled offline scheduling with stop controls,
and separate opt-in paid evaluations after explicit spending approval. There is no
production migration, deployment, real fund transfer or new live spending budget here.

## Using the approved experience

The local preview remains `http://127.0.0.1:3200`. Sign in through `/operator`,
open **Intelligence**, and expand **Run an offline autonomy workflow**. Enter a
goal, read the fixed limits, explicitly select the authorization checkbox, then
submit. **Inspect saved run** selects the new identity and its real saved cycles,
goals, actions and memories. Economy shows its separate simulated ledger.
The current selection does not determine which identity runs: each submission
creates a fresh rehearsal identity and leaves existing identities untouched.

Public visitors do not receive this control or private records. The browser uses
same-origin `/api/autonomy`; both the local preview and Next.js staging host enforce
operator authentication, matching Origin, JSON and the explicit action header.
All other browser mutations remain blocked. No backend token is shipped to clients.

The browser retains only a request UUID in session storage to guide inspection
after reload or uncertain transport results. It does not automatically submit again.
**Prepare another workflow** clears this local hint, never stored history, and still
requires a new goal and consent. The form is an expandable addition to Intelligence;
the ten sections, intro, hands, portal, logo, navigation wheel and branch animations
retain their approved design. Textarea keyboard input cannot trigger wheel shortcuts.

## Capability gaps and next milestone

Autonomy primitives were already implemented; the approved experience previously
exposed saved reads only. Lifecycle eligibility, disabled scheduling, explicit tool
permissions, simulated treasury checks and separate real inference authorization
intentionally prevent unattended execution. This workflow connects those primitives
without widening financial or external capabilities.

Existing frontend readers expose identities, lifecycle, goals, memory, cycle/action
history, social reviews, simulated finances and read-only wallet records. The new
operator form supplies a complete goal-to-outcome path. It does not add arbitrary
tool arguments, lifecycle activation of existing identities, live inference,
continuous scheduling or automatic goal completion. Offline plans use fixed,
deterministic heuristics; their progress suggestions are not intelligence scores.

Recommended next milestone: operator lifecycle and goal controls for existing
identities, bounded continuous offline scheduling with explicit pause/emergency
stop, and durable job status for interrupted workflows. Live GPT evaluation should
remain a separate, explicitly authorized and metered step under the existing budget.

## Validation evidence — 2026-10-28

The end-to-end demonstration used a separate local `supa_autonomy_test` PostgreSQL
database, a real Rust API, a built Next.js host and Chrome. Its authorized form ran
one rehearsal with these actual persisted results:

| Result | Observed value |
|---|---|
| Reasoning cycles | 3 completed |
| Permission-checked tool actions | 8 |
| Memories | 6 |
| Evidence-linked goal evaluations | 2; goal remains incomplete |
| Final lifecycle | paused; scheduling disabled |
| Simulated seed | 10,000 microUSD |
| Simulated inference / operating expenses | 3,000 / 300 microUSD |
| Remaining simulated balance | 6,700 microUSD |
| Paid API requests / transactions | 0 / 0 |
| Completed request replay | 409; cycles and balance unchanged |
| API restart | paused state, cycles, memory and balance preserved |

This is deterministic execution evidence, not a new live GPT evaluation. The separate
legacy single-cycle fixture also passed. Existing Devnet/evaluation databases were
not used for the demonstration. The local port-3200 services were refreshed with
their unchanged private configuration; its existing identity was verified unchanged,
and its form was inspected without submission.

Validation commands and results:

- `cargo fmt --all --check` and workspace/all-target Clippy with `-D warnings` pass.
- `cargo test --workspace --all-targets --locked --offline -- --include-ignored`
  with local `TEST_DATABASE_URL`: all 106 cases pass, including database, worker,
  permissions, finance, restart and the new isolated autonomy integration test.
- `npm --prefix reference-web run test:unit`: 14 adapter tests pass, including
  authorization, strict policy bounds, private-response stripping and no retry.
- `npm --prefix reference-web test`: 135 desktop/tablet/mobile tests pass, including
  nine new workflow cases. The focused nine also pass after fixture alignment.
- `npm --prefix web test`: all 144 preserved console browser tests pass.
- Next.js production build, ESLint, TypeScript, reference syntax check and static
  asset build pass. Fixture Node warnings about module detection remain non-fatal.
- `scripts/staging-smoke.mjs` with the dedicated test database passes the real
  browser-to-database workflow, private/public boundaries and API restart checks;
  it generates fresh evidence rather than overwriting historical reports.

REA used `list_browser_targets`, `capture_browser_scenario`, `inspect_web_page` and
`capture_web_screenshot`. Its scenario navigated to Intelligence and expanded the
workflow; it did not submit authorization. The real workflow submission was performed
by the Playwright end-to-end harness. REA screenshots verify the actual rendered
form at 1440×1000 and 390×844, without horizontal overflow. Reduced motion, keyboard
navigation, responsive menus, loading/error/empty states and cinematic interactions
are covered by the browser suite. Passive REA inspection cannot recover earlier
network traffic; its limitations are preserved with the evidence.

Evidence: [system results](../artifacts/autonomy/system-check.json),
[REA provenance](../artifacts/autonomy/rea-evidence.json),
[desktop controls](../artifacts/autonomy/rea-desktop.png),
[mobile controls](../artifacts/autonomy/rea-mobile.png), and
[persisted result on mobile](../artifacts/autonomy/offline-workflow-mobile.png).

Security scanning uses the existing targeted pattern/known-local-secret gate; it is
not proof against unknown secret formats. No paid inference, wallet signing, social
publishing, real financial activity, public push or deployment is part of this work.
