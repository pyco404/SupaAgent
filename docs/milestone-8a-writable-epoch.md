# Milestone 8A — writable-epoch authorization

2026-10-31. Disposable implementation and validation only. This document is not an
activation approval. The primary PostgreSQL cluster was never started or connected.
The approved website, ten clean routes, branding and animations are unchanged.

## Architecture and trust boundary

Migration 12 adds an authorization overlay, not a blanket writable epoch state.
The existing epoch remains `prepared` (or its separately approved read-only state),
and the database default remains read-only. Installing schema, setting environment
flags, knowing an epoch UUID, or possessing an ordinary operator API token does not
authorize a write.

An independently authenticated database custodian must issue a finite immutable
operational approval. The exact semantic plan binds database name, epoch UUID,
accounting namespace, expected epoch revision, verified principal, credential UUID,
one agent UUID, one scope, absolute expiry, maximum committed write transactions,
maximum cycles and simulated seed/expense ceilings. Unknown fields, null bindings,
wrong identities, wrong databases/namespaces, expired credentials, expired consent,
unsupported scopes and overlong consent fail closed. Consent lasts at most one hour.

Custodian membership uses the existing protected authority registry. A separate
protected credential binding must record independent verification, login, principal,
verifier and expiry, and match the existing owner executor registry. Neither a
proposal file nor a display name proves identity. No migration or normal login
creates those bindings. Runtime logins cannot insert or edit the protected tables,
issue approvals, provision database users or grant themselves authority.

`claim_operation` consumes an approval once for one process. It checks the exact
resource bindings, scoped non-superuser login, credential state, local-only server,
read-only connection default and lack of security-table DML/schema-creation access.
The process receives an unlogged random capability; only its hash is durable.
That capability stays in the Rust pool closure, not environment variables, frontend
parameters, LLM context, reports or credentials files. Per-connection configuration
is checked against protected records, not trusted because it is a PostgreSQL GUC.
A guessed or forged GUC value does not establish permission.

Every current public application table has a write/truncate guard, except migration
and epoch-transition metadata, which retain their existing separate protection.
The guards enforce the plan and resource on direct SQL as well as Rust/API writes.
They check immutable resource identities, append-only execution/accounting evidence,
offline capabilities, genuine owner binding, finite scheduling consent, actual cycle
permit and simulated cost ceilings. A deferred guard rechecks authorization at
commit and verifies agent balances equal their simulated ledger. Root/database-owner
compromise, trigger removal and malicious changes to trusted code are outside this
boundary; operational services must never possess those privileges.

## Separate operation permissions

| Operation | Independent boundary |
| --- | --- |
| Primary migration | Root-controlled, independently signed migration-only executable and exact checksums; it issues no operational consent |
| Credential/owner/custodian provisioning | Separately approved administrative identity verification, credential installation and protected binding/grants; operational approval functions cannot do this |
| Agent creation | `agent_creation`: one server-selected agent UUID, inactive offline capabilities, goals and its explicitly verified initial owner |
| Initial ownership adoption | `ownership_grant`: insert-only initial owner/audit for the exact verified principal and agent; no delegation or overwrite |
| Offline activation/control | `offline_control`: separate finite schedule consent, bounded simulated seed, pause/emergency-stop/recovery; cannot execute a cycle |
| Offline execution | `offline_execution`: separately admitted process, existing finite owner consent, exact agent, cycle/transaction/cost limits; cannot create or renew consent |
| Paid inference | Unsupported: paid tables/routes/providers are denied in this mode; no operational approval can authorize them |

The server, not a submitted agent ID, chooses the creation UUID from trusted
admission. Existing JSON schema, owner/operator/viewer access checks and per-tool
capability checks remain in place. Controlled API startup requires one matching
named owner credential, separate public-service token and configured credential
deadline. Browser identity/role/expiry claims cannot choose that principal.

## Expiration, revocation and restart

Approval, credential, admission and committed-operation records are durable and
immutable. Custodian revocation appends a separate durable event; it does not edit
history or refund an allowance. Credential revocation affects every associated
permit. Revocation serializes with admitted transactions: an already executing
transaction must finish or roll back before revocation returns; every subsequent
operation fails. This is a database linearization boundary, not cancellation of CPU
instructions already executing outside a database transaction. Long administrative
SQL transactions can delay revocation and must be avoided.

Rust acquires the permit fence before agent/worker row locks. Heartbeats, cycle
writes, scheduling and recovery use that same order to avoid lock inversions.
Every mutation and deferred commit check uses the current database clock and
revocation records, with read-committed isolation required. Cycle leases are clipped
to both operational expiry and the finite agent authorization expiry. A different
admission cannot complete an old cycle. Health reports do not infer execution from
consent: revoked/expired cycle permits make the execution lease invalid even before
the last heartbeat ages out.

