# Milestone 10 — Controlled GPT work evaluation

Validated locally on 2026-11-01. **No paid inference was performed.** This is an owner-controlled evaluation path for unpaid synthetic jobs, not autonomous commercial delivery or primary-epoch activation.

## Baseline and implementation

Milestone 9 uses objective-ranked extracts, lexical memory matching, literal requirement checks, static code patterns, checked integer totals and application-selected tool sequences. These remain deterministic. They do not demonstrate GPT reasoning, internet retrieval, semantic truth verification or model training.

The new `ControlledInference::run_work` path uses the existing `ControlledProvider` abstraction and OpenAI Responses transport. It binds fresh owner consent to the exact agent, unpaid simulated job, job revision, SHA-256 of the saved input, model, approved tools, memory choice, token limits, request ceiling, expiry and independent inference budget. It supports one structured model interpretation, evidence findings, proposed steps, incomplete-information status or abstention. There is no automatic corrective paid request. A later attempt requires fresh consent and sufficient unencumbered budget; ambiguous requests block further requests against that budget and job.

The existing economic execution path still rejects live providers. Work evaluations do not accept, deliver or complete economic jobs, consume customer funds, earn revenue, post simulated expenses or authorize financial operations. The job remains `created` and unpaid even when an evaluation report passes mechanical checks.

No migration was added. Work records extend existing `live_sessions.data`; older records without the new optional field still deserialize. Authorization/settlement audit events and independent `live_budgets` accounting use existing tables and transactions.

### Controls

- Owner-only GET/POST `/agents/{id}/live-work`, including agent-scoped ownership. Operator/viewer roles cannot access these private reports. The older `/live` list excludes work evaluations and its approval route cannot certify them.
- `SUPA_CONTROLLED_WORK=false` by default. Enabling it requires the existing controlled provider, offline normal runtime and disabled scheduler. The protected controlled-offline epoch startup branch explicitly rejects this flag. Epoch API write admission is unchanged.
- Transactional reservation locks the agent, job and budget; rejects stale hashes/revisions, concurrent or replayed consent, paid/expired jobs, wrong customers, missing capabilities and insufficient funds. The historical budget quarantine runs before reservation.
- Model metadata verification precedes exactly one generation dispatch. Each stage is durably counted and checks consent expiry, ownership, job binding and the execution lease. A UI flag or model output cannot authorize dispatch.
- Tools are task-specific application allowlists: input inspection, evidence extraction, requirement checking, static code/data analysis, proposed task decomposition and optionally memory reading. There is no arbitrary command, hosted tool, external mutation or financial tool. Application step limits and permissions remain authoritative.
- At most two relevant verified memories, only with explicit consent and same-agent source events. Memory-free requests contain no memory lessons or identifiers. Observations and model interpretations remain separate.
- Exact quoted source lines and verbatim observations are checked in Rust. Integer totals must equal checked application results. Unknown fields, fabricated observations, references to unavailable memories, unauthorized tools and unsupported success with missing literal requirements are rejected. Independent research requests must abstain.
- Recognizable credential/key markers are rejected in approved input and retrieved lessons before provider dispatch, with a provider-side defensive check too. This conservative guard is not general secret discovery: humans must still approve non-sensitive inputs. No real credentials were supplied to prompts or logs during validation.
- Known valid provider usage is retained even when a report fails validation or ownership is revoked. Configured per-model rates calculate costs in micro-USD, independently of simulated capital. Missing/malformed usage, timeouts, transport ambiguity and usage overruns retain conservative holds and require review. There is no automatic retry, refund or release of uncertain charges.
- Settlement rechecks ownership, capabilities, expiry and job binding atomically. Revocation discards the report while preserving known costs. Expired dispatch leases cannot initiate a generation; persisted uncertain sessions remain fenced on reconnect/recovery.

### Files

- Domain: `crates/domain/src/live_work.rs`, `controlled.rs`, `lib.rs`.
- Existing provider transport extension: `crates/providers/src/work.rs`, `controlled.rs`, `lib.rs`.
- Persistence/accounting: `crates/persistence/src/controlled.rs`, `economy.rs`.
- Runtime: `crates/runtime/src/live_work.rs`, `lib.rs`.
- API: `crates/api/src/live_work.rs`, `controlled.rs`, `lib.rs`, `main.rs`.
- Frontend: `web/src/lib/contracts.ts`, `web/src/app/experience/[[...path]]/route.ts`, `reference-web/{app.js,platform.js,backend.mjs,server.mjs}`; `.env.example` documents disabled enablement.
- Evaluation inputs/plan: `config/work-evaluation-fixtures.json`, `config/live-work-evaluation-plan.json`.
- Approval runner and tests: `scripts/live-work-evaluation.py`, `scripts/test-live-work-evaluation.py`, `scripts/live-work-smoke.mjs`.
- Mock-only support/evidence tests: `crates/api/tests/live_work.rs`, `crates/api/tests/support/work_provider.rs`, `crates/api/examples/work_fixture.rs`, `web/tests/contracts.spec.ts`. The existing accounting quarantine test gains only the optional session field required for compilation.

