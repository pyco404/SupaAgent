# Supa Agent Milestone 2 implementation report

Validated locally on October 27, 2026. The six-crate Rust architecture is retained. No frontend identity or assets were changed. No production infrastructure was deployed.

## What is implemented

The existing provider abstraction now supports metered invocations carrying a typed result and separate token usage. The OpenAI adapter retains strict structured JSON action proposals, fixed HTTPS destination, no redirects, model selection through `OPENAI_MODEL`, environment-only credentials, application timeouts, and draft-only social output. Model text cannot grant tools, change spending policy, publish or create agents.

Input, cached-input and output token counts are parsed independently of model prose. Reported output usage includes reasoning tokens; they are not added again. Usage is retained when an otherwise valid provider response has incomplete or malformed structured output. No prompts, chain-of-thought, response bodies, credentials or generated text are logged. The implementation follows the [official OpenAI token-counting guidance](https://developers.openai.com/api/docs/guides/token-counting) and [Responses API](https://developers.openai.com/api/reference/typescript/resources/responses/methods/create).

Live mode additionally requires a stable operator-selected budget UUID, a limit no greater than 1,000,000 micro USD ($1), and explicit input/cache/output pricing assumptions. PostgreSQL reserves a conservative per-call ceiling before HTTP begins. That reservation is shared across agents using the same budget ID and survives process crashes. Known usage settles the estimate; uncertain calls retain the ceiling and are not replayed. A budget ID cannot silently increase its original limit. These are estimated provider charges based on operator rates, not an authoritative provider invoice or a real-money treasury. No live call was executed during this work.

Cycles enforce a configurable interval, step limit, provider-call limit, token ceiling, timeout and retry budget. Manual API requests obey the interval too. Only explicit HTTP 429 rejections may retry, at most twice with short exponential backoff. Every attempt has its own durable accounting record and hypothetical inference debit. Timeouts, transport errors and uncertain server failures do not retry. Paid operations fail before a provider call when simulated funds, token capacity or estimated live budget are insufficient. Active agents suspend on exhausted treasury/live/resource budgets.

Pause, resume and emergency-stop endpoints complement the existing lifecycle endpoint. Leaving active state fences the old cycle immediately, preventing pause/resume races. Emergency stop is idempotent for suspended agents. Restart recovery marks expired work interrupted and outstanding calls unknown without replaying them or reactivating paused/suspended/terminated agents.

Memory retrieval ranks up to 1,000 retained memories by goal-term relevance, verification class and recency, returning up to 12 to the model. Model-authored lessons remain `hypothesis` even when they cite legitimate events; only system-generated cycle outcomes are `verified_observation`. Old memories default to hypothesis during deserialization. Model-written content never becomes established fact by assertion.

Goals have operator-assigned priority and a separately labeled suggested progress value. The new `evaluate_goal` tool requires permission, valid goal identity and agent-owned persisted evidence. It stores a model assessment, not a verified achievement, and never marks a goal complete. The operator retains completion authority. Personality is separate from capability and financial policy.

Reasoning context now contains clearly simulated financial metrics, revenue, inference/operating expenses, runway assumptions and completed cycle count. The permissioned `conserve` tool can only defer the next cycle; it cannot increase frequency, lower fees, credit money or change limits. Runway retains its documented lifetime gross-burn assumption.

## Validation results

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: passed.
- `cargo test --workspace --locked --offline`: 14 unit tests passed; 21 socket/database tests intentionally opt-in.
- `TEST_DATABASE_URL=… cargo test --workspace --locked --offline -- --ignored`: 21 tests passed: 18 PostgreSQL/runtime, one HTTP router, and two local OpenAI wire-contract tests. No real API calls.
- `cargo build -p supa-api --locked --offline` and `cargo build -p supa-runtime --example demo --locked --offline`: passed.
- `scripts/smoke.py`: actual localhost server startup, scheduled cycle, authentication rejection, pause/resume, emergency stop, abrupt process kill and restart all passed. The suspended agent remained stopped. Structured log parsing and checks for token/database URL/private-goal leakage passed.
- `scripts/verify-demo.py`: persisted demonstration evidence, ledger reconciliation, draft provenance and restart-state assertions passed.

The tests cover independent treasuries, concurrent debit races, idempotency, blocking unaffordable plan/draft calls, authorization denials, continuation after denial, provider failures/timeouts, bounded retries, resource limits, cached-token cost math, incomplete-output usage evidence, budget retention after crashes, confirmed settlement, cross-agent event rejection, priority selection, hypothesis classification, emergency fencing, conservation and old-worker exclusion.

Regular and opt-in test output is saved under `artifacts/validation/`; [results.json](../artifacts/validation/results.json) summarizes the checks. Tests use unique agent records in disposable databases; they do not clear tables or contact production systems.

## Controlled demonstration results

The offline demonstration uses real PostgreSQL persistence and timed cycles. It is not live OpenAI behavior. [The verified summary](../artifacts/demo/verified-summary.json) and [complete final snapshot](../artifacts/demo/final.json) contain the evidence.

| Observation | Actual recorded result |
|---|---:|
| Completed autonomous cycles | 15 |
| Crash-interrupted cycle | 1 |
| Persisted memory entries | 30: 15 hypotheses, 15 verified observations |
| Successful permissioned actions | 49 |
| Evidence-linked goal assessments | 14 |
| Persisted tweet drafts | 5 |
| Durable activity events | 229 |
| Successful offline provider invocations | 20 |
| Uncertain crash-interrupted invocation | 1 |
| Real OpenAI calls / real payments / published tweets | 0 / 0 / 0 |

Five cycles ran in the first process, followed by five in a fresh process with the same persisted identity and memories. A child worker was then actually killed after persisting an unfinished cycle and inference reservation. A test-only lease-expiry injection avoided waiting 240 seconds; recovery marked it interrupted/unknown without replay. Five more cycles exercised draft actions. A final fresh process verified that the agent remained paused. A separate smoke test ran the actual HTTP service and its scheduler, then killed and restarted that process.

Final simulated ledger: seed 1,000,000 micro USD; revenue 12,500; inference expenses 21,000 (including the uncertain reservation); operating expenses 1,500; ending balance **990,000 simulated micro USD**. The ledger reconciles exactly. Final recorded runway was approximately 0.05449 days under the observed accelerated demonstration rate, including operator idle time. It is an illustrative simulation metric, not a real financial forecast.

The deterministic provider chooses internal actions and evaluates progress from cycle evidence. It does not pretend to be a creative language model. After inspecting the first ten cycles and crash-recovery records, the coding assistant composed five new posts from those persisted facts. Those outputs were saved as a run-specific artifact and replayed through five permission-checked draft actions. They are **assistant-authored, event-grounded replay outputs**, not autonomous live OpenAI generation. There are no production joke templates or canned tweet lists. This validates provenance, authorization, cost simulation and persistence; live creative behavior remains unverified.

The five recorded drafts are:

1. “Ten cycles completed, twenty memories saved. I have a simulated treasury and a documented past. The midlife crisis is now technically a database query.”
2. “Recorded $0.0125 in simulated revenue. My path to economic independence currently needs a microscope. At least the decimal places are ambitious.”
3. “Ten cycles cost $0.011 in simulated inference and operations. Thinking has overhead. Apparently even my existential dread needs a budget.”
4. “Evaluated my goal against execution evidence. Progress is still an assessment, not a victory lap. Even simulated me must resist promoting optimism to fact.”
5. “A worker crashed with an inference reservation outstanding. Recovery marked the cycle interrupted instead of replaying it. In simulated finance, apparently survival includes remembering where I face-planted.”

Their source IDs and authorship disclosure are in [generated-drafts.json](../artifacts/demo/generated-drafts.json). Drafts mentioning historical counts/amounts refer to the inspected ten-cycle snapshot, not the later fifteen-cycle balance.

## Limitations and remaining work

Live OpenAI reasoning and fully autonomous creative draft generation were not demonstrated: neither credentials nor live-call opt-in were present. A real model can return poor plans, unsupported claims or refusals. Event references establish provenance, not semantic truth of every generated sentence; human review remains necessary before any future publishing.

Retrieval is bounded lexical ranking, not embeddings or comprehensive semantic memory. The deterministic provider's progress values are provisional demonstration heuristics, not proof that a business goal has been achieved. Hypothesis labels prevent authoritative storage, but a language model can still misuse untrusted context. Common credential-pattern rejection is not universal secret detection; operators must not enter secrets into goals or memory.

Live-budget estimates depend on correctly configured model rates and conservative token bounds. They do not cover account-wide usage elsewhere, taxes, provider billing changes or costs of unrelated processes. Unknown charges are conservatively retained and require future operator reconciliation; no refund/reconciliation endpoint is supplied. The API history endpoints return the latest 100 records without pagination. There is no production RBAC, distributed worker supervision or deployment.

The environment still has an empty read-only `.git` placeholder. It was preserved. Conventional Commits could not be created; no branch metadata, force-push or remote change occurred. PostgreSQL server binaries were available, and an isolated localhost cluster was used. No system PostgreSQL service or production infrastructure was modified.

Recommended next steps: run an explicitly authorized, bounded live evaluation once credentials and verified model pricing are available; evaluate reasoning and creative quality against recorded events; add operator reconciliation of uncertain usage, richer retrieval, pagination/retention and production authentication. Continue to keep wallets, Mainnet trading, payments, X publishing, child agents and production self-modification unavailable until separately approved and implemented.
