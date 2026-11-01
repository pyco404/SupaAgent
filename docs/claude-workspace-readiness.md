# Claude workspace-bound evaluation readiness

Prepared 2026-11-01 (Africa/Lagos). This report supersedes the unassigned workspace status in `claude-local-credentials.md`. No authenticated provider request or generation was performed.

## Configuration and authentication boundary

The operator supplied public workspace `wrkspc_01FargC8PH6NDA25MHMRgz3F`. It is now recorded with `claude-haiku-5-5` in the private, Git-ignored `.env.claude` file (owner-only mode 0600). **The API key remains absent**, including from the effective process environment. No placeholder key was installed, no inference flags were enabled, and OpenAI configuration was not changed. A public workspace declaration is not proof of membership.

The owner must enter the key in their own interactive terminal:

```bash
cd ~/Desktop/supa-agent
python3 scripts/claude-local-credentials.py set-key
python3 scripts/claude-local-credentials.py check
```

The prompt hides input. Never put the key in chat, command arguments, Git or logs. File loading and secure enrollment are documented in [the credential report](claude-local-credentials.md).

After key enrollment, the following generation-free request requires **separate explicit approval**; it has not been run:

```bash
python3 scripts/claude-local-credentials.py probe \
  --workspace-id wrkspc_01FargC8PH6NDA25MHMRgz3F \
  --model claude-haiku-5-5 --authorize-metadata-check
```

This sends one fixed model-metadata GET with the explicit workspace header. A successful exact-model response establishes endpoint-specific access only, not all workspace roles, available credit, billing limits or generation permission. The reported $10 credit is unverified. No admin credential is requested.

## Exact proposed execution scope

Only `project_id` (the existing plan's legacy workspace field) changed in `config/claude-work-evaluation-plan.json`. All task bytes, evidence, identities, limits, nonce and expiration remain unchanged. The old unassigned scope is superseded; neither scope has paid consent.

- **New SHA-256:** `b11a1cb723e32742e23988b897504dd524e026390e027a0def5398ae44eb931b`.
- Provider/model: `anthropic` / `claude-haiku-5-5`; exact workspace above.
- Budget: `3bae1525-7e83-4220-9682-d30d06cc06f2`; **$0.005 total, $0 committed**.
- One generation maximum, **$0.004 per-request ceiling**, 24,192 input / 1,024 output tokens. Conservative reservation $0.003536; no incurred charges.
- Agent: `1d892448-ce83-4e38-a2b9-fc166411b15a` (paused).
- Job: `fd408604-98d9-48e4-a2b0-c4cd92a3a3e0` (created, unpaid, simulated).
- Input SHA-256: `613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`.
- Expiry: **2026-11-02 13:30:58.948063 WAT**, unchanged. Approximately 23.6 hours remained during database verification; recheck immediately before execution. Never silently renew it.
- Task: compare supplied replication and offline-snapshot recovery strategies. The three supplied material lines are the only factual evidence; measured recovery times are explicitly unavailable.
- Tools: `inspect_input`, `extract_evidence`, `check_requirements`. Memory disabled; no external retrieval or mutations. Unsupported/incomplete work earns no revenue.

The separate disposable database is `supa_claude_3f722ca85789_test`, socket port 55683, system identifier `7703179569575286737`; future passive API port 55684. It was started solely for read-only validation and stopped afterward. No new records or authorizations were created.

For review only, **do not execute without separate paid authorization**, verified credentials/account limits and the approved disposable API configuration:

```bash
python3 scripts/live-work-evaluation.py \
  --plan config/claude-work-evaluation-plan.json \
  --run --authorize-paid \
  --scope-sha256 b11a1cb723e32742e23988b897504dd524e026390e027a0def5398ae44eb931b \
  --model claude-haiku-5-5 --max-cost-microusd 4000
```

The runner requires matching server policy, actual database checks, fresh finite consent, fixed nonce and exclusive journal. Ambiguous charges suspend execution rather than retrying. A changed scope or expired job requires new review.

## Provider and pricing verification

The existing Anthropic Messages adapter remains implemented and tested, alongside OpenAI. Application permission, structured-output/evidence validation, reservations and recovery remain outside model control. Mock success/error/timeout/schema tests verify the adapter; actual Claude access and work quality remain untested.

Official [pricing](https://platform.claude.com/docs/en/about-claude/pricing), [Haiku model documentation](https://platform.claude.com/docs/en/models/haiku-5-5/overview) and [metadata API](https://platform.claude.com/docs/en/api/http/models/retrieve) were reviewed on 2026-11-01. For this bounded standard global Haiku request, rates per million tokens are $0.10 input, $0.50 output, $0.01 cache read and $0.125 five-minute cache write. No caching is requested. One-hour caching or uncertain provider costs fail closed. The adapter also supports `claude-sonnet-5-5`, but it cannot substitute into this budget/scope.

## Validation and preserved environment

- Rust workspace: **87 passed, 0 failed, 88 opt-in tests ignored** across 42 test binaries.
- Provider suite including local HTTP mocks: **33 passed**, no real provider requests. The initial sandbox run denied loopback socket creation; the repeat with local-socket permission passed all tests.
- Credential tests: **6 passed**, including non-echoing terminal enrollment and safe error handling.
- Evaluation/permission/replay tests: **9 passed** after making missing-workspace fixtures independent of the enrolled public workspace.
- Real disposable PostgreSQL read-only checks: exact database/system identity, paused agent/owner scope, original unpaid input/job/expiry, independent zero-spent budget, no calls/reservations/cycles/actions/schedules/economic entries, and all **12 migration checksums** verified.
- All **26 protected primary/backup/accounting hashes** matched the starting baseline. Primary PostgreSQL was never started or queried; both clusters are stopped. Historical $0.004048 remains quarantined, unchanged.
- No frontend or Rust source changed in this workspace-enrollment step. Existing frontend validation remains applicable; no browser redesign or new UI testing was necessary.
- Full-history and secrets scans passed before commits; commit hooks and post-commit audits are required. Ignored local evidence is under `artifacts/claude-workspace-readiness/`. No artifact, key or database file is included in commits.

Remaining human steps: private key entry; separate metadata-request approval; verify account credit/usage limits; review this new scope; prepare the authorized disposable passive API; then give fresh explicit paid execution approval before job expiry. No continuous inference, primary activation, worker activation, real financial activity, push or deployment is authorized.
