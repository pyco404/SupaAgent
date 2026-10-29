# Owner identity provisioning and local cutover readiness

Reviewed on 2026-10-29. **Preparation and isolated validation complete; real owner access and real cutover are not approved or applied.** No frontend component, visual asset, application migration, production configuration or historical record was changed.

## Prepared owner

- Proposed person: KI Daniel (user supplied); intended global role: `owner`.
- Stable proposed principal UUID: **`6fd863e1-745d-4a06-b68e-eea69e854a69`**.
- State: `prepared_ungranted`; identity binding approved: false; authentication enabled: false.
- Private identity record: `postgres-data/owner-provisioning/identity.json`.
- Dormant credentials: `postgres-data/owner-provisioning/credentials.pending.json`.
- Both files are excluded from Git, owned by the local OS user and mode 0600, under mode-0700 directories. They were not copied into `.env.eval`, `web/.env.local`, an API environment or a running service. The previous milestone's unassigned roster and evidence remain unchanged.

The current identity architecture has no principal/user table. Rust authenticates a server-configured bearer token into a UUID and global role. Per-agent ownership is separately persisted in `agent_access` and `access_audit`. Preparing a UUID does not create either a credential grant or an agent ACL. A future owner credential still cannot read an unrelated or unowned agent. No historical identities are adopted on first login.

`scripts/owner-provisioning.py prepare` creates a UUID once and generates three distinct random secrets without printing them: a local browser password (36 random bytes), an operator bearer token (48 random bytes), and a separate public-proxy service token (48 random bytes). Repeated preparation preserves all values. Locking serializes concurrent preparations; files reject unsafe permissions, symlinks, hardlinks and malformed state. Missing identity with surviving credentials fails closed instead of rebinding those credentials to a new UUID. Recovery or rotation requires an explicit review.

## Proposed local authentication

The existing mechanism is sufficient for this loopback-only development review:

1. The browser authenticates to `/operator` with HTTP Basic, using the proposed local username `ki-daniel` and the pending high-entropy password.
2. The frontend server's `SUPA_WEB_OPERATORS` account maps that login to the distinct operator bearer token. No UUID or role is accepted from the browser.
3. The Rust server's `SUPA_OPERATOR_CREDENTIALS` maps that token to the approved UUID and `owner` ceiling. The shared `SUPA_API_TOKEN` remains a different credential for explicitly public projections.
4. Rust still checks per-agent ACLs on private routes. All mutation routes remain denied on a marked read-only epoch, even for an authenticated owner.

These are **proposed mappings**, not installed access. The secret envelope is deliberately JSON, not an automatically loaded dotenv profile. CLI `status`, `plan` and SQL rendering never print passwords/tokens. No credential retrieval endpoint was added.

Browser-provided `x-supa-principal`, `x-supa-role`, `x-user-id`, forwarded-user headers and JSON owner claims cannot replace the server-authenticated principal. The frontend forwards only its allowlisted Authorization/Accept headers on reads. Shared service credentials cannot access owner routes; invalid/pending credentials are rejected.

Local-only limitations: Basic over loopback HTTP is not encrypted transport, passwords/tokens are stored as private plaintext server configuration, and there is no MFA or hardened identity provider. A compromised OS account or administrator can read those files or process environments. Browser Basic credentials can be cached; revocation requires removing the server mappings and restarting the services, then clearing/closing the browser session. This is not a production authentication design. Do not expose these services over a LAN, tunnel or Internet; a future deployment requires its own authentication/TLS review.

## Exact proposed grant and cutover changes

Review `artifacts/owner-readiness/change-set.json`, `activation.review.sql` and `rollback.review.sql`. These files contain no credentials. The SQL files are rendered review transactions, not automatically executed commands. An approval UUID is a correlation identifier, not proof of human consent.

The real target remains:

- Epoch `fd2f52e0-33fc-4c0d-893a-7b08b26ab748`.
- Database `supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748`.
- Accounting namespace `641457b2-5fd8-4b31-b85f-c58839626831`.
- State `prepared`, migration 8, database default read-only on.
- Socket `postgres-data/infrastructure-v1/socket`, port 55440, no PostgreSQL TCP listener.

After separate explicit approval, the concrete sequence would be:

1. Bind the named person to the exact principal UUID and intended role; approve credential installation. Preserve a private approval record with approver, time, scope, reviewed change-set SHA-256 and configuration hashes. Do not treat this report, a display name or a UUID alone as approval.
2. Verify a fresh prepared-epoch backup and an independent local restore into a unique database. Preserve the missing historical database as unrecovered. Agree to the remaining same-disk limitation or separately choose and authorize an independent destination.
3. Approve and apply only additive migration `0009_epoch_cutover_review.sql` to this development candidate with a separately approved administrative identity. **No migration was applied to the real candidate here.** Do not run normal writable API startup as a migration shortcut.
4. Provision an approved SELECT-only database service role, password and one exact database/role-specific local SCRAM rule before peer auth, as specified in the change set. Keep peer administration, socket ownership, no TCP, and read-only defaults. Review inherited/public privileges too: do not assume proposed GRANT statements prove least privilege. These role/authentication changes have not been performed or certified here.
5. In an owner-only private cutover directory, prepare `api.env` and `site.env`. API `SUPA_OPERATOR_CREDENTIALS` must contain exactly the approved `{id, role: "owner", token}` mapping. Site `SUPA_WEB_OPERATORS` must contain `{username: "ki-daniel", password, api_token}` referencing that same operator token. Public `SUPA_API_TOKEN` must use the distinct service token. Do not place the identity UUID/role in browser headers or JavaScript.
6. JSON rosters must be literal single-quoted dotenv values, as supported by `owner-provisioning.dotenv`; reject quote/newline injection. Escaped JSON string literals inside double-quoted dotenv values are not interchangeable with Node `parseEnv` syntax. Preserve exact pre-cutover config bytes and hashes. Use a clean allowlisted environment, not inherited evaluation settings.
7. Use the API variables in the change set: loopback bind, exact database/epoch, `SUPA_EPOCH_MODE=active_read_only`, `SUPA_EPOCH_READONLY_PREVIEW=false`, and the exact approved activation audit UUID. Keep `SUPA_PROVIDER=offline`; `SUPA_ALLOW_OPENAI`, `SUPA_CONTROLLED_INFERENCE`, `SUPA_API_SCHEDULER_ENABLED`, `SUPA_WALLET_ENABLED` and `SUPA_PRIVATE_STAGING` all false. Unset `SUPA_OPERATOR_ID`, legacy single-account fallback, inherited provider/project/budget secrets and wallet custody/poll settings. Keep the worker stopped.
8. Obtain fresh explicit approval for the exact real epoch activation and primary frontend switch. Supply an approved unique `approval_id`, `expected_revision=0` and approved `plan_sha256` to the rendered activation transaction via psql variables. The transaction binds the exact database, epoch, namespace and proposed owner UUID; requires nine successful migrations and read-only default; locks metadata; checks revision/state; appends an audit record; and changes only epoch metadata. Duplicate, stale or mismatched requests fail. It does not create agents, ACLs, spending budgets or execution permissions.
9. Finalize a private active-state operational manifest and backup profile before starting the reviewed services. Retain `postgres-data/development-epoch.json` as the prepared historical manifest; its prepared-only helper intentionally rejects active metadata. The underlying active-state dump/restore procedure passed on disposable clones, but the actual service-role/backup-role profile still needs approval and validation.
10. Start only the explicitly reviewed API and site configurations, with frontend `PORT=3200` and `SUPA_SITE_ENV_FILE=<approved_private_directory>/site.env`. Verify exact health epoch/revision/approval, genuine empty directory, anonymous/operator boundaries, mutation denials and disabled worker/provider paths. The primary frontend was not repointed during this milestone.

There is **no per-agent owner SQL grant** in this cutover because no real agents exist. Future agent creation or ownership adoption requires a separate approved scope, exact agent references and the existing explicit ownership methods; do not bulk claim historical agents or insert fictional records. Global owner authentication is not spending or execution consent.

The helper exposes only `prepare`, `status`, `plan`, `render-activation` and `render-rollback`. It has no grant, install or apply command. Review SQL is supplied for future approved administrative execution, not a way to bypass the approval boundary.

## Verified rollback procedure

1. Stop the API/site and keep workers stopped. If live configuration hashes, epoch or approval differ from the approved manifest, stop for review instead of overwriting changes.
2. Through the separately approved administrator, run the rendered rollback transaction with a **new** approval UUID, the current revision (1 after a first activation) and the reviewed plan digest. It checks the exact target and active state, appends a rollback audit, returns metadata to prepared and increments revision. It never erases the activation entry or changes the read-only default.
3. Restore the exact previous private API/site profiles only after current and backup hashes match. This removes the newly installed global owner mapping from the running configuration. Retain the private prepared identity and approval evidence; do not delete the database or downgrade migrations.
4. Restart only the previously approved profile. A stale active profile must fail startup; prepared preview requires explicit matching epoch/preview flags. If the restored profile points to missing historical state, show disconnected status honestly. No request retry or simulated reconstruction is allowed.
5. Verify data/probe preservation, both transition audits, prepared health, no grants/budgets/authorizations, quarantine unchanged and worker/provider paths disabled. An already open page can take its existing 15-second health poll to converge; the successful drill explicitly waited for that transition.

