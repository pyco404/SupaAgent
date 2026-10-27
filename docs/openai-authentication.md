# OpenAI authentication diagnosis — 2026-10-27

The current credential from the private project `.env.eval` successfully authenticated a single `GET https://api.openai.com/v1/models/gpt-6-luna`: **HTTP 200**, with a matching model-metadata response. No generation request was sent. The full evaluator was not retried. No key, credential file, existing report or evaluation spending record was changed.

## What was verified

- The actual shared Rust dotenvy loader selects the default project file; mode is 0600, it is not a symlink, and existing ignore rules exclude it.
- In the coding agent's launch environment, no exported `OPENAI_API_KEY` overrides the file. This does not inspect or establish the contents of another interactive terminal's environment.
- The selected key is nonempty ASCII with the expected API-key prefix, no whitespace/quotes and no extra Bearer prefix. These are format checks, not authentication proof; the HTTP 200 supplies the latter for model metadata.
- The routine model is `gpt-6-luna`; the separate complex default remains `gpt-6.1-sol`. Only the routine model was probed.
- Supa uses reqwest's Bearer authorization against the fixed OpenAI HTTPS host. No organization or project headers are added. No base-URL, organization/project or proxy overrides were present in the inspected launch environment. The adapter does not honor alternative OpenAI base URLs or organization/project environment variables; diagnostics identify such ignored names without showing their values.
- The preserved historical planning call remains `provider_http_401`, at the HTTP response stage, with no model output or usage. Its key/source was never fingerprinted or retained, so it cannot be compared with the current credential. A stale shell export, changed credential or transient account condition is possible, but none is established by the available evidence.

No reproducible Supa authentication-header or endpoint bug was found. The current key works for this read endpoint; no account change is currently demonstrated as necessary for authentication. The actual intended project cannot be verified without an expected project identity or account confirmation. A project-scoped credential chooses its own project. This probe does not verify Responses API write permissions, billing readiness, structured-generation acceptance, or inference capability.

Official documentation: [Bearer authentication and optional organization/project headers](https://developers.openai.com/api/reference/overview), [retrieve model metadata](https://developers.openai.com/api/reference/resources/models/methods/retrieve).

## Commands and controls

Inspect the selected credential source in the terminal that will eventually launch evaluation, without network access:

```bash
cargo run -p supa-runtime --example auth_probe --locked -- --check
```

This prints source/override flags and format booleans, never key values, fragments or fingerprints. To deliberately prefer the private file in a terminal with a stale export, `unset OPENAI_API_KEY` before running the check. The file loader preserves explicit shell-export precedence.

The separate opt-in authentication probe is:

```bash
cargo run -p supa-runtime --example auth_probe --locked -- --probe --authorize-auth-probe
```

This requires both probe flags, independently of paid-inference authorization. It performs one GET for routine-model metadata. No POST, prompts, inference reservation, database access, key rotation, publishing or transaction occurs. Redirects and automatic transport retries are disabled. Bodies are bounded; only fixed allowlisted error codes or model-ID match booleans survive sanitization. The audit is created privately before dispatch, recording a one-request upper bound for crash recovery, and finalized after the attempt. Prepared audits are never automatically replayed. Exit status 2 indicates that authentication/model-metadata access was not verified; that can also mean insufficient read permission rather than an invalid key.

This probe was executed **once** for the diagnosis above. Its sanitized audit is [95e801ef-e22c-470a-a2b9-2f741141f4d6.json](../artifacts/auth-probe/95e801ef-e22c-470a-a2b9-2f741141f4d6.json), mode 0600. It records one HTTP request/response, zero generation requests, zero retries, zero inference reservations and HTTP 200. Audit filename UUIDs are probe identifiers, not spending budgets.

## Preserved finances and readiness

Read-only PostgreSQL verification after the probe found the original budget `ad91e475-f1da-495a-8247-139643d2d8b1` unchanged: ceiling **100,000 micro USD ($0.10)**, retained conservative reservation **4,048 micro USD ($0.004048)**. The database still contains exactly one historical inference call with no token usage, and four explicitly simulated ledger records. The GET audit is separate from inference accounting and creates no additional budget.

The full evaluator's non-paid `--check` remains configuration-only. It intentionally does not consume probe results or claim account/inference verification. In this process, it still reports disabled live opt-in and a missing stable budget UUID, plus absent paid-run authorization. Those settings and `.env.eval` were left unchanged. Any eventual separately approved evaluation must retain the original UUID and $0.10 ceiling. No full paid command was run during this task.

## Implementation and validation

- Shared private-file loader: `crates/runtime/examples/live_eval_support/environment.rs`; both example commands use the same dotenvy parsing and precedence.
- Separate command: `crates/runtime/examples/auth_probe.rs`.
- Format diagnostics, bounded sanitized GET and explicit probe gate: `crates/providers/src/authentication.rs`.
- Provider transport: malformed credentials now fail locally; automatic transport retries are explicitly disabled. Existing runtime retry policy remains independently enforced.
- Passed Cargo formatting, Clippy with warnings denied, 32 unit/example tests (including shared helper tests in both example binaries), 25 integration tests (20 PostgreSQL/runtime, one HTTP, four local mock-provider tests), and seven environment-file subprocess checks. Local mock tests include 200, 401, 403 and redirect-class responses, zero generation/body checks, authorization gates and private-value filtering.
