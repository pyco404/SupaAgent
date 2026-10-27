# Security and simulated economics

## Authorization boundaries

All HTTP routes require a local operator bearer token of 32–256 non-whitespace ASCII characters. Generate it randomly, protect it like a password, and do not pass it in JSON or URLs. The service accepts only loopback IP bind addresses. There is no unauthenticated agent-management API or browser CORS allowlist. This is a single-operator development service, not production RBAC or a public multi-tenant API.

Only operator routes create agents, establish capabilities, activate agents, set goals, and seed or credit simulated revenue. The LLM receives no API token, environment variables, database URL, provider credential, wallet credential, or raw HTTP headers. Personality, goals, memory and event content do not grant permissions. Only the closed Rust registry and persisted capability set authorize tools. Unknown tools are denied. There is no model-facing network/wallet tool, exchange, X publisher, shell executor, autonomous factory, or self-modifying code path. Milestone 3 adds a separate operator-only Devnet observation service; see [wallet security and accounting](milestone-3.md).

Operator-authored goals, personality and model-authored lessons are bounded and checked for common credential patterns before persistence. This is defense in depth, **not a universal secret detector**. Operators must never enter secrets into these fields. Arbitrary sensitive user content is outside the permitted input contract. Memory and drafts are private application data, available only through the authenticated local API; protect the database and its backups accordingly.

The OpenAI adapter owns its key without a Debug implementation, sends it only as transport authentication to the fixed HTTPS OpenAI endpoint, disallows redirects, and excludes it from the JSON payload. Calls have transport/runtime timeouts and bounded input/output sizes. Production logging enables only application targets, never verbose SQL/HTTP dependency logs. Error responses use controlled codes, not upstream bodies or connection strings. Audit proposals omit raw lesson text. Logs contain IDs, event kinds/codes, simulation flags and state, not prompts, generated prose, headers, or chain-of-thought. Lifecycle reasons are persisted but not logged or supplied to the model.

## Treasury and accounting

Every agent has an independent **simulated** balance. Units are integer micro USD: 1,000,000 simulated micro USD represents one hypothetical dollar, never real USDC. All financial responses carry `simulated: true` and `unit: simulated_micro_usd`; ledger rows have a database constraint fixing `simulated=true`. Ledger kinds are `seed`, `revenue`, `inference`, and `operating`.

Only positive operator-specified seed/revenue entries credit the balance. Configured inference and operating costs debit it. Per-entry limits and a total-balance ceiling protect integer arithmetic. Each change locks the agent row, checks the balance, writes its ledger entry and audit event in the same transaction, and uses an agent-scoped UUID idempotency key. Reusing a key with a different amount, kind, or cycle is invalid.

A simulated charge and provider-call reservation are committed before each potentially paid provider attempt. Failure or timeout does not refund it because usage may have occurred. Offline cycles also consume configured hypothetical costs for exercise of the simulation. At insufficient funds the paid operation does not run, the cycle fails, and an active agent becomes suspended. Earlier committed expenses remain recorded. Adding funds never authorizes automatic reactivation.

Fees are fixed **simulation assumptions**, not live OpenAI prices or measured bills. Operators choose them at agent creation. Zero is allowed to model a free operation. No actual money is held or earned. The simulated ledger never authorizes real spending. Milestone 2 separately enforces a durable estimated OpenAI budget based on operator rates and conservative token reservations; it is not an authoritative invoice. OpenAI activation can incur real API charges and requires explicit local opt-in; leave offline mode enabled unless authorized. Actual spending controls, metered usage, and human spending approvals must precede a real autonomous treasury.

Runway is `balance / observed gross expense rate`, where expense rate uses lifetime simulated inference plus operating debits divided by time since the first expense. It requires at least 60 seconds of history and positive expenses; otherwise `runway_days` is null. Revenue is excluded from the future burn estimate. The metric assumes this historic rate continues, including idle time, and is not a financial forecast.

## Draft provenance

Draft generation receives only selected persisted event metadata and personality. It has no publishing adapter. Drafts must reference known selected event UUIDs, fit a conservative 280-character limit, pass credential-pattern checks, and explicitly say “simulated” when any source event is simulated. They are always persisted as `draft`.

Event references prove source provenance, not semantic truth of every generated sentence. Automated checks do not prove that a model never invents a claim. Human review is required before any future publication. No hardcoded tweet or joke template is used by the production adapter. The offline provider does not generate tweets; integration tests use controlled fixture drafts solely to verify plumbing.

## Future authorization

Mainnet activity, real payments, external publishing, autonomous agent creation, and production deployment remain unavailable. Future implementations require explicit operator authorization, separate capability policy, transaction/spending limits, credential isolation and appropriate tests. Nothing in model output, a personality file, or this milestone's simulated funding endpoint can enable them.

## Milestone 2 boundaries

Leaving active state invalidates the current cycle, so an old result cannot regain authority after an explicit resume. Emergency stop records suspension and marks pending work interrupted. HTTP 429 is the only retryable provider error; retry counts, total calls and tokens are bounded by operator configuration. Unknown calls keep their conservative live-budget reservation across restarts. Reported usage can settle an active reservation even when output validation fails; after recovery has marked a reservation unknown it stays held for future operator reconciliation.

Live budget rows are separate from every simulated balance and transaction. Configuration must retain the same budget UUID across restarts. A changed limit for an existing UUID is rejected. The model never receives the budget ID, rates, credentials or secret configuration. The context contains only simulated economic metrics.

Every model-authored memory is labeled hypothesis. System cycle outcomes are verified observations. Goal progress is a model assessment with agent-owned event references, never automatic verified completion. The new conservation capability can only delay future cycles; it cannot lower policy costs or transfer/fabricate funds.

## Milestone 2A additions

Live evaluation requires explicit CLI authorization in addition to key, opt-in and durable budget configuration. Requests pin standard service tier and explicit-only 30-minute caching without breakpoints. Cache writes are accounted separately, and the reservation uses the highest applicable input rate. The routine/complex model routes share a common budget ID, and model output cannot change either route.

Evaluation agents set `scheduler_enabled=false`; the server excludes them from automatic work even after a crash. Live draft claims must match supplied canonical event observations verbatim, reference owned persisted events and avoid numerical embellishment. Machine validation is not proof that all free-form prose is factually sound; semantic review remains required. See the Milestone 2A report for the unperformed paid phase and exact launch command.

## Social decision update

The new social path verifies canonical claims against owned persisted events and approved public fact fragments, checks recent accepted drafts, and permits abstention. Personality cannot grant social permission or alter billing. Review metadata stays outside public text and publishing remains disabled. Repetition rejection is final for that attempt; no automatic creative rewrite is performed. See [the social contract](personality-and-social.md) for the distinction between machine claim checks and required human semantic review. The legacy verifier remains available for legacy callers.
