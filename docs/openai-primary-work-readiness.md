# OpenAI primary-provider work readiness

Prepared 2 November 2026. No authenticated provider requests, paid inference, primary database access, persistent workers, historical accounting updates, deployment or push occurred.

## Provider preference and personality

Private, Git-ignored `.env.eval` now explicitly selects `OPENAI_MODEL=gpt-6-luna`, `OPENAI_PROJECT_ID=proj_npsyg93Ofk7ivKheGa8IDw5q`, `SUPA_CONTROLLED_PROVIDER=openai` and `SUPA_PLANNING_MODE=routine`. Its API key and existing execution gates/budget settings were preserved; file permissions remain 0600. `SUPA_ALLOW_OPENAI` remains false. Preference is not execution authorization: the normal runtime stays offline and controlled inference requires its separate opt-ins and finite consent.

The existing Rust default already selects OpenAI for controlled inference and Luna for routine reasoning/social drafting. A discovered gap was corrected: routine OpenAI construction now validates and binds the configured project, as controlled work already did. Both paths share the project validator. Mock tests verify the project header, keep the project/key out of model input, reject invalid identifiers and confirm Luna routing without complex-model escalation.

Anthropic remains an explicit optional provider; its adapter, private configuration and historical scope were not changed. No automatic provider fallback was added. `config/supa.json`, the canonical personality, shared social prompt, event grounding, permitted memories, repetition/topic cooldown, abstention and review/publishing restrictions are unchanged. The new test agent uses the canonical personality rather than a separate provider-specific personality. No existing agent or memory was modified.

## Model, pricing and account evidence

