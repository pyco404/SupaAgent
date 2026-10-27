# Milestone 2A: live evaluation readiness

Initial offline validation was completed October 27, 2026. A subsequent operator-authorized attempt reached OpenAI and received HTTP 401; no validated model output or live drafts were produced. See the [failure investigation and current retry requirements](live-evaluation-debug.md). Readiness observations below describe the original offline baseline unless explicitly updated.

## Model and API verification

The official model pages list Responses API, structured outputs and function calling for both requested models. Documentation does not prove that a particular account can access them; account access remains unverified without credentials.

| Role | Default | Standard input / cached read / 30m cache write / output, USD per million tokens |
|---|---|---|
| Routine reasoning and drafts | `gpt-6-luna` | 0.10 / 0.01 / 0.125 / 0.50 |
| Explicit complex planning | `gpt-6.1-sol` | 2.00 / 0.10 / 2.50 / 10.00 |

Sources: [Luna model](https://developers.openai.com/api/docs/models/gpt-6-luna), [Sol model](https://developers.openai.com/api/docs/models/gpt-6.1-sol), [Responses API](https://developers.openai.com/api/reference/typescript/resources/responses), and [prompt-cache accounting](https://developers.openai.com/api/docs/guides/prompt-caching). Pricing assumptions were verified on the date above; review them before a future paid run. They are standard short-context text rates, excluding taxes and other account-wide costs. This implementation's bounded requests stay below long-context price thresholds and enable no billed external tools.

The existing adapter already used the correct POST `/v1/responses`, `store: false`, and `text.format` strict JSON schema shape. It parses assistant `output_text`, fails on refusal/incomplete output, and keeps usage separate from private model text. Agent tool requests remain structured action proposals executed by the existing Rust permission registry. Native API function calling is documented as supported, but built-in tools, shell, web search, publishing and signing tools are not exposed by this integration.

New defaults use Luna. `OPENAI_MODEL` can select either verified model for routine work. `OPENAI_COMPLEX_MODEL` defaults to Sol, but **only** `SUPA_PLANNING_MODE=complex` routes planning to it. Drafts continue to use the routine model. No model output can trigger an automatic upgrade. Unknown model names fail closed.

Requests now explicitly pin `service_tier: default`, low reasoning effort, and explicit-only 30-minute caching with no breakpoints, avoiding inherited premium tiers or implicit cache writes. Input/output caps, transport/runtime timeouts, redirect rejection and the fixed OpenAI HTTPS destination remain in place.

## Independent spending controls

Each actual invocation must pass the runtime's PostgreSQL reservation before HTTP starts. The stable budget UUID and ceiling are shared across agents and model routes. Model choice is reflected in the reservation's policy, including complex-planning rates. Changing model cannot bypass the common budget.

The earlier limiter omitted cache-write usage. It now partitions input into ordinary, cached-read and cache-write tokens without double counting, rounds costs upward, and reserves using the highest applicable input price. Unknown/timeout/crashed requests keep their ceiling across restarts. Explicit 429 rejection can release its provider reservation; simulated per-attempt fees follow their separate hypothetical policy. Token, call and cycle limits still apply, and this evaluator disables retries.

A local spending guard is an estimate based on a verified price schedule and token bounds, not an authoritative invoice or a cap on unrelated account activity, taxes or future provider price changes. Optional existing `SUPA_*_RATE_MICROUSD_PER_MILLION` overrides can only increase the verified baseline. The new write-rate override is `SUPA_CACHE_WRITE_RATE_MICROUSD_PER_MILLION`. Underquoted, malformed or contradictory overrides fail closed. Global overrides apply to both selected routes; leave them unset to use each model's own table entry.

## Evaluator behavior

The command is an example executable in the existing runtime crate, preserving all six crates:

```bash
cargo run -p supa-runtime --example live_eval --locked -- --check
```

`--check` constructs no HTTP provider, opens no database connection and makes no paid request, even when credentials exist. It prints only non-secret defaults, readiness flags and missing requirements. `configuration_ready` is separate from `authorization_supplied`; a dry run never authorizes spending. `--run` alone is denied. Paid execution requires **both** `--run --authorize-paid` and explicit environment configuration. The evaluator also requires a separately named, loopback-only `SUPA_EVAL_DATABASE_URL`; it will not accidentally reuse an application's production `DATABASE_URL`.

Once authorized, the evaluator creates its own simulated agent with `scheduler_enabled=false`, persists a hypothetical seed/revenue and a goal, then:

1. Runs three actual model-driven planning cycles through the existing runtime. All proposed actions face normal permission and resource checks.
2. Computes the deterministic provider's plan on the identical model-visible context for each live planning invocation. The saved comparison contains safe tool names and usage/validation outcomes, not prompts or chain-of-thought.
3. Requests five additional, explicitly operator-driven draft cycles through the same registry and spending guard. These rounds are labeled separately from autonomous decisions. There is no replay fixture, manually composed prose, or offline tweet fallback.
4. Requires at least five distinct drafts, valid selected event references, length/simulation labels and canonical claim evidence before machine validation succeeds.
5. Suspends its agent on normal completion or handled failure. The agent is excluded from the server scheduler even after an abrupt crash, preventing accidental later continuation.

Reports are created privately (0600 on Unix), never overwrite an existing report, and default to `artifacts/live-eval/<budget-id>.json`. If the default exists, an exclusive `<budget-id>.attempt-<attempt-uuid>.json` filename preserves earlier evidence while retaining the original spending budget. An existing explicit `SUPA_EVAL_OUTPUT` path fails with `report_already_exists`. They include the agent ID, real response/usage evidence, durable accounting, safe comparisons, actions and drafts. Quality and semantic factual review remain a human task; tool-selection differences alone are not an intelligence score. For another authorized attempt, retain the same budget UUID; automatic attempt filenames preserve spending history. If specifying `SUPA_EVAL_OUTPUT`, select an unused path.

## Factual draft checks

Live drafts now carry `claims` with event UUIDs and canonical facts supported by event metadata. The adapter supplies only permitted canonical observations and requires their exact text in the draft. Numeric embellishments are rejected; simulated expense/revenue observations explicitly say simulated. These canonical observations are factual constraints, not tweet or joke templates. The model generates its own subjective humor around them.

The parser verifies each declared claim against an owned persisted event, requires all referenced events to have corresponding claims, and retains human review before any publication. It cannot mathematically prove that unconstrained natural-language humor contains no additional factual assertion. A real evaluation must review those outputs semantically rather than treating a source UUID as proof of all prose. No draft is published.

## Exact authorized launch procedure

Do not execute the final command until approving the paid run. It authorizes an estimated **100,000 micro USD ($0.10)** ceiling. Use a dedicated local evaluation database. With the existing isolated development PostgreSQL cluster on localhost port 55432:

```bash
createdb -h 127.0.0.1 -p 55432 supa_agent_eval
export SUPA_EVAL_DATABASE_URL='postgresql://pikoo@127.0.0.1:55432/supa_agent_eval'
read -rs -p 'OpenAI API key: ' OPENAI_API_KEY
export OPENAI_API_KEY
export SUPA_ALLOW_OPENAI=true
export OPENAI_MODEL=gpt-6-luna
export SUPA_PLANNING_MODE=routine
export SUPA_MAX_OUTPUT_TOKENS=2048
# Retain this budget ID across restarts and retries; do not create a fresh ID to reset spending.
export SUPA_LIVE_BUDGET_ID=ad91e475-f1da-495a-8247-139643d2d8b1
export SUPA_LIVE_BUDGET_MICROUSD=100000
cargo run -p supa-runtime --example live_eval --locked -- --check
# Executing this final command is the explicit paid authorization:
cargo run -p supa-runtime --example live_eval --locked -- --run --authorize-paid
```

Do not put the key in a command argument, source file, report, chat or committed `.env`. The shell prompt above does not echo it. The command does not enable the regular server's provider. If using a different PostgreSQL role/port, adjust only the dedicated local database URL. Do not recreate an existing database.

Complex planning can be evaluated in a **separately authorized** run with `SUPA_PLANNING_MODE=complex` and `OPENAI_COMPLEX_MODEL=gpt-6.1-sol`; keep the budget identity and understand that the same ceiling may prevent completion. A model without account access or an unsupported response fails safely and remains a failed/partial evaluation, never a reported live success.

## Offline validation and current readiness

The inherited suite passed first: formatting, Clippy, 14 unit tests and 21 offline integration tests. The updated suite passes formatting, Clippy with warnings denied, **21 unit tests** and **23 offline integration tests** (20 PostgreSQL/runtime, one HTTP, two local mock-provider tests). The live evaluator builds successfully. Eight subprocess guard tests prove that credentials alone, a run flag alone, missing authorization, missing ceiling/key/opt-in and underquoted prices cannot start paid execution. Unsupported model configuration is redacted. The previous Milestone 2 demonstration artifacts still verify.

Evidence is saved under [artifacts/live-2a](../artifacts/live-2a/readiness.json). Run `python3 scripts/verify-live-gates.py` after building `live_eval` to repeat the non-paid gate checks. Integration tests still use `TEST_DATABASE_URL` and `--ignored`, never a real OpenAI endpoint.

**Pending:** actual model responses, five real-model drafts, live autonomous results, account-level model access and empirical live-versus-deterministic comparison. These were not run during the original offline baseline. The subsequent authorized attempt failed authentication; current evidence and preserved accounting are documented in the investigation linked above. No X publishing, real financial transactions, Mainnet activity, child-agent creation, production modification or deployment was enabled. Git metadata remains the preserved empty read-only placeholder, so commits are still blocked.

## Dedicated local database setup follow-up

The dedicated `supa_eval` database and restricted login now exist in the previously validated localhost cluster. The current connection is persisted in private ignored `.env.eval`, automatically loaded by the evaluator with shell-export precedence. It contains no database password because the inherited local development cluster uses trust authentication, and no OpenAI credential was available during setup. `SUPA_ALLOW_OPENAI` remains false.

Use the existing `.env.eval` database configuration rather than the earlier illustrative `supa_agent_eval`/administrator-role URL above. The setup preserved all existing project data and credentials. Database connectivity and a rolled-back write succeeded; privilege checks found no write access to existing project databases. See the current local-setup section for details and remaining opt-in requirements. `--check` now reports `openai_api_key_detected` as yes/no and `evaluation_database_configured` without printing credential values.