An API/worker crash loses its in-memory capability; restarting with the consumed
approval is rejected. A new independently reviewed approval is required. A PostgreSQL
restart fences **all** old admissions through the recorded postmaster start time.
Pool reconnection cannot reauthorize them. A restored database has a different
name or postmaster incarnation, so copied claims cannot resume. Ambiguous admission
or execution is not automatically retried. Durable audit/accounting survives; new
scoped control consent can fence/recover interrupted offline work, never silently
complete or replay it.

Trusted host/database time must remain correct. Restore into a separate database
or a new server incarnation, never over a live original database: a same-name,
same-incarnation overwrite is not a supported recovery procedure. Immutable identity
bindings are administrative trust records, not an automatic credential-renewal flow;
expiry/revocation requires separately reviewed reprovisioning/rebinding. No online
custodian/owner identity verification service was introduced.

The mechanism does not kill a process immediately on revocation. It prevents new
protected writes and authenticates private API requests against the current permit;
a worker may remain alive reporting failed/fenced ticks. Offline computation already
in memory can finish, but its results cannot be committed. External mutations and
paid inference are outside every permitted scope.

## Integration and configuration

Implemented files:

- `migrations/0012_operational_epoch_permits.sql`: protected bindings, immutable
  approvals/claims/revocations/transaction audit, scoped SQL guards and commit fencing.
- `crates/persistence/src/operational.rs`: consume-once admission and private pool
  capability; reusable authorization-first transaction entry point.
- Persistence `lib.rs`, `access.rs`, `scheduling.rs`, `intelligence.rs`, `health.rs`:
  transaction lock order and genuine revoked-permit health.
- API `lib.rs` and `main.rs`: server-selected identity, fresh request authorization,
  narrow route/scope gates and controlled offline startup without migrations/recovery.
- Runtime `lib.rs` and `main.rs`: force deterministic offline reasoning and require
  independent database admission before any epoch worker heartbeat or execution.
- Migration admin `lib.rs`, `tests.rs`, `bin/supa-migration-twelve-admin.rs`: separate
  signed schema 8→12 profile, retaining old 8→10 and 8→11 scopes unchanged.
- `crates/api/tests/writable_epoch.rs`: real restored-database security, concurrency,
  expiry and actual API/worker process tests.
- `scripts/validate-writable-epoch.py`: full Rust/database rehearsal and restoration,
  cross-database denial, PostgreSQL crash fencing and primary protected-file checks.

Future controlled services require `SUPA_EPOCH_MODE=controlled_offline`,
`SUPA_EXPECTED_EPOCH_ID`, `SUPA_EXPECTED_ACCOUNTING_NAMESPACE` and
`SUPA_OPERATION_APPROVAL_ID`, plus their separately installed scoped database
credentials. These are selectors only: no database approval means startup fails.
API scheduling remains disabled. The separately admitted worker requires an explicit
enable flag **and** an unused `offline_execution` approval; a flag alone is insufficient.
No dotenv or real credentials are installed by the validator.

The new migration-only command has the same exact private-file interface:

```sh
env -i PATH=/usr/bin:/bin HOME=/home/pikoo \
  /home/pikoo/Desktop/supa-agent/target/debug/supa-migration-twelve-admin \
  --approval-file /ABSOLUTE/PRIVATE/approved-plan.json \
  --signature-file /ABSOLUTE/PRIVATE/approved-plan.sig \
  --connection-file /ABSOLUTE/PRIVATE/migration-connection.json
```

This is a future template, not an installed approval or authorization to run it.
Its distinct scope is `migrate_development_epoch_through_twelve_only`, expected
schema 8, target 12, inventory 1–12, exact reviewed executable/backup/database/epoch
bindings and independently verified root-controlled Ed25519 authority. A signed
8→11 plan cannot authorize 12. It is not an 11→12 upgrade command. No trust anchor,
signature or real approval was created. Primary remains at its last verified schema 8.

## Validation and evidence

Reproduce only with disposable local socket/process permissions:

```sh
python3 scripts/validate-writable-epoch.py
```

Completed full evidence: `artifacts/milestone-8a/ec16da409fc2/validation.json`,
with the complete test-name inventory under `rust-tests/tests.json` and the private
`operational.dump`. All generated records, dumps, synthetic identity data and logs
remain private and ignored by Git.

