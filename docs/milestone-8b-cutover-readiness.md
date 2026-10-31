# Milestone 8B — owner verification and primary cutover readiness

This milestone prepares and tests disposable state only. **It does not authorize
primary startup, migration, credential installation, ownership grants or execution.**

## Verified baseline

Offline inspection uses `python3 scripts/validate-local-onboarding.py inspect`.
The primary is stopped; its last SQL-verified baseline remains schema 8, epoch
`fd2f52e0-33fc-4c0d-893a-7b08b26ab748`, state `prepared`, revision 0, database
`supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748`, accounting namespace
`641457b2-5fd8-4b31-b85f-c58839626831`. This milestone verifies protected file
hashes, not a fresh SQL connection to the stopped primary.

The approved Stage 1 dump is
`postgres-data/preflight/1d0e9efe-e66e-4e4f-afc4-478eee4f0833/database.dump`, SHA-256
`600d2bb19b52b4f17d4278459ba722ea5f1135df703b8efc1b0b3db0c8360671`.
Historical database recovery remains **unrecovered**. Budget
`ad91e475-f1da-495a-8247-139643d2d8b1`, its 100000 microUSD ceiling ($0.10), and
4048 microUSD unresolved hold ($0.004048) remain quarantined archival evidence.
No refund, settlement, replacement historical agent or imported budget is prepared.

The proposed KI Daniel owner is `6fd863e1-745d-4a06-b68e-eea69e854a69`.
The distinct proposed custodian is `e8c6ade3-917f-4fe4-a092-f8a58eccb487`.
The custodian remains `proposed_unverified`, with independent control unverified,
no prepared/installed credentials and no issuance authority. Existing dormant owner
material remains private and uninstalled; it is not evidence of identity.

## Owner verification and provisioning

A name, email, proposal, self-submitted key or successful login cannot establish
ownership. Before any grant, a human verifier must independently establish the
owner-to-principal binding and the custodian's separate control through an agreed
out-of-band process. Document the verification method and reference without storing
identity documents, private keys or recovery secrets in Git or application logs.
Separate principal UUIDs and keypairs alone do not prove independent custody.

`scripts/verify-owner-evidence.py` implements **read-only evidence review**, not an
identity provider or approval issuer. Its production entry point accepts no alternate
trust path. It requires an independently established root-owned, non-symlink,
non-group/world-writable `/etc/supa-agent/onboarding-authority.json`. This record is
**not installed**. Root-controlled ancestors and public-key fingerprints prevent a
requester from substituting a self-trusted proposal. The verifier runs unprivileged.
A public trust file may be root-owned 0644 inside a root-owned 0755 directory; it
contains no secret keys. Granting this trust requires separate operator approval.

Trust-record shape, for future independent review:

```json
{
  "version": 1,
  "epoch_id": "fd2f52e0-33fc-4c0d-893a-7b08b26ab748",
  "expires_at": 0,
  "owner": {
    "principal_id": "6fd863e1-745d-4a06-b68e-eea69e854a69",
    "public_key_sha256": "REVIEWED_SHA256_OF_ED25519_SPKI_DER",
    "identity_verification_reference": "INDEPENDENT_CHECK_REFERENCE",
    "independent_control_verified": false
  },
  "custodian": {
    "principal_id": "e8c6ade3-917f-4fe4-a092-f8a58eccb487",
    "public_key_sha256": "SEPARATELY_REVIEWED_SPKI_SHA256",
    "identity_verification_reference": "SEPARATE_CONTROL_CHECK_REFERENCE",
    "independent_control_verified": false
  }
}
```

This deliberately invalid template cannot pass verification. Never set verified
fields until the independent checks actually occur. An authenticated root
administrator remains part of the trusted computing base.

The owner and custodian separately sign identical canonical JSON (UTF-8, sorted
keys, no whitespace) using externally controlled Ed25519 keys. Exact challenge
fields are `version:1`, `purpose:"owner-provisioning-review-only"`, `epoch_id`,
`owner_principal`, `custodian_principal`, a fresh UUID `nonce`, and integer Unix
`issued_at` / `expires_at` (at most 15 minutes). Both public keys must match their
independent anchors and differ. Reject altered fields, wrong target/purpose,
expired/future challenges, absent verification and invalid signatures.

