# Final local activation safety validation — read-only

Report finalized 2026-10-30. **Validation complete; activation is blocked.** The operator chose to keep the real development epoch prepared and inactive. This work did not activate it, apply migration 9 to it, install its pending credentials, grant its owner access, or repoint the primary frontend. No application UI or production implementation was changed.

## Scope and real state

- Real epoch: `fd2f52e0-33fc-4c0d-893a-7b08b26ab748`.
- Real database: `supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748`.
- Proposed owner: KI Daniel, UUID `6fd863e1-745d-4a06-b68e-eea69e854a69`, role `owner`, state `prepared_ungranted`.
- At completion of the disposable run, the real database and protected configuration fingerprints matched their before-run values: prepared, migration 8, no real agents, ACL grants, budgets, authorizations or worker-health records. Pending owner files and the primary frontend configuration were unchanged.
- **Post-interruption qualification:** the primary PostgreSQL service is now stopped (`pg_ctl status` returned 3; its socket is absent). The final SQL recheck could not connect. It was left stopped; no real database restart/recovery was attempted. The preparation manifest still says prepared, pending owner records still deny grants/authentication, and current file checks found none of the pending secrets installed in existing runtime profiles. Live migration/state claims refer to the successful pre-interruption check, not a fresh query after restart of the tooling environment.

All mutating tests used a new, independent PostgreSQL 16 cluster under ignored private directory `postgres-data/safety/75628523232a`. The real cluster was used only for identity checks, read-only fingerprints and `pg_dump`. Cluster identity and socket-only networking were checked before disposable operations. The new cluster, API, browser and site services stopped after validation. Earlier failed disposable attempts were retained privately and stopped too.

Evidence: `artifacts/final-local-safety/validation.json`, `browser.json`, `rea-evidence.json`, `desktop.png`, and `mobile.png`.

## Service and authentication profiles

**Passed in the disposable environment:**

- Proposed API bind is exactly `127.0.0.1:8080`; proposed frontend port is `3200`. Actual test services used temporary loopback ports so neither primary port/configuration was repointed. Frontend server source binds loopback explicitly.
- A real synthetic PostgreSQL service role authenticated over a private Unix socket using a database/role-specific SCRAM rule. Wrong password failed. Existing peer administration stayed available in the disposable cluster; PostgreSQL TCP stayed disabled.
- The service role had SELECT/schema-USAGE/database-CONNECT privileges, no role memberships and no superuser, database creation, role creation, replication or RLS bypass attributes. Write, DDL, TRUNCATE, temporary-table and administrator SET ROLE attempts were rejected. Tests used `BEGIN READ WRITE` for write/DDL probes, so denials did not rely solely on the read-only default.
- Synthetic browser, operator API and public-service credentials were distinct. Legacy `SUPA_OPERATOR_ID`, single-account fallback and provider keys were absent from the clean test environments. Rust mapped the operator token server-side; the frontend mapped the Basic account server-side.
- Unauthorized and forged-role access was denied, the public service token could not become an owner, and even the synthetic owner could not mutate the read-only epoch. Existing Rust cross-user isolation tests passed in separate disposable test schemas.
- Current pending real credential files are mode 0600 under mode-0700 owned directories, ignored and untracked. Symlink/hardlink/permission and preparation-stability tests passed. Their credentials were not installed into services. Current `.env.eval`, `.env.devnet` and `web/.env.local` contain none of those pending secret values.

**Profile blocker: invalid proposed database URL.** The previous review template uses userinfo with an empty authority: `postgresql://role:your-password@/database?...`. The SQLx executable rejected that shape in the disposable test. An explicit hostname plus Unix-socket override worked and authenticated the least-privilege role, for example:

```text
postgresql://role:your-password@localhost/database?host=/absolute/private/socket&port=55440
```

Here SQLx's `host` query option selects the Unix socket; `localhost` is the URL authority, not evidence of a TCP connection. This is a sanitized illustrative shape, not a configured credential. The existing real proposal/profile was not rewritten or installed. The URL rejection probe was also executed separately against the live disposable cluster during the run, before it stopped; it was then incorporated into the reusable validator.

## Active-state backup and restoration

- The prepared source dump restored into `supa_cutover_test_safety` in the independent disposable cluster. Table data, column definitions, constraints, indexes and sequence-state fingerprints matched the real source before migration.
- Migration/transition exercises then produced an active **disposable** clone with three transition audit records: a wrong-digest probe activation, its compensating rollback, and a normal synthetic activation. None was an actual operator authorization or a spending authorization.
- A new private active-state dump restored into `supa_safety_restored_test`. It matched the active source fingerprint, retained all three audit rows and active epoch metadata, and was explicitly frozen read-only. Existing dump paths were never overwritten and no restore targeted the real database.
- Dumps were mode 0600 under private directories. Backup commands ran under separate local peer administration; the application reader was not promoted into a backup administrator. This validates the underlying active dump/restore procedure, not an installed operational backup service.
- Historical ledger evidence was not imported into either epoch. Quarantine remains enforced in application code, and the Rust accounting regression tested preservation of the archived commitment/hold and denial of UUID reuse/refund in synthetic records. A PostgreSQL application dump alone does not archive repository quarantine code, external evidence, credentials, global roles or `pg_hba.conf`; those need a separately reviewed recovery package.

Independent storage remains **unset** and uploads remain disabled. Same-disk backups do not survive disk loss, host loss or compromise. Private plaintext dumps and credentials are not encrypted disaster recovery. Retention, encrypted independent copies and cross-host restoration require separate decisions/authorization. No private data was uploaded.

