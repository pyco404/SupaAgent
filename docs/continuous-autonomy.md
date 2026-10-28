# Continuous offline autonomy and lifecycle management

## Existing architecture and changes

The six-crate architecture is retained. Rust already supplied lifecycle transitions,
durable cycle leases, interval enforcement, permission-checked tools, goal priorities,
memory retrieval, simulated accounting, and separate API/worker execution processes.
The approved experience already exposed protected records and the three-cycle offline
rehearsal. Its terminal appearance, ten sections, introductory sequence, logo, hands,
portal, wheel and branch animations are preserved.

The missing layer was durable operator authorization for continuous scheduling. Older
configuration defaulted agent scheduling on, and the worker could select an active
identity without a persisted cycle or cumulative spending ceiling. Defaults now disable
both agent scheduling and the API scheduler. A worker also defaults paused. Legacy
active identities are never adopted automatically: selection requires a new row in
`agent_schedules`, written only by an explicitly authorized operator control.

## Operator workflow

1. Sign in through `/operator`, select an agent and open Intelligence.
2. Expand **Agent lifecycle and bounded scheduling**.
3. For **Start scheduling**, explicitly select unfinished goals, interval, maximum
   cycles, simulated expense ceiling and authorization duration. Enter a reason and
   select the authorization checkbox. No form submits automatically.
4. **Pause agent** fences ongoing work and disables its schedule.
5. **Resume scheduling** needs fresh consent and the current revision. It retains
   the original expiry, approved goals, cycle counter and consumed allowance.
6. **Stop permanently** terminates the identity and fences work. It is irreversible;
   saved records remain available. Terminated agents have no executable controls.

The selected identity is the target; this is not another agent-creation workflow.
Its existing capabilities and personality are preserved. Start explicitly normalizes
cycle configuration to the documented fixed offline limits. Agents require at least
one existing memory/goal capability and an approved, owned, unfinished goal. No tool
permission is granted by scheduling. If an identity has wider permissions, scheduled
context and per-action authorization use only the intersection with `read_memory`,
`remember`, and `evaluate_goal`. Social, conserve and external tools are excluded.

Monitoring reuses existing goals, cycles, actions, memory, activity and financial APIs.
The schedule status includes lifecycle, revision, enabled authorization, counters,
expiry, cooldown timestamp and a controlled last-outcome code. Idle pages poll every
15 seconds and pause polling when hidden or interacting. **Reload saved schedule**
is an explicit read. No heartbeat or provider intelligence score is invented; an
authorized schedule does not prove a worker process is currently running.

An enabled offline execution host is required in addition to per-agent consent:

```bash
# Separate worker; use private server-side environment, never browser credentials.
SUPA_WORKER_ENABLED=true SUPA_PROVIDER=offline cargo run -p supa-runtime --locked --offline
# Alternatively enable the existing API coordinator at local startup:
SUPA_API_SCHEDULER_ENABLED=true SUPA_PROVIDER=offline cargo run -p supa-api --locked --offline
```

Do not start both to increase throughput without reviewing resource needs. Both can
coexist safely for duplicate prevention because every claim uses the same database
lock. Their global concurrency is not a cluster-wide resource limit.
Production/staging configuration remains paused by default and was not deployed.

For this local development session, the verified inactive API was refreshed with
its offline coordinator explicitly enabled. All existing agent data was compared
before/after and remained unchanged. There were zero authorized schedules. The
port-3200 preview now serves the controls, while its existing identity remains
`created`, unauthorized for scheduling. No existing agent was started. Only the
additive local schema migration was applied outside the dedicated test databases.

## Limits and accounting

| Boundary | Enforced behavior |
|---|---|
| Approved goals | 1–10 distinct owned, unfinished goal IDs at start |
| Interval | 5–86,400 seconds; stored cooldown survives restart and resume |
| Cycles | 1–100 starts per authorization, including interrupted attempts |
| Simulated ceiling | 1,100–110,000 microUSD per authorization |
| Fixed cycle expense | 1,000 simulated inference + 100 operating microUSD |
| Tools | At most three steps; existing permissions narrowed to three internal tools |
| Inference | Deterministic Offline, one invocation; zero retries; live reservations denied |
| Authorization expiry | 60–86,400 seconds; resume cannot extend it |
| Execution/lease | Existing 180-second cycle deadline and 240-second lease |