Future review command, which itself grants nothing:

```sh
python3 scripts/verify-owner-evidence.py \
  --challenge /PRIVATE/challenge.json \
  --owner-public-key /PRIVATE/owner.pub --owner-signature /PRIVATE/owner.sig \
  --custodian-public-key /PRIVATE/custodian.pub --custodian-signature /PRIVATE/custodian.sig
```

A valid result is evidence for a **new explicit provisioning decision** only.
Replaying it never grants access. The provisioning administrator must consume the
nonce once in the immutable administrative audit, bind the exact evidence digest,
principal, epoch, SQL login, credential UUID, role, expiry and approved scope, and
reject duplicate evidence before installing anything. This read-only tool cannot
consume nonces or replace the existing trusted SQL approval/admission ledger.

Following separate approval, provision distinct custodian and executor logins
with NOINHERIT, NOSUPERUSER, NOCREATEDB, NOCREATEROLE, finite credential expiry and
local-only socket access. Use the existing server-side principal registry and
immutable approval functions. Never infer verification from database migration or
first login. Do not enable shared-owner fallback. Keep operator API credentials,
public projection token, SQL executor credentials and custodian credentials
separate. Use independently controlled password entry or protected file descriptors,
not command-line secrets, shell history, `.env.eval` or copied evaluation tokens.
Private runtime directories require 0700 and credential files 0600; redact logs.
Custodian issuance credentials must never be available to the API, worker or LLM.

Revocation has two layers: revoke the operational credential/permit in the trusted
SQL ledger, then remove/rotate its API credential and set/expire its server session
deadline. An expired API session fails authentication; an operationally revoked
permit cannot read private agent records or write. The status-only projection may
still report revocation until the API session expires. Neither layer renews itself.

## Operator-status integration

`GET /operator-status` is authenticated in Rust. It takes no caller identity or role
from frontend fields. `GET /api/operator-status` in the terminal adapter and
`GET /api/console/operator-status` in Next.js require the existing operator
credential mapping. The public service token cannot read this endpoint.
Responses use `Cache-Control: no-store` and strict whitelisted frontend contracts.
No password, API token, admission secret or capability hash is projected.

Status includes the actual epoch/database/schema, database read-only default,
credential binding/revocation/expiry, admitted permit scope/target/expiry/ceilings,
worker heartbeat and lease, and scope eligibility. Missing metadata access is
reported as unavailable, with eligibility false. No approval is invented.
Creation additionally requires an absent target; offline controls require the
bound owned agent and saved goals. These flags describe scoped permission, not
fresh agent consent, available quota or a guarantee the next command will succeed.
Every action still passes the authoritative transaction, lifecycle, budget,
finite schedule and permission checks. A healthy/active permit alone never proves
execution. Running requires an actual unexpired lease, recent enabled-worker
heartbeat, live execution permit, active lifecycle and finite schedule.

A separate original read-only pool permits truthful status after operational
revocation or PostgreSQL restart; admitted operational connections remain fenced.
The exception in authentication is restricted to this exact GET endpoint. It does
not reopen private memories, agent history or writes. Intelligence renders terminal
rows within the existing layout and polls through the existing 15-second,
visibility-aware read-only refresh. Paid controls and the legacy batch wizard
remain suppressed for prepared/read-only epochs. The ten clean routes and visual
assets remain unchanged.

Future provisioning must explicitly review these **column-only SELECT grants** for
the scoped executor login (synthetic fixtures receive them; primary does not):

| Relation in `epoch_guard` | Allowed columns |
| --- | --- |
| operational_principals | credential_id, principal_id, login, verified_by, verified_at, expires_at |
| credential_revocations | credential_id |
| authorities | principal_id |
| executors | login, principal_id, role |
| operational_approvals | id, plan |
| operational_revocations | approval_id |
| operational_claims | approval_id, login, server_start |

