# Authorization renewal, operator roles and worker health

This milestone extends the existing six-crate runtime and approved ten-section
experience. It does not activate paid inference, signing, transfers, trading,
launches, publishing, child agents, or deployments. Scheduling defaults remain off.

## What existed

The runtime already had explicit lifecycle states, finite offline scheduling,
revision-checked idempotent commands, PostgreSQL row locks, one running cycle per
agent, conservative simulated allowance consumption, tool permission checks,
verified memories, interrupted-cycle recovery and persistent accounting.
Authentication was one shared bearer credential: it did not establish ownership.
Start could not replace an existing authorization; resume preserved its limits.
There was no durable process heartbeat or worker-to-cycle association.

## Ownership and roles

Rust authenticates a server-configured credential to a principal UUID and role
ceiling. Browser identity and role headers have no authority. Each agent has one
immutable owner and optional operator/viewer grants. Both the credential ceiling
and the agent grant must permit a request.

| Role | Private records | Existing pause/resume/stop | Start/renew | Grant roles / create identities |
| --- | --- | --- | --- | --- |
| Viewer | Explicitly granted agents | No | No | No |
| Operator | Explicitly granted agents | Yes, within approved limits | No | No |
| Owner | Owned agents | Yes | Yes | Yes |

Wallet mutations additionally require the wallet service's configured operator
identity to match the authenticated principal; existing wallet policy and disabled
signing checks remain. The shared service credential alone can read only health,
minimal public directory identities and fixed offline capability metadata.
`GET /agents` lists only accessible full records. `GET /directory` exposes exactly
ID, name, lifecycle and creation time. Unknown ownership denies private access.

New API-created identities and offline rehearsals are owned atomically. Migration
0006 never guesses owners, changes lifecycle, funds agents, or discards ledger or
execution rows. Legacy approvals retain their allowance evidence, with unknown
approvers/times represented as unknown rather than invented.

Owners can POST `{ "principal_id": "<uuid>", "role": "viewer" }` or `operator`
to `/agents/{id}/access`. Owner grants/transfers and self-escalation are rejected.
Changing a grant is audited. HTTP has no unauthenticated claim or signup endpoint.

### Server-only configuration

- `SUPA_API_TOKEN`: existing private service credential.
- `SUPA_OPERATOR_ID`: optional explicit principal UUID binding that credential to
  an owner role ceiling. Omitting it does **not** grant legacy private access.
- `SUPA_OPERATOR_CREDENTIALS`: optional private JSON array of objects containing
  `id`, `role` and `token`, for additional distinct principals. Duplicate IDs or
  credentials, nil IDs, unknown fields/roles and malformed tokens fail startup.
- Existing `SUPA_WEB_USERNAME` / `SUPA_WEB_PASSWORD` maps to `SUPA_API_TOKEN`.
- `SUPA_WEB_OPERATORS`: optional private JSON array of `username`, `password`,
  `api_token` objects. Each Basic account forwards its own Rust credential.
  Duplicate usernames fail closed. Keep passwords at least 24 characters and
  credentials 32–256 printable non-whitespace ASCII characters.

These are server configuration, never `NEXT_PUBLIC_` values. Keep them in an
owner-only ignored environment file or a private staging secret store. Restart
services after changing the registry; this milestone does not provide SSO, public
onboarding, dynamic credential issuance or a credential-management UI.

### Legacy adoption requires a separate explicit decision

Existing local agent ownership was not changed during implementation. After
checking the intended owner's identity and wallet scope, an operator can run:

```sh
cargo run -p supa-api --example access_admin --locked --offline -- \
  --authorize-local-ownership <agent-uuid> <owner-principal-uuid>
```

This requires the intended **local** `DATABASE_URL` to be loaded securely. It
refuses non-loopback hosts, ownership replacement and conflicting wallet ownership.
The explicit operation is audited; it does not issue an execution authorization.
Legacy schedules without a recorded approver are excluded from due selection and
cannot be resumed through HTTP: pause and explicitly renew with fresh limits.
Do not run this against an agent without authorizing that ownership assignment.

## Audited renewal

`POST /agents/{id}/schedule` accepts `operation: "renew"` only from the owner.
It requires a new request UUID, the current revision, explicit offline consent,
operator reason, specifically selected unfinished goals and a complete policy.
`consented_at` must be within 120 seconds of server time (at most five seconds
in the future), including after waiting for the agent lock.

