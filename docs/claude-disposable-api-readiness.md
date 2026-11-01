# Claude disposable API execution readiness

Verified 2026-11-01 at approximately 19:00 WAT (Africa/Lagos). Supersedes the API-not-running blocker in `claude-final-execution-readiness.md`. **No paid inference or new provider authentication request was performed.**

## Running isolated services

- Disposable Rust API: **http://127.0.0.1:55684**, built from current source with `cargo build -p supa-api --bin supa-api --locked --offline` using the existing dedicated validation target/profile.
- Disposable PostgreSQL: `supa_claude_3f722ca85789_test`, `/tmp/supa-claude-pg-3f722ca85789/data`, socket port 55683, system identifier **7703179569575286737**.
- SQLx connection options pin the database, private Unix socket and port, with a distinguishing application name. Read-only PostgreSQL inspection confirmed every observed API connection used that exact database with no TCP client address. The cluster remains socket-only. No primary database connection configuration is inherited.
- The child process receives a deliberately constructed environment, not the shell's unrelated settings or historical OpenAI budgets. The Claude key is read from the existing private file; it was not copied into another file or printed.
- API and disposable PostgreSQL remain running as requested. Primary PostgreSQL remains **stopped**, untouched and unqueried. No frontend routing/configuration was changed.

## Scoped test authentication

Generated distinct private service and test-owner bearer credentials solely for this disposable API. The server maps the owner token to the **existing disposable principal** `d5536ed1-b84e-4ec6-b6d2-4aa079176b19`; no database ownership grant, real owner credential or custodian permission was created. Legacy shared-owner authentication is absent.

Owner authentication expires at Unix second `1793622658`, **2026-11-02 13:30:58 WAT**, slightly before the job's fractional-second expiration. Credentials and runtime profile are ignored, untracked and owner-only (0600), beneath a private directory where applicable:

- `.env.claude-evaluation`: only the exported scoped runner token; source it silently in the owner terminal, never print it.
- `artifacts/claude-disposable-api/runtime.json`: private API process environment, containing disposable service/operator credentials. Never copy it into Git, Railway, prompts or logs.
- Private PID/log/readiness files are under `artifacts/claude-disposable-api/`; no raw request headers or credential values were logged.

Actual local checks: unauthenticated health **401**; service-token access to owner-only live work **403**; scoped-owner live-work metadata **200**. Server-owned mapping, existing database access records and the session deadline govern access, not frontend identity fields. Normal local-user/administrator trust still applies; loopback and private files are not a sandbox against the account owner.

## Unchanged scope and records

- Scope SHA-256: **`b11a1cb723e32742e23988b897504dd524e026390e027a0def5398ae44eb931b`**. Protected plan bytes are unchanged.
- Model/workspace: **`claude-haiku-5-5`** / **`wrkspc_01FargC8PH6NDA25MHMRgz3F`**, matching actual API metadata and effective private configuration. Prior HTTP 200 model authentication was not repeated.
- Budget **`3bae1525-7e83-4220-9682-d30d06cc06f2`**: **$0.005 ceiling, $0 committed**. Maximum **one generation**, **$0.004 per-request ceiling**, conservative reservation **$0.003536** not yet incurred. Token bounds remain 24,192 input / 1,024 output.
- Test agent **`1d892448-ce83-4e38-a2b9-fc166411b15a`** remains paused.
- Test job **`fd408604-98d9-48e4-a2b0-c4cd92a3a3e0`** remains created, revision zero, simulated, unpaid, with zero earned revenue. Owner-only API GETs confirmed the original input and SHA-256 **`613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`**.
- Expiration unchanged: **2026-11-02 13:30:58.948063 WAT**. Roughly **18 hours 30 minutes** remained during the follow-up liveness check. Recheck immediately before paid authorization/execution; never silently renew the job, nonce or scope.
- Supplied evidence: line 1 says replication reduces downtime but copies accidental deletion; line 2 says offline snapshots preserve earlier state and require a tested restore; line 3 says neither document provides measured recovery times. No external research or private memories enter this work scope.
- Approved tools remain exactly `inspect_input`, `extract_evidence`, `check_requirements`. Source-line quotations, schema validation, numeric checks, unsupported-claim rejection, incompleteness/abstention, and human semantic review remain unchanged. A model response alone is not commercial delivery or payment.