Never grant SELECT on `operational_claims.secret_hash`, DML on these tables, public
schema creation, or approval-issuance functions to the runtime. The executor is a
trusted server identity; caller isolation is enforced by Rust's server-bound
principal and SQL-login filters. Database administrators can access more than API
users and must remain independently controlled. No new migration is needed for
status reporting and no existing migration checksum was changed.

## Migration and recovery procedure

The dedicated `supa-migration-twelve-admin` profile is **8→12**, including 9, 10, 11
and 12 in one guarded migration-only transaction. It is not an 11→12 command and
cannot reuse an 8→10/11 approval. Dependencies, SHA-384 SQLx checksums, exact target,
expected schema 8, epoch/namespace/revision, backup and reviewed executable digest
must match an independently signed finite approval with scope
`migrate_development_epoch_through_twelve_only`. Missing or forged approvals fail
before connecting. Runtime services are not migration administrators.

Only after the separate primary migration approval:

```sh
env -i PATH=/usr/bin:/bin HOME=/home/pikoo \
  /home/pikoo/Desktop/supa-agent/target/debug/supa-migration-twelve-admin \
  --approval-file /ABSOLUTE/PRIVATE/approved-plan.json \
  --signature-file /ABSOLUTE/PRIVATE/approved-plan.sig \
  --connection-file /ABSOLUTE/PRIVATE/migration-connection.json
```

This template is **not authorization to execute**. No real trust anchor, signed
migration approval, connection file or executor/custodian grant is installed here.
Migration creates guards/tables/functions and a migration receipt; it creates no
owner, operational permit, budget, running agent or writable default.

An interrupted uncommitted migration rolls back to 8. After successful commit, no
in-place down migration is offered: stop services, preserve the migrated database
and logs, restore the verified pre-migration dump into a separately named isolated
database, verify it, and obtain explicit rollback/cutover approval. Never overwrite
primary data or discard post-migration accounting/audit evidence. Alternatively
review an additive forward fix. Any server restart or copied database invalidates
old operational claims; never replay ambiguous admission or paid requests.

## Independent backup readiness

The destination remains **unset**. There is no independent copy, external upload
or claim of disk-loss protection. Local same-disk backups aid logical recovery but
cannot survive disk failure, host theft or some ransomware events.

`scripts/independent-backup-preflight.py` reports unset readiness without copying.
For a future approved mounted directory, `--destination /MOUNT/private-supa-backups`
requires an owner-controlled 0700 directory without writable/symlink ancestors and
resolves block devices through partitions and device-mapper slaves. A different
partition of the primary NVMe disk is rejected. Unknown/virtual/network topology
fails closed and requires separately reviewed storage evidence.

Ordered future backup procedure, each transfer only after destination approval:

1. Select independently controlled physical storage outside the primary disk;
   approve encryption and retention. Do not select an unapproved cloud service.
2. Verify destination topology and permissions with the preflight. Require encrypted
   removable storage or separately reviewed public-key encryption; never place the
   decryption key beside the backup or in Git.
3. Under separate primary preflight permission, take a fresh consistent custom-format
   `pg_dump` using the reviewed local socket/admin mechanism. Capture schema inventory,
   SQLx checksums, epoch/namespace/revision, migration receipts, audit rows, safe ACL
   metadata and the historical quarantine evidence in a private manifest. Hash the
   dump and evidence before transfer. Do not copy plaintext credential files or
   password-bearing global role dumps into the evidence manifest.
4. Copy a newly named encrypted backup and sanitized integrity manifest only to the
   approved destination; no overwrite. Verify hashes at the destination and verify
   that source/protected records are unchanged.
5. Restore a decrypted copy into a separately named disposable database/server, with
   TCP disabled and isolated synthetic/approved roles. Reapply database-level read-only
   settings (not captured by a database-only dump), verify rows, audit history,
   checksums, effective permissions and quarantine hashes. Confirm claims are dead in
   the wrong database and after restart. Record successful restore evidence.
6. Securely unmount/store independent media under its approved retention procedure.
   Preserve old backups; deleting or rotating them requires its own retention policy.

## Validation results

All evidence dumps, screenshots, ephemeral credentials and database files remain
ignored and unpushed. Actual execution results:

