# Persistent PostgreSQL infrastructure and accounting integrity

The persistent **nonproduction validation** cluster is implemented and tested. The historical PostgreSQL cluster remains **unrecovered**. No application adoption, historical import, production initialization, paid inference, migration of historical data, push, or deployment occurred. The approved frontend and all ten sections remain unchanged.

## Storage and service boundaries

- PostgreSQL 16 data: `postgres-data/infrastructure-v1/data`, outside `/tmp` and excluded by the existing `/postgres-data/` Git rule. No `.gitignore` change was needed.
- Parent, data, socket and backup directories are owner-only; manifests and backup material are private. Initialization enables PostgreSQL data checksums. `fsync`, `synchronous_commit`, and `full_page_writes` are on.
- Socket directory: `postgres-data/infrastructure-v1/socket`, local port number 55440. `listen_addresses` is empty: **no TCP listener**. Local peer authentication uses the operating-system identity; host authentication is rejected. No new passwords or API credentials are generated or loaded.
- Database `supa_infrastructure_test` is explicitly nonproduction and contains synthetic Rust test fixtures. Seven existing migrations were applied there by the test suite. This is not `supa_eval` or `supa_devnet`.
- The application configuration is not repointed. The Rust API and autonomous worker are not started. No new live budget, operator ownership or execution authorization is adopted for actual use.
- Data survives PostgreSQL restart. Automatic boot startup is deliberately not installed; persistent storage does not imply a running service after reboot. Only the dedicated cluster is managed by the helper.

The helper refuses reinitialization of a nonempty directory, insecure permissions, symlinks, wrong cluster identity and unignored storage. It serializes normal infrastructure operations. It never follows an arbitrary database URL, accepts an existing restore destination, runs `--clean`, imports a supplied external dump, resets a budget, or launches inference.

```sh
# First initialization only; already completed for this workspace:
python3 scripts/persistent-postgres.py prepare --confirm-test-infrastructure

python3 scripts/persistent-postgres.py status
python3 scripts/persistent-postgres.py stop
python3 scripts/persistent-postgres.py start
python3 scripts/persistent-postgres.py backup
python3 scripts/persistent-postgres.py verify --backup-id <UUID-returned-by-backup>
```

The new database is intentionally not connected to the public directory. Existing `.env.eval`, `.env.devnet`, and `web/.env.local` retain their previous meaning. Do not point them at this test database to make the site appear recovered.

## Historical evidence and quarantine

The previous read-only assessment remains applicable: `/tmp/supa-agent-pg` is absent, the historical configuration refers to loopback 55432, and no original cluster or complete backup was located. The user confirmed no known backup path. Boot temporary-directory cleanup is the likely explanation, not a proven file-by-file deletion event. No fresh historical search or model-access probe was necessary for this milestone.

Preserved local evaluation snapshots are partial accounting evidence, not a restorable database:

| Archived item | Preserved evidence |
| --- | --- |
| Budget UUID | `ad91e475-f1da-495a-8247-139643d2d8b1` |
| Cumulative ceiling | 100,000 microUSD / $0.10 |
| Latest archived commitment | 10,299 microUSD / $0.010299 |
| Unresolved hold | 4,048 microUSD / $0.004048 |
| Unknown request | Historical HTTP 401, no usage evidence |
| Historical recovery status | Unrecovered |

These values are not a current balance or provider invoice. No remaining allowance is granted from these snapshots. A new metadata HTTP 200 does not refund the historical hold.

The legacy inference method previously could insert a missing budget automatically. The new persistence-layer quarantine rejects the archived UUID **before** any reservation or simulated charge in both legacy and controlled workflows. Settlement, request-stage mutation and controlled tool approval also reject this UUID. Both relational budget IDs and JSON policy IDs are checked, so inconsistent restored metadata cannot authorize an automatic refund. Read-only history remains accessible. There is no environment switch to override quarantine: lifting it requires an explicitly reviewed reconciliation and code change.

No historical accounting rows were inserted by this milestone. Regression tests explicitly create synthetic archived-UUID records only inside isolated test schemas to prove that reuse, concurrent reservation and settlement cannot change the fixture's commitment or hold. These fixtures are not adopted history. The public test schema has zero rows for the archived UUID.

This is an application safeguard, not protection against the database superuser directly editing SQL or changing code. The current OS owner administers this local test cluster. Production least-privilege runtime roles remain an adoption decision.

## Backup and restoration verification

`backup` creates a new private UUID directory, a custom-format `pg_dump` archive, a password-free global-role SQL archive, and an exclusive manifest with SHA-256 hashes, source system identity, purpose and quarantine evidence. No private backup or role SQL is committed. A quiescent source is required; before/after state hashes must match or the backup is left unverified. No writers or worker were running during the drill.

`verify` accepts only one of these local UUID manifests. It checks identity, format version and both hashes **before** creating a new `supa_restore_test_<random>` database. `pg_restore --exit-on-error --single-transaction` restores without `--clean`, `--create`, or changing the source. Roles are archived and hash-verified but never imported automatically. The existing matching local owner is used.

