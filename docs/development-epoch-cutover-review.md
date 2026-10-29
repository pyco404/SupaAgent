# Development epoch activation review and controlled cutover preparation

**Review completed; the real epoch is not activated.** The real candidate remains `prepared`, read-only, at migration 8. The primary frontend has not been repointed. Migration 9 and all activation/rollback transitions were exercised only in disposable databases. Historical data is still unrecovered.

## Proposed first cutover

The first proposed operational mode is **`active_read_only`**, not writable agent operation. It makes the epoch and genuine saved records visible while keeping ownership changes, agent creation, execution authorizations, all tool execution, wallets, paid inference, trading, launches and transactions unavailable. Writable operation would require a separate review and authorization.

Exact target:

- Epoch: `fd2f52e0-33fc-4c0d-893a-7b08b26ab748`.
- Database: `supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748`.
- Accounting namespace: `641457b2-5fd8-4b31-b85f-c58839626831`; this is not a spending budget.
- Persistent cluster: `postgres-data/infrastructure-v1/data`; Unix socket `postgres-data/infrastructure-v1/socket`, port number 55440, no TCP listener.
- Before state: `prepared`, revision 0 after the proposed additive migration. Database default remains read-only throughout the proposed cutover.

The complete, sanitized configuration change set is **`artifacts/cutover-review/change-set.json`**. It is review material, not a runnable environment file. Unresolved identity and secret fields are explicit placeholders; `ready_for_activation` is false.

## Exact configuration and database changes requiring approval

1. Verify a fresh private backup and independent restoration before applying anything to the real candidate. Check its eight current migration checksums, epoch/namespace and zero application-record inventory. Preserve `postgres-data/development-epoch.json` as the preparation record; do not rewrite it to imply history was recovered.
2. Approve and apply only `0009_epoch_cutover_review.sql` to this candidate through an administrative migration identity. It allows `prepared` and `active_read_only`, adds revision/approval metadata and creates an append-only `epoch_transitions` audit. It does not activate any row or create an agent, grant or budget.
3. Review the real-target administrative transition using the tested row-lock/revision/unique-approval algorithm in `Store::exercise_epoch_transition`. It must bind the exact epoch, current revision, independently verified approver UUID, unique approval UUID and SHA-256 of the approved plan. The shipped exercise CLI **refuses the real database name**. Real-target enablement or a rendered administrative transaction is a remaining approval/implementation boundary; do not bypass the guard by renaming the database or using manual ad hoc updates.
4. Provision a separately approved SELECT-only service database role. The proposal in the change set grants only database connection, schema usage and table SELECT. Do not give it DDL/DML, sequence mutation, superuser, role-creation, database-creation, replication or migrator membership. No such role or grants were created here. Validate the actual role after provisioning; the disposable test uses the existing OS administrator with a read-only connection default and is not evidence that the service role is deployed.
5. To authenticate that future role over the private socket, review one database-and-role-specific `local <exact_database> <approved_role> scram-sha-256` rule before the existing peer rule. Set its password in a private secret workflow, retain current peer administration, and keep host authentication rejected and TCP disabled. No PostgreSQL authentication file was modified.
6. Prepare new owner-only `api.env` and `site.env` files in a private cutover directory, with distinct server-only service and operator credentials. Do not reuse/import evaluation keys or turn a shared service token into an owner. Snapshot and hash the prior private files without printing their values. Load the reviewed profiles through a clean allowlisted environment.

API profile values:

| Variable | Required value |
| --- | --- |
| `DATABASE_URL` | Exact candidate database/socket, approved SELECT-only service role and server-only secret; template in change set |
| `SUPA_BIND` | `127.0.0.1:8080` |
| `SUPA_EXPECTED_EPOCH_ID` | `fd2f52e0-33fc-4c0d-893a-7b08b26ab748` |
| `SUPA_EPOCH_MODE` | `active_read_only` |
| `SUPA_EPOCH_READONLY_PREVIEW` | `false` |
| `SUPA_EPOCH_APPROVAL_ID` | Exact approved activation audit UUID, currently unset |
| `SUPA_OPERATOR_CREDENTIALS` | Approved named principal/role roster with separate server-only tokens; unresolved |
| `SUPA_API_TOKEN` | New private service credential, not an owner credential |
| `SUPA_PROVIDER` | `offline` |
| `SUPA_ALLOW_OPENAI`, `SUPA_CONTROLLED_INFERENCE`, `SUPA_API_SCHEDULER_ENABLED`, `SUPA_WALLET_ENABLED`, `SUPA_PRIVATE_STAGING` | All `false` |

