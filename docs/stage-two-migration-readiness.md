# Stage 2 — migration-only entry point readiness

Stage 2 prepares and validates administration code only. **Actual migration execution is not authorized.** The primary PostgreSQL cluster stayed stopped and was never connected to during this milestone. Its last live verification remains the successful Stage 1 check: prepared, migration 8, read-only, with no agents, ownership grants, spending budgets or execution authorizations. Primary control/configuration files, pending identities, Stage 1 backup, frontend configuration and historical accounting evidence match their pre-validation hashes.

## Fixed target and process boundary

`crates/migration-admin` builds `supa-migration-admin`. It has no Supa API, runtime, provider, wallet, tool or worker dependency. It does not load dotenv files, start PostgreSQL, launch services, change frontend routing, import history or create agent records.

| Property | Required value |
| --- | --- |
| Database | `supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748` |
| Epoch | `fd2f52e0-33fc-4c0d-893a-7b08b26ab748` |
| Accounting namespace | `641457b2-5fd8-4b31-b85f-c58839626831` |
| Proposed migration custodian | KI Daniel, `e8c6ade3-917f-4fe4-a092-f8a58eccb487`; still unverified/ungranted |
| Separate owner/executor | KI Daniel, `6fd863e1-745d-4a06-b68e-eea69e854a69`; unchanged and ungranted |
| Data directory | `/home/pikoo/Desktop/supa-agent/postgres-data/infrastructure-v1/data` |
| Socket directory | `/home/pikoo/Desktop/supa-agent/postgres-data/infrastructure-v1/socket` |
| Port | `55440`, Unix socket only; no TCP |
| Starting/target schema | Exactly 8 → exactly 10 |
| Starting/ending epoch state | Prepared; ending revision 0 and approval ID NULL |
| Historical database | Unrecovered |

Database, socket, data directory, epoch, namespace, custodian principal, Stage 1 backup hash and reviewed state/catalog fingerprints are pinned in the executable. The signed request cannot override them. There is no arbitrary database argument, environment target, disposable switch, activation operation or resume-from-9 mode. The existing `review_cutover` executable and its disposable-name restriction remain unchanged.

The cluster system identifier, read from its **offline** control file without starting it, is `7702158196701990299`. Future authority configuration and signed consent must bind that identifier; execution compares it with the connected server. The checked backup is `postgres-data/preflight/1d0e9efe-e66e-4e4f-afc4-478eee4f0833/database.dump`, SHA-256 `600d2bb19b52b4f17d4278459ba722ea5f1135df703b8efc1b0b3db0c8360671`.

## Independent approval bootstrap

Migration 10 creates the activation-approval infrastructure. It cannot be used to approve its own installation. The migration runner therefore requires a **separate Ed25519-signed migration-only approval**, not an activation approval or a caller-supplied hash.

The verifier always reads `/etc/supa-agent/migration-authority.json`. That path cannot be replaced through command arguments or environment variables. The file must be a regular, single-link root-owned file with no group/world write access, beneath root-owned directories without group/world write access or symlink substitution. The requesting service cannot register its own verifier key there.

The trust file's strict fields are:

- `version`: 1.
- `principal_id`: the proposed custodian UUID above.
- `public_key_hex`: independently verified Ed25519 raw public key, 32 bytes as 64 lowercase hexadecimal characters.
- `system_identifier`: the verified primary cluster identifier.
- `migrator_login`: the explicitly approved existing administrator login; Stage 1 has only `pikoo` besides PostgreSQL predefined roles.
- `identity_and_independent_control_verified`: true only after actual independent verification and explicit authorization. The preparation template leaves it false.

**No trust file or real signing key was installed or generated.** The proposed custodian identity file is not a trust anchor. The separate signer must retain the private key outside application environments and the repository. Installing its public trust configuration requires new approval. This does not grant the custodian any PostgreSQL activation-approval authority.

The signed approval is the exact UTF-8 JSON byte sequence; signature verification precedes database connection. It binds a fresh nonzero approval UUID, scope `migrate_development_epoch_only`, custodian UUID, issue/expiry times, all fixed target identifiers, system identifier, expected version 8, target version 10, backup hash, reviewed state/catalog fingerprints, original-row snapshot, executable SHA-256 and all ten compiled SQLx migration SHA-384 checksums. Validity is at most one hour; future-dated or expired requests fail. Unknown fields, forged signatures, changed bytes, wrong principals/targets/scopes/artifacts/checksums fail closed. Expiry is checked again before commit.

