# Claude local credential enrollment — no paid execution

Prepared 2026-11-01. The Anthropic Messages adapter already exists in `crates/providers/src/anthropic.rs`; OpenAI support is preserved. No authenticated Anthropic request, generation, API startup, worker activation, primary-database mutation, push or deployment was performed.

## Private file and secure loading

Use **`~/Desktop/supa-agent/.env.claude`**, a dedicated private file already covered by Git's `.env.*` ignore rule. It is untracked and must remain so. This avoids changing `.env.eval` or any existing OpenAI configuration. No empty credential file or placeholder key was installed during preparation.

`scripts/claude-local-credentials.py` provides terminal-only hidden key enrollment. It refuses non-TTY key input and suppresses echo through Python's terminal password reader; warnings about unsupported hidden entry fail closed. There is no key command-line argument. Invalid arguments and errors are sanitized rather than rendering secrets.

Writes are atomic and fsynced, use mode **0600**, reject existing unsafe permissions, symlinks, hardlinks, foreign ownership and unexpected values, and retain the existing Claude workspace/model values. Only literal `ANTHROPIC_API_KEY`, `ANTHROPIC_WORKSPACE_ID`, `ANTHROPIC_MODEL` entries are allowed. The utility checks that the exact private path is ignored and untracked before enrollment. It never stores inference-enable flags, budget grants or OpenAI variables.

The Rust adapter now loads this same private file only after the existing explicit Claude/controlled-inference enablement guards pass. It checks file ownership and restrictive permissions, rejects links/duplicates/shell syntax, bounds the file size, zeroizes credential buffers and never modifies the process environment. Explicit environment values take precedence. `ANTHROPIC_ENV_FILE` may select another reviewed server file; an empty value disables file loading. The enrollment utility always manages the default `.env.claude` path. No runtime was started or configured to permit paid inference here.

## Human key entry

Run these commands in **your own interactive terminal**, not in chat:

```bash
cd ~/Desktop/supa-agent
python3 scripts/claude-local-credentials.py set-key
python3 scripts/claude-local-credentials.py check
```

Paste the key only at `Anthropic API key (hidden):`. The prompt does not echo it, and the secret is not a shell command or argument. Do not paste it into Codex, logs or Git. The helper sets a missing model to the prepared `claude-haiku-5-5`; it preserves an existing model/workspace and does not enable inference.

`check` prints presence/format/source booleans and the public model ID only. Format is not authentication. If it reports `process_environment`, an explicitly exported value takes precedence over the private file; it does not claim the file's key is the effective one. The latest preparation check reported **no private Claude file, no key, no workspace**. Key entry remains a human action; it was not falsely reported as completed.

After independently identifying the intended public workspace, it can be recorded without exposing a key:

```bash
python3 scripts/claude-local-credentials.py set-workspace \
  --workspace-id <VERIFIED_wrkspc_ID> --model claude-haiku-5-5
```

This is a local configuration operation only. It does not verify membership, update the frozen evaluation scope or authorize any provider request. Workspace ID remains unassigned unless the operator supplies it through the established review process.

## Authentication request prepared, not authorized or executed

The helper contains only a generation-free model metadata GET, never a Messages POST. It pins `api.anthropic.com`, sends the bearer key and explicit workspace/version headers, uses a 15-second socket timeout, limits the body to 16 KiB, follows no redirects and makes no retries. It returns sanitized status/access booleans, never raw bodies, credentials or headers.

For review only, after key enrollment and separate explicit operator approval of the workspace and model:

```bash
python3 scripts/claude-local-credentials.py probe \
  --workspace-id <REVIEWED_wrkspc_ID> \
  --model claude-haiku-5-5 --authorize-metadata-check
```

The explicit public arguments must match effective local configuration before a connection is constructed. The approval flag is mandatory. **This command was not run against Anthropic.** HTTP 200 with matching model metadata would verify endpoint-specific access only; generation permission, billing credit and usage limits remain separate questions. The reported $10 balance is not verified by local key presence or metadata access.

## Models and pricing rechecked

