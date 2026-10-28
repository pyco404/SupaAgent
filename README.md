# Supa Agent

**Born to think. Built to survive.**

Supa Agent is a Solana-native autonomous-agent project. Milestones 1 and 2 implement a local Rust backend with durable identities, bounded reasoning cycles, permissioned tools, provenance-aware memory, prioritized goals, personality, metered OpenAI calls, draft-only social content, and simulated economics. Milestone 3 adds encrypted Devnet wallet identities, read-only payment monitoring and separate verified test-token accounting.

Wallet support is opt-in and Devnet-only. No transaction signing/broadcast, Jupiter trade, X publisher, shell tool, autonomous agent-creation tool, or code-modification tool is included. Milestone 4 adds a local, read-only Next.js console with the requested near-black, mint and pixel-typographic design. See [the website implementation and actual browser evidence](docs/milestone-4.md).

## Local setup

Requires Rust 1.93+, PostgreSQL 16+, and a local PostgreSQL database. The optional website requires Node.js 22.18+ (validated with Node.js 24.22.1 and npm 11.19.1). Docker is optional and not required. See [Ubuntu and isolated PostgreSQL setup](docs/local-setup.md).

For the website, run `npm ci --prefix web`, then `npm run build --prefix web`. Use `./scripts/web-local` with the existing private `.env.devnet` to start the loopback-only console and offline API against the inactive, dedicated Devnet agent. The launcher creates ignored, mode-600 `web/.env.local` credentials if absent; it never loads `.env.eval`. Open `http://127.0.0.1:3000` and use the operator credentials from that private file. See [manual/disconnected setup, authentication and tests](docs/milestone-4.md#local-commands).

```bash
export DATABASE_URL='postgresql://supa:your-local-password@127.0.0.1:5432/supa_agent'
export SUPA_API_TOKEN="$(openssl rand -hex 32)"
export SUPA_PROVIDER=offline
export SUPA_BIND=127.0.0.1:8080
cargo run -p supa-api
```

Migrations run at startup. Configuration is read from exported environment variables; `.env` is not automatically loaded. See [.env.example](.env.example). The API rejects non-loopback bind addresses, requires a bearer token even for health checks, and has no CORS policy permitting browser origins. Keep credentials out of source control and request bodies.

Offline mode exercises the runtime without network reasoning. It reads memory and records an event-based observation when granted those capabilities. It does **not** pretend to generate witty posts. OpenAI mode generates plans and contextual tweet drafts dynamically; see [provider setup and cost boundaries](docs/local-setup.md#openai-opt-in).

## First agent

```bash
curl -sS http://127.0.0.1:8080/agents \
  -H "Authorization: Bearer $SUPA_API_TOKEN" \
  -H 'Content-Type: application/json' \
  --data-binary @config/supa.json
```

Copy the returned ID into `AGENT_ID`. Agents begin in `created` and have zero simulated balance. Funding alone never activates an agent.

```bash
export AGENT_ID='returned-agent-uuid'
curl -sS "http://127.0.0.1:8080/agents/$AGENT_ID/simulated-funding" \
  -H "Authorization: Bearer $SUPA_API_TOKEN" -H 'Content-Type: application/json' \
  -d "{\"kind\":\"seed\",\"amount_microusd\":1000000,\"idempotency_key\":\"$(cat /proc/sys/kernel/random/uuid)\",\"simulated\":true}"
curl -sS "http://127.0.0.1:8080/agents/$AGENT_ID/goals" \
  -H "Authorization: Bearer $SUPA_API_TOKEN" -H 'Content-Type: application/json' \
  -d '{"description":"Review verified execution events and learn from mistakes."}'
curl -sS "http://127.0.0.1:8080/agents/$AGENT_ID/lifecycle" \
  -H "Authorization: Bearer $SUPA_API_TOKEN" -H 'Content-Type: application/json' \
  -d '{"state":"active","reason":"Operator authorizes local simulated cycles"}'
```

The scheduler now runs this agent at its configured interval. Pause it using the lifecycle endpoint with `state: "paused"`. Paused, suspended and terminated agents never become active on restart. Only a token-authenticated operator can activate eligible agents. Termination is irreversible in this API.

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
# Explicit opt-in: use a separate disposable database, never production.
export TEST_DATABASE_URL='postgresql://supa:your-local-password@127.0.0.1:5432/supa_agent_test'
cargo test --workspace --locked -- --ignored
```

Regular tests need no database or API key. Database tests are explicitly marked ignored in the regular suite; the second command runs them and fails if `TEST_DATABASE_URL` is absent or inaccessible. Tests create uniquely identified records; they do not truncate databases. Reset the disposable database yourself if needed.

## Mandatory Git workflow

Work directly on `main`, with exactly one explicitly staged file and one descriptive Conventional Commit per commit. Use `./scripts/commit-one <exact-file-path> '<type>(<scope>): <description>'`. Hooks enforce the one-file rule and secrets scanning; public push remains unapproved and blocked. See [the persistent Git workflow](docs/git-workflow.md) and root `AGENTS.md`.

## Milestone 3A — local Git and real Devnet checks

Git is available through `./scripts/git-local`; separate ignored metadata preserves the original read-only `.git`. Local pre-commit scanning and a first-push guard are installed. No public push has occurred.

The dedicated encrypted Devnet wallet and canonical USDC mint were checked against actual RPC in two processes. The wallet is unfunded; actual receipt verification awaits separately approved external test funding. See [the Milestone 3A report and exact commands](docs/milestone-3a.md).

## Milestone 3 wallets

See [wallet architecture, configuration, API, financial policies and mocked Devnet validation](docs/milestone-3.md). Wallet support defaults to disabled. Test tokens never fund paid model calls, and no signing or publishing is enabled.

## Project documentation

- [Personality, event selection, social safety and optional creative evaluation](docs/personality-and-social.md)

- [Architecture and failure recovery](docs/architecture.md)
- [Local setup and commands](docs/local-setup.md)
- [HTTP API](docs/api.md)
- [Security and economics](docs/security-and-economics.md)
- [Frontend and future integration requirements](docs/future-integrations.md)
- [Milestone 1 validation baseline](docs/milestone-1.md)
- [Milestone 2 implementation report, tests and demonstration](docs/milestone-2.md)

Use Conventional Commits on `main` when Git metadata is functional. Never force-push. The original workspace contains an empty read-only `.git` placeholder; it has been preserved. Milestone 3A adds working separate metadata through `./scripts/git-local`; public push remains blocked pending approval.

## Milestone 2 controls and accounting

`POST /agents/{id}/pause`, `/resume` and `/emergency-stop` accept `{ "reason": "Operator reason" }`. These routes require the operator token. Pause/stop invalidate old work, and only an explicit authorized resume reactivates an eligible agent. Manual cycles obey the configured interval.

`GET /agents/{id}/inference` exposes durable attempts, usage, safe failures and estimated provider-cost accounting separately from the simulated ledger. `GET /agents/{id}/goal-evaluations` exposes evidence-linked model assessments. Model memories are hypotheses; only system cycle observations are verified facts. Operators can set goal priority through `POST /agents/{id}/goals/{goal}/priority` with `{ "priority": 10 }` (0–10).

New cycle settings are `retry_budget` (0–2, only explicit 429 rejections), `max_provider_calls` (1–9), and `max_cycle_tokens` (1,000–500,000). The example agent permits goal evaluation and conservation. The latter only slows future cycles. OpenAI live mode additionally requires a durable budget ID, a small spending ceiling and operator-supplied model rates; see local setup. No live calls occurred during validation.

Run `python3 scripts/verify-demo.py` to verify the saved offline demonstration artifacts. It records 15 completed cycles, 30 memory entries and five drafts. Draft prose was newly authored by the coding assistant from inspected persisted events, then deterministically replayed through the tools; it is not evidence of live OpenAI autonomy. See the full report for provenance and limitations.

## Milestone 2A live evaluation

The opt-in evaluator is ready for offline preflight; no live intelligence result is claimed. Routine reasoning/drafts default to `gpt-6-luna`; explicit complex planning can use `gpt-6.1-sol`. Native model capabilities and prices were checked against official documentation, not account access. Cache-write accounting and per-model spending reservations are enforced independently of model decisions.

```bash
cargo run -p supa-runtime --example live_eval --locked -- --check
```

See [Milestone 2A readiness and the exact authorized launch procedure](docs/milestone-2a.md). A key alone never enables this evaluator; paid execution also requires a stable budget UUID, ceiling, dedicated local database, environment opt-in and `--run --authorize-paid`. No paid evaluation was performed in this environment.

The live evaluator now auto-loads private, ignored `.env.eval` (mode 0600) so its dedicated `supa_eval` database setting persists across terminal sessions. Exported variables take precedence. The API server still uses exported configuration. See [the configured database setup](docs/local-setup.md#configured-dedicated-evaluation-database). Paid execution remains disabled by the local configuration.
