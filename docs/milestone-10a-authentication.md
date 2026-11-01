# Milestone 10A — Project authentication and budget verification

Verified 2026-11-01 under the operator's authorization for a minimal generation-free metadata check. No paid evaluation was authorized or executed.

## Actual provider observation

Exactly one authenticated `GET https://api.openai.com/v1/models/gpt-6-luna` was attempted with `OpenAI-Project: proj_npsyg93Ofk7ivKheGa8IDw5q`. It returned **HTTP 200**, with the exact requested model ID and `object: model`. This confirms accepted credentials and model-metadata access with the intended project header. There were **zero generation requests** and no retries.

The key was loaded inside the existing preflight process from an owner-only private file. No credentials, partial credentials, key hashes, headers or raw provider responses were printed or committed. No private configuration was modified or runtime credentials installed.

This is endpoint-specific access evidence, not verification of every project membership role, generation permission, billing entitlement or project spending limit. Billing and account limits still require independent account-side verification. ChatGPT subscription access does not establish API billing access. Official references: [authentication](https://developers.openai.com/api/reference/overview), [retrieve model](https://developers.openai.com/api/reference/resources/models/methods/retrieve).

## Unchanged scope and independent budget

Reviewed scope SHA-256 remains:

`cf2f80163a97e914725972664c33962cb615cb43a77693573ec759553fb2cef2`

Input SHA-256 remains:

`613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`

The exact synthetic task, evidence inventory, agent, job, test owner, nonce and tools remain as recorded in [the project-bound plan](milestone-10a-project-scope.md). No protected scope field changed and no replacement identity, nonce, budget or expiry was created.

Read-only SQL verified the existing disposable database `supa_m10a_0b780645f120_test`, its exact data directory, system identifier `7703020438665878846`, socket-only connections and all 12 SQLx migration checksums. Independent budget `e6076529-e302-403e-b846-1ff902c1f641` remains **5,000 micro-USD ($0.005), committed 0**. No new budget insertion was needed.

Limits remain **one generation**, **4,000 micro-USD ($0.004) per request**, **24,192 input / 1,024 output tokens**. The network-free calculator still proposes a conservative 3,536 micro-USD reservation; no reservation was made. The agent remains paused and the job created, unpaid, simulated and revision 0. Memory is empty; inference calls, live sessions, cycles, tool actions, scheduling authorizations, economic entries and earned revenue remain zero.

The job expires at **2026-11-02 02:14:32.831959 UTC (03:14:32.831959 WAT)**. Do not silently extend it. An expired or changed scope requires fresh review.

## Safety and validation

- Four JavaScript preflight tests passed, covering scope equivalence, tampering, credential-safe diagnostics, database identity and independent budget requirements. The initial sandboxed test runner failed; rerunning with permitted local subprocess access passed all four tests.
- Six Python consent, targeting and execution-journal tests passed.
- The network-free readiness command returned the exact reviewed digest, one maximum provider request and `paid_execution_authorized: false`.
- Real disposable PostgreSQL read-only verification passed. Only the existing isolated cluster was started for verification and stopped afterward; no API or worker was started.
- All **26 protected primary, backup and accounting file hashes** remained unchanged. Primary PostgreSQL was never started or connected to. Both clusters are stopped.
- Historical budget `ad91e475-f1da-495a-8247-139643d2d8b1`, its $0.10 ceiling and unresolved **$0.004048 hold** remain quarantined and unchanged.
- Sanitized local evidence remains ignored under `artifacts/milestone-10a/authentication-20261101/`: authentication response summary, database verification and protected-file comparison. The metadata-only report's `proposal_only` budget label is a static default; the separate database check authoritatively reports `installed_zero_spent`. Similarly, local credential checks alone remain false for model access; the actual provider observation supplies access evidence.
- No frontend or Rust implementation changed, so build, UI and Rust suites were not rerun for this documentation-only update. No deployment, push, primary migration, agent activation, financial transaction or paid request occurred.

## Remaining approval boundary

Before paid execution: verify project billing/spending limits, approve passive disposable API setup and scoped test-owner authentication, and explicitly authorize this exact single-request project/model/scope and ceilings. No scoped runtime operator token has been generated or installed. Tokens must remain server-side, never in command arguments or chat. A successful metadata GET does not override these requirements.

The exact protected execution command, **not executed**, is:

```bash
python3 scripts/live-work-evaluation.py \
  --plan config/live-work-evaluation-10a.json \
  --run --authorize-paid \
  --scope-sha256 cf2f80163a97e914725972664c33962cb615cb43a77693573ec759553fb2cef2 \
  --model gpt-6-luna --max-cost-microusd 4000
```

It also requires the approved disposable database/API and a private `SUPA_WORK_OPERATOR_TOKEN`. Execution must revalidate the unexpired scope and budget. Keep continuous scheduling and workers disabled. The fixed nonce, durable journal, reservation and existing fail-closed recovery must remain in force; no ambiguous paid request may be automatically replayed.

This report is committed alone using the one-file Conventional Commit workflow on `main`. Full history and secrets audit outcomes are recorded in the final operator response; no push is authorized.