Official [model documentation](https://platform.claude.com/docs/en/models/haiku-5-5/overview), [pricing](https://platform.claude.com/docs/en/about-claude/pricing) and [model metadata contract](https://platform.claude.com/docs/en/api/http/models/retrieve) were opened on 2026-11-01. Public documentation access is distinct from an authenticated account request.

The current adapter allowlist and standard global USD rates per million tokens remain:

| Model | Input | Output | Cache read | 5-minute cache write |
| --- | ---: | ---: | ---: | ---: |
| `claude-haiku-5-5` | $0.10 | $0.50 | $0.01 | $0.125 |
| `claude-sonnet-5-5` | $2.00 | $10.00 | $0.10 | $2.50 |

Haiku's rates above apply up to 100,000 input tokens; the prepared bound is 24,192. No caching is requested. One-hour caching is unsupported and produces uncertain accounting instead of being priced as five-minute caching. Model availability in documentation does not establish access for the new key. No automatic model substitution is allowed. Sonnet exceeds this small evaluation's request ceiling and requires a separate reviewed budget/scope.

## Bounded evaluation retained

The existing independent Claude plan is unchanged: `config/claude-work-evaluation-plan.json`.

- Budget `3bae1525-7e83-4220-9682-d30d06cc06f2`: **$0.005 ceiling, $0 committed**.
- Exactly **one** future `claude-haiku-5-5` generation; **$0.004** request ceiling.
- Token bounds: **24,192 input / 1,024 output**; conservative reservation **$0.003536**, not incurred.
- Agent `1d892448-ce83-4e38-a2b9-fc166411b15a` remains paused; job `fd408604-98d9-48e4-a2b0-c4cd92a3a3e0` remains created, unpaid and simulated.
- Original supplied-document task and source-line evidence validation retained. Memory disabled; only `inspect_input`, `extract_evidence`, `check_requirements` are allowed.
- Input SHA-256: `613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`.
- Current unapproved scope SHA-256: `b754f7ef2c575e30874edb5b9b5a3bd47eb1cdce015f384b5459db6472765644`.
- Expiration unchanged: **2026-11-02 13:30:58.948063 WAT**. No renewal, new nonce or replacement identity was made.

A real read-only check of the exact disposable PostgreSQL cluster reconfirmed the budget, paused agent, unpaid job, absence of requests/revenue and all 12 migration checksums. The cluster was stopped afterward. The workspace remains unassigned in the frozen plan's legacy `project_id` field, so execution readiness remains false.

Network-free review:

```bash
python3 scripts/claude-local-credentials.py check
python3 scripts/claude-work-preflight.py
```

Adding a workspace to the private credential file does not silently change this plan. The eventual evaluation requires a separately reviewed workspace-bound plan and new digest, approved passive API/authentication setup, verified account/billing limits and fresh paid consent. The [provider report](anthropic-controlled-provider.md) contains the protected execution command shape. Do not run a paid command or silently extend the job as part of credential enrollment.

## Validation and preserved state

- **6 synthetic credential tests passed**, including a real pseudoterminal test that confirmed the key is not echoed, non-TTY refusal, secret-safe argument errors, atomic/private updates, unsafe configuration rejection, approval-before-connection and mocked single-GET responses.
- **33 provider tests passed**, including local HTTP mocks; no real Anthropic/OpenAI requests. The new Rust private-file test rejects shared files, duplicate keys, shell syntax, unknown settings and symlinks.
- **9 network-free evaluation tests passed**. Rust formatting and strict provider Clippy checks passed.
- Existing OpenAI private configuration remained byte-for-byte unchanged. All **26 protected primary/backup/accounting hashes** remained unchanged. Primary PostgreSQL was never started or queried, and the historical **$0.004048 hold** remains quarantined and untouched.
- No frontend, migrations, budget authorizations, owner grants, worker settings or financial permissions were changed. Local evidence remains ignored under `artifacts/claude-credentials/`.

The key and workspace still require human entry/selection. Independent account access, credit and billing limits remain unverified. No external authentication check is authorized by this report. All source changes use separate one-file Conventional Commits on `main`; final full-history and secrets audit outcomes are reported after commits. No push or deployment is authorized.
