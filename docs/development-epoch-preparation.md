# New development application epoch — prepared, not activated

The candidate is prepared and validated separately from historical data. **No application cutover, agent adoption, spending authorization or live execution occurred.** The primary frontend configuration is unchanged. The historical database remains unrecovered.

## Exact candidate identity

| Item | Prepared value |
| --- | --- |
| Epoch UUID | `fd2f52e0-33fc-4c0d-893a-7b08b26ab748` |
| Database | `supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748` |
| Accounting namespace | `641457b2-5fd8-4b31-b85f-c58839626831` |
| Kind / state | `development` / `prepared` |
| Historical database | `unrecovered` |
| Agent identities / ownership grants | 0 / 0 |
| Execution authorizations / live budgets | 0 / 0 |
| Simulated ledger / provider calls / live sessions | 0 / 0 / 0 |
| Migration history | Eight successful, checksum-verified migrations |
| Database default | Read-only |
| Independent backup destination | Unset, as explicitly requested |

The accounting namespace is an identity marker for this separate database. It is **not** a funded account, live inference budget, replacement ceiling or permission to spend. No historical IDs, owners, agents, memories, authorizations, wallet records or accounting rows were copied. The only populated application table is `application_epoch`; `_sqlx_migrations` records schema preparation. All 27 other application tables are empty.

## Infrastructure and isolation

The candidate uses the existing PostgreSQL 16 cluster under `postgres-data/infrastructure-v1/data`, outside `/tmp`, with checksums, durable writes, owner-only Unix sockets and no TCP listener. It has a distinct database from both the historical names and `supa_infrastructure_test`. PostgreSQL database isolation, separate UUIDs and empty authorization/accounting tables prevent accidental reuse of validation fixtures or historical identities.

Private preparation configuration is `postgres-data/development-epoch.json`; an exclusive intent record prevents repeated preparation from silently replacing a failed or completed candidate. Existing `.env.eval`, `.env.devnet`, `web/.env.local`, PostgreSQL infrastructure identity, archived evidence and Git exclusions were not repointed or replaced. `postgres-data/` is already ignored.

Migration 0008 is additive. It creates an empty singleton metadata table on databases receiving it and never guesses an epoch, owner or recovery status. The preparation example verifies the exact UUID-derived database name and an empty database before migrating and inserting metadata. After preparation the database is frozen with `default_transaction_read_only=on`. No historical schema migration ran.

This is logical development isolation, not hostile-tenant isolation: the local OS/database administrator can change settings and SQL directly. Separate least-privilege application and migration roles remain required before writable adoption. Database read-only defaults are an additional safeguard, not a substitute for those roles.

## Backend and operator-facing behavior

- `GET /health` includes an explicit epoch object with UUID, namespace, kind, state and historical status. Older/unidentified databases return `epoch: null`; they are never labeled recovered.
- API startup examines metadata **before migrations, recovery or worker-heartbeat startup**. A candidate requires the exact `SUPA_EXPECTED_EPOCH_ID`, `SUPA_EPOCH_READONLY_PREVIEW=true`, verified read-only defaults, loopback binding, offline provider and disabled paid inference, scheduling and wallets.
- Preview mode skips migrations, recovery, worker registration, autonomous scheduling and wallet services. It exposes read-only routes using an isolated service credential. All authenticated non-GET/HEAD operations against a marked epoch are denied by middleware, including ownership, lifecycle, goals, authorization and inference submissions.
- The worker rejects a marked epoch before migration or heartbeat registration, including when configured paused. The schema permits only `prepared` state in this milestone. There is **no activation command or environment switch that makes it writable**.
- The server-only frontend health contract preserves and validates epoch metadata. Every one of the ten approved terminal sections displays the epoch and unrecovered-history notice. Prepared Intelligence views omit execution forms and explain that ownership changes, authorizations and execution are disabled.
- Disconnected and unidentified health responses display their actual uncertainty. Empty candidate records are displayed as empty, not as recovered history, successful activity or performance.

