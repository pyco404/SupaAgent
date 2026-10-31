# Milestone 8 — controlled local activation and owner onboarding preparation

Prepared on 2026-10-31 (Africa/Lagos). This is a disposable rehearsal and unsigned
operating proposal, not approval, primary activation, owner verification, restored
history, or permission to launch a real worker.

## Baseline and evidence boundary

The primary PostgreSQL cluster remained stopped. Inspection used its offline
control/configuration files, protected-file hashes, dormant identity records and
the approved Stage 1 backup. No fresh SQL query against the primary is claimed.
The previous Milestone 7A protected hashes match; the current rehearsal also
compares the broader Stage 1/Stage 2 protected inventory before and after testing.

| Property | Reviewed value |
| --- | --- |
| Primary database | `supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748` |
| Epoch | `fd2f52e0-33fc-4c0d-893a-7b08b26ab748` |
| Accounting namespace | `641457b2-5fd8-4b31-b85f-c58839626831` |
| Last primary SQL verification | Migration 8; prepared; no agents, grants, live budgets or execution authorizations |
| Primary cluster | `postgres-data/infrastructure-v1/data`; system identifier `7702158196701990299`; stopped |
| PostgreSQL access profile | Unix socket on port 55440; no TCP listener; peer administration; database default read-only |
| Proposed owner | KI Daniel, `6fd863e1-745d-4a06-b68e-eea69e854a69`; prepared/ungranted; identity binding unapproved |
| Proposed custodian | KI Daniel, `e8c6ade3-917f-4fe4-a092-f8a58eccb487`; proposed/unverified; independent control not established |
| Real credentials | Existing dormant owner envelope, mode 0600; not installed; no custodian credentials prepared or installed |
| Migration trust anchor | `/etc/supa-agent/migration-authority.json` absent |
| Historical database | Unrecovered |
| Historical budget quarantine | `ad91e475-f1da-495a-8247-139643d2d8b1`; ceiling 100,000 microUSD ($0.10); unresolved hold 4,048 microUSD ($0.004048) |
| Independent backups | Unset; no upload or external copy authorized |
| Devnet recipient | Unassigned |

The approved source is
`postgres-data/preflight/1d0e9efe-e66e-4e4f-afc4-478eee4f0833/database.dump`,
SHA-256 `600d2bb19b52b4f17d4278459ba722ea5f1135df703b8efc1b0b3db0c8360671`.
Newer disposable dumps are test outputs, not a replacement approved primary
backup. No fresh primary backup was attempted without startup approval.

The unresolved historical hold is archived evidence, not a restored live budget
row in this empty development epoch. Preservation means its evidence stays
unchanged and its UUID remains barred from recreation, reservation or refund.
It is never imported into a new agent's simulated ledger.

## Implementation and reproduction

- `config/first-agent-plan.json`: unsigned first-agent intent, with no agent UUID,
  ownership grant, execution consent, live budget or recipient.
- `scripts/validate-local-onboarding.py`: offline inspection and fresh-cluster
  rehearsal. It has no arbitrary database/data-directory target, real activation,
  credential-installation or approval-issuance switch.
- `scripts/onboarding-smoke.mjs`: real Rust API, local frontend and browser
  rehearsal against the exact disposable socket/database. All authenticated
  principals and credentials are synthetic and process-local. It never reads the
  pending real owner secrets or starts a worker.
- `scripts/test-local-onboarding.py`: target/approval/proposal, fingerprint-default
  and sequence-recovery guard regressions.
- `scripts/test-onboarding-process.mjs`: real subprocess shutdown regressions.
- `scripts/persistent-postgres.py`: strict fingerprints remain the default; the
  rehearsal can separately compare stable row/schema state while validating
  forward-only sequence counters after crash recovery.

Read-only inspection:

```sh
python3 scripts/validate-local-onboarding.py inspect
python3 scripts/test-local-onboarding.py
```

Disposable reproduction, requiring local socket/browser subprocess access:

```sh
python3 scripts/validate-local-onboarding.py exercise-disposable
```

For REA inspection, add `--rea-cdp-port 9236`. The browser remains open only after
all onboarding assertions pass, while the fixture agent is suspended. Read the
private `browser-ready.json` for its local origin and PID; send SIGTERM to that
verified Node rehearsal process after inspection. This releases its wait, closes
its API/frontend/browser, and lets the validator finish dump/restore and shutdown.
No URL, command argument, screenshot or report contains a credential value.