Frontend launch values: `PORT=3200` and `SUPA_SITE_ENV_FILE=<private_cutover_directory>/site.env`. The private file sets `SUPA_BACKEND_URL=http://127.0.0.1:8080`, the matching service token, and explicit `SUPA_WEB_OPERATORS` account-to-backend-token mappings. No frontend asset, routing tree or visual redesign is required. Keep API credentials outside browser bundles.

Explicitly unset inherited OpenAI/project/live-budget credentials and wallet custody settings. Unset `SUPA_OPERATOR_ID` to prevent automatic owner mapping of the shared token, and omit the legacy single-account username/password fallback when using the named roster. Do not start the worker. No worker, inference, trading or financial enablement belongs in this cutover.

7. Keep a distinct private operational manifest for any eventually approved active state, binding epoch, revision, approval, source configuration hashes and backup procedure. The existing `development-epoch.py` helper is intentionally prepared-only and will reject active metadata; do not make it pretend the active database is still prepared. The active-state dump/restore procedure was tested in the disposable drill. An approved real operational backup invocation/profile must be finalized with the service/migration identities before cutover.
8. Only after the above is concrete and reviewed, obtain fresh explicit approval immediately before real activation and primary-site configuration switch. No current command applies these changes to the real candidate.

## Named operator identity requirements

The user-provided display name and intended `owner` role are recorded privately in `postgres-data/cutover-review/operators.json`. The sanitized plan references that private display-name field. **Principal UUID remains null; identity verification is pending; permissions granted is false.** No person was invented, no UUID was silently allocated, and no ownership or application role grant was made.

The operator must establish/confirm the principal UUID, approve its binding to the named identity, and approve the intended role and credential/account mapping. A display name or OS account alone is not authenticated ownership. Global operator roles and per-agent ownership are separate checks; even a future global owner does not inherit historical agents. There are no agents or ownership records in this candidate.

Frontend named accounts must map to unique backend tokens and principal identities. Existing authentication rejects ambiguous account/token mappings, browser-supplied identity headers are not trusted, and Rust endpoints retain ownership checks. Creating future agents, establishing per-agent ownership or issuing execution authorizations remains outside this read-only cutover.

## Transition and rollback safeguards implemented for review

- State changes lock the epoch row, compare an expected revision and require a non-nil actor, unique approval and valid plan digest. Duplicate or stale requests fail without a second transition.
- Audit rows are append-only: UPDATE, DELETE and TRUNCATE are rejected by a PostgreSQL trigger. Rollback appends a new audit row and increments revision; it never erases activation evidence.
- Active API startup verifies the expected epoch, active read-only state, revision, matching latest activation approval and audit record, and read-only database default. It skips migrations, recovery and heartbeat/scheduler/wallet setup.
- Wrong/missing approval, paid inference flags, scheduling flags and wallet flags fail closed. The worker continues to reject every identified epoch. The API continues to deny all marked-epoch mutation routes, including owner-authenticated ones covered by the existing epoch tests.
- Configuration install/rollback primitives accept only private UUID-named fixture directories. They preserve snapshots, verify expected hashes and refuse restoration after operator edits or snapshot tampering. They cannot target `web/.env.local` or any real configuration file.

These safeguards protect against normal stale requests and mistakes. A database/OS administrator can bypass SQL privileges or alter code. Actual least-privilege roles and deployment controls still need approval and validation; the fixture is not adversarial-admin protection.

## Exact rollback sequence

1. Stop the reviewed API and frontend processes first. Keep the worker stopped. Do not resume uncertain operations or invoke a model/provider.
2. Verify the exact epoch and current active revision/approval. If configuration hashes or database state differ from the reviewed plan, stop for inspection instead of overwriting changes.
3. Through the separately approved administrative path, append a `rollback` transition bound to a new approval UUID and the current revision. Set metadata to `prepared` and advance revision. Leave the database read-only default on, and retain all business data and audit rows.
4. Restore the original API/frontend private configuration bytes from their preserved snapshots only if both snapshot and current-file hashes match. Do not reset a database, run a clean restore, delete records or downgrade migrations. Restore the prior configuration, not a fictional recovered database.
5. Restart only the previously approved profile. An active profile with the old approval must now fail startup. A prepared preview requires the matching epoch and explicit read-only preview mode. If the old configuration refers to the missing historical database, report its disconnected status honestly.
6. Verify health state and revision, unchanged records, retained activation/rollback audits, disabled workers/paid services, and the archived budget quarantine. Preserve failed-operation evidence and snapshots. Do not automatically retry a divergent rollback.

