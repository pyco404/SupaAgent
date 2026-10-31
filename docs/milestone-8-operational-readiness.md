# Milestone 8: controlled operational activation preparation

2026-10-31. This report supersedes the execution counts and migration-11 limitation
in `milestone-8.md`; that earlier report remains historical evidence. Nothing here
approves primary startup, migration, grants, credentials, execution or cutover.

## Baseline and root causes

Inspection was offline. The primary stayed stopped; no current SQL observation is
claimed. The approved Stage 1 backup and all 26 protected files were checked before
and after the rehearsal. Last verified primary state: schema 8, prepared epoch
`fd2f52e0-33fc-4c0d-893a-7b08b26ab748`, accounting namespace
`641457b2-5fd8-4b31-b85f-c58839626831`, database
`supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748`. Socket-only PostgreSQL uses
port 55440, peer administration and database-default read-only transactions.

Two independent blockers explain why the application has no writable development
state:

1. The old signed migration executable intentionally permitted only schema 8→10.
   Embedding migration 11 never authorized it. A separately scoped 8→11 executable
   is now implemented and tested, without changing any migration SQL or accepting
   old 8→10 approvals for the new profile.
2. The epoch architecture supports `prepared` and `active_read_only`, not an
   operating writable state. Migration 9, migration 10's trusted activation scope,
   Store verification, API startup/mutation guards and worker startup all enforce
   that boundary. Any epoch row prevents worker startup; authenticated owners also
   cannot mutate it through the API. Migrations 9–11 alone do not remove this
   blocker. No guard was relaxed and no epoch row was removed from a restored clone.

A future writable-mode implementation needs separately reviewed trusted approval
scope, durable state/transition invariants, least-privilege database roles, API
startup/write policy, worker fencing and regression tests. A read-only cutover
cannot safely substitute for it. Approval alone will not create that missing code.

## Migration-only entry point

`crates/migration-admin/src/bin/supa-migration-eleven-admin.rs` exposes only the
existing fixed-target migration administration interface. There is no database,
socket, epoch or disposable override in its production CLI. The old executable
remains pinned to 8→10; the new one requires exact starting schema 8 and installs
9–11 atomically, including its immutable signed receipt, leaving the epoch prepared.
It is not a 10→11 upgrade command.

The new independently signed plan requires:

- scope `migrate_development_epoch_through_eleven_only`, expected version 8,
  target version 11 and exact SHA-384 inventory 1–11;
- the reviewed database, epoch, namespace, PostgreSQL system identifier, socket,
  data directory, verified custodian principal and migration login;
- exact executable SHA-256, reviewed source/catalog hashes and approved backup;
- a unique approval identifier, fresh issuance and expiry, and Ed25519 signature
  from the independently verified root-owned trust anchor.

Missing/untrusted approvals fail before connecting. The requesting service cannot
supply its own trust anchor. No real trust file, key, signature or approval was
created. Existing expected-state checks, peer-only/no-TCP settings, transaction and
SQLx locks, source-row preservation and empty authority/operational-state checks
remain. Changed checksums, scope, principal, epoch, target, executable or plan fail
closed. Duplicate/concurrent execution cannot apply twice.

Future command, **only after separate primary-startup/migration approval and
independent custodian verification**, from a clean environment:

```sh
env -i PATH=/usr/bin:/bin HOME=/home/pikoo \
  /home/pikoo/Desktop/supa-agent/target/debug/supa-migration-eleven-admin \
  --approval-file /ABSOLUTE/PRIVATE/approved-plan.json \
  --signature-file /ABSOLUTE/PRIVATE/approved-plan.sig \
  --connection-file /ABSOLUTE/PRIVATE/migration-connection.json
```

Paths above are placeholders, not installed credentials or a runnable approval.
The signer must review the actual built executable and complete exact plan.
Files require private owner-only permissions and no links. The program additionally
checks `/etc/supa-agent/migration-authority.json` and its root-controlled parents.
The backup is pinned to the approved Stage 1 digest
`600d2bb19b52b4f17d4278459ba722ea5f1135df703b8efc1b0b3db0c8360671`;
a new primary backup/review requires explicitly rebinding and revalidating the
profile, not silently substituting a dump. Run a fresh approved preflight before
execution and stop on drift.

There are no down migrations. Failure before commit rolls the whole transaction
back to schema 8. After an interrupted/ambiguous commit, inspect schema, checksums
and signed receipt read-only; never automatically retry. A completed migration
requires forward recovery, or separately approved restoration into a new database
and reviewed reconnection. Preserve the original database and audit evidence;
restoring a dump over it or erasing a receipt is not rollback.

