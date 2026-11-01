# Milestone 10A — Project-bound evaluation scope

Prepared on 2026-11-01 after the operator supplied `proj_npsyg93Ofk7ivKheGa8IDw5q`. This supersedes the earlier unassigned-project scope for future review. **No OpenAI API request or paid inference was made.** No primary database, real owner credential, agent activation or historical accounting was changed.

## Prepared state

The existing isolated database and synthetic task were verified and retained, not recreated:

| Item | Value |
| --- | --- |
| Intended OpenAI project | `proj_npsyg93Ofk7ivKheGa8IDw5q` |
| Model / maximum generations | `gpt-6-luna` / **1** |
| Request ceiling | **4,000 micro-USD ($0.004)** |
| Input / output token ceilings | **24,192 / 1,024** |
| Independent evaluation budget | `e6076529-e302-403e-b846-1ff902c1f641` |
| Budget ceiling / committed | **5,000 / 0 micro-USD** |
| Conservative reservation, not yet made | 3,536 micro-USD ($0.003536) |
| Database | `supa_m10a_0b780645f120_test` |
| PostgreSQL directory | `/tmp/supa-m10a-pg-0b780645f120/data` |
| Socket / port | `/tmp/supa-m10a-pg-0b780645f120/s` / `55681`; TCP disabled |
| Database system identifier | `7703020438665878846` |
| Test agent | `f9a17921-f31f-4154-843d-1f348622a2f4` — **paused** |
| Unpaid job | `ec1f95dc-cf3b-4907-83b4-624937802061` — **created**, revision 0 |
| Job expiration | **2026-11-02 02:14:32.831959 UTC** |
| Test-only owner | `28092c72-3475-48fa-b703-a68728ea4fa4` |
| Fixed single-request nonce | `0d40f529-6574-4349-afb3-ebe5660d33cd` |
| Proposed accounting namespace label | `c73bf689-0a1b-439b-918d-ca30f6d13267` |
| Future API origin | `http://127.0.0.1:55682`; no API started |

The user-requested budget preparation inserted only this new zero-spent ceiling in the verified disposable database. No paid consent, reservation or authorization was created. The namespace is a manifest label; physical isolation is the separately verified database and system identifier. No historical records were imported or represented as recovered history.

Before insertion, a read-only identity/accounting check verified the previous prepared state and absence of budgets/sessions. The insertion transaction independently required the exact database, data directory and system identifier, no application epoch, no prior budget and no inference history. It inserted only `(id=e6076529-e302-403e-b846-1ff902c1f641, limit_microusd=5000, committed_microusd=0)` into `live_budgets`. Restart and subsequent read-only checks confirmed persistence. Both the disposable cluster and primary cluster are stopped at completion.

## Task and evidence

Objective: **Compare the supplied recovery strategies.** This is a non-sensitive synthetic customer-style task, not commercial work.

- `material:1`: Document A: Replication reduces downtime but copies accidental deletion.
- `material:2`: Document B: Offline snapshots preserve earlier state but need a tested restore.
- `material:3`: Neither document contains measured recovery times.

The exact 293-byte UTF-8 JSON input, without a trailing newline, is preserved in the frozen configuration and [initial preflight report](milestone-10a-preflight.md). Its SHA-256 remains:

`613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`

The proposed tools remain `inspect_input`, `extract_evidence`, `check_requirements`. Memory is disabled; no external retrieval or mutation is allowed. Structured observations must match verbatim supplied text and exact source-line quotations. Interpretations and summary prose require human review; quotation checks alone do not certify semantic truth. Unknown recovery times must remain unknown. Incomplete, unsupported or even mechanically accepted evaluation output cannot credit earned revenue: this evaluation path does not complete or deliver the economic job. Received and earned job amounts, economic entries, reasoning cycles, worker authorizations, reservations and inference sessions remain zero.

## Credential and project verification limitation

The existing key was loaded only inside the network-free preflight from the existing owner-only regular private file. File ownership, restrictive permissions and basic key-format checks passed. No key, partial key, key hash, authorization header or secret-bearing environment was displayed, committed or passed as a command argument. The selected project was supplied to the preflight process explicitly; no private dotenv file was modified and no running model service received credentials.

**The key's authorization for this project is not verified.** Local configuration and opaque key format do not provide authoritative current membership, endpoint permission, revocation, model-access or billing evidence. No project association was inferred from the key or historical evaluations.