The UI polls health every 15 seconds when visible and idle. The first browser check used hash navigation before the poll refreshed, so it still saw the old label after successful database/config rollback. The corrected drill explicitly verified convergence through the existing poll, then checked all ten sections again. Execution controls remain disabled in both states; no design change was needed.

## Disposable exercise and actual evidence

The successful drill used `supa_cutover_test_f1acf4044299428eb05806a7a46144e1`, a separate clone retaining the candidate UUID for faithful configuration testing. **Screenshots showing active state belong to that disposable clone, not the real candidate.** Only a non-agent marker row and synthetic transition approvals were introduced. No real users, agents, ownership grants, spending budgets or execution authorizations were created.

The drill activated read-only metadata, rejected a duplicate approval, restarted services, backed up/restored the active database into `supa_cutover_test_restore_b27907ad2a6c48a68a6901cd005bbd24`, then stopped services, rolled metadata back, restored configuration bytes and restarted the prepared preview. The marker survived and both audit entries remained. The stale active profile failed startup. The open browser converged to prepared state and all ten views passed at desktop and mobile sizes before and after rollback (40 view checks).

A first disposable browser attempt stopped at the stale-label assertion after database/config rollback had already succeeded. Its private fixture and logs were preserved; it did not touch the real candidate. The subsequent successful drill is the source of the final evidence.

Validation performed this milestone:

- **127 Rust/PostgreSQL tests passed**, including concurrency, duplicate prevention, immutable audit, ownership/auth boundaries and historical budget quarantine.
- **22 frontend contract tests**, **4 configuration rollback safety tests**, and **4 existing epoch-tool safety tests** passed.
- **15 focused browser regressions** passed across desktop, tablet and mobile: intro/back navigation, responsive scenes, ten destinations, loading/empty/error recovery, and keyboard-accessible mobile navigation. The prior 150-test suite is historical evidence, not counted as a new full-suite run here.
- The real disposable API/database/frontend drill passed 40 phase/view/viewport checks, service restart, five unsafe startup denials, worker rejection, mutation denial, rollback convergence and local backup restoration.
- REA `list_browser_targets`, `inspect_web_page` and `capture_web_screenshot` inspected the rendered active clone and captured desktop/mobile active and mobile rollback screenshots. Playwright performed navigation and sizing. Default REA accessibility text was filtered; screenshots and direct browser assertions support the wording checks.
- Rustfmt, Clippy with warnings denied, frontend syntax checks, ESLint, TypeScript and production builds passed. No visual assets, typography, geometry, portal, hands, wheel or motion system were redesigned.
- Real candidate final audit: still migration 8, `prepared`, read-only, zero application records and unchanged fingerprint. Temporary drill services were stopped.

Backups are private local files on the same disk, not independent disaster recovery or a complete cross-host role/extension/privilege validation. The independent destination remains unset under the earlier instruction; encryption, retention, destination and any copy/upload require separate approval.

Evidence files: `artifacts/cutover-review/change-set.json`, `verification.json`, `drill.json`, `rea-evidence.json`, `active-desktop.png`, `active-mobile.png`, and `rollback-mobile.png`.

## Historical accounting and final approval boundaries

Historical budget `ad91e475-f1da-495a-8247-139643d2d8b1` remains quarantined with its $0.10 ceiling, $0.010299 archived commitment and unresolved $0.004048 hold. No refund, release, import, replacement budget or spending authorization occurred. An epoch transition is not an execution or spending authorization.

Remaining: establish/verify the proposed owner's principal UUID; approve named roles and credentials; approve actual service-role/authentication setup; finalize the real-target transition and operational backup profile; approve migration 9 on the candidate; then explicitly approve exact read-only activation and primary frontend cutover. Writable agent operation remains a separate future boundary.

No real activation, primary repointing, paid requests, provider requests, financial activity, token launches, trading, publishing, uploads, push or deployment occurred.
