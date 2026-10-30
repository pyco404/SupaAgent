# Activation security blocker remediation

This change remediates the two safety-validation blockers without activating the real development epoch. The primary PostgreSQL cluster is not started or connected to by the new validation harness. No real approval, ownership grant, runtime credential installation, worker activation, frontend repointing, paid request, push, or deployment is authorized or performed.

## Trusted activation approval

Migration `0010_trusted_epoch_approvals.sql` is additive. Migration 9 and all earlier SQLx checksums remain unchanged. Migration 10 creates a separate `epoch_guard` schema; it creates **no login, principal mapping, approval, ownership grant, or activation**.

There are three distinct duties:

| Duty | Access |
| --- | --- |
| Reviewed database administrator/migrator | Installs reviewed schema and separately approved login mappings/ACLs; not automatically an approval issuer or executor |
| Independent approval custodian | Separate authenticated database login mapped to an independently established principal; may issue/revoke consent through narrowly granted functions |
| Transition executor | Different authenticated database login bound to the approved principal and `owner` role; may consume an existing approval, never issue one |
| API service | SELECT in the application schema and a boolean trusted-activation verifier; no approval-table access, approval issuance, or transition execution |

Security-definer functions use a fixed `pg_catalog` search path and qualified objects. Identity comes from PostgreSQL `session_user`; request fields, session configuration, and `SET ROLE` cannot establish the binding. Functions reject a login mapped as both issuer and executor. PUBLIC receives neither approval-table privileges nor issuance/execution privileges. Approval, authority, executor, revocation, and consumption records reject update/delete/truncate. Normal service accounts cannot alter the functions or their owning schema.

An approval binds exact UTF-8 JSON bytes, a database-computed SHA-256 digest, database name, epoch UUID, accounting namespace, principal UUID, owner role, operation, expected revision, and the sole scope `local_development_read_only`. Its strict versioned schema also requires API/frontend/PostgreSQL configuration SHA-256 hashes and the exact successful SQLx migration SHA-384 inventory, including version 10. A trusted custodian must independently verify the configuration files against those hashes; PostgreSQL does not attest filesystem contents. Approval expires within one hour and cannot be edited or reused.

Execution locks the approval and epoch, checks expiry/revocation, exact bytes/digest, current migration inventory, authenticated identity, state, namespace and revision, then atomically appends consumption and transition audit records and advances the epoch revision. Concurrent/replayed requests cannot consume the same approval twice. Direct epoch/audit DML is guarded too, including the old caller-chosen-hash transaction and accidental ordinary DML grants. There is no activation HTTP endpoint. The Rust review executable still refuses real database names and now requires a private exact-plan file rather than a supplied hash.

Active startup requires trusted approval/consumption/audit verification, not merely `state=active_read_only`. An old version-9 activation lacking this evidence fails closed. A renamed backup restore preserves evidence but cannot inherit permission for another database name. Approval expiry governs the transition; it is not a live execution authorization. Revocation also prevents subsequent active startup verification. This milestone grants no agent execution permission.

Database superusers, object owners, and the operating-system owner remain trusted administrators: they can alter/drop database controls. These mechanisms protect against the requesting service and restricted executor, not a malicious cluster administrator. Keep migrator credentials out of API/frontend processes and independently control the custodian login. Provisioning/rotating those roles needs a separately reviewed administrative action.

## SQLx local connection profile

The review template uses an explicit authority and Unix-socket override:

```text
postgresql://<approved-service-role>:your-password@localhost/<exact-database>?host=<private-socket-directory>&port=55440
```

Percent-encode actual username/password, database name, and query values. The literal example is not an installed credential. SQLx rejects the previous empty-authority form. Tests cover reserved credential characters, percent-encoded database/socket names, actual SCRAM authentication with reserved characters, incorrect passwords, and malformed connection startup. PostgreSQL listens only on a private Unix socket; TCP is disabled and host authentication rejects access. This URL does not authorize a primary-cluster connection.

Application-table SELECT includes records the Rust API must scope to authenticated users; existing server-side ownership/role checks remain necessary, and database credentials must never reach browsers. A read-only service receives CONNECT, schema USAGE and SELECT only: no temporary objects, DDL, DML, role membership, migration ownership, approval issuance or execution. A separate administrative connection runs migrations only. The existing migration-only executable accepts disposable database names; even misleading worker/provider flags do not start application services, create grants or activate the epoch. A real-target migration runner remains a future reviewed and authorized operation. Epoch security objects are anchored in the public application schema; arbitrary custom application search paths are not supported. Test fixtures now isolate databases rather than sharing security objects across schemas.

## Validation and preserved environment

The new harness, `scripts/validate-activation-remediation.py`, creates a new private, socket-only PostgreSQL 16 cluster. It reads an existing verified prepared backup without contacting the primary, checks its SHA-256, restores a disposable copy, and replaces real epoch/namespace identifiers with synthetic identifiers **before** creating any test approval. SQLx verifies migrations 1–10. Corrupt checksums are rejected without upgrading the bad clone. Historical SQL and earlier failure evidence are retained unchanged.