The cycle claim reserves 1,100 units of **authorization allowance**, atomically with
the agent lock and cycle insertion. This is a conservative execution ceiling, not
another simulated ledger debit and not real funds. Existing financial endpoints show
the actual simulated inference/operating debits. Storage enforces a separate maximum
1,100 simulated microUSD of debits per managed cycle. Existing treasury checks remain
independent. Unused allowance from an interrupted attempt is not refunded on resume.

At a cycle/allowance limit the schedule automatically pauses. Expiry or completed
approved goals prevent a new cycle. Insufficient treasury suspends without starting
or charging a cycle. Execution failure or expired lease disables the schedule and
requires review. Future starts cannot renew an existing authorization: counters and
historical spending cannot be reset by issuing another request UUID.

## API and authorization

All Rust routes require the existing server-side bearer token. The new private route
is `GET/POST /agents/{id}/schedule`. POST accepts only:

```json
{
  "request_id": "<new UUID>",
  "expected_revision": 0,
  "operation": "start",
  "authorize_offline": true,
  "reason": "Operator-approved offline observation",
  "policy": {
    "goal_ids": ["<owned unfinished goal UUID>"],
    "interval_seconds": 15,
    "max_cycles": 3,
    "simulated_budget_microusd": 3300,
    "valid_for_seconds": 3600
  }
}
```

For pause/resume/stop, `policy` must be null. The revision is an opaque monotonic
version read from GET; it need not increase by exactly one. Unknown fields, live
provider settings, missing consent, invalid goals, stale revisions and exceeded
limits fail closed. Reusing a request UUID with identical content returns **current
state**, never reapplies an old start after a newer pause/stop. Changed payloads
with a reused UUID are rejected. Row locks serialize commands and execution claims.

The browser's `/api/agents/{id}/schedule` route requires operator authentication,
matching Origin, JSON, `X-Supa-Action: offline-autonomy` and a body no larger than
4 KiB. The server adapter validates and strips response fields, restricts upstream
destinations, and never retries. General funding, signing and arbitrary lifecycle
writes remain unavailable through the browser. Public users cannot read schedules.

The form becomes read-only after a submitted operation until status is reloaded.
Failures instruct the operator to inspect current records before another consent.
Legacy resume/lifecycle activation cannot reactivate a managed schedule; it must
use the revision-checked control. Other operator pause/emergency-stop paths also
invalidate scheduling authorization and fence running cycles.

## Recovery and non-idempotent operations

Database transactions and the partial unique running-cycle index prevent two workers
claiming the same identity concurrently. Claims consume the durable allowance before
execution. Missed ticks are skipped rather than replayed in a catch-up burst.
Every tool write rechecks active lifecycle, lease validity and schedule expiry.
Pause/stop fence old leases; the old process cannot commit another tool effect.

An expired managed lease marks the cycle interrupted, pending actions failed and
in-flight inference accounting unknown. Existing cost evidence is retained. The
agent is suspended and scheduling disabled; no crash automatically reactivates it.
The operator may resume only within the original remaining limits. An HTTP disconnect
after a control commit is handled by UUID/revision inspection, not blind replay.

Only local transactional tools are executable here. Their action IDs, cycle IDs,
authorization outcomes and effects are persisted, and duplicate execution cannot
repeat an effect. No payment, publication, launch, signing or trading is attempted.
Before any future non-idempotent external tool is allowed, require a durable intent/
outbox, stable provider idempotency key where supported, and receipt reconciliation.
An uncertain outcome must remain blocked for operator review; never infer failure
from a timeout or replay an unverified external operation.

## Validation and evidence

The real end-to-end harness is `scripts/scheduling-smoke.mjs`. It requires a loopback
database ending `_test`, generates temporary server-only credentials, creates only
new test identities, and performs real browser controls against Rust, PostgreSQL,
Next.js and a separate worker. It never loads evaluation secrets or changes the
existing real inference budget. The migration is additive; no existing agent is
authorized or changed by applying it. Production migrations were not executed.

The recorded demonstration produced three completed cycles, eight allowed tool
actions and six memories, with 3,300 simulated microUSD spent from a 10,000 seed.
The unapproved goal remained untouched. Default-disabled execution, pause across
API/worker restarts, non-reactivating start replay, resumed counters, automatic
limit pause and permanent stop were verified. Final balance was 6,700 simulated
microUSD. Three offline invocation records have no live token usage; no actual
token usage is fabricated. Live inference calls and transactions were zero.