| Check | Result |
| --- | --- |
| Full Rust workspace, all targets including opt-in PostgreSQL tests | 151 passed, zero failed/ignored in the successful full run |
| Final writable-epoch/API/worker suite after status-health refinements | All 7 passed; actual Rust API, independent synthetic passwords, finite real worker process and browser fixture |
| Migration profiles | Independently signed 8→10, 8→11 and 8→12 disposable profiles passed; checksums 1–12 verified |
| Prepared-state integrity | All 29 restored schema-12 security fixtures checked remained `prepared:0`; no primary connection |
| Backup/restart | Row and effective-ACL round trip passed; wrong-database claims rejected; immediate PostgreSQL crash/restart preserved committed audit/accounting and fenced claims |
| Owner evidence tests | 6 passed, synthetic Ed25519 keys only; no actual human identity verified |
| Independent-disk tests | 5 passed; same physical disk, partition aliases, unknown topology and mapper cases covered |
| Disposable harness safety tests | 5 passed; wrong/running/symlink/shared directory and configuration-target substitutions rejected before startup |
| Read-only harness inspection | Passed against the actual stopped disposable cluster using PostgreSQL `-C`; zero database connections |
| Terminal frontend unit contracts/security/epoch/economy | 31 cases passed across the final unit runs (14 + 7 + 6 + 4) |
| Next.js contracts and security | 30 passed across desktop, tablet and mobile |
| Production build, lint and type checks | Passed; Next.js build required unrestricted local child-process execution; restricted build could not parse its child TypeScript output |
| Terminal static build and JavaScript checks | Passed |
| Actual Rust-backed frontend | All 10 routes at 1440×1000, 768×1024 and 390×844 passed; anonymous status denied (401), authenticated status active with no running worker, zero page errors and no horizontal overflow |
| Rust formatting / diff whitespace | Passed |

The long terminal browser run completed 210 of 231 cases before interruption:
208 passed, and two cached Rust-editor assertions failed. The source function had
moved from line 106 to 108 before this milestone; the presentation offset and
accessible description were stale. Only that metadata and its cursor assertion
were corrected. The focused editor rerun passed all 9 desktop/tablet/mobile cases;
the 18 remaining mobile cases passed separately. All 231 unique cases therefore
have passing coverage across the initial and focused runs; **there is no single
uninterrupted 231-pass report**. The initial evidence is preserved locally rather
than silently replaced. No performance benchmark is inferred from these tests.

Local evidence inventory:

- `artifacts/milestone-8a/8d53f596b7bc/validation.json` and
  `rust-tests/tests.json`: full 151-case suite, checksums, restoration and crash
  recovery. The old aggregate `writable_epoch_tests:8` label counted an unrelated
  epoch test; the actual focused file has 7. The validator now counts exact test
  names, and the final focused report records 7.
- `artifacts/milestone-8b/3706a2b77155/operator-validation.json` and
  `operator-tests.log`: final 7-case actual API/worker/browser suite, 29 schema-12
  fixtures and all 26 protected hashes unchanged. Earlier failed fixture attempts
  remain local; their failures were Home intro setup and an API test client that
  did not reuse browser Basic authentication. The final harness uses the actual
  page's authenticated fetch and cleans owned processes even on assertion failure.
- The same directory contains `browser/results.json`, desktop/tablet/mobile PNGs,
  `next-security-contracts.json`, `rust-editor-browser.json` and
  `remaining-mobile-browser.json`.
- `rea-export.json`, `rea-desktop.png` and `rea-mobile.png` retain sanitized REA
  evidence. Desktop scenario `ev_9e72a98ab3021118425e242757cc8d7d6b67774890f5ed43115a4e86ca840cbc`
  performed terminal readiness and a genuine authenticated GET. Mobile scenario
  `ev_faea5f1575b793bb62941a22ddad37307ddfdd45796594f6d20efca3f90cf6d3`
  opened navigation, selected Memory and returned to Intelligence at 390×844.
  Every declared step completed. REA reported incomplete event capture
  (`missing_sections: events`); its screenshots/steps are evidence, not a claim of
  complete event-stream coverage. The independent Playwright fixture recorded
  zero page errors over its own observation window.

