# First authorized Claude work evaluation — uncertain outcome

Executed once on **2026-11-01, approximately 20:21:03–20:21:05 WAT** (Africa/Lagos). The operator separately confirmed billing capacity and explicitly authorized one paid generation attempt under the reviewed limits. No retry, refund, settlement adjustment, second generation, deployment or push was performed.

## Exact authorized execution

- Scope: `b11a1cb723e32742e23988b897504dd524e026390e027a0def5398ae44eb931b`.
- Model: `claude-haiku-5-5`.
- Workspace: `wrkspc_01FargC8PH6NDA25MHMRgz3F`.
- Independent budget: `3bae1525-7e83-4220-9682-d30d06cc06f2`, **$0.005 ceiling**.
- Request ceiling **$0.004**, maximum **one generation**, no automatic retries.
- Token bounds: 24,192 input / 1,024 output.
- Application session/nonce: `f80246de-acbe-416f-b6b3-98ecbf15e286`. This is **not** an Anthropic provider request ID.

Immediate read-only preflight verified the exact disposable database/system identity, original unpaid job, paused agent, zero-spent budget, empty request journal and actual loopback API model/workspace/policy. The job had approximately **17 hours 10 minutes** remaining. Its unchanged expiry is **2026-11-02 13:30:58.948063 WAT**. No plan field, task byte, evidence reference, identity, nonce or expiration changed.

The only execution command run was the prepared command, after silently sourcing the private test token:

```bash
. ./.env.claude-evaluation
python3 scripts/live-work-evaluation.py \
  --plan config/claude-work-evaluation-plan.json \
  --run --authorize-paid \
  --scope-sha256 b11a1cb723e32742e23988b897504dd524e026390e027a0def5398ae44eb931b \
  --model claude-haiku-5-5 --max-cost-microusd 4000
```

## Observed result and complete available model output

The runner returned normally with session status **`unknown`**, failure code **`provider_charge_requires_review`**, `usage=null`, `proposal=null`, `work.report=null`, `work.tool_results=null`, and `accounted_microusd=3536`. A successful shell exit indicates the runner received and journaled the session; it does **not** indicate successful model work.

**No usable Claude-generated work text or evidence citations were retained.** There is therefore no complete model report to reproduce or assess. The deterministic baseline stored in the session is not Claude output and must not be presented as such.

| Measurement | Actual observation |
| --- | --- |
| Provider metadata stages | 1 recorded; model/workspace verification succeeded |
| Generation stages | 1 recorded attempted dispatch |
| Upstream receipt/completion | Unconfirmed |
| Automatic retries | 0 |
| Input/output/cache token usage | Unknown; no authoritative usage retained |
| Actual provider charge | Unknown |
| Conservative unresolved accounting hold | **$0.003536** |
| Independent budget ceiling | $0.005 |
| Nominal uncommitted difference | $0.001464; not permission for another attempt |
| Model work / citations | Unavailable |
| Accepted application tool execution | None |
| Earned revenue / real customer payment | Zero |

The generation counter records the application's dispatch stage, not proof the provider received, completed or billed the request. Runtime accounting conservatively retains the full reservation when valid priced usage is unavailable. **The $0.003536 is a hold, not a confirmed charge or measured token cost.** The budget's unknown session blocks reuse until reviewed. The exclusive journal and fixed request ID prevent this command from being automatically replayed.

The persisted failure code does not retain the underlying provider HTTP/transport/response-parsing error for this uncertain branch. The private API log contains startup information, not a diagnostic generation response. Exact cause, provider request ID and any provider-side response cannot be established from the available records. Do not assert a timeout, refusal, invalid schema, billing failure, or successful generation without evidence.

## Evidence inventory and verification outcome

Original supplied material remains:

1. `Document A: Replication reduces downtime but copies accidental deletion.`
2. `Document B: Offline snapshots preserve earlier state but need a tested restore.`
3. `Neither document contains measured recovery times.`

These are the approved task inputs, **not model-generated citations**. Source input hash remains `613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`. Without a model report, factual grounding, synthesis quality and useful work completion are **not verified**. No outcome is recognized as earned revenue or commercial success.

## Read-only postflight and preserved boundaries

The persisted disposable database confirms exactly one live session with the recorded stage counts and uncertain status; the independent budget has ceiling 5000 and committed 3536 microUSD. The original job remains created, revision zero, unpaid, with zero received/earned amounts and unchanged input/expiry. The test agent remains paused. There are no cycles, actions, scheduling authorizations or economic entries from this evaluation.

Primary PostgreSQL remained stopped and unqueried. All **26 protected primary/backup/accounting hashes** and the entire work-plan file hash match the pre-execution baseline. The historical **$0.004048** unresolved hold is untouched and distinct from the new disposable **$0.003536** hold. No historical data was recreated or relabeled recovered. No funding, transfers, trading, token launches, X publishing, production migration, deployment or GitHub push occurred. Existing scheduling remains disabled.

The runner's ignored private journal and read-only verification evidence remain local under `artifacts/milestone-10/live-requests/` and `artifacts/claude-paid-evaluation/`. No credentials, raw authenticated headers, database dumps or artifacts are included in this report's commit.

## Required next step — review, not retry

Review the intended workspace's Claude Console usage/request records around **2026-11-01 19:21:03–19:21:05 UTC (20:21:03–20:21:05 WAT)** for this model. Determine whether a request was received, whether it completed, its authoritative usage/cost, and whether a response/error/request identifier can be recovered. Account attribution must use actual provider evidence; the application nonce is not a provider identifier.

No provider/admin request was made to perform this review, no hold was refunded or settled, and no further paid request is authorized. Any reconciliation mutation or new paid evaluation requires separate explicit approval and a newly reviewed execution scope as applicable. Preserve this attempt, journal and hold even if later diagnostics identify an adapter issue.

No implementation change was made to chase a successful result. Completed tests were not rerun unnecessarily; validation here consists of actual immediate preflight, the single authorized runner invocation, durable read-only postflight and protected-state checks. A one-file Conventional Commit and secrets/history audits document this outcome without publishing it.