The comparison covers all non-system schemas: ordered table contents, column definitions, constraints, indexes, migration records and sequence state. Both the source and restored databases matched. A stop/start of the source also preserved the same fingerprint. The source, backups and restore verification databases remain available; there is no deletion or retention command.

Limitations: same-disk backups do not survive disk loss, are private but not encrypted, and are not continuous WAL/PITR backups. The fingerprint is not a complete privilege/function/extension audit. Restoring to a different host requires review of roles, ACLs, extension availability, version compatibility and dump provenance. An attacker with the same OS account can alter both archives and manifests. Agree on encryption, an independent backup destination, retention, recovery-point and recovery-time targets before application adoption. No off-host copy or upload was performed.

PostgreSQL documents [consistent custom-format dumps](https://www.postgresql.org/docs/16/app-pgdump.html), [restore execution and overwrite options](https://www.postgresql.org/docs/16/app-pgrestore.html), and [local peer authentication](https://www.postgresql.org/docs/16/auth-peer.html). Only locally generated, trusted archives are automatically verified here.

## Explicit adoption and reconciliation procedure — not executed

1. **Decide the recovery track.** Either provide an original cluster/full backup and restore an isolated forensic copy, or explicitly accept that historical state is unavailable and authorize a separately named application epoch. A new epoch must not be called recovered history or used to bypass earlier spending limits. No new paid budget is authorized by adopting storage.
2. **Inventory before import.** Hash the supplied source; compare PostgreSQL version, migrations, agents, memories, goals, wallet networks, execution leases, calls and ledger totals against later archived evidence. Produce a dry-run mapping with every source identifier and conflict. Preserve the source and raw evidence privately.
3. **Establish ownership explicitly.** Do not assign legacy agents to whichever operator logs in first. Require independent identity evidence and operator approval for each source-agent-to-owner mapping. Unknown ownership stays unassigned and inaccessible. Treat partial memories as provenance-tagged historical evidence or hypotheses, never verified new achievements.
4. **Do not import execution consent.** Preserve old authorization records as closed historical evidence. Adopted agents start stopped/paused, with no active leases or schedules. Mark interrupted external work uncertain and require review; never replay it. New offline limits and ownership grants require fresh, scoped consent and audited migration actions.
5. **Reconcile accounting separately.** Preserve the original UUID, $0.10 ceiling, all calls and the $0.004048 hold. Establish a canonical ledger from complete records and provider billing evidence. An older backup below $0.010299 requires discrepancy review, not a lower opening commitment. No refund, release, settlement, budget replacement or ceiling increase follows from missing data, a successful authentication probe, or storage adoption.
6. **Review a concrete change set.** List proposed IDs, roles, closed authorizations, deduplication keys, preserved reservations and schema changes; include before/after totals and rollback-by-new-copy recovery. Obtain explicit approval before applying it. No adoption/import SQL or schema upgrade is executed by the helper.
7. **Verify the isolated candidate.** Run migration checksum checks, cross-user denial tests, restart/restore tests, quarantine tests and frontend checks against the approved candidate. Record an immutable adoption decision and retain evidence of conflicts. An unapproved mapping must fail closed.
8. **Authorize cutover separately.** Select durable application storage, restricted migrator/runtime roles, operational backups and service startup policy. Only then approve configuration changes and any schema migrations. Paid GPT inference, continuous scheduling, signing, trading, launches and transactions remain disabled; they require their own later authorization.

No verified OpenAI project ID or account-side spending controls are available from the preceding milestone. Infrastructure adoption cannot resolve that blocker.

## Validation and browser evidence

- **125 Rust tests passed**, full workspace/all targets with ignored PostgreSQL integrations enabled, using only the new test database and mocked providers. The additional quarantine regression verifies missing-budget rejection, concurrent attempts, unchanged simulated balance, blocked legacy 429 settlement, blocked controlled no-generation refund, and unchanged records after reconnect.
- **6 infrastructure safety tests passed**: private paths, exclusive manifests, secret-free child environments, no reinitialization, path traversal and tampered-backup rejection.
- Rustfmt and all-target Clippy with warnings denied passed. No frontend code changed, so the frontend was not rebuilt unnecessarily.
- Real PostgreSQL backup, separate-database restore, all-user-schema state comparison and source restart verification passed. Actual paid/provider requests: **zero**.
- REA `list_browser_targets`, `inspect_web_page`, and `capture_web_screenshot` inspected the actual local site. Desktop 1440×1000 and mobile 390×844 show the approved Intelligence interface with unavailable agent records. Playwright verified HTTP 503 `backend_disconnected`, disabled controls, completed typing and no horizontal overflow, and resized the owned browser for REA screenshots.
- REA accessibility text was policy-filtered; no claim is made that its passive inspection performed the interactions. No recovered-agent end-to-end workflow was possible. Existing ten-section navigation and frontend source remain unchanged.

Evidence: `artifacts/persistent-postgres/verification.json`, `browser.json`, `desktop.png`, and `mobile.png`. Private database files and backups are excluded from Git. The one-file commit audit and secrets scan are required before completion; no push is authorized.