REA discovered the live test target, navigated, expanded controls and reloaded saved
status. Desktop (1440×1000) and mobile (390×844) captures show real persisted records
without horizontal overflow. The lifecycle mutations themselves were exercised by
the Playwright system harness. Passive screenshots do not establish past network
activity; scenario limitations and artifact digests are retained.

- [System results](../artifacts/continuous-autonomy/system-check.json)
- [REA scenario and provenance](../artifacts/continuous-autonomy/rea-evidence.json)
- [Desktop status](../artifacts/continuous-autonomy/rea-desktop.png)
- [Mobile status](../artifacts/continuous-autonomy/rea-mobile.png)
- [Explicit start form](../artifacts/continuous-autonomy/desktop-authorization.png)
- [Paused mobile control](../artifacts/continuous-autonomy/mobile-paused.png)

Database tests cover unauthorized requests, strict fields, stale/replayed commands,
concurrent claims and controls, cooldowns, independent cycle/cost/treasury/expiry
limits, duplicate tool effects, emergency fencing, live-reservation denial and
expired-lease recovery. Lease expiry is deliberately induced in isolated test rows;
the system harness separately restarts actual API and worker processes while paused.

Validation results:

- Workspace formatting and all-target Clippy (`-D warnings`) pass.
- Full workspace/all-target Rust tests with local PostgreSQL and ignored integrations
  enabled: 108 passed. Final targeted API tests also verify broader stored permissions
  remain unchanged while scheduling excludes social/conserve tools, and the updated
  cooldown test verifies unapproved scheduling is denied.
- Approved-experience desktop/tablet/mobile suite: 144 passed, including nine lifecycle
  UI cases. All ten sections, cinematic transitions, hands, pointer tethers, navigation,
  reduced motion and responsive layouts remain covered.
- Preserved Next.js console suite: 144 passed with one worker and retries disabled;
  the final sequential run completed in 2.1 minutes.
- Server adapter tests: 15 passed, including strict scheduling inputs, operator/same-
  origin requirements, response stripping and rejection of a live-enabled response.
- ESLint, TypeScript, Next.js production build, reference syntax and asset build pass.
- The initial restricted Next build could not parse its TypeScript subprocess output;
  configuration inspection succeeded and the normal local build passed. Overlapping
  browser suites later hit timeouts with nearly full RAM/swap. Competing owned test
  processes were stopped, and sequential reruns retained the original assertions and
  disabled retries. The final approved-experience run passed cleanly in 5.4 minutes.

Secrets scans include the existing known-local-secret gate and a scan of all changed
and newly eligible files. They are targeted checks, not guarantees against unknown
secret formats. The unrelated `.gitignore` was not modified by this milestone.

Source changes are confined to domain scheduling contracts/defaults, the additive
`0005` migration, persistence claims/control/accounting fences, runtime context and
selection, API routes/defaults/logs, the existing frontend adapter/contracts and
Intelligence controls, scoped form styling, tests, the controlled system harness,
README/API documentation and new evidence. There are no new Rust crates, frontend
dependencies, scene/branding assets, wallet tools or deployment changes.

## Remaining limits and recommended next milestone

Offline reasoning is deterministic observation and provisional goal evaluation; this
does not establish new GPT intelligence, real earnings or autonomous economic success.
Each authorization is finite and non-renewable. Operators need a future audited
renewal flow to run the same identity after its limits expire; no budget reset or
silent extension was introduced. Goal creation/prioritization remains in existing
operator APIs and the rehearsal flow; this milestone adds lifecycle/scheduling UI.
Local/private staging authentication is not a public multi-user authorization system.
Fleet-wide concurrency, pagination, durable worker heartbeat and retention need work
before scaling. Database outages can prevent immediate persistence; disabled/expired
authorization and leases fence effects when connectivity returns.

Next: versioned authorization renewal with separate cumulative budgets, user-scoped
roles and worker health, followed by a separately authorized live GPT evaluation.
External actions remain unavailable until their uncertainty/reconciliation protocol
is independently implemented and approved. No deployment or public push is authorized
by this milestone.
