# Milestone 10A — First live GPT work evaluation preflight

Prepared on 2026-11-01. **No paid inference, provider API request, budget installation or primary-database operation occurred.** The operator explicitly left the OpenAI project unassigned. This is a frozen preflight proposal, not an executable paid authorization.

## Exact task and evidence

Objective: **Compare the supplied recovery strategies.** Input is synthetic, non-sensitive and supplied locally; it is not real customer work or independent research.

1. `material:1`: Document A: Replication reduces downtime but copies accidental deletion.
2. `material:2`: Document B: Offline snapshots preserve earlier state but need a tested restore.
3. `material:3`: Neither document contains measured recovery times.

The exact stored UTF-8 input is 293 bytes, with JSON-escaped newlines and **no trailing newline**:

```json
{"material":"Document A: Replication reduces downtime but copies accidental deletion.\nDocument B: Offline snapshots preserve earlier state but need a tested restore.\nNeither document contains measured recovery times.","objective":"Compare the supplied recovery strategies","requirements":[]}
```

Input SHA-256: `613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`.

The frozen `supplied-synthesis` fixture supplies a human rubric: explain replication versus backup, cite both documents and do not invent recovery times. Structured factual observations must be verbatim text supported by exact supplied line quotations. Interpretations and summary prose remain unverified model analysis requiring human review; exact quotation checks do not prove semantic truth. Missing measurements must remain unknown. No external retrieval or private memory is permitted.

## Isolated preparation

| Item | Exact value |
| --- | --- |
| Test database | `supa_m10a_0b780645f120_test` |
| Data directory | `/tmp/supa-m10a-pg-0b780645f120/data` |
| Unix socket / port | `/tmp/supa-m10a-pg-0b780645f120/s` / `55681` |
| PostgreSQL system identifier | `7703020438665878846` |
| Test agent | `f9a17921-f31f-4154-843d-1f348622a2f4` |
| Test owner principal | `28092c72-3475-48fa-b703-a68728ea4fa4` |
| Unpaid test job | `ec1f95dc-cf3b-4907-83b4-624937802061` |
| Test service | `7f39fe0f-081d-4e0d-b97e-71233017d379` |
| Fixed single-request nonce | `0d40f529-6574-4349-afb3-ebe5660d33cd` |
| Proposed accounting namespace | `c73bf689-0a1b-439b-918d-ca30f6d13267` |
| Proposed independent budget | `e6076529-e302-403e-b846-1ff902c1f641` |
| Job revision / expiration | `0` / `2026-11-02T02:14:32.831959Z` |
| Future local API origin | `http://127.0.0.1:55682` — no API started |

PostgreSQL 16.15 was initialized empty, with checksums, owner-only storage/socket permissions, peer local authentication, rejected TCP authentication and `listen_addresses=''`. The disposable database received migrations 1–12. All 12 SQLx SHA-384 migration checksums matched source files. This is new test data, not restored historical state or an activated application epoch. The accounting namespace is a proposal label in the frozen manifest; actual ledger isolation is the separate verified database/system identifier, not a newly activated epoch namespace.

The agent was created **paused**, with `read_memory`/`evaluate_goal` capabilities, no memories and no reasoning cycles. Only the disposable test owner record was assigned; no real owner credential or primary grant was installed. The job remains `created`, simulated and unpaid: received and earned amounts are zero. Its nominal offer is 5,000 simulated atoms, not revenue or funding for API costs. No budget row, inference reservation, live session, worker authorization, action or economic ledger entry exists in this database.

A stop/start rehearsal retained the same agent, job, input hash, ownership and empty accounting. The disposable cluster was stopped after validation. It is intentionally ephemeral; if lost or expired, create a newly reviewed scope rather than representing recreated data as this preserved state. No expiration is extended automatically.

## OpenAI and budget readiness

The existing API key was read only inside the preflight process from the existing owner-only `.env.eval` file. Ownership, regular-file status, permissions and basic format checks passed. Only boolean diagnostics and source labels were emitted; no key, partial key, key fingerprint or secret-bearing environment was logged or committed. No credential was installed into a running API. Presence/format does **not** establish validity, model permissions or billing access.

**Project ID remains null, as explicitly directed by the operator.** It was not inferred from the key, prior calls or historical budgets. No model metadata GET was attempted. Project identity/access, model access, inference permissions and billing/spending limits are unverified. ChatGPT subscription access does not establish API billing access.