The validator builds existing local Rust code offline, reuses the strict signed
migration-admin test harness, then creates a different socket-only PostgreSQL 16
cluster beneath `postgres-data/onboarding-validation/<unique-run>/` on port 55452.
The restored epoch clone and the empty operational fixture are separate databases.
The prepared epoch row is retained in the clone and is never deleted to enable
execution. The operational fixture has no application epoch row and is explicitly
identified as synthetic test state, never the operating development epoch.

## Migration and recovery readiness

The approved migration-8 backup is restored into a new disposable database. The
existing disposable-only review entry point applies 9–11. Original source columns
and rows, SQLx SHA-384 checksums, prepared revision 0, empty approvals/grants/budgets,
and empty economic state are checked. A SELECT-only fixture role cannot update
epoch metadata, insert agents, create schema objects or execute approval functions.

| Migration | Effect | Primary execution boundary |
| --- | --- | --- |
| 9 | Epoch revision/approval fields, read-only active state and transition history | Existing signed 8→10 migrator, with fresh independent approval |
| 10 | Trusted approval authority/executor bindings, immutable approvals/consumption/audit protections | Same exact reviewed 8→10 transaction; no real authority registered |
| 11 | Separate bounded service/job/receipt/economic accounting tables and protections | **Not covered by the current primary migrator.** Separate reviewed migration-only entry point/plan and explicit approval required |

The strict migration harness tests forged/tampered/replayed approvals, wrong
principals/targets/scopes, stale revisions, checksums and permissions, atomic
failure rollback, interrupted execution, concurrency and restart. Its production
entry point stays pinned to migrations 9–10 even when migration 11 is embedded.
No signed real-target approval or trust file is generated.

Migrated-clone and operational-fixture custom dumps are separately restored into
new databases. Rows, constraints, indexes, sequences, ownership, ACLs, functions
and triggers are compared. Database-level read-only defaults are explicitly
reapplied to the restored epoch clone: a database dump must not be assumed to
reconstruct every role, authentication or database setting. Password-free global
role definitions are retained privately and are never automatically installed.
An immediate shutdown/restart exercises crash recovery on this disposable cluster.
Effective ACLs include grantors and grant options: PostgreSQL can restore explicit
default ACLs as equivalent `NULL` defaults. The rehearsal explicitly introduces
and removes UPDATE and approval-function privilege escalations in the restored
clone and confirms its comparator detects both. No permission mismatch is ignored.

Exact sequence counters are checked for dump/restore. After a crash, PostgreSQL
may advance cached/WAL-protected sequence values without adding rows. Restart
checks preserve all rows and sequence definitions, reject counter rewind, and
require owned sequences to stay above committed IDs. No sequence is reset to make
fingerprints match; the shared helper's original strict default is unchanged.

**Primary migration assessment:** disposable compatibility can establish that the
reviewed SQL preserves this backup. It cannot authorize real execution, prove the
current primary's SQL state while stopped, or make migration 11 runnable through
a tool intentionally pinned to 10. Primary migration remains blocked on a fresh
approved preflight, independently verified signer, exact signed artifacts and a
separate reviewed plan for 11.

Future primary recovery procedure:

1. With separate startup/preflight approval, keep all writers and workers stopped;
   verify the exact cluster, database, epoch, namespace, schema and grants.
2. Create a fresh mode-0600 custom dump, password-free role archive and private
   manifest recording hashes, SQLx checksums, cluster identity, database settings,
   HBA/configuration fingerprints, epoch metadata and quarantine evidence. Retain
   the approved Stage 1 dump without overwriting it.
3. Restore to a newly named isolated database; verify logical/catalog fingerprints,
   private-file modes and the intended read-only profile before any migration.
4. Obtain approval tied to the exact reviewed migration artifact and target.
5. On failure before the signed 9–10 transaction commits, verify rollback to 8.
   On an ambiguous COMMIT result, inspect migration receipts/checksums with
   separately authorized diagnostics; never retry automatically.
6. After a successful commit, preserve migrations and immutable audits. Reverting
   an executable does not undo DDL. Use reviewed forward repair or a new isolated
   restore; never overwrite primary, delete audit rows or edit applied checksums.

Same-disk backups protect against some logical failures but not disk loss, host
compromise or a destructive administrator. No independent destination was set.

## Owner onboarding and revocation

The person-to-principal binding still needs independent verification and explicit
approval. A UUID, display name, pending secret file or frontend-supplied role is
not identity proof. Owner and custodian proposals remain distinct, unverified
where previously unverified, and ungranted. The process-level onboarding actors use distinct synthetic UUIDs, never the
prepared owner/custodian credentials or grants.