Policy limits remain 1–10 distinct goals, 5–86,400 seconds between cycles, 1–100
cycles, 1,100–110,000 simulated microUSD and 60–86,400 seconds validity. Fixed
cost allowance remains 1,100 simulated microUSD per cycle, at most three local
memory/goal tools, one deterministic planning request and zero provider retries.

Renewal requires an existing authorization, non-active/non-terminated lifecycle
and no unresolved running cycle, even if its lease is expired. Recovery must
resolve that lease first. An active authorization must be paused explicitly.

The transaction closes the previous authorization epoch and opens a new one.
Each epoch retains approver, approval timestamp, policy, expiry and consumed
cycles/allowance. The agent schedule points to the current epoch. New cycles
reference its UUID; ledger charges retain their existing cycle references.
Current-epoch counters restart only under this explicit new approval. Historical
counters and the simulated treasury/ledger are never reset or replenished.
This does not change or create any real OpenAI spending budget.

Commands retain the authenticated approver. Request IDs are globally unique;
reuse with another principal or changed body is rejected. Identical replay reads
current truth and cannot undo a subsequent pause. Expired consent is rejected
rather than replayed. Concurrent renewals serialize on the agent lock and revision;
only one can create the next epoch. Resume never extends expiry or counters.

## Measured worker health

Every API coordinator and dedicated worker registers a fresh process incarnation
UUID, even when globally paused. A separate task writes a genuine heartbeat every
three seconds. A process that misses the 15-second freshness window cannot revive
its old identity; restart registers another UUID. Graceful shutdown records stop.

Enabled schedulers record tick start/completion and sanitized failure codes. Each
tick selects at most one due agent, preserving due-time order and bounded progress.
Cycles record their worker UUID and retain the existing 240-second lease and
180-second runtime timeout. Tool writes and cycle completion validate the worker
heartbeat as well as the cycle lease; worker-row locks fence concurrent stop. Freshness and lease validity use current
database wall time after lock waits, rather than transaction-start time. A
blocked-charge regression verifies that an expired lease cannot debit the treasury.

`GET /agents/{id}/execution-health` is scoped to that agent's authorized readers:

- `healthy`: a fresh enabled process with recent successful tick progress.
- `delayed`: a fresh enabled process with overdue tick progress, a tick longer
  than 30 seconds, or a recorded tick error.
- `unavailable`: no fresh enabled process can be verified.
- `recovery_required`: the agent is suspended or its running lease has expired.

`execution_running` additionally requires an active lifecycle, a current known
execution approval, an actual valid cycle lease and the associated fresh worker.
An enabled authorization alone never establishes running execution. A healthy
idle worker can legitimately report `execution_running: false` for a paused agent.
For an active lease, health follows its particular worker rather than an unrelated
healthy process. Global worker health is a process-level observation, not evidence
that a particular goal has progressed. Snapshots lock the agent and use database
time for freshness comparisons; `checked_at` is response-generation time.

The existing lease-expiry recovery interrupts uncertain work, retains accounting,
suspends managed execution and requires explicit review/resume or renewal. New
workers cannot blindly replay an interrupted operation. No external non-idempotent
actions have been added.

## Frontend changes

Only existing terminal content and expandable Intelligence details changed.
Typography, CSS, intro, hands, portal, logo, wheel, branches and ten sections are
preserved. Intelligence displays measured worker health, heartbeat/lease times,
read-only approval history and effective role. Owners can explicitly choose renew
and enter fresh limits; viewers receive saved records without lifecycle controls.
Changing the operation, policy or reason clears consent. The existing same-origin,
strict-schema, no-retry mutation adapter is retained. Both frontend hosts forward
the authenticated account's server-side credential instead of sharing private
access across accounts. Backend enforcement remains authoritative.

## Validation and evidence

See `artifacts/authorization-renewal/` for actual isolated system results,
desktop/mobile captures and REA evidence. Synthetic browser regression fixtures
are labeled separately. No screenshot or heartbeat is described as live GPT
reasoning. Rust integration tests exercise actual PostgreSQL transactions,
concurrent renewals, stale consent, ownership, escalation, legacy upgrade,
heartbeat expiry, stopped workers, lease fencing and process restart behavior.

