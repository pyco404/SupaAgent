# Durable database recovery and live GPT readiness

Assessment: **blocked for live inference**. No paid request, production migration, data reset, ownership assignment, reservation release, push or deployment was performed. The frontend and its ten sections are unchanged.

## Historical storage diagnosis

The private evaluation configuration still points to loopback port 55432, database `supa_eval`; the Devnet configuration points to `supa_devnet` on the same cluster. The historical setup documentation explicitly placed PostgreSQL 16 at `/tmp/supa-agent-pg`. That directory, its old log and its port-55432 socket are absent. The configured connection cannot be restored by correcting a password or merely starting the documented cluster: its files are missing.

Read-only host inspection found:

- A reboot at **2026-10-29 17:50:06 WAT**, followed by temporary-file setup at **17:50:07 WAT**.
- The loaded boot unit invokes `systemd-tmpfiles --create --remove --boot`.
- The local rule is `D /tmp ... 30d`. The installed `tmpfiles.d(5)` documentation specifies that type D additionally removes directory contents with `--remove`; the 30-day cleanup age does not make this PostgreSQL storage durable across that boot operation.
- These observations provide a concrete, likely explanation for the disappearance. No per-file deletion audit was found, so the exact deletion event is not independently proven.
- The existing system PostgreSQL cluster on 5432 is running, but its database catalog contains no Supa-named database. Unrelated application databases were not opened or modified.
- Searches of the project, home/Desktop/local-share locations, `/tmp`, `/var/backups`, PostgreSQL directories, mounted `/mnt` and `/media` locations, and relevant Trash names found no matching original cluster or PostgreSQL backup. Other projects' clusters were left untouched. This is a bounded search, not proof that an external/offline backup does not exist.

No original data directory was initialized or reconstructed. Preserved evaluation JSON files are **partial historical evidence**, not a complete PostgreSQL backup. They cannot safely recreate roles, ownership, leases, complete execution history or a canonical financial ledger.

The disposable `/tmp/supa-controlled-pg` cluster on 55439 belongs to mocked validation and is explicitly identified as test data. It is not the historical database and must never be repointed to masquerade as `supa_eval`.

## Historical state and conservative accounting

Current agents, ownership, memories, authorizations, reservations and execution history **cannot be verified against the missing original database**. Partial snapshots preserve some identities, nine memories, goals and activity, but they are not imported or treated as current state.

The newest preserved evaluation snapshot records:

| Item | Historical evidence |
| --- | --- |
| Budget UUID | `ad91e475-f1da-495a-8247-139643d2d8b1` |
| Cumulative ceiling | 100,000 microUSD / $0.10 |
| Committed estimate including conservative hold | 10,299 microUSD / $0.010299 |
| Apparent remaining allowance at snapshot time | 89,701 microUSD / $0.089701 |
| Inference records | 24 succeeded, 1 unknown |
| Unknown-charge amount retained | 4,048 microUSD / $0.004048 |
| Unknown record | `provider_http_401`, no token usage |
| Evaluation database migrations in that snapshot | 1–3 |

These are archived observations, not a current database balance or provider invoice. The earlier 7,948-microUSD snapshot is superseded by this newer evidence. A new successful metadata GET does **not** settle or refund the historical 401 reservation. Nothing was released, reset or recreated. Any restored older backup whose commitment is below preserved evidence must be reviewed rather than silently granting extra headroom.

The new read-only preflight reports that discrepancy, checks the pinned UUID and exact ceiling, counts reserved/unknown calls across both inference tables, and never updates accounting. Uncertain costs require independent provider/billing evidence and operator review. No automatic replay is allowed.

## OpenAI readiness

The key is present in owner-only mode-0600 `.env.eval`, has valid basic formatting, and is not overridden by a shell key. Its value, fragments and fingerprint were not printed. The credential file was not changed.

One explicitly requested, generation-free metadata probe used the existing authentication diagnostic:

```sh
cargo run -p supa-runtime --example auth_probe --locked --offline -- \
  --probe --authorize-auth-probe
```

Actual result: **HTTP 200 for `gpt-6-luna`**, one GET attempt/response, zero generation requests, zero automatic retries, zero inference reservations and no budget modification. The private probe audit remains mode 0600 and is excluded through local Git metadata; only a sanitized summary is committed.

This verifies current credential-scope model metadata access. It does **not** verify Responses write permission, billing/credit availability, the intended project, or project spending limits. The expected `OPENAI_PROJECT_ID` and an explicit controlled-workflow `OPENAI_MODEL` are currently absent. Controlled inference and live opt-in remain disabled. No Admin credential is configured for reading project spend limits; no administrative endpoint or limit-changing request was sent.

Official OpenAI documentation rechecked during this task:

- [Luna model and capabilities](https://developers.openai.com/api/docs/models/gpt-6-luna): Responses, structured outputs and function calling are documented.
- [Structured Outputs](https://developers.openai.com/api/docs/guides/structured-outputs): strict `text.format` JSON Schema remains the integration contract.
- [Pricing](https://developers.openai.com/api/docs/pricing?latest-pricing-expanded=true): the existing standard short-context Luna rates remain applicable to the prepared request.
- [Model metadata GET](https://developers.openai.com/api/reference/resources/models/methods/retrieve): metadata access is separate from generation.
- [Spend limits](https://developers.openai.com/api/docs/guides/spend-limits): alerts do not enforce caps; hard-limit enforcement can lag, so Supa's independent request reservations remain necessary.
- [Admin APIs](https://developers.openai.com/api/docs/guides/admin-apis): project spend-limit inspection requires an appropriately authorized administration/account workflow.

A ChatGPT subscription is not evidence of API billing access. The account owner must verify the intended project and its spending controls through the account or an authorized administrative read before live readiness can be claimed.

## Prepared single session — not authorized or launched

- One explicitly owned, non-suspended agent must first be selected from restored, verified state; no agent UUID or owner is guessed.
- Model: `gpt-6-luna`; exactly **one** generation request, at most **24,192 input tokens** and **1,024 output tokens**.
- Session ceiling: **5,000 microUSD / $0.005**, within the unchanged cumulative $0.10 budget.
- Existing worst-case reservation at those limits: **3,536 microUSD**. Available headroom alone never resolves an unknown-charge hold.
- Harmless goal: compare safe approaches to reviewing available memories, identify evidence gaps, and propose at most one `read_memory` action. No data mutation or claim of task completion.
- Existing structured response includes a concise summary, permitted memory references and at most one allowlisted action; it does not store hidden chain-of-thought.
- Separate owner approval is required before the proposed memory read. Financial actions, signing, trading, token launches, publishing, external mutations and live scheduling stay disabled.
- Fresh request UUID, consent timestamp, expiry and affirmative paid authorization must be supplied **immediately before** an approved request. The prepared artifact deliberately leaves them unset and sets `authorize_paid: false`.

This specification is not a launch command. Do not start the API against a recovered historical database yet: its normal startup applies migrations automatically, while this task requires approval before schema changes.

## Read-only preflight command

```sh
node scripts/live-readiness.mjs --check
```

The command reads the private evaluation configuration without exporting it, reports only safe configuration fields, and uses PostgreSQL transactions and connection options that enforce read-only operation. It does not call OpenAI. It checks table availability, all migration SHA-384 checksums, aggregate state counts, canonical budget/headroom, outstanding reservations and archived commitment evidence. Missing schema or mismatches are reported without migrating. All blocked assessments exit **2**; there is no `--run` path. Raw subprocess/connection errors and credentials are suppressed.

`SUPA_READINESS_DATABASE_URL` is an optional loopback-only, query-free connection override for diagnosis. Output identifies an override and `_test` databases explicitly. Credentials travel through the child environment, never command arguments. A test database cannot be described as recovered historical state.

## Validation performed now

- **124 Rust tests passed**, full workspace/all-targets including actual test PostgreSQL, mocked transport, permissions, budget/concurrency, token accounting, malformed response, timeout, rate-limit and interrupted-request recovery cases.
- **8 new read-only readiness tests passed**: private-file security, redaction, local-only connections, missing schemas/budgets, checksum mismatch, unknown reservations, older-backup commitment discrepancies, and inability to authorize paid execution.
- **18 frontend server/contract tests passed**. Syntax, lint, typecheck, static frontend build, Next production build, Rust formatting and all-target Clippy with warnings denied passed.
- The actual read-only preflight connected to the separate `_test` database and verified all **seven migration checksums**. It correctly found the historical budget absent and refused readiness. Test agents and their usage were not relabeled as restored state.
- Real frontend + Rust API + PostgreSQL end-to-end validation used an isolated schema and **mock provider**. It recorded 120 input + 60 output **mock** tokens and a 42-microUSD **mock** estimate, persisted a separately approved memory read, preserved simulated balances, and reset consent after ceiling changes.
- REA exercised agent selection, limit review and saved-session reload on **1440×1000 desktop** and **390×844 mobile**. It inspected the rendered accessibility tree and captured the prepared harmless goal. Ten navigation sections, no horizontal overflow, reduced-motion `still`, and unchecked paid consent were verified. No real paid request was submitted.
- The previously completed 150 experience / 144 console browser regression suites are historical validation, not counted as new runs here. No frontend source or approved asset changed in this milestone.

Fresh evidence is in `artifacts/durable-readiness/`. The provider summary records one real metadata GET; browser token/cost evidence is explicitly mocked. No new actual inference usage or invoice charges were obtained.

## Recovery and approval gates still required

1. Supply the original PostgreSQL directory, a physical PostgreSQL-16 backup with required WAL, or complete database/role dumps. The user confirmed there is no known backup path; `.env.eval` is last-known connection configuration, not a verified backup.
2. Inspect the source and restore a separate verified copy into durable storage outside `/tmp`, without overwriting existing data. A suitable future location is the user's persistent local application-data directory. This location has **not** been created or configured.
3. Verify identities, ownership, memory, authorizations, cycles/actions, cumulative budget and every inference call against both the backup and preserved later evidence. Do not invent owners or import a partial JSON snapshot as a full ledger.
4. Compare migration history with the current seven migrations. Stop for explicit approval before any required schema changes. The archived evaluation snapshot is only at migration 3, so it cannot be assumed ready for current ownership/session endpoints.
5. The user confirmed no verified project ID is available. Obtain the intended project ID and verify account-side billing/spend limits. Configure the explicit model and output ceiling server-side, preserve the budget UUID and $0.10 ceiling, and leave scheduling disabled.
6. Resolve uncertain charges using provider evidence through a reviewed reconciliation process. Then rerun read-only readiness and inspect the original agent through the frontend.
7. Only after these gates, ask for fresh approval of the single $0.005 session. No blanket, historical, or implied paid authorization is carried forward.

No restore, budget reset, automatic settlement, live inference or schema change is proposed as already approved. The milestone remains blocked on the historical recovery source and account/project verification.