Regression fixtures use separate disposable databases so their security schemas and authority records cannot leak between tests. Mocked inference, offline workers and synthetic records exist only inside disposable tests. The new active-state backup is restored into another disposable database and all user-schema rows, columns, constraints, indexes and sequences are compared, including approval/consumption/audit records. A renamed restore fails operational approval verification as intended. Dumps remain owner-only and untracked; no private database file is uploaded. Independent backup storage remains unset; same-disk copies do not protect against disk loss or host compromise. The restore drill uses the same disposable cluster and synthetic login inventory; new-host role/global-ACL provisioning and credential recovery still require separate review.

The browser harness uses synthetic accounts, SCRAM SELECT-only database access, ephemeral loopback ports, and the existing frontend source. It checks all ten sections at desktop, tablet and mobile sizes, authentication/identity spoofing, read-only mutation denial, restart persistence, unsafe startup flags, invalid connections, disabled worker startup and overflow. REA captures actual desktop/mobile output. These are disposable read-only empty-epoch views, not recovered historical records or an enabled agent runtime. No application design, logo, navigation, motion or section source is changed.

Evidence and exact results are recorded in `artifacts/activation-remediation/validation.json`, `browser.json`, `rust-tests.json`, and `rea-evidence.json`, with desktop/mobile screenshots. Protected primary control/configuration files, pending owner credentials, existing frontend configuration, prepared backup and archived accounting evidence are hashed before and after validation. The primary remains stopped. Consequently current SQL state is deliberately **not queried**: the last live verification remains prepared at migration 8, and the protected files are unchanged.

Historical PostgreSQL remains **unrecovered**. Archived budget `ad91e475-f1da-495a-8247-139643d2d8b1` retains its $0.10 ceiling and unresolved $0.004048 hold. Existing quarantine regressions prevent accidental budget reuse, release or refund. Nothing here imports or settles that ledger.

## Test results

- Full workspace/all-target Rust suite with PostgreSQL integrations enabled: **130 passed, zero failed or ignored**. Providers were mocked/offline; no paid transport was used.
- Python safety tests: owner provisioning 9, configuration rollback 4, epoch 4, infrastructure 6; **23 passed**.
- Approved frontend: **23 unit tests passed**; syntax checks and static build passed. Node reported the existing typeless-package warning when importing shared TypeScript contracts; no frontend package changes were needed.
- Browser: **30 section/viewport checks passed**, zero page errors and no horizontal overflow. Four unsafe startup flags, two invalid connection profiles, worker startup, spoofed identities and read-only mutations were rejected; restart retained trusted approval.
- SQLx checksum validation and corrupted-checksum rejection passed on clones. Trusted active-state backup/restoration comparison passed. The restored database's different name correctly fails operational approval verification.
- Rustfmt and all-target Clippy with warnings denied passed. Secrets scanning is a targeted local pattern/known-secret gate, not proof against unknown secret formats; one-file-per-commit history audit is also required before finishing; retained evidence is synthetic or sanitized, and private database/authentication files stay untracked.

## Exact future authorization requirements

The previous version-9-only approval artifacts and raw DML examples are historical evidence, **not executable cutover instructions**. Generate a new reviewed change set; do not reuse their approval ID or digest.

1. Obtain new explicit authorization to start/inspect the primary, verify its exact database/epoch/revision and take a new private backup. Verify restoration and accept same-disk-only risk or separately approve an independent destination.
2. Separately approve migration-only installation of **both** migrations 9 and 10 against the exact development database. Check unchanged earlier SQLx checksums. Schema changes are not rolled back by deleting migration records; preserve the upgraded schema and data.
3. Independently verify KI Daniel's principal `6fd863e1-745d-4a06-b68e-eea69e854a69`; explicitly approve the proposed role/identity binding, any owner grants and private credential installation. A display name alone grants nothing.
4. Approve distinct custodian, executor and read-only service login configuration and permissions. Credentials stay owner-only and server-side; legacy shared-owner authentication remains unset. No real trusted mappings or approval are created by this milestone.
5. Review exact API/frontend/PostgreSQL configuration bytes, compute their hashes, and construct the strict plan for epoch `fd2f52e0-33fc-4c0d-893a-7b08b26ab748`, its verified namespace, current revision and exact migration inventory. The independently authenticated custodian must publish a fresh finite approval for that exact plan after explicit operator consent. The requesting API cannot issue it.
6. Obtain explicit permission for the separate executor to consume that approval and for private configuration installation/frontend repointing. Keep loopback API 8080/frontend 3200, provider offline, worker stopped, scheduling/inference/wallet features disabled, and no live budgets or agents created without separate consent.
7. Verify actual health, scoped records and restart persistence. If rollback is needed, stop the reviewed services, obtain a **different** independently approved rollback plan/ID at the new revision, append the rollback transition, restore exact prior configuration only when hashes match, and retain all data and audit entries. Do not restore over the primary or remove migration/audit history.

Activation and rollback each permanently append audit evidence and advance revision. Rollback restores the prepared state/configuration; it does not erase those events or undo schema upgrades. Execution, paid inference, trading, launches, financial activity, pushing and deployment require their own later authorization. This report is not that authorization.