The dedicated browser harness creates an isolated schema, performs explicit
start/pause/renew/pause controls with scheduling globally disabled, verifies two
approvals and unchanged finances, and restarts the API. It enables an offline
worker only after every scoped agent is paused: genuine heartbeats/ticks are
observed with **zero planning cycles**. Test schemas containing that evidence are
preserved; no existing development identity is activated or reassigned.

The earlier test database retains an intermediate uncommitted migration checksum
from development. It was not reset or patched. Final whole-suite validation uses
the dedicated `supa_authorization_test` database and fresh per-scenario schemas.
No production or existing development database was migrated by this milestone.

## Live GPT readiness criteria

Managed scheduling is deliberately offline-only. A healthy worker, an API key or
an offline renewal is not permission to spend. Before any future live milestone:

1. Explicitly establish owners and private per-user credentials; verify role and
   cross-user tests against the intended staging configuration.
2. Review and authorize the target agent, exact goals, tools, model, cycle/token
   ceilings, expiry and independent cumulative real-API spending budget.
3. Add a separately reviewed live authorization type; do not reinterpret this
   milestone's simulated budget or offline consent as paid permission.
4. Reconcile previous real charges and uncertain reservations without resetting
   the existing live budget; reserve worst-case costs before each request.
5. Verify model access and credentials only through separately authorized checks.
   Keep provider secrets out of model context, browser payloads and logs.
6. Validate heartbeat/lease recovery, revocation, emergency stop, model timeouts,
   accounting and non-idempotent uncertainty under a small approved test budget.
7. Obtain explicit authorization before the first paid evaluation or staging run.

No paid requests, wallet actions, trades, token launches, production migration,
push or deployment are performed as part of these checks.

## Existing local service rollout

The previously running development stack and its agent ownership were not
reconfigured or migrated automatically. The new binary was validated in isolated
local test schemas. Before adopting existing development records, review the
intended principal UUID, persist its server-only configuration, restart the API
with scheduling explicitly disabled, and apply the additive migration locally.
Only then perform separately authorized legacy ownership assignments. Keep
execution hosts disabled until a fresh finite owner approval is in place.
Production migration, staging deployment and paid inference remain separate gates.

## Final validation results

- Rust: **115 passed**, including all ignored local PostgreSQL/process cases.
  Six new integration scenarios cover ownership/roles, renewal/accounting/races,
  stale consent/unresolved leases, legacy upgrades, worker health/fencing and
  expiry after agent-lock contention.
- Approved experience: **150 browser cases passed** across desktop, tablet and
  mobile. The full run passed 147; its three mocked-clock polling cases failed.
  Those cases were changed to exercise the actual 15-second interval and all
  three passed separately, preserving the original assertions.
- Preserved Next.js console: **144 browser cases passed**, one worker, no retries.
- Server adapters: **17 unit cases passed**.
- Rust formatting, workspace/all-target Clippy with `-D warnings`, binary build,
  Next production build, TypeScript, ESLint, experience syntax/static build and
  explicit provisioning-example compilation all passed.
- The real isolated demonstration retained two approving-owner epochs and
  unchanged finances across API restart. All agents were paused when the worker
  was enabled. Actual healthy heartbeats/ticks were observed with zero cycles
  and zero inference requests. Cross-user reads and viewer mutations were denied.
- REA exercised the agent picker, saved-schedule reload, expandable controls and
  mobile navigation. Final captures are 1440×1000 and 390×844. Supplementary
  keyboard checks reached approved-goal input in order; viewer mutation controls
  were absent from accessibility, with no document overflow. Reduced motion and
  completed typing were verified before final mobile capture.
- The 12 approved visual files, including layout, styles, scenes, wheel, logo
  assets and fonts, are byte-for-byte unchanged from the milestone baseline.

Evidence: [validation and changed-file inventory](../artifacts/authorization-renewal/validation.json),
[real system result](../artifacts/authorization-renewal/system-check.json),
[REA observations](../artifacts/authorization-renewal/rea-evidence.json),
[desktop renewal](../artifacts/authorization-renewal/rea-desktop.png),
[mobile viewer](../artifacts/authorization-renewal/rea-mobile.png),
[mobile owner](../artifacts/authorization-renewal/mobile-owner.png).

Existing Node module-format and color-environment warnings remain informational.
No new frontend dependencies or animations were introduced. These checks are not
a full accessibility certification, penetration test or performance benchmark.
The one-file history audit and complete secrets gate are run again after the last
evidence commit. No push is authorized for this milestone.