## Reproducible evaluation and genuine evidence

The nine frozen non-sensitive cases are supplied-document synthesis, integer data analysis, static Rust review, bounded planning, missing measurements, adversarial unsupported observations, memory-free work, memory-assisted work and a hypothetical offer/cost comparison. Each fixture includes a human quality rubric.

The PostgreSQL-backed matrix produced eight mechanically accepted reports awaiting human review and one rejected fabricated observation. It recorded nine single-request mock invocations and 315 micro-USD of **synthetic usage-based accounting**, not actual charges. Memory-free work used zero memory references; the matched memory-assisted case used one real verified observation created in that disposable database. All nine jobs remained unpaid/created, all agents remained paused after fixture setup, and economic ledger entries were zero.

These results verify grounding, permission and accounting mechanics. They do not measure real GPT synthesis quality, prove improved reasoning from memory, certify a code review, or establish commercial success. Model summaries and interpretations are explicitly unverified. Recorded quality checks count validated observations/citations/totals and state `human_review_pending`.

The browser rehearsal exercised the real Next.js proxy, Rust API and PostgreSQL with a mock provider incapable of contacting OpenAI. Two UI evaluations recorded 70 synthetic micro-USD in total and zero actual charges, customer payments, earned revenue or commercial deliveries. Economy and Intelligence show the unpaid job, model/evidence classification, reports, exact citations, memory IDs, token usage, conservative holds and a labelled nominal offer-minus-inference comparison. All ten clean routes, styles, branding and animation architecture remain intact.

## Model and contract review

Official documentation checked during this milestone lists `gpt-6-luna`, Responses and structured output support. Listed rates per million tokens are $0.10 input, $0.01 cached input, $0.125 cache-write and $0.50 output. These are configuration assumptions, not an account-specific invoice or hard provider spending limit.