After approval, the existing provisioning architecture would install a private
frontend account mapped to an operator token, and a separate Rust credential
roster mapping that token to the verified principal and role ceiling. The public
service token remains distinct. Per-agent ownership is established by Rust's
`create_owned` transaction, not a browser `owner_id` field. Unknown ownership denies
private access; legacy shared-owner fallback remains disabled in this rehearsal.
The prepared credential files remain excluded from Git, single-link, owner-only
files under owner-private directories. No values are rendered in this report.

| Role | Private reads | Pause/resume/emergency stop | Start/renew | Grant access |
| --- | --- | --- | --- | --- |
| Viewer | Only granted agents | No | No | No |
| Operator | Only granted agents | Within existing limits | No | No |
| Owner | Only owned agents | Yes | Fresh bounded consent | Approved viewer/operator grants; no self-escalation |

Credential revocation uses server configuration, not a browser claim or a hot
revocation endpoint. The safe sequence is pause/fence the affected agent, keep
workers stopped, remove/rotate both frontend and Rust credential mappings, restart
both services, then verify the old browser password and bearer token fail.
Rotation for the same verified principal preserves ownership, memory, history and
accounting. A removed credential does not erase its historical ACL/audit records.
Removing a login alone is **not** revocation of an existing execution authorization;
pausing/stopping execution is a separate required step.

Basic credentials may be cached in a browser. Clearing the browser is useful but
not the security boundary: backend rejection of the removed token is required.
This remains loopback-only development authentication, without MFA or production
session management. OS/database administrators can read private configuration or
override database controls. Independent custody and production identity hardening
remain future work.

## First-agent readiness and the writable-epoch blocker

The unsigned proposal requests one fresh `Supa` identity, initially `created`,
with scheduler disabled and only `read_memory`, `remember`, `evaluate_goal`.
The two goals concern verified outcomes and evidence-based progress. It assigns
no agent UUID and creates no primary rows, simulated capital, schedule or budget.

A later separately approved offline authorization would select the approved goal
UUIDs and cap execution at three cycles, 60-second intervals, 180 seconds and
3,300 simulated microUSD (1,100 per cycle). This is not a paid-provider budget.
The existing scheduling control applies its fixed offline execution configuration:
one provider invocation, three steps, five-second configured timeout, no retry,
and existing per-tool checks. A separate approved simulated seed would be needed
for execution; no historical budget can supply it. Pause/resume retain consumed
limits; emergency stop suspends and fences work. Expiry requires fresh renewal.

**A real first-agent activation is not currently executable through the prepared
epoch cutover.** Both `prepared` and `active_read_only` epochs deny mutations, and
the API startup contract accepts only preview/read-only modes for a marked epoch.
Read-only activation therefore cannot create an agent, add goals or run a cycle.

A future writable development-epoch mode needs its own reviewed implementation,
trusted transition/approval scope, startup checks, permissions, migration/recovery
rules and tests before approval could enable it. Do not remove the epoch row,
point to an unmarked database, disable the write gate or use an administrator to
work around it. The disposable operational demonstration proves existing runtime
functionality; it does not prove that a writable primary cutover exists.

## Validation results and local evidence

Completed end-to-end evidence: `artifacts/milestone-8/90e1d7d9afc2/validation.json`.
The final run passed with all its disposable clusters stopped, no primary
connection/startup and all 26 protected-file hashes unchanged. It preserved the
original columns/rows of 28 restored source tables, validated all eleven SQLx
checksums, and created zero operational records in the prepared-epoch clone.