The visual change is confined to terminal status wording and control availability. Colors, typography, logo, geometry, cinematic intro, typing, portal, diagrams, hands, wheel, motion and ten destinations remain intact.

## Historical accounting is still quarantined

Budget `ad91e475-f1da-495a-8247-139643d2d8b1` retains its documented **$0.10 ceiling**, **$0.010299 archived commitment**, and **$0.004048 unresolved hold**. These are preserved evidence, not a current invoice or available new allowance. The existing persistence quarantine still rejects recreation, reuse, request mutation and settlement/refund attempts involving that UUID. No guard was removed and no override was introduced.

A fresh database does not resolve unknown provider charges. No new live budget or spending authorization was created. Historical reconciliation remains a separate reviewed process requiring complete records and provider evidence; successful authentication or a new epoch cannot release the hold.

## Ownership and execution authorization procedure — no actual adoption performed

1. Identify the operator independently and approve the exact principal UUID and role. Do not infer ownership from the OS user, shared service token, first login or missing historical records.
2. For fresh agents, create new UUIDs and explicit owner grants only after writable activation is approved. Existing `create_owned` / `establish_owner` audit paths remain the mechanisms; no new automatic claim endpoint exists.
3. Historical adoption requires a separate evidence-backed source-to-target mapping. Unknown owners remain unassigned and inaccessible. Never create a missing historical agent from a partial snapshot or relabel a new identity as restored history.
4. Ownership grants do not authorize execution. Historical authorization records, if later recovered, remain closed evidence. New goals, finite cycle limits, expiry, simulated-cost limits and fresh consent require their own operator-approved authorization. Scheduling defaults to disabled; live inference remains unavailable.
5. Conflicts, missing provenance and uncertain external actions stop adoption. Do not replay interrupted actions or import live leases. Retain source evidence, proposed mappings and approval decisions separately from active records.

The new PostgreSQL regression validates an isolated synthetic ownership workflow: an unowned fixture denies access, explicit ownership succeeds once, duplicate/outsider claims fail, missing identities remain missing, and no schedule or budget appears. It then marks the fixture as prepared and proves owner-authenticated mutation routes and direct frozen-store writes fail. These fixtures live only in test schemas, never the candidate.

## Backup preparation and independent destination

```sh
python3 scripts/development-epoch.py status
python3 scripts/development-epoch.py backup
python3 scripts/development-epoch.py verify --backup-id <UUID-returned-by-backup>
```

First preparation has already completed. The explicit first-use command is `python3 scripts/development-epoch.py prepare --confirm-inactive-development`; repeating it fails closed and preserves the existing intent/candidate.

Backups use private custom-format dumps and SHA-256 manifests identifying the exact epoch and prepared state. Verification checks identity and checksum before restoring into a new randomly named test database, compares every non-system schema's rows, columns, constraints, indexes and sequence state, then freezes the restored copy. No overwrite or historical import occurs. Candidate backup `426b9b6d-cce4-4bcb-97f8-d2d3dfa4758a` passed restoration into `supa_epoch_restore_test_cdb0d00e32144563b19b336a166dfcc0`. The source fingerprint stayed unchanged.

The private backup policy records `destination: null`, `transfer_authorized: false`, and `upload_enabled: false`, matching the operator's “leave unset” answer. Future `configure-backup --destination /absolute/independent/mount` writes an immutable configuration proposal only; it rejects URLs, temporary storage, repository paths and traversal. It does not create the destination or copy/upload any file. Mounted-volume identity, symlink resolution, encryption, retention and approval must be verified before implementing/authorizing an actual transfer.

Current backups remain private plaintext on the same local disk. They are not an independent disaster-recovery copy, continuous WAL archive, complete privilege/extension audit or verified cross-host role migration. Agree on those operational requirements before relying on them for writable application state.