`artifacts/stage-two-migrations/requirements.json` records the exact plan schema and pinned values for the built binary. It is **an unsigned preparation template**, not an approval: nonce/timestamps/public key are unset and authority verification is false. Rebuilding or replacing the binary requires reviewing its new SHA-256 and obtaining a matching fresh signature. The original Stage 1 backup remains pinned: reverify it and retain any additional fresh backup separately. Do not overwrite that archive or substitute another backup hash without a newly reviewed binary/approval. Never substitute the synthetic test keys or clone approvals.

## Migration procedure and effects

The dedicated process:

1. Rejects inherited `PG*`, `SUPA_*`, `DATABASE_URL` and `OPENAI_API_KEY` variables. Only its three exact file arguments are accepted.
2. Verifies root-controlled authority, private file ownership/modes, signature, expiry, fixed target and artifacts. Approval/connection files must be effective-user-owned, single-link, regular files without group/world permissions; final-component symlinks are rejected. The signature file is exactly 128 lowercase hexadecimal characters, **without a trailing newline**.
3. Builds SQLx options directly from the pinned socket/database/port and a private connection JSON containing only `username` and optional `password`. The username must match the approved migrator. It does not accept a database URL or read a password file. Existing local peer authentication can use `{"username":"pikoo","password":null}` after explicit authorization; no new database login is required.
4. Checks server data directory, Unix socket, port, system ID, authenticated login, TCP absence, durability, read-only default, private storage permissions, expected peer/reject HBA rules, absence of unexpected custom roles/default grants, and expected database ownership/ACL.
5. Acquires the standard SQLx advisory migration lock with a bounded wait. In one serializable transaction, it enables read-write for that transaction only, locks migration/application tables and checks successful versions 1–8 against compiled SQLx SHA-384 checksums. It does not create or repair a missing migration table.
6. Requires the exact prepared epoch and empty application tables. Compares original rows and the Stage 1 logical fingerprint, including columns, constraints, indexes and sequences; separately checks owners, ACLs, functions and triggers. Drift fails before migration DDL.
7. Executes only embedded migrations 9 and 10, recording their original SQLx checksums/descriptions in `_sqlx_migrations`. Earlier records are preserved. `execution_time=-1` is retained as explicit unmeasured bookkeeping; there is no post-commit accounting update.
8. Adds `epoch_guard.migration_receipts` and its immutable update/delete/truncate trigger, revoking PUBLIC access. Stores the migration approval UUID, signer UUID, exact plan text/digest, signature/public key, binary hash, backup hash, system ID, source snapshot and 8→10 completion record. This is **migration audit evidence**, not activation consent or agent activity. A verifier must compare its stored public key with the independently trusted signer key.
9. Verifies unchanged original application rows, prepared revision 0, NULL epoch approval and empty activation authorities/executors/approvals/consumptions/audit. Commits migrations, SQLx bookkeeping and receipt together, then closes its connection.

Migration 9 adds epoch revision/approval fields, permits the later `active_read_only` state, and creates immutable `epoch_transitions`. Migration 10 adds guarded approval functions/tables and epoch/audit protection triggers. It provisions no logins, identity mappings or approvals. PUBLIC gets only schema USAGE and the boolean `verify_active` function; issuance/execution functions and approval tables receive no PUBLIC access.

No owner/executor/custodian mappings, agents, authorizations, budgets or runtime services are created. Database read-only defaults remain on. The additional migration receipt objects are part of the exact future change set and must be included in operator approval.

## Precise future execution command — do not run yet

Build and review the artifact before signing:

```sh
cargo build -p supa-migration-admin --locked --offline
sha256sum target/debug/supa-migration-admin
```

After **new explicit operator approval**, independent custodian/key verification, approved trust-file installation, a fresh backup/preflight confirmation and preparation of the three private files, the execution command is:

```sh
env -i PATH=/usr/bin:/bin HOME=/home/pikoo \
  /home/pikoo/Desktop/supa-agent/target/debug/supa-migration-admin \
  --approval-file /home/pikoo/Desktop/supa-agent/postgres-data/migration-approval/approval.json \
  --signature-file /home/pikoo/Desktop/supa-agent/postgres-data/migration-approval/approval.signature.hex \
  --connection-file /home/pikoo/Desktop/supa-agent/postgres-data/migration-approval/connection.json
```

Those input files and the trust anchor were **not created** by this milestone. Real PostgreSQL startup requires new explicit authorization too; the executable never starts it. Run as the approved local administrator, not as the API/frontend service. No API/worker is started automatically after success. Credentials and raw database diagnostics are never printed; failures emit bounded static codes.