Official [authentication documentation](https://developers.openai.com/api/reference/overview) describes bearer authentication and the explicit `OpenAI-Project` header. The [retrieve-model endpoint](https://developers.openai.com/api/reference/resources/models/methods/retrieve) provides model metadata through an authenticated GET. A separately authorized GET for `gpt-6-luna` with the exact intended project header can test credential/model-metadata access; it still does not prove every membership role, generation permission, billing entitlement or account spending limit. Those require appropriate independent account evidence. No such GET or administration call was made during this network-free preflight.

Public documentation review is separate from authenticated provider API access. ChatGPT subscription access is not evidence of API billing access.

## Final review fingerprint

Configuration: `config/live-work-evaluation-10a.json`, locally read-only. Canonical sorted JSON over `{plan, fixture}` yields:

`cf2f80163a97e914725972664c33962cb615cb43a77693573ec759553fb2cef2`

This binds the selected project, task, input hash, agent/job/owner, fixed nonce, expiration, database identity, budget, model, tools and limits. It replaces `299a2bd75c3eb41cc2cc06a64925b385b308b323d0040fb526dce1c218a904ec`; the older fingerprint cannot authorize the changed scope. File permissions alone are not authorization. Any later scope change requires a new digest and explicit approval.

The exact paid execution command requiring **fresh separate authorization** is:

```bash
python3 scripts/live-work-evaluation.py \
  --plan config/live-work-evaluation-10a.json \
  --run --authorize-paid \
  --scope-sha256 cf2f80163a97e914725972664c33962cb615cb43a77693573ec759553fb2cef2 \
  --model gpt-6-luna --max-cost-microusd 4000
```

**This command was not executed.** It also requires the stopped disposable cluster and an explicitly approved passive local API/test-owner authentication configuration. No runtime operator token was generated or installed here. A future token must remain server-side in `SUPA_WORK_OPERATOR_TOKEN`, not a command argument or chat. Workers and continuous scheduling must remain disabled; the agent stays paused.

At execution, the runner rechecks socket/database/system identity, migrations, ownership, exact input, expiration and the fresh zero-spent budget before HTTP. The existing controlled provider must verify the explicitly selected model/project metadata before its single generation. Missing consent/configuration, rejected metadata access or stale data fails closed. The fixed nonce, exclusive durable journal and Rust reservation prevent automatic replay or replacement requests; ambiguous charges retain conservative accounting and require review.

## Validation and remaining readiness

- Network-free JavaScript checks: **4 passed**. Python consent/target/journal tests: **6 passed**. Focused domain/provider unit tests: **53 passed, 8 opt-in wire tests ignored**. The full workspace all-target check also passed: **84 passed, 83 opt-in ignored**; its final log is retained locally.
- Real local PostgreSQL checks verified all **12 migration checksums**, unchanged paused identity, exact unpaid job/input/expiration, zero-spent independent budget and absence of execution/financial activity. The budget survived restart and remained zero committed.
- Python and JavaScript agreed on the new fingerprint. Structural readiness is now explicitly labelled `scope_structure_only` and `paid_execution_authorized:false`; complete configuration does not claim account access or paid authority. Database checks report `installed_zero_spent`, replacing the earlier proposal-only budget status when verified.
- All **26 protected primary/backup/accounting hashes** remained unchanged. Primary PostgreSQL was never started or queried. The unrecovered historical budget `ad91e475-f1da-495a-8247-139643d2d8b1`, its $0.10 ceiling and unresolved **$0.004048 hold** remain untouched and excluded from this evaluation.
- Evidence is ignored under `artifacts/milestone-10a/project-scope/`: budget preparation, final scope, credential-safe database checks, restart verification, integrity and test logs. No database files, credentials or artifacts are committed. The frontend, Rust runtime, API contracts and financial controls were not changed. No agent activation, payment, transfer, trading, launch, X publication, Railway operation, deployment or push occurred.

**Ready for review, not paid execution.** Remaining requirements are authoritative project/model-access and billing verification, approval for the passive disposable API/test-owner setup, and explicit authorization of this exact one-request scope. The job must still be unexpired; do not silently extend it or generate a replacement nonce. Primary migration, real owner onboarding and persistent activation retain separate approval boundaries. Approximately 9.4 GiB free remains below the earlier protected-primary-preflight reserve; no protected primary preflight is authorized here.

Changes use one exact file per Conventional Commit on `main`. Final clean-status, full-history audit and secret-scan results are reported after the commits. No push is authorized.