Reproduce the complete disposable Rust/migration/recovery run with
`python3 scripts/validate-writable-epoch.py`. Then use its emitted validation path:

```sh
python3 scripts/validate-operator-status.py \
  --validation artifacts/milestone-8a/VALIDATED_RUN_ID/validation.json \
  --inspect-only
python3 scripts/validate-operator-status.py \
  --validation artifacts/milestone-8a/VALIDATED_RUN_ID/validation.json
```

`VALIDATED_RUN_ID` must be the actual 12-character run identifier, not a new target.
The harness rejects redirected configuration and noncanonical/private directories
before starting its disposable postmaster. It has no primary target or cutover
switch. Optional `--rea-port` exposes only the owned disposable browser on loopback
for at most four minutes; end it gracefully after capture. This never exposes real
credentials or an unapproved worker.

## Remaining blockers and limits

- No real owner/custodian identity or independent credential control has been
  established. Evidence review is deliberately not automatic provisioning; consuming
  proof nonces and recording an approved binding remain trusted administrative work.
- Independent backup storage/encryption/retention is unassigned; no outside-disk
  restore has occurred. Tested local restores do not establish disk-loss resilience.
- The primary remains at its last verified schema 8 and has not been queried during
  this milestone. A fresh approved preflight must stop on drift. No root authority
  or real migration approval has been installed.
- New status metadata grants require separate review/provisioning. Without them,
  status fails closed. Scope eligibility is not a quota reservation or authorization
  renewal; transaction-time checks remain authoritative.
- Server-side read-only status after permit revocation is directly tested. Actual
  PostgreSQL restart fencing is tested separately; an uninterrupted API-status HTTP
  request spanning the PostgreSQL crash is not claimed as tested.
- Local validation used PostgreSQL 16. Railway deployment and provider access are
  separate milestones. The local disk remains low on space; no Docker installation
  or new billable infrastructure was attempted.
- Secrets scanning is the repository's targeted pattern/known-secret gate, not a
  third-party security certification. Root/database administrators remain trusted.

Git work stays on `main` using `scripts/git-local` / `scripts/commit-one`; each commit
stages exactly one named file and passes hooks. Final history/secrets audit results
are reported with repository status. No artifacts, databases, private credentials
or logs were staged; no push or deployment was performed.

## Separate approval checklist — do not combine

1. **Independent backup:** name and verify outside-disk storage; approve encryption,
   transfer, retention and restore test. This grants no primary migration authority.
2. **Primary preflight/backup:** authorize starting only the existing primary cluster,
   verify no baseline drift, take a fresh private backup and prove restoration; stop
   on differences. Primary startup is not authorized by this milestone.
3. **Primary migrations:** independently verify the migration custodian and exact
   signed 8→12 plan/checksums/executable/backup/target, then approve migration-only
   execution. Verify prepared state and no runtime grants afterward.
4. **Owner/custodian provisioning:** establish the independent identity/key evidence,
   review its fresh challenge digest and consume nonce in the administrative audit;
   separately approve exact role/login/credential UUID/expiry bindings and installation.
   Custodian authority and runtime executor credentials remain separately controlled.
5. **Agent creation and ownership:** approve a new isolated identity and explicit goals
   under a finite agent-creation permit. Never recreate historical agents. Any distinct
   ownership grant requires its own operation-specific permission.
6. **Offline control:** approve simulated seed amount, goal IDs, cycle ceiling, interval,
   expiry and budget; obtain a separate `offline_control` permit and fresh schedule
   consent. The agent stays inactive before this decision.
7. **Offline execution:** separately approve a bounded execution host and unused
   `offline_execution` admission. Confirm pause/emergency stop and revocation access,
   monitor actual heartbeat/leases/costs, and stop at the ceiling. Restart requires new
   approval; no continuous scheduling or automatic renewal is authorized.
8. **Future capabilities remain separate:** externally verified Devnet recipient and
   funding/signing approval (recipient currently unassigned); one paid GPT request
   approval; Railway resources/deployment approval. None is implied above.