| Validation | Actual result |
| --- | --- |
| Rust workspace/all targets, including opt-in database tests | 139 passed; zero failed or ignored |
| Strict migration-admin rehearsal | Signed scope/identity/forgery/replay, checksum/ACL, interruption, atomic rollback, concurrency and restart checks passed; real-target scope remains 9–10 |
| Python preparation/security tests | 31 passed: owner 9, cutover 4, preflight 5, persistence 6, new onboarding guards 7 |
| Real child-process cleanup regressions | 3 passed, including already-signaled children and forced cleanup of an unresponsive owned fixture |
| Frontend adapter unit tests | 28 passed |
| Devnet runner boundary tests | 3 passed; unset-recipient preparation also returned unassigned without network/database/wallet changes |
| Rust formatting and warnings-as-errors Clippy | Passed |
| Frontend JavaScript/static build, ESLint, TypeScript | Passed |
| Real process onboarding | Synthetic owner provisioned an inactive identity and two goals; viewer/operator/outsider boundaries and forged claims rejected |
| Offline execution | Two manual deterministic cycles; persistent memory/history; 2,200 simulated microUSD expense; zero live requests or real spending |
| Revocation/restart | Old API token and browser password rejected after both mappings were replaced; same-principal ownership, memory, cycles and balances persisted while paused |
| Expiry/emergency stop | Expired authorization rejected without replenishing allowance; emergency stop left fixture suspended and fenced resume |
| Restored backups | Both migrated-epoch and operational dumps restored into separate new databases, with matching records/schema/effective privileges; crash/restart and sequence bounds passed |
| Browser | All ten sections rendered without mutation requests or horizontal overflow; actual API/frontend checks at 1440×1000 and 390×844 |

The process rehearsal compresses the proposed interval/duration to five seconds
and sixty seconds and injects a past deadline only into its disposable schedule
to exercise expiry after two cycles. It is not a natural 180-second continuous
run. It launches zero worker processes and leaves its agent suspended. The
complete Rust suite separately launches its own isolated offline worker fixtures
to verify leases, health and restart behavior. No primary/persistent worker runs.

REA inspected the genuine isolated Rust/API/frontend fixture during the successful
onboarding phase of run `e979a1ca5e32`. Desktop evidence
`ev_450c02c6e7a0a0771fd87fc5de128bd7dd157befeb286239dc7ae5a15af85fc0`
and mobile evidence
`ev_6d664112a22621b311d9e2a7607c37c0ac51c13cb856bb4f73e9f0c468773ad2`
record completed navigation/inspection steps and visible suspended state. Attached
event coverage is incomplete; absence of browser errors/mutations is established
by the owned browser listeners, not inferred from missing REA events. Viewports
are Chromium emulation, not physical-device certification. The final corrected
end-to-end run has its own desktop/mobile captures and passing restoration report.
Frontend source, Rust runtime, migrations, branding and all ten sections were
unchanged by this milestone.

Harness development caught an invalid disposable name, repeated shutdown of a
SIGTERM-exited child, explicit-versus-default ACL representation, safe post-crash
sequence gaps, and a lifecycle query that needed the existing `agents.data` JSON
field. Those checks were corrected without weakening primary targeting or
permissions, and the complete rehearsal was rerun successfully. Incomplete or
failed attempts remain retained and are not counted as passing end-to-end runs.

Local dumps are mode 0600 under private directories. No generated screenshot,
credential, role archive or sensitive log belongs in Git. Test data/evidence is
retained locally; approximately 2.8 GB free disk remained after validation, so
additional large image/build/database work needs capacity planning. Independent
backup storage remains unset. Same-disk restoration does not protect from disk loss.

## Separate future approvals

1. **Primary migration:** separately approve PostgreSQL startup/fresh preflight,
   independently verify custodian identity/key control, approve trust-file setup,
   review the exact signed 9–10 executable/plan and obtain its migration-only
   signature. Review and authorize 11 separately through an appropriate guarded
   entry point. This grants no owner access or activation.
2. **Owner verification/credentials:** bind KI Daniel to the prepared owner UUID,
   approve the scoped credential mappings and least-privilege database login/HBA
   changes. Custodian issuance authority is a distinct grant. No principal gains
   access merely by claiming the name/UUID.
3. **Epoch/first agent:** a read-only cutover needs its own immutable approval and
   frontend-switch consent. Writable mode requires the missing reviewed feature
   first. Then separately approve fresh agent/goal creation, simulated seed and
   finite offline execution/host. The current proposal authorizes none of these.
4. **Devnet receipt:** leave recipient unassigned until an externally controlled
   public address is selected and independently verified. Separately approve
   isolated setup, external Devnet test funding/signing and receipt reconciliation.
   No outgoing signer/key is added to Supa.
5. **One live GPT request:** separately verify project credentials, approved model,
   capabilities, rates and billing limits; establish an independent bounded live
   allowance; obtain fresh request consent immediately before transport. Historical
   quarantined allowance and simulated balances cannot fund it. Continuous live
   scheduling remains off.
6. **Railway staging:** obtain successful container/runtime/vulnerability evidence
   and review deployment secrets, network/volume roles and backups. Pushing,
   resource creation, deployment and worker enablement each retain their applicable
   approval boundaries. Never copy local evaluation secrets or fixture treasury.
