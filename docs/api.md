# Local HTTP API

Default origin: `http://127.0.0.1:8080`. All routes require `Authorization: Bearer $SUPA_API_TOKEN`. POST bodies are JSON and are limited to 16 KiB. Creation/configuration types reject unknown fields. UUIDs identify agents, goals, cycles, actions, events and ledger idempotency keys. Timestamps are UTC instants serialized as RFC3339 by application models; PostgreSQL JSON history timestamps include an offset.

| Method | Path | Behavior |
|---|---|---|
| GET | `/health` | Database liveness, simulation mode, publishing disabled |
| GET / POST | `/agents` | List up to 100 agents / create with zero balance in `created` state |
| GET | `/agents/{id}` | Identity, lifecycle, config, personality, goals, memory and simulated finances |
| POST | `/agents/{id}/lifecycle` | `{ "state": "paused", "reason": "Operator requested pause" }` |
| GET / POST | `/agents/{id}/goals` | List / add `{ "description": "..." }` |
| POST | `/agents/{id}/goals/{goal}/complete` | Mark goal complete |
| GET | `/agents/{id}/memory` | Retained lessons with timestamps and cycle IDs |
| GET | `/agents/{id}/activity` | Latest 100 sanitized durable events |
| GET | `/agents/{id}/financials` | Clearly labeled simulated balance, revenue, inference/operating costs and runway |
| POST | `/agents/{id}/simulated-funding` | Operator credit; see payload below |
| GET / POST | `/agents/{id}/cycles` | Latest cycles / request one cycle for an active agent |
| GET | `/agents/{id}/actions` | Proposed tool metadata, authorization_outcome, execution status and failure codes |
| GET | `/agents/{id}/drafts` | Legacy saved drafts and event references; never published |
| GET | `/agents/{id}/social` | New social drafts, abstentions, rejected-decision codes and separate review metadata; publishing disabled |
| GET | `/agents/{id}/transitions` | Lifecycle timestamp, reason and actor history |
| GET | `/agents/{id}/simulated-ledger` | Latest 100 hypothetical ledger entries, all `simulated: true` |

Creation accepts `name`, optional `personality`, optional `config`, and optional `capabilities`. Omitted permissions default to none. Use [the Supa example](../config/supa.json) for the approved personality. Config bounds: interval 1–86,400 seconds, maximum 1–8 steps, per-provider timeout 1–30 seconds, and nonnegative simulated per-operation costs up to 1,000,000,000 micro USD. The total cycle deadline is 180 seconds.

Funding payload:

```json
{
  "kind": "revenue",
  "amount_microusd": 1000000,
  "idempotency_key": "7284f1fa-735b-46d4-b099-10244ccbd415",
  "simulated": true
}
```

Only `seed` and `revenue` are accepted by this endpoint. Amounts must be positive and no greater than 1,000,000,000,000 micro USD. Retry the same operation with the **same UUID**, amount and kind. A new UUID means a new hypothetical credit. Funding does not activate agents.

Financial responses use `simulated: true`, `unit: "simulated_micro_usd"`, `balance_microusd`, `revenue_microusd`, `inference_microusd`, `operating_microusd`, `runway_days` and `runway_assumption`. Seed funds are excluded from revenue. Null runway means insufficient expense history, never infinite real wealth.

Application errors are `{ "error": "controlled_code" }`: 400 invalid input; 401 missing/invalid operator authentication; 403 denied operation; 404 absent agent/goal; 409 inactive agent, invalid lifecycle transition, busy/stale cycle, or insufficient simulated funds; 504 reasoning timeout; 503 database/provider failure. Invalid UUID paths use framework validation. History is bounded without cursor pagination in this milestone.

The cycle endpoint is synchronous. A client disconnect may leave work to finish or expire; it must not assume success and blindly retry. Read cycle/activity history first. Pausing blocks subsequent tool effects; it cannot unsend an in-flight provider request. No endpoint changes permissions after creation or executes arbitrary tools.

## Milestone 2 additions

