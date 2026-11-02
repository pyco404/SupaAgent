# Authenticated work-evaluation review support

## Outcome and retained state

The dedicated review mechanism is implemented and tested against restored disposable copies. The approved decision has **not** been recorded in the retained evaluation: verified human-owner authentication is unavailable. No substitute owner credential, ownership mapping, database role, or grant was created.

Approved semantic decision:

- Decision: `accept_evaluation_only`.
- Session: `41316d3e-40b8-4b7d-8605-83db7aab36b8`.
- Agent: `82c254ff-4659-4404-94c4-c497565567e6`.
- Canonical report SHA-256: `01fcfc0576fb7d8cebfc09666c060eb1bff5c85a08b2682d2b17842cfe5abc42`.

The retained evaluation database remains at schema 12, without the new review table or proposed review-service database role. The primary cluster remains stopped, schema 8 and inactive. Read-only fingerprint checks found the retained evaluation tables, original result files, earlier evaluation records and historical accounting evidence unchanged. The historical $0.004048 hold and Claude $0.003536 conservative reservation were not released, settled or modified.

The original successful evaluation still has 1,007 input tokens, 391 output tokens, estimated cost $0.000297, one consumed generation, a $0.005 budget with $0.000297 committed, a paused agent and an unpaid `created` job with zero earned revenue. The original session remains `awaiting_approval`; review is a separate record and status projection, not a rewrite of its generation record.

## Implementation

- `crates/domain/src/work_review.rs`: strict accept/reject request and audit types. Rejects unknown fields, invalid hashes, nil request IDs, absent confirmation and stale confirmation. Confirmation lasts at most five minutes.
- `crates/domain/src/lib.rs`: exports the review types.
- `migrations/0013_work_evaluation_reviews.sql`: additive, private, append-only review table, canonical report hashing and insertion guards. One decision per session and one use per request UUID. Guard rejects mismatched session, agent, original approver, report or owner access, malformed/null confirmation and application epochs. Update/delete/truncate are rejected. No accounting, lifecycle or job mutation.
- `crates/persistence/src/work_review.rs`: owner-scoped review context, locked transactional decision and durable review readback. Locks the agent, owner ACL and session before inserting. Rechecks freshness after waiting. A later report mismatch displays `review_requires_reconciliation` rather than treating the old acceptance as current.
- `crates/persistence/src/lib.rs`: registers persistence support.
- `crates/api/src/live_work.rs`: dedicated authenticated GET/POST review route, server-derived reviewer identity and `Cache-Control: no-store`. Requires both configured owner role and per-agent ownership. Confirmation must expire before an applicable credential deadline. Does not call the model or existing memory-tool approval endpoint.
- `crates/api/tests/work_review.rs`: restored-copy PostgreSQL/API security and preservation regressions.
- `crates/api/examples/work_review_api.rs`: dormant review-only server pinned to the retained database, cluster identity, agent, session, report hash and database role. Loopback port 55683; accepts only health and exact review routes. No migration, recovery, credential creation, provider dispatch or other mutation route. Requires supplied owner authentication and a finite credential deadline; exits closed without it.

Both `accept_evaluation_only` and `reject_evaluation_only` are final evaluation decisions. Neither delivers customer work, completes a job, recognizes revenue, changes balances/budgets, approves tools or activates an agent. A retry after an uncertain HTTP response must first GET the review context; do not submit another decision automatically.

## Validation performed

78 distinct relevant Rust tests passed: domain 41, API library 4, authentication 8, controlled inference 6, economy 1, live work 9, new PostgreSQL review integration 8, scoped entry-point unit test 1. The final eight review integration tests were rerun after the credential-deadline check and all passed.

Covered scenarios:

- Exact report acceptance and rejection; original report/citations, usage/cost, job, revenue, lifecycle and all other source tables unchanged.
- Public, unauthenticated, different-owner, operator, viewer, expired credential and client identity spoofing denied.
- Wrong hash, altered report, stale/future/overlong confirmation and generation-session nonce reuse rejected.
- Duplicate and concurrent decisions: exactly one committed event, conflicting requests rejected.
- Revoked ownership blocks review and context access; review confirmation cannot outlive the credential.
- Reconnected store reads the durable decision; previous memory-tool approval remains unavailable for work sessions.
- Direct insertion guards reject forged/null confirmations; update/delete/truncate rejected; UTF-8/escaped-string canonical hash compatibility checked.
- Restored schema-12 clone migrates to 13 with matching SQLx SHA-384 checksum, source tables unchanged; transactional rollback leaves no new table.
- Application epochs cannot use this mechanism to obtain blanket write authorization.
- Review-only entry-point route filter blocks generation and other mutations; built executable exits 1 with `work_review_startup_failed_closed` in an environment without credentials, before database access.