Official [GPT-6 Luna documentation](https://developers.openai.com/api/docs/models/gpt-6-luna) and [pricing](https://developers.openai.com/api/docs/pricing) were reviewed. The proposed identifier remains `gpt-6-luna`; this documents public model capability, not access for this account. The existing transport selects Standard/default processing and explicit 30-minute caching.

The new budget is **proposal only**, capped at 5,000 micro-USD ($0.005), separate from simulated operating capital and every historical budget. One generation request is proposed, with a 4,000 micro-USD ($0.004) request ceiling, 24,192 input-token bound and 1,024 output-token ceiling. At the reviewed configured rates the conservative reservation is 3,536 micro-USD ($0.003536). It is not a measured charge. Project billing changes and actual usage remain subject to provider reconciliation; this is not a claim of a provider-side hard spending cap.

The exact approved analysis tools would be `inspect_input`, `extract_evidence`, `check_requirements`; memory is off. The existing application rejects financial authority, hosted mutations, invalid quotations and unsupported observations. Even a mechanically accepted report remains evaluation-only and awaiting human review: this path never marks the economic job completed or credits earned revenue. Incomplete/rejected work also earns zero.

## Frozen scope and protected command

The public configuration is `config/live-work-evaluation-10a.json`, frozen locally read-only. Canonical sorted JSON over `{plan, fixture}` produces:

`299a2bd75c3eb41cc2cc06a64925b385b308b323d0040fb526dce1c218a904ec`

The digest binds the exact task, database/system identifier, logical namespace proposal, agent/job/owner/service, fixed request nonce, expiration, model, limits, tools and **unassigned** project. Python and JavaScript independently agreed on the digest. File permissions alone are not tamper-proof authority: an independently reviewed digest and fresh authenticated owner consent are required. Changing the project or any scope content requires a new digest and a new explicit approval.

The exact protected command for this current proposal is shown for review only and was **not executed**:

```bash
python3 scripts/live-work-evaluation.py \
  --plan config/live-work-evaluation-10a.json \
  --run --authorize-paid \
  --scope-sha256 299a2bd75c3eb41cc2cc06a64925b385b308b323d0040fb526dce1c218a904ec \
  --model gpt-6-luna --max-cost-microusd 4000
```

It currently fails before HTTP because `project_id` is unassigned. There is no valid paid execution command for a verified project yet. The runner additionally performs read-only socket/database/system-identity, ownership, input, job-expiration, migration and fresh-budget checks before HTTP. It requires exactly the proposed new budget installed with zero spent; a proposal cannot pass that gate. A scoped test-owner token would be supplied privately through `SUPA_WORK_OPERATOR_TOKEN`, never a command argument or chat. No such runtime credential was generated or installed here.

The fixed nonce prevents a new model request after a missing local journal: the existing Rust reservation rejects the same request ID. An exclusive fsynced journal also stops ordinary replay. Unknown charges suspend further budget/job requests without automatic retries, refunds or hold releases. A paid request requires fresh two-minute consent and application-side reservation before provider dispatch; a later failure cannot silently mint a replacement request.

## Tests and evidence

- Workspace all-target Rust tests: **84 passed, 83 opt-in ignored**; new preparation executable compiled. Focused domain/provider unit checks also passed (35 + 18; eight local-wire opt-in tests remained ignored). Existing Milestone 10 database/provider integration and browser results remain applicable; their implementation/UI was not changed here.
- Network-free JavaScript preflight tests: **4 passed**, covering cross-language digest agreement, tampered inputs/targets/limits/nonces, private-file/symlink rejection, credential diagnostic privacy, read-only query enforcement, stale/consumed scope and independently installed zero-spent budget requirements.
- Network-free Python runner tests: **6 passed**, including previous approval/journal checks, rejection before HTTP when disposable identity checks fail, and fixed nonce retention across lost journals. Mock HTTP in those tests is in-process; no provider is contacted.
- Actual disposable preparation, read-only inventory, all 12 checksums, persistence after stop/start and input hash checks: **passed**. The unapproved execution-readiness gate failed closed as expected because the budget is not installed.
- A first SQLx URL serialization attempt rejected the Unix socket before connecting; it was corrected to the previously reviewed `postgresql://localhost/<test-db>?host=<socket>&port=55681` form. No primary SQL or inference occurred. The preparation executable refuses a nonempty database; it never overwrites existing state on repeat invocation.
- Primary cluster remained stopped. All **26 protected hashes unchanged**, including approved backup/control files and historical accounting evidence. Historical budget `ad91e475-f1da-495a-8247-139643d2d8b1`, $0.10 ceiling and unresolved **$0.004048 hold** were neither recreated nor modified. Historical data remains unrecovered.
- Local-only evidence under ignored `artifacts/milestone-10a/`: preparation, local/restart checks, readiness, execution-gate, integrity and test logs. No database files, private data, credentials or generated artifacts are committed. No frontend, backend financial permission, model provider implementation or primary migration was modified. No frontend cutover, persistent activation, transfer, trading, launch, publishing, Railway operation, deployment or push occurred.

## Remaining separate approvals

1. Identify and independently verify the intended OpenAI project. Keep it unassigned until then. A separately requested metadata GET can check model access with that explicit project header; billing/limits require separate account evidence. No key inference or prior-evaluation reuse.
2. Review a new scope/digest containing that verified project and an unexpired exact job. If this job expires, prepare a new job/scope; do not extend the current scope silently.
3. Approve installation of only the proposed isolated $0.005 evaluation budget and scoped disposable operator/API configuration. No primary owner or epoch activation is included.
4. Explicitly approve **one** `gpt-6-luna` request for the exact new digest, project, task, agent/job, token bounds and $0.004 ceiling. Only then may the stopped disposable cluster/API be restarted for the request. Paid continuous scheduling remains disabled.

About 9.4 GiB is free; the earlier 10 GiB protected-primary-preflight headroom is not met. Primary startup/migration, real owner provisioning, persistent agent activation, historical accounting reconciliation, Devnet funding and Railway deployment remain separately protected and outside this scope.

Git changes follow one exact file per Conventional Commit on `main`. Final history/secrets audits and clean-status evidence are reported after these preparation commits; no push is authorized.