The existing read-only disposable preflight passed **before and after API startup**, including all twelve migration checksums, exact database/system identity, owned paused agent, original job input/expiry, independent zero-spent budget, zero inference sessions/calls, zero cycles/actions/schedule authorizations and zero economic entries. Startup applied no new migration. The API's normal process-health row and heartbeat are the expected disposable operational metadata changes; they do not authorize or execute agent work. Scheduler is disabled, health registration has `enabled=false`, and there are no enabled execution workers.

The ordinary provider is offline; controlled Claude work is available only behind explicit application consent/reservation checks. Merely starting this API does not authorize a generation. No POST was sent. Wallet support and signing are disabled, publishing is disabled, commercial delivery is false, and no trading, transfers, token launches or X publishing were enabled. All **26 protected primary/backup/historical-accounting hashes** remain unchanged, including evidence for the unresolved **$0.004048** hold. Historical data remains unrecovered; this fixture is not restored primary history.

## Remaining billing confirmation

No authenticated Claude Console session or authorized administrative billing data is available to this task. Public documentation was reviewed, not the private account. The earlier operator-reported $10 balance and successful model GET do not establish current usable credit or remaining account limits.

Using the same organization as the Supa Agent API key, the operator should confirm:

1. On [Console Billing](https://platform.claude.com/settings/billing): organization identity, current **available API credit**, any billing suspension, current period spend and effective organization monthly spending cap/remaining allowance. At least **$0.005 available credit** and **$0.004 remaining allowance** should be available for this reviewed request. Do not purchase credits or change auto-reload as part of this check. Official [API billing guidance](https://support.claude.com/en/articles/8977456-how-do-i-pay-for-my-claude-api-usage) explains where credit balance and usage appear.
2. In the selected [Console workspace](https://platform.claude.com/settings/workspaces), confirm the full ID **`wrkspc_01FargC8PH6NDA25MHMRgz3F`**, its spend limit, current-period spend, effective remaining allowance of at least **$0.004**, and whether it inherits the organization cap. A notification threshold is not itself a hard spend limit. See official [workspace controls](https://platform.claude.com/docs/en/manage-claude/workspaces).
3. Confirm organization/workspace request and token limits for **`claude-haiku-5-5`** are not exhausted or zero and permit one bounded request under the reviewed 24,192-input/1,024-output maximums. Report applicable limit/reset information if blocked. See official [rate/spend limits](https://platform.claude.com/docs/en/api/rate-limits).

Provide only the relevant organization/workspace identifiers, balances/limits, billing-ready confirmation and observation timestamp; never keys, passwords or card details. No account limit is modified by this preparation. These account checks do not replace the stricter independent application budget or fresh paid authorization.

## Protected execution command — not run

Technical API/database prerequisites are now ready. **Billing confirmation and new explicit paid authorization remain required.** After those gates and an immediate expiry/state check, run in the owner terminal:

```bash
cd ~/Desktop/supa-agent
. ./.env.claude-evaluation
python3 scripts/live-work-evaluation.py \
  --plan config/claude-work-evaluation-plan.json \
  --run --authorize-paid \
  --scope-sha256 b11a1cb723e32742e23988b897504dd524e026390e027a0def5398ae44eb931b \
  --model claude-haiku-5-5 --max-cost-microusd 4000
```

Sourcing the private file exports the test token silently; no secret appears in command arguments/history. The unchanged runner checks the actual disposable database, API policy, owner-scoped job and exclusive journal before paid dispatch. Ambiguous outcomes suspend execution for review, never automatic retries. No paid consent was issued in this milestone.

API build, actual startup/liveness, exact target/policy/job/authentication GETs, post-start database invariants, private-file permissions and protected-state hashes passed. Completed provider/unit suites were not repeated. The only tracked change is this report; private credentials, process profiles, logs and other generated evidence remain ignored. One-file commit, secrets scan and complete history audit apply. No push, deployment or production infrastructure change is authorized.