Formatting, relevant Clippy checks with warnings denied, and the scoped executable build passed. Tests used PostgreSQL 16 restored copies in a separate socket-only cluster; that new cluster was stopped after validation. No provider HTTP calls were made. No frontend files changed, so no new browser or visual-validation claim is made.

Local private evidence is under ignored `artifacts/work-review-support/`; database dumps and diagnostic artifacts are not committed. Test/build logs remain local. New build output left approximately 0.72 GiB free; no additional cleanup was performed. More compilation or operational rehearsals need storage headroom.

## Authentication blocker and separate prerequisites

The proposed real owner `6fd863e1-745d-4a06-b68e-eea69e854a69` remains prepared/ungranted, with identity binding unapproved and authentication disabled. The retained evaluation belongs to test principal `acede749-746b-4541-b178-0085434dcd39`. There is no verified human binding between them. Machine-generated disposable generation credentials are not proof of human ownership and were not used to record this decision. The user's semantic approval is already explicit; it is not being requested again.

Before recording through the API, the authorized identity process must establish a verified human binding and supply a properly scoped, finite owner credential privately. Do not silently change ACLs, impersonate the test owner or generate a token to bypass that requirement. If the intended reviewer cannot authenticate as the existing authorized principal, a separately reviewed ownership/provenance procedure is needed first; the current endpoint deliberately rejects mismatched approvers.

Additional separately reviewed operational prerequisites for the retained disposable database:

1. Back up and approve applying migration 13 to that database only. No retained migration was performed here; never use normal API startup as an implicit migration/recovery action.
2. Approve a least-privilege `supa_work_review` database service role/profile, restricted to local connections and this exact database. It needs review-context reads, review-event INSERT and only column-level UPDATE permissions needed for row locks on agent/ACL/session identity columns. It must not receive financial UPDATE, session-data UPDATE, session INSERT, job UPDATE or permission-changing ACL UPDATE. Validate the actual restricted profile in a clone before installation; this task did not install or claim to test a real service credential.
3. Supply verified owner/public service credentials and a finite deadline through private configuration. The scoped executable requires exactly the retained test owner principal and rejects financial-write privileges at startup. Never repurpose generation runtime credentials.
4. Start only the scoped review server after those prerequisites are authorized. Do not start the primary cluster, general API, worker or generation launcher.

Future command, **not executed against retained state**:

```sh
# Run only after approved database/profile preparation and verified authentication.
# Credentials must already be supplied privately; do not place tokens in arguments.
./target/work-review-validation/debug/examples/work_review_api
```

## Exact authenticated review action

GET and then POST:

```text
http://127.0.0.1:55683/agents/82c254ff-4659-4404-94c4-c497565567e6/live-work/41316d3e-40b8-4b7d-8605-83db7aab36b8/review
Authorization: Bearer <verified owner credential, supplied privately>
```

GET returns the original report and evidence inventory, canonical hash, usage/cost, existing review and server time. Verify the exact approved session/hash and absence of an existing decision. POST the following body with a fresh review-only UUID and current UTC timestamps; expires_at must be within five minutes and before the credential deadline:

```json
{
  "request_id": "<fresh review-only UUID>",
  "decision": "accept_evaluation_only",
  "report_sha256": "01fcfc0576fb7d8cebfc09666c060eb1bff5c85a08b2682d2b17842cfe5abc42",
  "confirm_review": true,
  "consented_at": "<current UTC RFC3339 time>",
  "expires_at": "<finite UTC RFC3339 time>"
}
```

These are review confirmation timestamps, not an extension of the evaluation job, provider authorization or consumed generation allowance. No owner/role identity is accepted in the body. The response records the server-authenticated reviewer and server review timestamp. GET afterward verifies the event and all retained-data invariants. Rejection uses `reject_evaluation_only` with the same binding rules.

## Recovery, limitations and Git

A crash before transaction commit leaves no review; a committed decision survives restart and cannot be replayed. Rollback means stop/disable the review service and preserve the audit event and additive schema; never delete a review or rewrite historical session/accounting records to undo it. Superusers/database owners can alter schema or disable triggers, so database administration and server credential mapping remain trusted boundaries. No claim is made that PostgreSQL triggers authenticate an external human independently of the API.

Changes are committed individually on main using the repository's one-file Conventional Commit workflow, with secrets scanning and a full history audit. Four pre-existing logo deletions are unrelated and preserved unstaged. No push, deployment, primary migration, human credential installation, ownership grant, paid inference or retained review was performed.
