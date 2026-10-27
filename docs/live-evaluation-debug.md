# Live evaluation failure investigation — 2026-10-27

Follow-up: the [separate authentication diagnosis](openai-authentication.md) subsequently received HTTP 200 from one generation-free model-metadata GET using the current private-file credential. The historical evidence below remains unchanged.

No OpenAI requests were made during this investigation. The paid evaluator was not invoked, and no evaluation financial records, credentials, budget identifiers, ceilings or existing reports were modified.

## Findings and evidence

Two distinct failures must be distinguished:

1. The preserved evaluation report and the sole `inference_calls` row record `provider_http_401` for the first planning request. Database initialization, migrations, agent creation, goal/treasury setup, cycle creation and request submission therefore succeeded. The failure occurred at the OpenAI HTTP response stage, before any validated model output or token usage. Authentication was rejected. The adapter intentionally discards upstream error bodies; it is impossible to distinguish an incorrect/revoked credential, organization/project issue or IP restriction from the preserved status alone. Credential presence in readiness is not authentication verification.
2. The default report name was derived solely from the budget UUID. Opening that same existing file with `create_new(true)` produces `AlreadyExists`, which the old executable mapped to `invalid_input`. This exact failure was reproduced against the existing report without changing it. It occurs after migration checking and before creating a new agent or submitting an API request. It explains the generic error when reusing this budget's default report path. No detailed invocation trace exists to independently establish the flags/environment of the reported terminal invocation.

The selected routine model is `gpt-6-luna`, routine planning, with an output cap of 2048. Official documentation lists Responses API and structured-output support for this identifier, including low reasoning effort. Planning and drafting use `text.format.type=json_schema`, `strict=true`, object roots, complete required property lists and `additionalProperties=false`. These contracts are exercised against fake local HTTP servers. There is no evidence that the HTTP 401 was caused by model names or schema parameters; actual account-level access and schema acceptance remain unverified.

References: [OpenAI error codes](https://developers.openai.com/api/docs/guides/error-codes), [Luna model](https://developers.openai.com/api/docs/models/gpt-6-luna), [structured outputs](https://developers.openai.com/api/docs/guides/structured-outputs).

## Fix

Default reports retain the original `<budget-id>.json` name for the first attempt. If it already exists, a new exclusive `<budget-id>.attempt-<attempt-uuid>.json` file is created. The attempt UUID is only a report filename suffix: it does not create or change a spending budget. Existing evidence is never overwritten. An explicitly selected `SUPA_EVAL_OUTPUT` that already exists fails safely with `report_already_exists` at `report_initialization` rather than silently selecting another path. Reports remain private (0600), and updates retain the exclusive file handle.

The evaluator now emits sanitized `stage` and `error` fields for environment loading, argument validation, authorization, provider configuration, database configuration/connection/migration, report initialization/writing/collection, agent setup/stop, cycles, transport, HTTP responses and response validation. Reports also include `failure_stage`. Request serialization/size failures have their own `provider_request_invalid` code, distinguished from HTTP errors. Errors never include raw I/O/SQL/HTTP bodies, paths from underlying errors, credentials or prompts. HTTP 401 adds a static remediation hint without releasing its conservative reservation or retrying.

No code change can repair an invalid/revoked key or account restriction without changing credentials/account configuration. Those remain the operator's private follow-up, not a claimed fix in this patch.

## Database and spending verification

Read-only checks verified PostgreSQL on `127.0.0.1:55432`, connected as restricted role `supa_eval` to database `supa_eval`. Both migrations (foundation and intelligence) are successful, and their SHA-384 checksums match the repository. No migration was applied during diagnosis.

The original budget `ad91e475-f1da-495a-8247-139643d2d8b1` retains its **100,000 micro USD ($0.10)** ceiling and **4,048 micro USD ($0.004048)** committed reservation, leaving **95,952 micro USD ($0.095952)** by local accounting. There is exactly one recorded request, status `unknown`, failure `provider_http_401`, with no usage. This retained reservation is a conservative estimate, not proof of provider billing. It was not released or reset. The four simulated ledger records are unchanged and all explicitly simulated. The original agent is suspended with `scheduler_enabled=false`.

## Validation

All checks passed: 24 unit/example tests, 24 integration tests (20 PostgreSQL/runtime, one HTTP, three local mock-provider), seven environment-file checks and one executable stage-error check. Formatting and Clippy passed with warnings denied.

- `cargo fmt --all -- --check` and Clippy with warnings denied.
- Workspace/all-target offline tests, including report collision, existing evidence preservation, retained budget identity, private report permissions, sanitized stage errors and oversized-request rejection before transport.
- PostgreSQL/runtime and HTTP integration tests on the separate `supa_agent_test` database; no evaluation records used or modified.
- Local fake-provider tests cover planning and draft schema contracts, credential isolation and sanitized 401/429 errors. No real provider is contacted.
- Seven environment-file subprocess checks use `--check` only, covering precedence, permissions, malformed files, unsupported keys, symlinks and ignore rules.

## Files changed

- `crates/runtime/examples/live_eval.rs`: annotate execution stages and use exclusive attempt reports.
- `crates/runtime/examples/live_eval_support/mod.rs`: safe diagnostics, report allocation/writing and collision/sanitization regression tests.
- `crates/domain/src/lib.rs`: explicit request-construction error code.
- `crates/providers/src/lib.rs`: classify request-size failures, verify strict planning/draft schemas and reproduce sanitized HTTP 401 with a fake provider.
- `docs/milestone-2a.md`: update attempt-report behavior and link current evidence.
- `docs/live-evaluation-debug.md`: investigation, validation, unchanged accounting and retry requirements.

## Retry status

The report-path collision is fixed. Do not retry paid execution until privately resolving the HTTP 401 and confirming the intended credential is selected: exported shell values override `.env.eval`. Keep the original budget UUID and ceiling above. Do not create another budget or discard the earlier reservation. An explicit existing output path must be unset or changed to an unused report path.

The non-paid readiness check in this agent's current environment detects the key and dedicated database configuration, but reports missing stable budget UUID and disabled live opt-in, in addition to the intentionally absent run authorization. The user's earlier terminal had different exports. The private file was left unchanged. `--check` intentionally performs no database connection or account-access request; its successful exit reports configuration checks, not successful authentication. Database connectivity was verified separately as described above.