## Owner onboarding and credential sessions

KI Daniel's proposed owner principal remains
`6fd863e1-745d-4a06-b68e-eea69e854a69`; no grant or credential was activated.
The separate custodian proposal `e8c6ade3-917f-4fe4-a092-f8a58eccb487` remains
unverified, without issuance authority. A name or UUID is not identity proof.
Independent custodian control and owner binding need explicit operator verification
before the separate installation/grant decisions.

Authentication maps server-held tokens to principal UUIDs and roles; client identity
or role headers cannot establish ownership. Public-service credentials remain
separate. Owner-only controls, operator execution privileges, viewer read access,
cross-owner denial and revocation are tested using synthetic principals.

Added `SUPA_OPERATOR_SESSION_DEADLINES`: optional server-side JSON mapping of
configured principal UUIDs to absolute Unix-second credential deadlines. Unknown
principals, zero/malformed deadlines or invalid configuration fail startup. A
credential is rejected at its exact deadline, including on public routes when a
legacy owner credential equals the service token. Browser-supplied expiry fields
cannot extend it. No token or private identity configuration is logged.

This is expiry of the configured operator credential, not a browser cookie session
or an identity-provider login flow. Unconfigured legacy credentials retain their
existing indefinite lifetime; finite deadlines must be explicitly configured for
future onboarding. Revocation/rotation remains server-configuration replacement
and process restart; it is not instantaneous centrally managed revocation. Short
sessions also require a correctly maintained host clock. Browser Basic credentials
are server-side mappings; revoke their mapping as well as the API token.

## Inactive first-agent preparation and disposable rehearsal

`config/first-agent-plan.json` remains unsigned: no agent UUID, no creation, no
ownership grant and no worker launch consent. It proposes a fresh Supa identity,
two explicit goals, persistent evidence-linked memory, allowlisted `read_memory`,
`remember`, `evaluate_goal` tools and finite offline permission. Its proposed
three-cycle limits were not silently raised.

The ten-cycle rehearsal obtains a separate synthetic owner authorization:
10 maximum cycles, five-second cooldown, 180-second expiry and 11,000 simulated
microUSD maximum costs, against a new unmarked disposable test database. It uses
20,000 synthetic microUSD seed, entirely separate from historical accounting.
One manual cycle is followed by nine actual offline-worker cycles. Two worker
incarnations exercise pause, process/API restart and explicit resume. Workers do
not execute while paused; the exact ceiling prevents an eleventh cycle. Memory,
goal assessments, authorization history, worker identifiers, action permission
outcomes and execution history come from real persisted test records.

A fresh one-cycle fixture authorization then tests expiry through an explicitly
labeled test-clock injection, without executing an eleventh cycle. Emergency stop
suspends the agent and fences resume. This clock injection is not claimed as a
natural wall-clock expiration. Credential deadline boundary and HTTP tests cover
session expiry independently.

The operational database has no application epoch metadata. It is distinct from
the approved-backup clone, which keeps its prepared epoch and creates zero agents,
grants, authorities, budgets or approvals. This demonstrates existing offline
runtime behavior, **not** a writable prepared-epoch cutover. The API scheduler and
all primary workers stay disabled; only the explicitly scoped disposable worker
processes are enabled during the test. No OpenAI transport is configured.

## Recovery, accounting and evidence

The approved backup is restored independently, migrations/checksums and original
source rows are verified, and both the prepared clone and synthetic operating
fixture are dumped privately and restored into separate databases. Comparisons
cover rows, constraints, indexes, effective ACLs/grantors, triggers, functions,
sequence definitions and ownership. Immediate PostgreSQL shutdown/restart checks
committed records and forward-only sequence recovery. Database-level read-only
settings are reapplied explicitly; a database dump does not replace host HBA,
roles or service configuration. All disposable clusters are stopped afterward.

The historical PostgreSQL database remains unrecovered. Archived budget
`ad91e475-f1da-495a-8247-139643d2d8b1`, $0.10 ceiling and unresolved $0.004048 hold
remain quarantined evidence, not newly restored live accounting. Existing UUID
reuse/refund protections and protected hashes remain. No historical records are
imported into fixture balances. Independent backup storage remains unset; same-disk
backup verification does not protect against disk loss.