## Failure, duplicate execution and rollback limits

- Errors, backend termination or connection loss **before commit** roll back migration 9, migration 10, receipt and SQLx records together. Confirmed rollback leaves schema 8.
- Concurrent invocations serialize under SQLx's migration lock. After one successful commit, subsequent requests see version 10 and are rejected before DDL. There is no automatic retry loop.
- A lost COMMIT response is ambiguous. Inspect schema/checksums and the matching receipt through separately authorized read-only diagnostics; never assume failure or blindly replay. The signature authorizes this bounded 8→10 commit, not an automatic recovery policy. Obtain fresh operator review/consent before manual retry.
- An already-partial legacy schema 9, dirty/unknown migration version, altered checksum or schema/permission drift is rejected. It requires a separate repair/recovery plan; this executable neither deletes migration records nor resumes arbitrarily.
- After commit, schema 10 and the receipt are durable. No down migration or audit-erasure command is provided. Preserve schema/audit history; do not delete `_sqlx_migrations` rows, rewrite checksums or drop approval infrastructure as rollback.
- Backup restoration is verified only into a new disposable database. Restoring over the primary, adopting a renamed restore, or changing the pinned target needs separate review and authorization. Existing configuration remains unchanged, so there is no frontend cutover to reverse in Stage 2.
- Existing schema-8 preparation/preflight helpers deliberately reject an upgraded schema and their archived epoch manifests must not be rewritten to conceal the upgrade. A separately reviewed schema-10 status/backup profile is needed before later application cutover; the disposable migrated-backup drill here proves restore integrity, not installation of that operational profile.
- Local OS owners, database superusers and the independent trust administrator remain trusted. They can alter binaries, database DDL or trust configuration; this runner is not a sandbox against a malicious administrator. Migration credentials must never be supplied to the requesting application.

## Validation evidence

`artifacts/stage-two-migrations/validation.json` and `tests.json` record **134 passing Rust tests**, zero failures/ignored tests, with mocked/offline inference only. Migration tests use fresh clones of the verified Stage 1 backup on a separate socket-only PostgreSQL 16 cluster. Their private test-only targets cannot be selected through the shipped CLI. A second disposable cluster isolates the existing runtime-suite SCRAM roles; neither is the primary.

Coverage includes signed scope/target/identity/time/artifact tampering, private input modes/links, absence of CLI targeting overrides, wrong epoch, corrupted checksums, permission and column drift, duplicate/concurrent requests, transaction failure after migration 9, actual backend termination, rejection of independently seeded schema 9, immutable migration receipts, signature re-verification, unchanged prepared data and zero activation authorities/approvals. Both migrations and the receipt survive a crash/restart of the disposable server. A migrated backup restores with matching rows/schema/constraints/indexes/sequences across all user schemas, including receipt/signature/audit material.

The production CLI rejects an unsigned/unconfigured request before primary access. **No successful real-target command was executed.** Real trust installation and credential/control verification remain outstanding. Rustfmt, all-target Clippy with warnings denied, workspace binary build, Python compilation and 11 existing preflight/infrastructure safety tests passed. No frontend source, routing, design or migration SQL file changed; browser testing was not required for this migration-only change.

The primary's offline control file still reports shut down, and its protected hash is unchanged. Both disposable clusters are stopped. Historical PostgreSQL remains unrecovered. Archived budget `ad91e475-f1da-495a-8247-139643d2d8b1`, its $0.10 ceiling and unresolved $0.004048 hold remain quarantined and unchanged. Same-disk backup risk is accepted for this local preflight; independent storage remains unset, with no uploads.

## Remaining approval boundaries

1. Independently verify the proposed separate KI Daniel custodian identity and control of its signing key. Approve the public-key fingerprint, administrator login, system identifier and installation of the root-owned trust file. No database custodian/owner/executor permission is implied.
2. Review the exact binary hash, unsigned requirements, migrations 9 and 10, additional immutable receipt objects, preflight/backup and rollback limitations. Authorize only the exact 8→10 migration operation and necessary local PostgreSQL startup/administrative access.
3. Obtain a fresh signed migration-only approval and private inputs immediately before the authorized command. No real signature or trusted approval exists today.
4. Verify results read-only and stop. Credential installation, owner/custodian/executor database mappings, activation approval, epoch activation and frontend connection remain later, separately authorized stages.

Workers, paid GPT inference, continuous scheduling, trading, token launches, financial transactions, pushing and deployment remain outside this approval. Stage 1 consent and this Stage 2 preparation request do not authorize actual migrations.
