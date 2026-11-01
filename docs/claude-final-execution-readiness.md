# First Claude work evaluation — final non-paid readiness check

Checked 2026-11-01 at 15:25 WAT (Africa/Lagos). No provider request, generation, paid reservation, primary-database startup, credential provisioning, worker activation, push or deployment was performed during this check.

## Decision

The unchanged evaluation scope and existing disposable database pass their checks. **Paid execution is not yet ready or authorized**: authoritative billing/spending-limit evidence is unavailable, and the isolated execution API plus its scoped runner authentication are not running/configured in the current shell.

## Verified scope, model and expiration

- Scope SHA-256: `b11a1cb723e32742e23988b897504dd524e026390e027a0def5398ae44eb931b`.
- Provider/model: `anthropic` / `claude-haiku-5-5`.
- Workspace: `wrkspc_01FargC8PH6NDA25MHMRgz3F`.
- Private key present with safe file permissions; effective model/workspace equal the reviewed plan. No key or private configuration values were printed.
- Prior separately authorized metadata GET returned HTTP 200 and the matching model. That successful probe was **not repeated**. It established endpoint access with the explicit workspace header, not all workspace roles, billing limits or generation permission. The network-free credential helper's `authentication_verified=false` means it performs no authentication itself; it does not negate the earlier HTTP 200 evidence.
- Exact expiration: **2026-11-02 13:30:58.948063 WAT** (`2026-11-02T12:30:58.948063Z`). Approximately **22 hours 5 minutes** remained at the check. No renewal or extension was made. Revalidate immediately before execution; an expired scope requires new review.
- Original 293-byte supplied-document task and input hash `613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e` unchanged. No agent, job, budget, evidence, memory policy, fixed nonce or protected plan field changed.

## Independent budget and actual disposable records

- Budget `3bae1525-7e83-4220-9682-d30d06cc06f2`: **$0.005 ceiling / $0 committed**.
- Maximum **one generation**, **$0.004 request ceiling**, 24,192 input / 1,024 output tokens. Conservative reservation **$0.003536** is a proposal, not a charge or new authorization.
- Exact database `supa_claude_3f722ca85789_test`, system identifier `7703179569575286737`, socket port 55683, verified read-only. It was started only for inspection and stopped afterward; no records were inserted, updated or migrated.
- Test agent `1d892448-ce83-4e38-a2b9-fc166411b15a` remains paused and scoped to its disposable test owner.
- Job `fd408604-98d9-48e4-a2b0-c4cd92a3a3e0` remains created, unpaid, simulated, revision zero, with original input and expiration.
- All twelve migration checksums verified. No active application epoch, inference calls, live sessions, cycles, actions, scheduling authorizations or economic entries exist in this fixture.
- No prior execution journal exists for this scope. Fixed nonce, exclusive fsynced journal and database reservation checks prevent automatic replay; unknown charges require review.

## Billing and execution prerequisites still outstanding

Only the operator's reported credit balance is available. There is no authorized connected billing/account session or verified credit/limit export available to this check. A normal model key and metadata HTTP 200 do not prove a usable credit balance or workspace/organization spending allowance. No additional provider/admin request was made, and no administrative credential is requested.

Before paid approval, the owner must verify in the authenticated Anthropic Console that this intended workspace/organization has available API credit, no billing suspension, sufficient remaining organization/workspace allowance for the bounded reservation/request, and compatible request/token rate limits. Share confirmation or a redacted account summary only; never credentials or payment-card data. The independent application budget remains binding regardless of the account balance.

No listener was present on disposable API port 55684, and no `SUPA_WORK_OPERATOR_TOKEN` was present in the checking process. These are execution prerequisites, not permission to create real owner credentials. Prepare only the exact disposable API and test-owner/service authentication under the existing reviewed isolated procedure when authorized; retain loopback binding, offline routine provider, scheduler disabled and the exact Claude budget/model/workspace. Do not repoint the primary frontend or enable recurring inference. Recheck API health/policy against the real fixture before paid dispatch.

## Evidence and financial safeguards

Application validation enforces strict schemas, exact source lines/quotes for observations, supplied verified memory IDs, an exact allowlist of local analysis tools, deterministic numeric totals, incomplete/abstained statuses and missing-information checks. The work scope keeps memory disabled and permits only `inspect_input`, `extract_evidence`, `check_requirements`; independent internet research and external mutations are unavailable. Source assertions are supplied evidence, not independently established truth. Free-text synthesis/interpretation still requires human semantic review; lexical validation cannot prove every natural-language assertion.

Incomplete/abstained work is not delivered as completed work. This job has zero payment and zero earned revenue; a model response is not proof of commercial completion or payment. Provider costs remain separate from simulated economics and historical budgets. No permission, financial policy or database schema changed.

Primary PostgreSQL stayed stopped and unqueried. All **26 protected primary/backup/accounting hashes** match the pre-check baseline, including preserved historical evidence for the unresolved **$0.004048** hold. The historical database remains unrecovered. No hold was released, refunded, settled or reused. Both primary and disposable PostgreSQL clusters are stopped after inspection.

## Tests and exact protected command

Passed **35 domain tests**, **36 provider/example tests with local mock servers**, **six credential tests**, **nine evaluation/consent/replay tests**, and the real disposable read-only identity/accounting/expiry/checksum checks. No test invoked a real provider endpoint. No source/frontend change was necessary. Ignored sanitized local evidence is under `artifacts/claude-final-readiness/`; only this report is committed. Secrets and full one-file history audits are required after that commit.

After billing confirmation, the isolated API prerequisites, a fresh expiry/state check and **separate explicit paid authorization**, the exact protected command is:

```bash
python3 scripts/live-work-evaluation.py \
  --plan config/claude-work-evaluation-plan.json \
  --run --authorize-paid \
  --scope-sha256 b11a1cb723e32742e23988b897504dd524e026390e027a0def5398ae44eb931b \
  --model claude-haiku-5-5 --max-cost-microusd 4000
```

**Not executed.** This command fails closed without the actual disposable database/API and scoped authentication. It does not provision them, renew the job or create a budget. Neither this report nor the earlier metadata approval authorizes paid inference, recurring agents, financial transactions, X publishing, deployment or GitHub push.