| Validation | Actual result |
| --- | --- |
| Rust workspace, all targets, opt-in PostgreSQL tests | 150 passed, zero failed/ignored |
| Writable-epoch security/process tests | All 6 passed in the full suite; all 6 passed again after strengthening fixtures to independently generated custodian/executor passwords; final focused log `artifacts/milestone-8a/a98044df96da/retest.log` |
| Migration profiles | Independent signed 8→10, 8→11 and 8→12 profiles passed against approved-backup clones; 12 installed zero operational bindings/approvals/claims; replay rejected |
| Scope and identity | Missing/unknown approvals, wrong epoch/namespace/database/principal/credential, unsupported paid scope and expired consent rejected; untrusted direct SQL/metadata forgery/truncation denied |
| Concurrency | One admission succeeds for concurrent replay; a one-transaction grant permits one concurrent creation; subsequent writes and another agent UUID denied |
| Expiry/accounting | A transaction begun before expiry failed its deferred commit afterward, leaving no agent; forged balance update failed commit and left the ledger/balance unchanged |
| Revocation/recovery | Active cycle writes/finish/heartbeat denied after revocation; health showed no running execution; replacement admission could not finish old cycle; separately admitted control fenced/recovered it as interrupted |
| Credentials | Credential revocation denied existing and unused admissions; ordinary API credential expiry/role/spoofing regressions remained passing |
| Actual processes | API was killed and could not restart with consumed admission; a real disposable worker completed exactly 2 offline cycles with 2,200 simulated microUSD expense, retained memory/history, and could not replay admission after being killed |
| Backup/restore | Row state and effective ACL/schema/function/trigger catalog matched in a separate restored database; copied admissions failed cross-database checks; read-only database default explicitly reapplied |
| PostgreSQL crash | Immediate shutdown/restart preserved committed rows/audit and forward-only sequences; every previous claim became invalid |
| Frontend adapters/process tests | 28 adapter tests + 3 process cleanup tests passed |
| Python safety tests | 31 passed across owner, preflight, cutover, persistence and onboarding guards |
| Quality | Workspace check, formatting, all-target Clippy with warnings denied, frontend ESLint and TypeScript checks passed |
| Preservation | All 26 protected hashes unchanged; primary never started/connected; all disposable clusters stopped; zero real credentials/grants, paid requests or financial transactions |

The earlier restoration probe correctly found no live admissions after the strict
harness had restarted PostgreSQL. The final validator now explicitly requires that
fencing, issues a **new synthetic-only** probe approval through the fixture custodian,
then verifies backup isolation and another crash. It does not revive or extend old
consent to make the test pass. Preliminary failed runs are not counted as passing
end-to-end evidence.

No frontend source changed. Lint/types and adapter regressions were rerun; a production
frontend build, broad visual/browser suite and REA session were not rerun for this
backend milestone. The actual subprocess HTTP/worker tests above use disposable
state, not the primary website. Existing prepared-epoch UI still deliberately labels
that state read-only and hides execution controls. Before a **UI-driven** operating
cutover, add an authenticated operator-status integration for temporary scopes and
wire the existing controls without changing the approved appearance. The current
backend demonstration does not claim that frontend integration is complete.

The existing full-history audit passed for all 1,179 prior main commits, and the
local targeted secret scanner passed across 1,180 reachable commits including the
archive. New files receive one commit each, staged individually with hook scanning;
a final full audit/scan follows. The scanner detects known local secrets and common
formats, not every possible semantic disclosure. No artifacts are tracked or pushed.
Rust incremental compilation caches alone were cleared to relieve disk pressure;
source, databases, backups and reviewed accounting evidence were retained.
Existing historical evidence is retained; incomplete development runs are not
counted as passing validation. No screenshot or frontend redesign is needed for
these backend authorization controls.

## Accounting and approval boundaries

The historical database remains unrecovered. Quarantined budget
`ad91e475-f1da-495a-8247-139643d2d8b1`, $0.10 ceiling and unresolved $0.004048 hold
are unchanged. No live budget is recreated, released, refunded or settled. Offline
fixture balances are newly generated simulated records, never imported history.
Independent backups remain unset; same-disk verification does not protect disk loss.

Separate future decisions, in order:

1. Approve primary startup/preflight/new private backup and stop on baseline drift.
   This milestone authorized none of those operations.
2. Independently verify the custodian and approve only migration trust/procedure;
   review/sign the exact 8→12 plan and separately approve primary migration. Installing
   migration 12 creates no users, grants, claims, operational consent or activation.
3. Independently verify the owner identity and approve scoped credential installation,
   deadlines, owner executor binding and custodian issuance authority as distinct
   administrative steps. The real custodian proposal remains unverified/ungranted.
4. Approve one exact fresh agent/goal/owner plan or explicit initial adoption; issue
   its finite resource-bound creation/adoption consent. No historical agent is inferred.
5. Separately approve simulated seed and finite offline control consent; separately
   approve the execution process/host, goals, interval, cycles, expiry and expense ceiling.
   Starting another process after a crash needs a new admission, not replay.
6. Keep paid GPT, real financial activity, Devnet funding/signing, GitHub push and
   Railway deployment under their separate future approvals. Devnet recipient stays
   unassigned. Mainnet, trading, launches and X publishing remain disabled.

Operational rollout still requires those real identity, migration, role/HBA,
backup and activation decisions. This implementation is not a claim that the
primary has been activated or is currently writable.