Completed evidence run: `artifacts/milestone-8/2ae1202596b6/validation.json`
and `onboarding.json`, with strict migration results in its
`strict-migration-tests/` directory. Dumps, process-local fixture information,
screenshots and logs remain ignored/private local files, not Git content.

| Check | Actual result |
| --- | --- |
| Rust workspace, all targets and opt-in PostgreSQL integrations | 143 passed, zero failed/ignored |
| Independent signed migration profiles | Both 8→10 and 8→11 passed; forged/tampered/scope/checksum/identity/expiry rejection, atomic rollback, replay/concurrency and backend termination tested |
| Restored source preservation | Original columns and rows in 28 source tables preserved; schema 1–11 SHA-384 verified; prepared clone revision 0 and zero operational rows |
| Backup/restore and PostgreSQL crash restart | Both databases restored into separate databases; effective ACLs, schema, rows, audit evidence and forward-only sequence recovery matched |
| Offline operation | Exactly 10 completed cycles: 1 manual + 9 worker; 2 actual worker incarnations; 9 evidence-linked goal assessments; memory retained |
| Accounting | 20,000 fixture microUSD seed; measured 10,000 simulated inference + 1,000 operating expenses; final balance 9,000; no real provider usage, live budgets, economic receipts or wallet identities |
| Authorization and recovery | Owner/operator/viewer/outsider boundaries; forged claims; revocation/rotation; credential expiry and fallback denial; paused restart; cycle ceiling; authorization expiry via fixture clock injection; emergency stop |
| Python safety guards | 31 passed: owner 9, cutover 4, preflight 5, persistence 6, onboarding 7 |
| Process-cleanup regressions | 3 passed, including forced cleanup and repeat shutdown |
| Frontend adapter unit tests | 28 passed |
| Browser process rehearsal | Real Chromium at desktop 1440×1000 and mobile 390×844; clean `/intelligence` route, authenticated scoped records, paused controls, zero page errors or horizontal overflow |
| Frontend lint / TypeScript | Passed; no frontend source changes; production build and broad visual suite were not rerun in this backend-focused follow-up |
| Rust quality | Workspace check, formatting and all-target Clippy with warnings denied passed |
| Primary/environment preservation | Primary never started/connected; all 26 protected hashes unchanged; all disposable clusters stopped |

Screenshots: `desktop-paused-owner.png` and `mobile-paused-owner.png` in that run.
These are owned Playwright/Chromium captures, not a new REA session or physical-device
certification. API state, leases and audit claims come from database/runtime tests,
not inferred from an image. Existing approved UI was not repointed or modified.

A failing development assertion initially tried to read migration-9 columns after
successful rollback to schema 8; the assertion was corrected to inspect schema-8
state. That attempt is retained separately and is not counted as a passing run.
The final complete run above passed without ignoring any test or weakening the
expected starting schema. Local free disk was approximately 1.6 GB afterward;
further retained disposable clusters/builds need capacity planning.

Git validation: the complete pre-change history audit passed for all 1,168 main
commits. The targeted local secret scanner passed across 1,169 reachable commits
(including retained archive history) and 935 distinct blobs. Each new file is
committed separately with the hook's staged-file scan; a final complete audit and
scan follow those commits. This scanner recognizes known local secrets and common
formats; it is not a guarantee against every possible semantic disclosure. No push
or deployment is authorized or performed.

## Separate approvals and remaining limitations

1. Approve fresh primary startup/preflight/backup separately; stop on any drift.
2. Independently establish custodian identity/key control and separately approve
   the trust anchor and migration-only administration. Review/sign the exact 8→11
   profile and explicitly approve primary migration. This grants no ownership.
3. Verify the proposed owner through the agreed authentication process, then
   separately approve scoped credential installation, finite session deadlines,
   owner binding/role grant and least-privilege database configuration.
4. Implement and validate the missing writable epoch architecture before requesting
   operating cutover approval. Keep read-only and writable activation distinct.
5. Separately approve first-agent creation, goals, simulated seed and a finite
   offline authorization/worker host. Ten test cycles do not authorize a real run
   or alter the unsigned three-cycle primary proposal.
6. Devnet recipient verification/funding, any paid GPT request, GitHub push and
   Railway deployment remain separate decisions. Recipient stays unassigned;
   live GPT scheduling, spending, Mainnet, trading, launches and X publishing stay off.

There is no safe primary operational activation command to issue yet. The remaining
writable-epoch blocker is technical, not a missing blanket approval. The website,
its ten routes, branding, animations, Rust execution algorithms and migration SQL
remain unchanged.