Sources: [model documentation](https://developers.openai.com/api/docs/models/gpt-6-luna), [structured outputs](https://developers.openai.com/api/docs/guides/structured-outputs?api-mode=responses), [Responses contract](https://developers.openai.com/api/reference/ruby/resources/responses/methods/create), [prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching).

The adapter reuses strict JSON Schema, `store:false`, no hosted tools, project headers, no redirects/automatic retries, request/response byte limits, output token limits, envelope model validation and authoritative usage parsing. Input reservation uses the existing conservative request-byte bound plus overhead. Local HTTP mocks verified the wire contract and retained usage on rejected structured output. No OpenAI credential, actual project access, billing entitlement or account limit was verified. A ChatGPT subscription does not establish API billing access.

## Tests and browser evidence

| Validation | Result |
| --- | --- |
| `cargo test --workspace --all-targets --offline -j2` | 84 passed; 83 opt-in tests ignored in the default run |
| New API work tests, `--test live_work -- --ignored` | 4 passed, including nine frozen cases, ownership, replay/concurrency, scope/budget rejection, revocation, expired lease, reconnect recovery, missing usage, metadata rejection and a 26-second mock timeout |
| Provider library opt-in local HTTP tests | 8 passed; no OpenAI network calls |
| Existing controlled-inference opt-in regression | 6 passed |
| Existing accounting quarantine opt-in regression | 1 passed |
| Existing Milestone 9 work regression | 1 passed |
| Python approval runner tests | 4 passed; network-free mocked HTTP, exact model/scope/budget checks and durable no-replay journal |
| Frontend production build, ESLint, TypeScript | Passed |
| Playwright desktop/tablet/mobile regression | 147 passed |
| Actual Next/Rust/PostgreSQL mock browser rehearsal | Passed at 1440x1000, 768x1024 and 390x844, zero browser errors or horizontal overflow |
| REA rendered interaction/screenshot review | Desktop and mobile reports inspected; keyboard disclosure and reduced motion checked |

Rust commands used `CARGO_TARGET_DIR=/tmp/supa-m9-target`, debug symbols/incremental compilation disabled and offline dependencies. PostgreSQL 16.15 tests used only the new `/tmp/supa-m10-pg-*` cluster on loopback 55680, `supa_m10_test` and isolated child databases. The older work regression required a separately created `supa_m9_test` root on that same disposable cluster. Its first invocation rejected the wrong database name before connecting; the correctly targeted rerun passed. An initial browser harness health probe omitted its service credential; that harness defect was fixed and both final rehearsals passed.

Local-only evidence is ignored under `artifacts/milestone-10/`: final logs, `system-final/work-system-check.json`, desktop/tablet/mobile screenshots and `browser-run3/rea-{desktop,mobile}.{json,png}`. REA desktop interaction evidence: `ev_82a74c9b56fde07386a6096de132ae65a3ef22d426720516c9e7d51c2914bce7`; screenshot `ev_acbbfbbb62348c8dcd8b1eb248ba36d8e428f96c5b5441fcd3d1f2d5b39ade52`; mobile screenshot `ev_506c5974f2105f40a0e86163112a5ec70f071eac8cdd6eda050a8f65fb17222f`.

REA attached to an owned headless Chrome 146 fixture; its interaction steps completed, but attach-time event coverage is incomplete. These are emulated viewport checks, not physical-device or screen-reader certification. Restart validation covers persisted sessions, simulated interruption/expired lease and store reconnect; it does not reproduce every OS/provider/network crash timing. A real paid model evaluation remains unperformed.

## Primary safety and remaining limitations

The primary PostgreSQL cluster was neither started nor queried. Offline checks preserved all 26 protected hashes, including database control, approved backup and historical accounting evidence. The last verified primary baseline remains schema 8, prepared inactive epoch `fd2f52e0-33fc-4c0d-893a-7b08b26ab748`; this milestone does not claim a fresh live database verification. Historical state remains unrecovered. Quarantined budget `ad91e475-f1da-495a-8247-139643d2d8b1`, its $0.10 ceiling and unresolved $0.004048 hold remain untouched and unavailable to new budgets.

No real owner credentials, ownership grants, persistent agents, continuous GPT scheduling, external recipient wallet, payments, Mainnet activity, trading, launches, X publication, Railway operation or deployment occurred. Test identities/credentials were disposable and kept out of Git. Private artifacts/screenshots remain ignored. Nothing was pushed.

Approximately 9.5 GiB remains free after validation; capacity is below the earlier 10 GiB primary-preflight reserve. No protected preflight, cleanup or primary cutover is authorized here. The secret scanner is a targeted local pattern/known-secret gate, not proof against arbitrary semantic secret disclosure.

The model may produce plausible unsupported interpretation despite exact quotes. Human rubric review is required; there is no hidden-reasoning recording or automatic commercial acceptance. Memory retrieval is still lexical and bounded. Plans propose at most four steps; this first workflow deliberately uses one model response rather than an autonomous multi-call loop. There is no internet retrieval, external tool execution, model training, genuine customer revenue or comparable real-model work-quality score. Prior historical paid evaluation evidence is not a comparable work-suite benchmark and was not reused as a new authorization.

## Fresh paid approval boundary — not executed

`python3 scripts/live-work-evaluation.py` is network-free readiness inspection. It currently reports six unassigned blockers: API origin, agent UUID, job UUID, exact saved input SHA-256, OpenAI project ID and a new independent budget UUID. It does not create any of those records or credentials.

Proposed first scope, for separate explicit approval only:

1. A separately approved disposable local non-epoch target, scoped owner authentication, one paused evaluation agent and one unpaid synthetic `supplied-synthesis` job with exact approved input hash. No primary target or credential activation.
2. An independently verified OpenAI API project/billing setup and approved server-side project credential. Verify account access/limits without confusing subscription access with API access.
3. Exact model `gpt-6-luna`; one generation request (plus one model metadata GET), no memory; maximum 24,192 input / 1,024 output tokens.
4. A **new** independent budget with $0.005 ceiling; request consent ceiling $0.004. Conservative reservation is $0.003536 at the reviewed rates. Recheck prices before approving. No historical budget reuse or simulated-fund payment.
5. A populated reviewed local plan and fresh printed scope digest binding that plan and frozen fixture. Explicit operator approval must name the target, project, agent/job/input, new budget, model, token/request limits and scope digest. None of these identifiers is invented by the runner.

Only after that approval, the exact protected command is:

```bash
python3 scripts/live-work-evaluation.py \
  --plan /path/to/reviewed-local-work-plan.json \
  --run --authorize-paid \
  --scope-sha256 <fresh-reviewed-scope-sha256> \
  --model gpt-6-luna --max-cost-microusd 4000
```

Supply `SUPA_WORK_OPERATOR_TOKEN` through a private server-side process environment, never a command argument, chat, Git or screenshot. The runner disables proxies/redirects, checks actual API policy/project and saved job data, then fsyncs an exclusive request journal before its single POST. Any ambiguous outcome stops for manual session/provider-charge review, without replay. The API independently enforces all consent, ownership and accounting checks. The local scope digest is an operator review binding, not a replacement for backend authentication or the existing trusted primary-epoch approval system.

Primary migration, owner onboarding, persistent agent activation, independent encrypted backup enrollment, Devnet receipt funding and Railway deployment retain their own separate approval boundaries. No approval is requested or implied for them by this work-evaluation command.

## Git

Work is committed directly on `main`, one explicitly named file per unique Conventional Commit through `scripts/commit-one`. Full main-history audit and staged/reachable-history secret scanning are required after the final report commit; their final counts and HEAD are reported to the operator. Generated artifacts, logs, database files and credentials are not committed. No push is authorized.
