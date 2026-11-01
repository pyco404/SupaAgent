# Milestone 10A — Final non-paid execution readiness

Validated on 2026-11-01. The successful project-scoped HTTP 200 metadata observation remains the evidence in [the authentication report](milestone-10a-authentication.md); it was not repeated. **No OpenAI requests or paid generations occurred during this validation.**

## Account-information limitation

No authorized project billing statement, dashboard session, spending-limit export or account-administration connection is available in this workspace. The existing application API key and accepted model metadata do not prove billing entitlement, generation permissions, available balance, organization limits or project usage limits. No key formatting inference, administrative API probe, credential retrieval or account-setting change was attempted. Billing and usage limits remain **unverified**.

An authorized account administrator must independently confirm the selected project `proj_npsyg93Ofk7ivKheGa8IDw5q`, active API billing/credit, applicable model usage limits and configured budget alerts before execution. Only report non-secret confirmation; do not supply keys or sensitive billing records. Application limits below are independently enforced local ceilings, not a verified provider hard spending cap.

## Passive API and disposable database validation

Added `crates/api/examples/work_readonly_preflight.rs`, an explicit test-only entry point using the existing Rust router and authentication architecture. It pins the prepared disposable database, Unix socket, system identifier and loopback API port **55682**. It does not accept an arbitrary database target. Every SQLx connection starts with `default_transaction_read_only=on`; startup verifies that setting and the target identity. Outer HTTP middleware denies all methods other than GET and HEAD. No migrations, recovery writes, worker monitor, scheduler, OpenAI provider, credential-file loader or financial capability is constructed.

Short-lived server-side test tokens were generated only in the validation process, passed through the child environment, and discarded when the processes exited. They were never printed, persisted, committed, installed into the real runtime or supplied to the primary environment. The test owner is the already prepared disposable identity, not a newly verified real owner. Its test session expires after 120 seconds.

The reproducible `scripts/test-work-readonly-preflight.py` starts only `/tmp/supa-m10a-pg-0b780645f120/data`, runs read-only checks and this passive HTTP fixture, and stops them in cleanup. It uses an explicit clean environment without OpenAI keys, private dotenv loading or proxy configuration. It preserves the reviewed budget and scope; it does not seed, recreate, renew or extend them.

Actual HTTP results:

- Unauthenticated health access: **401**.
- Authenticated health and scoped agent listing: **200**; exact agent remains paused.
- Authenticated live-work GET: **200**, exact input hash/job visible, empty sessions, `enabled:false`, `continuous:false`, no provider policy attached.
- Public-service credential accessing private live-work records: **403**.
- POST, PUT, PATCH and DELETE against live work: **403** before dispatch.
- Graceful API termination succeeded. All public-table row fingerprints were identical before and after the complete test.

The passive fixture validates HTTP wiring, scoped reads and safe refusal. It deliberately cannot execute the paid workflow. Approval and configuration of a separate bounded live-provider API are still required; this report does not claim that a paid-capable API is already running.

## Frozen scope, budget and validity

Scope SHA-256 remains exactly:

`cf2f80163a97e914725972664c33962cb615cb43a77693573ec759553fb2cef2`

Input remains `613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`. Original task, three supplied evidence lines, tools, no-memory policy, agent, job, test owner, fixed nonce and database identity remain unchanged. The manifest accounting namespace remains an isolation label, not an activated epoch. All **12 SQLx migration checksums** match source.

Independent budget `e6076529-e302-403e-b846-1ff902c1f641` remains **$0.005**, committed **$0**. Maximum generations **1**; request ceiling **$0.004**; input/output token ceilings **24,192 / 1,024**. Conservative proposed reservation remains **$0.003536**, not an incurred charge or created reservation.

Job expiration remains **2026-11-02 02:14:32.831959 UTC / 03:14:32.831959 WAT**. At the database check, **83,549 seconds (about 23 hours 12 minutes)** remained, exceeding the rehearsal's five-minute minimum. Recheck immediately before any future paid execution: this measurement is not indefinite validity. No expiry, consent or scope was extended. Any scope change requires a new digest and approval.

No inference calls, live sessions, reasoning cycles, actions, scheduling authorizations, economic entries or earned revenue were created. The job remains created, unpaid and simulated; the agent remains paused.

## Integrity and tests

All **26 protected primary/backup/accounting hashes** remained unchanged. Primary PostgreSQL was never started or queried; both clusters are stopped after testing. Historical budget `ad91e475-f1da-495a-8247-139643d2d8b1`, its $0.10 ceiling and unresolved **$0.004048 hold** remain untouched and quarantined.

- Offline Rust example build passed against locked dependencies.
- Actual disposable database and HTTP rehearsal passed all **7 grouped checks**, including four denied mutation methods and full public-table row preservation.
- Preflight unit tests: **4 JavaScript passed** and **6 Python passed**.
- The first rehearsal stopped safely because a sanitized PATH selected system Node 18 rather than the repository's newer runtime. The harness now resolves the invoking Node executable explicitly; the complete rerun passed. No prepared records changed during either run.
- Evidence remains ignored at `artifacts/milestone-10a/final-readiness-2a771069e2ba/report.json`; process logs contain no keys or tokens. No artifacts, database records or credentials are committed.
- Frontend and core runtime behavior were unchanged. No browser or frontend rebuild was needed for this passive backend test utility.

Reproduction (network-free; only the exact disposable cluster is started):

```bash
CARGO_TARGET_DIR=/tmp/supa-m9-target CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo build -p supa-api --example work_readonly_preflight --locked --offline -j2
python3 scripts/test-work-readonly-preflight.py
```

## Separate paid authorization required

Remaining blockers are independent project billing/usage-limit verification, explicit approval for the bounded live-provider API and private test-owner authentication setup, and fresh consent for the exact one-generation scope. The passive API is stopped and cannot be reused to generate. The scope must remain unexpired. No paid execution approval is implied by this successful rehearsal.

Protected execution command, **not run**:

```bash
python3 scripts/live-work-evaluation.py \
  --plan config/live-work-evaluation-10a.json \
  --run --authorize-paid \
  --scope-sha256 cf2f80163a97e914725972664c33962cb615cb43a77693573ec759553fb2cef2 \
  --model gpt-6-luna --max-cost-microusd 4000
```

The approved bounded API must be running first, with the exact project and model configured server-side and private `SUPA_WORK_OPERATOR_TOKEN` provided without command-line secret values. Keep persistent workers and continuous scheduling disabled. Preserve the existing exclusive request journal, fixed nonce, reservation and ambiguous-charge suspension protections. No automatic retry or replacement request is authorized.

Each added file is committed separately on `main`. Secrets and full-history audit outcomes are recorded in the final response. No push, deployment, primary-environment change or real financial activity is authorized or performed.