## Validation evidence

- **126 Rust tests passed**, full workspace/all targets including PostgreSQL, permissions, accounting, quarantine, scheduling and recovery tests. The new prepared-epoch regression passed against an isolated test schema.
- **18 existing frontend contract tests plus 3 epoch tests passed**. Epoch tests cover strict health metadata, all ten fresh-state views without private reads or execution forms, and honest unidentified/disconnected states.
- **6 infrastructure safety tests plus 4 new epoch-tool safety tests passed**.
- Rustfmt, all-target Clippy with warnings denied, frontend syntax checks, ESLint, TypeScript checks, static asset build and Next production build passed.
- The real empty candidate passed checksum verification for all eight migrations, a full local backup/restore drill, unchanged-fingerprint checks and zero-record assertions across 27 application tables.
- The read-only smoke test used the actual Rust API and candidate database on random loopback ports: five unsafe startup configurations failed, the worker refused startup, an authenticated write returned 403, and twenty view/viewport combinations showed the correct epoch without overflow or execution forms. No agents were seeded.
- REA discovered the owned browser, inspected the actual preview and captured desktop 1440×1000 and mobile 390×844 screenshots. Playwright performed navigation/viewport actions and separately verified menu keyboard behavior and reduced motion. REA's default accessibility-text capture was filtered; screenshots and the direct browser checks are the evidence for displayed wording.
- **150 existing desktop/tablet/mobile browser regression tests passed**, preserving intro, portal, typing hands, pointer tracking, navigation, scrolling, accessibility controls and existing operator workflows. Zero paid or provider requests, live authorizations, uploads or main-site cutovers occurred.

Evidence: `artifacts/development-epoch/verification.json`, `browser-check.json`, `rea-evidence.json`, `desktop.png`, and `mobile.png`. Preview services are temporary and are stopped after validation. The candidate remains prepared and read-only.

## Exact activation requirements — still outstanding

Activation is a separate milestone and approval boundary. A flag change or manually disabling read-only mode is insufficient and is not an approved procedure.

1. Approve this exact epoch UUID, database and namespace for a **fresh development application**, explicitly accepting that historical data is unrecovered and excluded. Specify the operator principal(s) and least-privilege roles; credentials must be created/stored server-side, never pasted into public reports.
2. Review a concrete activation change set: an additive metadata transition with immutable actor/approval/time records, explicit permissions for active development state, separate migration/runtime database roles, and corresponding API/worker policy. Current schema/API/worker deliberately refuse activation; those changes do not exist yet.
3. Re-run isolation, cross-user access, quarantine, accounting, backup restoration and startup-gate tests against the approved activation candidate. Preserve all archived budget evidence. Do not provision a replacement paid budget or lift historical quarantine.
4. Review and approve the exact private configuration cutover for the API and the frontend at port 3200, including expected epoch ID, local socket connection and new scoped credentials. Do not reuse evaluation settings or import secrets. Keep `SUPA_PROVIDER=offline`, `SUPA_ALLOW_OPENAI=false`, `SUPA_CONTROLLED_INFERENCE=false`, `SUPA_API_SCHEDULER_ENABLED=false`, wallets/signing disabled, and the worker stopped.
5. Define backup destination/encryption/retention or explicitly accept the documented same-disk limitation for development. Any private-file copy/upload needs separate explicit approval; none is inferred from activation.
6. Approve activation immediately before applying that reviewed change set. Record before/after inventories, health identity and ownership audit. Start with zero agents and zero execution authorizations; fresh agent creation and bounded offline execution require explicit subsequent consent.
7. Rollback must stop application services, preserve the candidate and its evidence, and restore the prior configuration without overwriting history or resuming uncertain work. Do not advertise the missing historical backend as recovered.

No GPT project/billing verification, live inference permission, financial transaction, token launch, trading, publishing, child-agent creation, push or production deployment is authorized by this preparation.