The disposable configuration rollback helper rejects operator edits, tampered snapshots and real configuration paths. Actual administrative approval, least-privilege roles and operational manifests remain future setup requirements. OS/database administrators can bypass these controls; they are not protection against a malicious administrator.

## Validation and evidence

Current milestone results:

- **128 Rust/PostgreSQL tests passed** using `TEST_DATABASE_URL` pointed only at `supa_infrastructure_test`, including the new forged-owner test, per-agent ownership/role ceilings, epoch mutation denial, transition fencing, and archived-budget quarantine.
- **23 frontend contract tests passed**, including new identity-header non-forwarding and pre-authentication denial checks.
- **9 owner preparation tests passed**, covering stable identity/credentials, concurrency, permissions, symlink/hardlink rejection, missing-identity refusal, corruption, secret collision, safe dotenv syntax and SQL rendering guards.
- **8 existing configuration rollback/epoch safety tests passed**.
- Rustfmt, Clippy with warnings denied, Rust workspace build, frontend JavaScript checks, static frontend build, TypeScript and ESLint passed.

Commands used include:

```sh
python3 scripts/test-owner-provisioning.py
python3 scripts/test-cutover-review.py
python3 scripts/test-development-epoch.py
cargo test --workspace --all-targets --locked --offline -- --include-ignored
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo build --workspace --locked --offline
npm --prefix reference-web run test:unit
npm --prefix reference-web run check
npm --prefix reference-web run build
npm --prefix web run typecheck
npm --prefix web run lint
SUPA_REA_CDP_PORT=19357 node scripts/owner-readiness-smoke.mjs
```

The browser drill uses real local API/PostgreSQL/site processes and a **synthetic operator**, not the pending real owner. All 30 active checks (ten sections, desktop/tablet/mobile) and ten mobile rollback checks passed, with zero browser errors/overflow. It verified service restart, four disabled-feature startup rejections, worker rejection, shared-token/forged-header rejection, real pending credential rejection, owner mutation denial, duplicate/stale/wrong-target transition denial, active backup restoration, exact config rollback and stale-profile startup rejection. No agent, ACL, execution authorization or budget was created in the cloned epoch. Existing Rust authorization tests use synthetic agents in separate test schemas; those are fixtures, never recovered history.

Successful clone: `supa_cutover_test_613533ad567d4979ad44c7e9a9d5a6e0`. Restored active backup: `supa_cutover_test_restore_e46c1b0c2c1348c89beff4c7ac317151`. **Screenshots showing active state belong to this disposable clone retaining the candidate UUID, not the real candidate.** REA listed the page target, inspected actual rendered DOM/accessibility and captured desktop/mobile screenshots; Playwright handled authentication, navigation, resizing and assertions. Prior attach-window network activity is outside REA coverage; secrets/header/storage values were not retained. See `artifacts/owner-readiness/rea-evidence.json` for evidence IDs and screenshot hashes.

Initial disposable attempts exposed a public-plan/private-hash mismatch, dotenv JSON escaping and a stale UI label after successful rollback. Those failures were preserved privately and corrected in the harness; the final drill passed. One diagnostic dump initially had permissive file bits inside a private parent and was tightened to 0600. No source database or primary configuration was overwritten. No changes to the approved frontend were needed.

## Real state and remaining approvals

Final real-candidate fingerprint matches its pre-drill value: prepared, migration 8, read-only, zero agents/ownership/authorizations/live budgets. The pending owner files and primary frontend configuration also match their pre-drill hashes. Temporary services stopped. No production migration, upload, paid request, external mutation, push or deployment occurred.

Historical database remains unrecovered. Budget `ad91e475-f1da-495a-8247-139643d2d8b1` remains quarantined: ceiling **$0.10**, archived committed **$0.010299**, unresolved hold **$0.004048**. The regression test rejects recreation, reservation, settlement/refund and session reuse of that UUID. No budget was imported, released or replaced, and the prepared principal receives no access to historical records.

Independent backups remain unset. Same-disk PostgreSQL dump/restore is verified; it is not independent disaster recovery. Private owner identity/credentials also need a separately approved encrypted backup/recovery decision before depending on this setup. No private data was copied/uploaded to an independent destination.

Explicit remaining boundaries: approve the owner UUID/person binding and credential installation; approve database service-role/authentication setup; approve migration 9 on the real development candidate; finalize the private operational manifest/backup profile; approve exact read-only activation and primary frontend switch. Any writable agents, ownership adoption, workers, paid inference or financial operations require separate future authorization.