| Method | Path | Behavior |
|---|---|---|
| POST | `/agents/{id}/pause` | `{ "reason": "..." }`; fences active work |
| POST | `/agents/{id}/resume` | Explicit operator activation of eligible agent |
| POST | `/agents/{id}/emergency-stop` | Idempotent suspension and immediate work invalidation |
| GET | `/agents/{id}/inference` | Latest 100 metered call attempts and reservations, separate from simulated ledger |
| GET | `/agents/{id}/goal-evaluations` | Latest 100 evidence-linked model assessments |
| POST | `/agents/{id}/goals/{goal}/priority` | `{ "priority": 0 }` through 10; operator-only |

Cycle configuration additionally accepts `retry_budget` (0–2), `max_provider_calls` (1–9), and `max_cycle_tokens` (1,000–500,000). Only explicit 429 responses retry. Both manual and scheduled cycles enforce the configured interval. Budget exhaustion uses controlled codes `resource_budget_exhausted` or `live_budget_exhausted` (409), suspending active agents. Safe provider errors can include `provider_http_429` or `provider_transport_failed` (503).

Memory records expose `kind` (`hypothesis` or `verified_observation`) and source event IDs. Existing model memories default to hypothesis. Goals expose operator `priority` and `suggested_progress_bps` (0–10,000), which is a model claim, not completion. Only the operator completion endpoint sets `completed=true`.

Inference history contains non-secret model/rate policy, whether the call is live, reservation/accounted estimated micro USD, token usage (or null), usage source, status and safe error code. Do not render those provider estimates as an agent's real wallet or simulated ledger entry. A null usage field means unavailable evidence, never zero-token usage. This history is protected by the same local operator authentication as other routes.

Social decisions use `consider_tweet` under the existing `DraftTweet` capability. Public text is `data.text`; review status and publishing-disabled fields are separate. See [social contracts and limitations](personality-and-social.md). `social_repetition` maps to HTTP 409.

## Milestone 3 wallet endpoints

The optional authenticated `/agents/{id}/wallet` family covers public identity, recovery, balances, verified payments/checks, sync, treasury, policy, operating reservations and financial audit. Signing always returns 403. See the [complete wallet API contract and request fields](milestone-3.md#authenticated-api-contract). All values must display network/evidence labels; Devnet test tokens have no monetary value.

## Bounded offline autonomy

Authenticated `GET /autonomy` returns the fixed rehearsal policy. Authenticated
`POST /autonomy` accepts only `request_id` (new UUID), `goal` (3–500 trimmed UTF-8
bytes) and `authorize_offline: true`. It creates an isolated operator-requested
identity, runs three deterministic cycles, and pauses. Maximum expenses are
3,300 simulated microUSD from a 10,000 simulated microUSD seed. The configured
live provider is never invoked. Reusing the UUID returns 409 without replay or
new funding. Inspect saved records after an uncertain result; do not auto-retry.

The approved experience exposes this operation only to authenticated operators
through `/api/autonomy`, requiring a matching Origin, JSON content type and
`X-Supa-Action: offline-autonomy`. Browser bodies are limited to 4 KiB. General
lifecycle, funding and arbitrary tool writes remain unavailable through the BFF.
See [authorization, recovery and workflow validation](autonomy.md).

## Continuous offline scheduling

Authenticated `GET/POST /agents/{id}/schedule` exposes persisted scheduling policy
and revision-checked operator commands (`start`, `pause`, `resume`, `stop`). Start
requires explicit consent, approved goal IDs, interval, cycle ceiling, simulated
expense ceiling and expiry. Other commands preserve those limits. Stop permanently
terminates the identity. Replayed UUIDs return current state without reactivation.
Managed schedules always use Offline; live inference reservations are rejected in
persistence. Browser access is operator-only and uses the existing same-origin JSON
action guard. See [complete limits, payloads and recovery](continuous-autonomy.md).

Agent and API scheduling now default disabled. Existing active identities require
new explicit scheduling authorization and are never adopted by the worker merely
because an old configuration flag was true.
