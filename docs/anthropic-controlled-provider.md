# Anthropic Claude — Controlled provider integration

Implemented 2026-11-01. **No real Anthropic authentication probe, Messages request or paid inference was performed.** The reported $10 account credit is operator-supplied information, not verified API billing evidence. OpenAI support, its reviewed plan, and historical records remain in place.

## Implemented path and safety

`crates/providers/src/anthropic.rs` implements the existing `ControlledProvider` interface for explicit single-session goal analysis and supplied-input economic work. It intentionally does not implement the autonomous `Reasoner` interface, so it cannot be selected by persistent workers or the recurring reasoning loop.

- Server-side `SUPA_CONTROLLED_PROVIDER=anthropic` selects Claude; absence preserves the existing OpenAI default. Unknown selections fail closed. Models cannot select providers through proposed actions or frontend identity fields.
- Uses the Messages endpoint, bearer API authentication, `anthropic-version: 2026-10-21`, and the explicit `anthropic-workspace-id` header. The API makes the workspace header optional for workspace-specific credentials; Supa requires an explicit reviewed workspace for account-scoped authorization and audit.
- Production transport is pinned to `https://api.anthropic.com`; only test builds can replace endpoints with local mock HTTP. Redirects, automatic retries and proxy forwarding are disabled. Total transport timeout is 25 seconds; metadata/response bodies are capped at 16 KiB/128,000 bytes.
- Before a controlled generation, a model metadata GET must match the exact model, structured-output capability and configured token limits. This is access/capability verification, not proof of available credit or spend limits. No real GET ran during implementation.
- Structured JSON uses `output_config.format`. The shared work prompt/schema and the existing `WorkProposal` evidence validator are reused across providers. Tool requests remain application-owned proposals, not provider-hosted tools. There are no server tools, tool-execution loops, cache breakpoints or hidden-thinking output. Thinking is disabled and standard service capacity requested.
- Factual observations must match exact approved source-line quotations. Wrong tools, invented memories, extra response fields and unsupported evidence fail validation. Interpretations and summaries still require human review; literal citation checks do not prove semantic truth.
- Existing owner authentication, permissions, finite consent, exclusive execution gate, durable nonce, leases, reservation, revocation and uncertain-charge recovery stay in force. There is no provider fallback or automatic paid retry.

## Workspace and accounting audit

New sessions retain an optional `provider_scope` in their existing JSON record. For Claude it is the configured workspace; for OpenAI it is the configured project. Metadata verification remains a separate status flag. Historical records without this field deserialize unchanged and are not backfilled. Work records identify `provider_evidence: anthropic`. Existing frontend contracts and text now distinguish Claude from GPT while preserving the approved layout and animations. The legacy live-work metadata field `project_id` carries the workspace ID for this provider; no OpenAI project ID is sent to Anthropic.

A budget row is locked before reservation. Budgets with prior sessions from a different model or provider scope are rejected. Legacy unknown scope fails closed on scoped reuse. Anthropic configuration rejects the archived hold's budget UUID, the existing M10A OpenAI budget UUID, and the configured generic OpenAI budget UUID. The proposed evaluation uses an entirely new budget in a separate database. Unused legacy budget rows have no provider-origin column; operators must continue using freshly created, independently reviewed budget identities rather than assigning existing unspent rows. No schema migration or historical rewrite was introduced.

Known-priced token usage survives refused, truncated or invalid structured responses so a failed task does not erase provider expense. Missing usage, wrong-model envelopes, unknown service/region/speed pricing, server-tool charges, ambiguous cache TTL or one-hour cache writes are treated as uncertain and retain the existing conservative hold for operator review. One-hour caching is deliberately unsupported rather than priced as five-minute writes.

Anthropic reports uncached input separately from cache read/write tokens. The adapter normalizes total input as their checked sum; the existing cost function then prices each non-overlapping category once and rounds upward. No caching is requested. If five-minute cache writes appear, their explicit TTL breakdown must match. The existing reservation uses the highest supported input-category rate. Actual provider usage is distinct from simulated operating funds, unpaid jobs and hypothetical service prices.

## Official contract and pricing basis

Official documentation was opened during implementation:

- [Messages API and workspace header](https://platform.claude.com/docs/en/api/http/messages/create)
- [Model metadata](https://platform.claude.com/docs/en/api/http/models/retrieve)
- [Structured outputs](https://platform.claude.com/docs/en/build-with-claude/structured-outputs)
- [Models and IDs](https://platform.claude.com/docs/en/models/overview)
- [Pricing](https://platform.claude.com/docs/en/about-claude/pricing)
- [Prompt caching](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)

The adapter allowlists these exact models and standard global USD prices per million tokens, checked independently on 2026-11-01:

| Model | Uncached input | Output | Cache read | 5-minute write | 1-hour write, unsupported |
| --- | ---: | ---: | ---: | ---: | ---: |
| `claude-haiku-5-5` | $0.10 | $0.50 | $0.01 | $0.125 | $0.20 |
| `claude-sonnet-5-5` | $2.00 | $10.00 | $0.10 | $2.50 | $4.00 |

Haiku's listed higher tier above 100,000 input tokens is outside the fixed 24,192 input-token bound. No Batch, priority, fast-mode, regional premium or hosted-tool pricing is assumed. Availability in a public table does not establish access in this account. Future price changes require review; no model alias substitution or automatic upgrade is permitted. Sonnet needs a separately approved larger budget and scope: at 1,024 output tokens its conservative reservation is $0.070720, exceeding this evaluation's ceilings.

## Private configuration requirements

Configure only through a private server environment, never frontend variables, source files, command-line secret arguments or Git:

```text
SUPA_PROVIDER=offline
SUPA_API_SCHEDULER_ENABLED=false
SUPA_CONTROLLED_PROVIDER=anthropic
SUPA_ALLOW_ANTHROPIC=true
SUPA_CONTROLLED_INFERENCE=true
SUPA_CONTROLLED_WORK=true
ANTHROPIC_API_KEY=<private Claude API key>
ANTHROPIC_WORKSPACE_ID=<independently verified wrkspc_... ID>
ANTHROPIC_MODEL=claude-haiku-5-5
ANTHROPIC_LIVE_BUDGET_ID=3bae1525-7e83-4220-9682-d30d06cc06f2
ANTHROPIC_LIVE_BUDGET_MICROUSD=5000
ANTHROPIC_MAX_OUTPUT_TOKENS=1024
```

These settings describe a future approved controlled evaluation, **not settings installed or enabled here**. Also require the exact disposable database URL, loopback bind `127.0.0.1:55684`, separate scoped test-owner/service authentication, and a private runner `SUPA_WORK_OPERATOR_TOKEN`. No real owner credentials were issued. Keep wallet configuration absent, continuous scheduling disabled and the primary environment untouched.

A private owner-only configuration presence check found **no Anthropic API key and no workspace ID** in the current environment or private `.env.eval`. It emitted booleans only, made no network requests, and did not modify the file. Do not reuse OpenAI credentials, budgets, ChatGPT/Claude subscriptions, personal wallet material or archived evaluation holds as proof of Claude API access or credit.

## Independent disposable evaluation

Plan: `config/claude-work-evaluation-plan.json`.

| Item | Prepared value |
| --- | --- |
| Provider/model | Anthropic / `claude-haiku-5-5` |
| Workspace | **Unassigned**, represented by legacy `project_id: null` |
| Budget | `3bae1525-7e83-4220-9682-d30d06cc06f2` |
| Budget ceiling/committed | **$0.005 / $0** |
| Maximum generations / request ceiling | **1 / $0.004** |
| Token bounds | **24,192 input / 1,024 output** |
| Conservative reservation, not incurred | **$0.003536** |
| Agent | `1d892448-ce83-4e38-a2b9-fc166411b15a`, paused |
| Job | `fd408604-98d9-48e4-a2b0-c4cd92a3a3e0`, created, unpaid, simulated, revision 0 |
| Test owner | `d5536ed1-b84e-4ec6-b6d2-4aa079176b19` |
| Database | `supa_claude_3f722ca85789_test` |
| Data directory | `/tmp/supa-claude-pg-3f722ca85789/data` |
| Socket / port | `/tmp/supa-claude-pg-3f722ca85789/s` / **55683**, TCP disabled |
| System identifier | `7703179569575286737` |
| Job expiry, Lagos time | **2026-11-02T13:30:58.948063+01:00** |

Task: compare supplied recovery strategies using the same three non-sensitive evidence lines as M10A: replication reduces downtime but copies accidental deletion; offline snapshots preserve earlier state but need tested restoration; neither document provides measured recovery times. Memory is disabled. Only `inspect_input`, `extract_evidence`, `check_requirements` are proposed. No external research, payment, commercial completion or earned revenue is represented.

The exact 293-byte task input remains:

`613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`

Current **unapproved** Claude scope SHA-256:

`b754f7ef2c575e30874edb5b9b5a3bd47eb1cdce015f384b5459db6472765644`

This is a separate scope from OpenAI's unchanged `cf2f80163a97e914725972664c33962cb615cb43a77693573ec759553fb2cef2`. The original source/evidence restriction remains intact.

The earlier interrupted rehearsal's `/tmp` database and build output were no longer present on resume. Its ignored preparation evidence was retained, but it was not treated as current state or recovered history. A new disposable cluster and new identities, nonce, budget and scope were prepared; the superseded unapproved plan remains in ignored evidence. No missing original OpenAI database or financial history was recreated. The current Claude budget survived a real PostgreSQL restart, all 12 migration checksums matched, and the cluster is stopped. No paid consent, inference reservation, live session, cycle, scheduling authorization or economic entry was created in this prepared database.

## Validation

- Rust workspace all-target unit/test compilation: **86 passed, 88 opt-in tests ignored**; optional socket/database suites were exercised separately below. Counts overlap with provider unit tests.
- Full provider suite including local wire mocks: **32 passed**. Six Anthropic tests cover known rates/cache arithmetic, unknown usage/prices, workspace/model metadata, all nine structured-response outcomes, 401/403/429/500 errors, bounded single-attempt timeout, oversized request rejection and bounded memory reasoning. No real provider was contacted.
- Disposable PostgreSQL suites: **6 controlled-session + 5 work tests passed**. They cover owner/role boundaries, consent, cumulative budgets, concurrency, duplicate prevention, permission revocation, crash/lease recovery, unknown charges, evidence rejection and no paid replay. The new synthetic-Claude case verifies durable workspace origin and rejection of both model and workspace changes on an already-used budget. Jobs remain unpaid and non-commercial.
- Network-free Python runner tests: **9 passed**; OpenAI preflight regression: **4 JavaScript passed**, preserving the existing OpenAI digest.
- Frontend production build, TypeScript checks, ESLint, reference syntax checks and reference unit suites passed. **5 Playwright contract tests passed**, including Claude-origin/workspace projection and consent. These tests use local fixtures; they do not demonstrate actual Claude quality or real UI activity.
- Rust formatting and **Clippy with `-D warnings`** passed. Small equivalent lint cleanups grouped the private settlement reports and removed redundant casts/allocations; accounting decisions and financial limits are unchanged.
- All **26 protected primary/backup/accounting hashes** remained unchanged. The primary PostgreSQL cluster was never started, connected to or migrated. The unresolved historical **$0.004048 hold**, archived $0.10 ceiling and all quarantined evidence remain untouched.

Sanitized evidence is ignored under `artifacts/claude-provider/final-validation/`; private config, database files, raw credentials and generated artifacts are not committed. There are no before/after design changes or fabricated screenshots. This milestone uses mocked responses and genuine disposable database checks; **actual Claude task quality, authentication, credit, billing, access and paid cost remain unmeasured**. Compare any eventual paid result against the same deterministic rubric and label interpretations for human review; no model superiority is asserted.

## Commands and remaining approvals

Network-free readiness:

```bash
python3 scripts/claude-work-preflight.py
python3 scripts/live-work-evaluation.py --plan config/claude-work-evaluation-plan.json
```

Both report the unassigned workspace blocker. The read-only database variant, only while the exact disposable cluster is separately running, is:

```bash
python3 scripts/claude-work-preflight.py --check-database
```

**Do not execute paid inference yet.** Required next steps: provide the intended public workspace ID through a trusted process; install a valid private Anthropic API key securely; independently verify API credit, workspace/model access and usage limits; prepare the isolated bounded API authentication; update and review the workspace-bound scope with a **new digest**; then give fresh explicit consent for exactly one generation and these token/cost ceilings. A generation-free access probe may be separately authorized; none was performed here.

Protected paid command shape, **not run and not currently executable**:

```bash
python3 scripts/live-work-evaluation.py \
  --plan config/claude-work-evaluation-plan.json \
  --run --authorize-paid \
  --scope-sha256 <NEW_REVIEWED_WORKSPACE_BOUND_SHA256> \
  --model claude-haiku-5-5 --max-cost-microusd 4000
```

Do not use the current unassigned-workspace digest as paid consent, silently renew the job, reuse the old OpenAI digest or create a replacement request after an ambiguous result. Any expiry or protected scope change requires fresh preparation and review. API startup must use only the exact disposable database, with the environment above injected privately; the primary database, real owner onboarding, agent/worker activation and deployment each retain their separate approval boundaries.

Changes use exactly one file per Conventional Commit on `main`. Final complete history/secrets audit and working-tree status are reported after commits. **No push or deployment is authorized or performed.**