## Migration 9 and approval safeguards

**Passed:** all eight source/clone SQLx checksums matched current migration files using SHA-384; migration 9 applied only to disposable clones; all nine checksums then matched. Repeating migration was a no-op. Deliberately corrupting a stored checksum in a separate disposable clone caused SQLx to reject migration without applying version 9.

The migration-only executable was exercised with hostile scheduling/owner/provider environment flags. It applied schema only: state remained `prepared`, revision 0, with zero grants, agents, worker heartbeats, transition audits or budgets. It did not instantiate the API, scheduler or provider. Its database-name guard still refuses real candidate targets. A superuser migration connection itself is **not** a sandbox; administrators can bypass safeguards outside that restricted executable. No new real-target runner or privileges were enabled.

Migration 9 adds revision/approval metadata and append-only transition audits, and expands the state constraint to include active read-only. It does not activate a row or establish a principal. The audit trigger rejects UPDATE, DELETE and TRUNCATE. Operational rollback preserves schema 9 and audit history, changes state back to prepared and advances revision; it is not a schema downgrade. Removing that schema/history would be a separate destructive action and is not authorized.

**Blocking finding: no trusted approval-digest binding.** Stale revisions, malformed hashes and duplicate approval requests were rejected. However, a deliberately wrong but syntactically valid 64-character hexadecimal digest was accepted and stored. The SQL checks format and records caller input; it does not compare against a trusted approved plan. This affects administrative approval integrity, not unauthenticated browser access: the test required a disposable administrative connection. An approval UUID/hash is not proof of consent.

The production transition implementation was left unchanged because this milestone is validation-only. Before activation, a reviewed path must bind the actual plan bytes/digest, owner/approver, exact epoch, migration checksums, limits and configuration hashes to a trusted explicit consent record, reject a different valid hash, and pass replay/concurrency/recovery tests. Merely adding another caller-controlled hash argument would not close the gap.

## Tests and browser evidence

- **128 Rust/PostgreSQL tests passed**, including ignored database tests, within this new disposable cluster. Permission, role escalation, cross-user isolation, accounting quarantine, epoch fencing, recovery and audit tests were included.
- **23 frontend contract tests passed**.
- **17 Python preparation/rollback safety tests passed**.
- JavaScript syntax checks, Python compilation, static frontend build and the disposable migration example build passed.
- **30 browser view checks passed:** all ten sections at 1440, 768 and 390 pixel widths, no page errors or horizontal overflow. Authentication, restart persistence, owner mutation denial, four unsafe startup flags and worker startup rejection also passed against the SCRAM-authenticated SELECT-only backend.
- REA actually listed the browser target, inspected the rendered Intelligence view and captured desktop/mobile screenshots; Playwright supplied navigation, authentication and viewport changes. The screenshots show the active disposable clone, which retained the candidate UUID for configuration testing. They do **not** show the real epoch activated.
- Tool-session interruption occurred before capture IDs were saved. The PNG bytes, hashes and known inspection Evidence ID remain in `rea-evidence.json`; capture IDs were not invented. A subsequent REA retained-record query returned no records in the new tool session. This limits metadata retrieval, not the existence of the actual captures visible during the run.

Reusable validation entry point (disposable writes only; real source must already be available read-only):

```sh
SUPA_REA_CDP_PORT=19358 python3 scripts/validate-local-safety.py
```

It preserves existing successful evidence by refusing to overwrite `validation.json`. It never starts the primary PostgreSQL service. REA capture stages wait for explicit process signals from the validation operator. The wrong-valid-digest acceptance assertion deliberately demonstrates a known blocker; it must be inverted into a rejection test after a separately scoped fix. No test calls a paid provider or enables real financial actions.

## Final safety status and prerequisites

Historical database remains unrecovered. Budget `ad91e475-f1da-495a-8247-139643d2d8b1` remains quarantined with ceiling **$0.10**, archived commitment **$0.010299** and unresolved hold **$0.004048**. No refund, release, replacement budget, ownership adoption or fabricated history occurred. These are archived evidence amounts, not reconstructed rows in the empty development database.

Real owner grants, credential installation, workers, paid GPT, trading, launches and financial transactions were not enabled. The primary frontend was not repointed. Only validation tools and this report/evidence were added; approved visuals and all ten sections were preserved. No pushing, deployment, production migration, real database mutation or paid request occurred.

Before any future activation:

1. Correct and test trusted consent/plan-digest binding; reject a well-formed wrong digest.
2. Correct and finalize the SQLx-compatible service URL and exact API/site profiles, with separately approved least-privilege role/SCRAM setup and distinct server-only credentials. Keep loopback 8080/3200 and all execution/provider flags disabled.
3. Finalize and test a real-target migration-only path that verifies cluster/database identity and the exact migration/checksum set without starting runtime services. Do not bypass the disposable tool's guard.
4. Finalize the active-state operational backup manifest/profile and its recovery package. Keep independent storage unset unless separately authorized; explicitly resolve acceptance of same-disk-only recovery limitations.
5. Re-establish availability for a fresh **read-only** real-state/backup preflight under appropriate authorization; the primary PostgreSQL service was left stopped after interruption. Confirm prepared/migration 8, no grants, unchanged quarantine and untouched primary routing.
6. Obtain a **new explicit operator approval** for the precise owner UUID binding, credential/role installation, migration 9 on the real candidate, and subsequent read-only activation/frontend switch. Approval must identify the reviewed plan and rollback scope. This validation and the prior readiness reports are not that approval.

**Stop at validation. Do not activate.** No approval is requested or inferred by this report.