The official [GPT-6 Luna model documentation](https://developers.openai.com/api/docs/models/gpt-6-luna) lists structured outputs and Responses API support. Current standard rates per million tokens are $0.10 input, $0.01 cached input, $0.125 cache write and $0.50 output; these match existing Rust accounting. The adapter requests default service tier and explicit 30-minute caching. No regional endpoint or hosted tools are enabled. [Official pricing](https://developers.openai.com/api/docs/pricing)

The intended project is explicitly configured in both private settings and the new scope. The existing private key is present, passes basic format checks and is loaded from an owner-only ignored file; that alone does not authenticate it. Prior separately authorized evidence in `docs/milestone-10a-authentication.md` records HTTP 200 for the exact model with the intended project header. That probe was not repeated. Current membership, generation entitlement and project spending limits were not independently queried.

The operator reports $5.99 available API credit with automatic recharge disabled. This is operator-supplied billing evidence, not a new account API observation. Confirm that the intended project has sufficient model/usage limits before paid execution; account credit does not override application ceilings or authorize spending.

## Fresh immutable proposal

Plan: `config/openai-work-evaluation-current.json`.

Scope SHA-256:

`0453528bc228c83e6137eabd53ebeb4a7b98a198e7e6937e397ece240a51a6f0`

| Binding | Value |
| --- | --- |
| Provider / model | OpenAI / `gpt-6-luna` |
| Project | `proj_npsyg93Ofk7ivKheGa8IDw5q` |
| Disposable database | `supa_m10a_a13db38ce0a5_test` |
| Socket-only cluster | `/tmp/supa-m10a-pg-a13db38ce0a5/data`, port 55681 |
| PostgreSQL system identifier | `7703509812952711377` |
| Passive API / future reviewed API origin | `http://127.0.0.1:55682` |
| Paused agent | `d4849ce9-5cef-4b76-9390-64ef6d83a99d` |
| Unpaid job | `5cdfe440-dc36-4f94-8fc3-93b766ea31c5` |
| Disposable owner | `f84eeab5-ee3c-46c5-b04c-71303bff9044` |
| Independent budget | `6ec8370e-fcd8-4675-84dd-1b754f6664c2` |
| Fixed single-request nonce | `5b667b4e-757d-4aec-8c58-490e0ef23eb7` |
| Accounting isolation label | `db294fb9-d225-4e50-8836-407c9aa1c309` |
| Job expiration | **2026-11-03 09:52:29.570357 UTC / 10:52:29.570357 WAT** |
| Independent budget ceiling | **$0.005**, committed **$0** |
| Generation ceiling | **1 request**, no automatic retry |
| Per-request ceiling | **$0.004** |
| Token ceilings | **24,192 input / 1,024 output** |
| Conservative reservation if separately authorized | **$0.003536**, not currently reserved or charged |

This is a new disposable evaluation, not renewal of either previous job. Previous plans, nonces, journals, expirations and holds remain unchanged. Its namespace is an isolation label, not an activated application epoch. All 12 migrations ran only in the new empty disposable database; it contains no application epoch or recovered historical records.

Task: **Compare the supplied recovery strategies.** Exact evidence:

```text
Document A: Replication reduces downtime but copies accidental deletion.
Document B: Offline snapshots preserve earlier state but need a tested restore.
Neither document contains measured recovery times.
```

The original canonical input bytes are retained, SHA-256:

`613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`

Allowed pure application tools: `inspect_input`, `extract_evidence`, `check_requirements`. Memory use is disabled for this controlled comparison; existing persistent memories are not exported or altered. Claims must reference supplied evidence. Measured recovery-time claims are unsupported and must be rejected or omitted. Results require application validation and human review; the unpaid simulated job cannot become earned revenue simply because inference completed.

## Preparation and non-paid validation

`openai_work_preflight` is a no-provider seed utility: it accepts only matching fresh local test paths, verifies database/system identity and empty schema before migrations, creates the paused canonical-personality test agent and unpaid job, and installs the independent zero-spent budget. Historical budget IDs are explicitly rejected. It does not load credentials or create a reservation. Re-running against an already seeded schema is denied; do not use it to renew a scope.

`openai_work_readonly` pins the exact new database/system identity and owner, enforces read-only SQL connections and accepts only GET/HEAD. `scripts/test-openai-work-readiness.py` pins the reviewed digest, starts only that disposable cluster and passive API, uses ephemeral process-only test credentials, and stops both afterward. It verifies all public table rows remain unchanged and that the stored personality matches canonical configuration. These synthetic owner credentials are not real owner onboarding or installed runtime credentials.

Reproduction after building the examples (no external API requests):

```bash
CARGO_TARGET_DIR=target/claude-validation CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo build -p supa-api --example openai_work_readonly --locked --offline -j2
python3 scripts/test-openai-work-readiness.py
node scripts/live-work-preflight.mjs --scope config/openai-work-evaluation-current.json --credentials
python3 scripts/live-work-evaluation.py --plan config/openai-work-evaluation-current.json
```

The passive rehearsal passed eight grouped checks: unauthenticated rejection; paused owned agent; exact unpaid job/live provider disabled; public credential denied private work; mutation methods denied; canonical personality; unchanged public rows; protected hashes. It confirmed zero inference sessions/calls, cycles, tool actions, scheduling authorizations, economic entries and earned revenue. The budget remains 5000/0 micro-USD after restart. At validation, over 23 hours of job validity remained. This must be rechecked immediately before any future paid execution; it is not an automatic extension.

Additional final results:

| Validation | Result |
| --- | --- |
| Rust workspace, all targets, offline/locked | 92 passed, 0 failed, 98 opt-in ignored |
| Provider suite with local mock HTTP fixtures | 44 passed, 0 failed |
| Explicit disposable controlled/live-work integration | 13 passed |
| Explicit disposable accounting-quarantine integration | 1 passed |
| Disposable crash/restart | 9 session/budget fingerprints preserved |
| JavaScript preflight tests | 4 passed |
| Python work-evaluation tests | 9 passed |
| Python Claude credential tests | 6 passed |
| Social comparison, authored fixtures only | 6 grounded cases; repeated drafts/foreign memories rejected; cooldown and empty-context abstention enforced |
| Formatting / Clippy all targets, warnings denied | Passed |
| Secret-scanner self-tests | 15 passed |

The first sandboxed Node test runner could not spawn a local subprocess; the authorized local-process rerun passed all four tests. Remaining ignored infrastructure suites are not claimed as passed. The social comparison makes no model calls and supplies no measured GPT/Claude quality or humor scores. No frontend changes were made, so no new visual/browser result is claimed. Relevant API interaction was tested over actual local HTTP.

Sanitized evidence stays in ignored `artifacts/openai-primary-readiness/`, including protected-file baseline, scope, private-file presence report, passive HTTP evidence and test logs. No artifacts, database files or credentials are committed.

## Financial and environment preservation

The primary cluster was never started or queried and remains stopped. All 26 protected primary/backup/accounting hashes match the baseline. Historical plans, private Claude settings, canonical personality and existing request journals are byte-identical. No previous evaluation database was started or altered.

The Claude $0.003536 reservation remains unknown and untouched; the historical OpenAI $0.004048 hold remains quarantined. No settlement, release, refund or budget increase occurred. The proposed new $0.003536 worst-case reservation is a separate calculation for a separate budget, not reuse of the Claude hold. There is no continuous live scheduling, persistent worker, financial execution, publication, backup upload or Railway activity.

## Separate paid authorization and remaining blockers

The new cluster and passive API are stopped. The passive API cannot perform inference. Before the command below can dispatch, separately approve the reviewed single paid request and bounded, scoped disposable live-provider API setup; configure its exact new budget/project/model and private test-owner token, while leaving normal reasoning offline, scheduling disabled and all financial tools unavailable. No live-capable runtime token has been installed. Confirm project usage limits, recheck expiry and scope against the database, and do not reuse old credentials or budgets. The reported account credit is not that approval.

Protected command, **prepared only and not executed**:

```bash
python3 scripts/live-work-evaluation.py \
  --plan config/openai-work-evaluation-current.json \
  --run --authorize-paid \
  --scope-sha256 0453528bc228c83e6137eabd53ebeb4a7b98a198e7e6937e397ece240a51a6f0 \
  --model gpt-6-luna --max-cost-microusd 4000
```

The runner performs frozen target/budget checks, uses the fixed nonce, and writes an exclusive request journal before its one authenticated work POST. Ambiguous outcomes require review, never automatic replay. Expiration or any changed protected field requires a newly reviewed scope; neither this report nor preference configuration grants paid permission.
