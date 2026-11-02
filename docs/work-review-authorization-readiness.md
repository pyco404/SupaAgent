# Retained evaluation review: legitimate authorization readiness

## Finding

The semantic decision `accept_evaluation_only` is already approved. **It is not recorded, and the real owner cannot yet record it legitimately through the current implementation.** This preparation did not rebuild or change the existing Rust endpoint, migration, executable or ownership model.

Read-only SQL confirmed:

| Item | Retained state |
| --- | --- |
| Database | `supa_m10a_8c6356653dc2_test` |
| Cluster system identifier | `7703596362529484555` |
| Session | `41316d3e-40b8-4b7d-8605-83db7aab36b8` |
| Agent | `82c254ff-4659-4404-94c4-c497565567e6` |
| Original approver and current agent owner | `acede749-746b-4541-b178-0085434dcd39` (test principal) |
| Report SHA-256 | `01fcfc0576fb7d8cebfc09666c060eb1bff5c85a08b2682d2b17842cfe5abc42` |
| Schema | 12; review table and `supa_work_review` role absent |
| Application epoch | No epoch row in this isolated evaluation database |
| Proposed human owner | KI Daniel, `6fd863e1-745d-4a06-b68e-eea69e854a69` |
| Owner identity | Prepared/ungranted; binding unapproved; authentication disabled |
| Proposed custodian | `e8c6ade3-917f-4fe4-a092-f8a58eccb487`, proposed/unverified |
| Independent onboarding trust anchor | Not installed |

The review API requires both configured owner role and per-agent ownership. Persistence and the database guard additionally require the reviewer to equal the original session approver. `establish_owner` rejects an already owned agent, `grant_access` cannot grant owner, and the database permits only one owner per agent. Even an administrative ownership replacement would fail the original-approver check. Rewriting `approved_by`, using the test token as a human login, or relabeling the test account would lose provenance or impersonate it; none is an acceptable workaround.

## Legitimate path: narrowly delegated review, not ownership transfer

The appropriate proposed path keeps the test principal, original authorization, agent ACL, generated output and accounting unchanged. After independent verification, a separately authorized fixture custodian/administrator can attest that the retained synthetic evaluation is under their legitimate custody and delegate **only this review** to KI Daniel's real principal. Possession of a test token, a proposal or the database administrator login does not itself establish that custody.

A future immutable delegation must bind the exact database/cluster, absent-epoch evaluation context, agent, session, report hash, original approver, actual verified reviewer, permitted decision, finite expiry, credential identity and maximum one decision. It must record the independently verified issuer, identity-evidence references, fixture-provenance reference and exact approved-plan digest. Runtime, frontend and model cannot issue or edit it. Revocation and single-use consumption must be durable, checked under locks and atomic with review insertion. The record must distinguish actual reviewer from original approver and record the delegation identifier. It grants no general agent ownership, memory access, other-session review, financial permission, generation allowance or agent activation.

`config/work-review-delegation.proposal.json` captures the fixed target and deliberately unassigned trust/expiry/credential fields. Its canonical JSON SHA-256 (sorted keys, compact separators, UTF-8) is:

```text
0af5008c77bb9c3a8953687f32943c3698f0e469e0a578c9094cbbf16a93d9d3
```

**This is a proposal fingerprint, not an executable approval or replacement generation scope.** It issues zero delegations, marks identity and authentication false, and explicitly says the current endpoint does not support the delegation. Completing authority, evidence, validity and credential fields will require a fresh final plan digest and explicit review.

A focused authorization extension remains a technical prerequisite: a review-specific resolver shared by context and decision handling; matching SQL guard support for independently issued/revoked delegations; immutable issuer/consumption audit; and a scoped server profile that accepts the verified reviewer through that record. The existing endpoint URL and six-field request can remain unchanged. The existing global `owner` helper for other operations must not be relaxed, and no arbitrary environment flag or client principal field may confer delegation. The current compiled entry point is pinned to the test principal, so it cannot be launched as KI Daniel and claimed to support delegation. This extension was **specified, not implemented or falsely demonstrated**, in accordance with the instruction not to rebuild the implemented review endpoint.

## Identity evidence and exact human steps

1. Appoint an independent technical identity verifier and establish an authenticated out-of-band channel. Verify KI Daniel's binding to principal `6fd863e1-745d-4a06-b68e-eea69e854a69`. Record a minimal verification reference; do not put identity documents in Git or application logs.
2. Independently verify the proposed custodian's identity and separate control. The proposal alone remains insufficient. Independently controlled keys must have different full SHA-256 fingerprints of Ed25519 SubjectPublicKeyInfo DER. Key control alone is not identity verification. Do not reuse wallet keys or application API tokens.
3. Have the authorized administrator independently anchor those public fingerprints, identity references and expiry in root-controlled `/etc/supa-agent/onboarding-authority.json`. Its parent paths must be root-owned, non-symlink and not group/world writable. This installation requires separate approval; no trust file was created here. The API must not control this authority or hold its private signing material.
4. For the existing read-only owner-proof review, have owner and custodian sign the same canonical challenge using their independently controlled keys. Exact fields: version 1, purpose `owner-provisioning-review-only`, epoch `fd2f52e0-33fc-4c0d-893a-7b08b26ab748`, the two principal UUIDs, fresh UUID nonce, integer issued_at/expires_at, maximum 15-minute validity. Signatures, public keys and independent anchors must agree. Do not send private keys or passphrases to Codex.

```sh
python3 scripts/verify-owner-evidence.py \
  --challenge /PRIVATE/challenge.json \
  --owner-public-key /PRIVATE/owner.pub --owner-signature /PRIVATE/owner.sig \
  --custodian-public-key /PRIVATE/custodian.pub --custodian-signature /PRIVATE/custodian.sig
```

That command only validates identity-proof evidence. It neither consumes an approval nonce nor provisions credentials, grants custody, activates the primary epoch or delegates evaluation review. In particular, its primary-epoch proof-review purpose must **not** be reused as an evaluation delegation. The future review-specific approval must separately bind the retained evaluation target and fixture-custody evidence.

5. After the missing delegation controls pass disposable tests and are independently reviewed, obtain explicit approval of the exact finite review delegation and restricted service/authentication profile. Provision the real reviewer under their actual principal, with server-side mapping, distinct public-service credentials, private files 0600 inside directories 0700, finite deadline and documented revocation. Never copy `.env.eval`, generation runtime tokens or dormant pending owner credentials into the service. No such provisioning was performed here.
6. Read the authenticated exact-session context, verify the report hash and absence of an existing decision, then submit the already-approved semantic decision with fresh review confirmation. Verify the durable event and unchanged original/accounting records afterward.

No primary database startup, primary migration or epoch activation is needed for this isolated review. The prepared primary identity can be verified without granting it primary database permissions.

## Disposable validation

The existing socket-only PostgreSQL 16 review fixture was started, cloned from its previously restored schema-12 seed, tested, and stopped again. No primary startup occurred. No retained database writes were made. Fixtures contain copied evaluation evidence but only synthetic test authentication; they do not establish real human identity.

New reproducible checks:

```sh
python3 scripts/test-work-review-profile.py -v
```

Requires the separately started, pinned disposable fixture cluster. The script never starts PostgreSQL or connects to the retained evaluation. It creates a new test database and a temporary **NOLOGIN** profile, not a real credential. It executes migration-13 DDL in the clone for permission testing; it does not forge a SQLx migration-history entry. The existing prebuilt API tests separately run `Store::migrate` on their restored clones, exercising the actual SQLx path. No new delegation schema was applied because delegation is not implemented.

All **13 selected tests passed**:

- Four new profile checks: minimal column-level lock permissions plus review INSERT succeed; financial/job/session-data/ownership mutations and review deletion/truncation fail with permission errors; real-principal review is denied even after a hypothetical rolled-back owner replacement; original session/hash/generation allowance remain unchanged.
- Three existing API/PostgreSQL regressions, run directly from the already-built test executable: acceptance preserves every source table and survives reconnect; rejection is evaluation-only and immutable; public/other-owner/operator/viewer/expired/spoofed authentication is denied.
- Six existing synthetic identity-proof tests: valid proof grants nothing; wrong target/purpose/fields, expired challenges, self-attested identity, altered signatures and missing/untrusted anchors fail.

The profile grants SELECT on required context tables, INSERT only on the review table, UPDATE(id) only on agents/live_sessions for row locks, and UPDATE(granted_at) only on the ACL table for row locks. It cannot change role/principal, financial values, job state or session JSON. Its grants were revoked and temporary NOLOGIN role removed after testing. This validates SQL privileges using `SET LOCAL ROLE`; it is **not** a claim that a real SCRAM login, HBA profile or human credential was installed/tested. Future delegation tables need separate restricted-profile tests.

The successful clone insert was rolled back. Whole-source-table fingerprints before/after match, excluding only the independent review-event table. API regressions also compare every other public table across actual committed review events. Thus evaluation acceptance cannot itself complete the customer job, create earned revenue, consume another generation or change inference accounting through the tested path.

Read-only retained-state checks found the same 38-table fingerprint:

```text
06193a74141e9ebb405cb917e754b89e4d8a038cafc2bdcac0dfad233c644373
```

All 29 protected/original file hashes matched the previous evidence, including historical accounting, primary control/configuration files, dormant owner material, original paid result and request journal. Both historical holds remain unchanged ($0.004048 and $0.003536). The retained evaluation remains $0.000297 committed, one generation consumed, unpaid and unreviewed. Local sanitized evidence/logs remain ignored under `artifacts/work-review-support/`; no dumps, credentials or sensitive artifacts are committed.

## Exact future review request — presently blocked

Use the actual verified human-owner credential supplied privately after the separate prerequisites. Never use the test-principal generation credential. The request remains bound to:

```http
POST /agents/82c254ff-4659-4404-94c4-c497565567e6/live-work/41316d3e-40b8-4b7d-8605-83db7aab36b8/review
Host: 127.0.0.1:55683
Authorization: Bearer <privately supplied verified reviewer credential>
Content-Type: application/json

{
  "request_id": "<fresh review-only UUID>",
  "decision": "accept_evaluation_only",
  "report_sha256": "01fcfc0576fb7d8cebfc09666c060eb1bff5c85a08b2682d2b17842cfe5abc42",
  "confirm_review": true,
  "consented_at": "<current UTC RFC3339 time>",
  "expires_at": "<UTC time within five minutes and before credential/delegation expiry>"
}
```

First GET the same URL with the same private authentication. Do not put reviewer identity or role into the body. The planned resolver must obtain the approved delegation from trusted server-side records. The existing implementation will reject a real-owner request; **do not execute this as though delegation already works**. Fresh confirmation does not extend the job or authorize another generation. A timeout requires read-only reconciliation, not an automatic second POST.

## Remaining approvals and blockers

1. **Identity/trust:** independently verify owner and separate issuing authority, then separately approve trust-anchor installation. No authority is inferred from a name, key, proposal or this conversation's semantic approval.
2. **Technical readiness:** implement and validate the narrow delegation resolver/immutable ledger and scoped-profile extension. No ownership transfer or historical-session rewrite is an approved substitute. This is a remaining implementation gap, not a need to re-approve the report's semantics.
3. **Retained database preparation:** separately approve a fresh backup and exact additive migration(s) for the retained evaluation database only, with SQLx checksums and recovery evidence. Migration 13 is already tested in clones; the delegation migration does not yet exist and therefore cannot be approved by an invented checksum.
4. **Delegation and provisioning:** separately approve the exact final delegation digest, verified human reviewer, independent issuer, proof/provenance references, one decision, expiry, service login and credential installation. No broad owner grants are necessary or authorized.
5. **Authenticated recording:** perform the above already-approved semantic decision under the verified finite delegation. Do not ask for or imply another paid allowance. No review action has been recorded by Codex.

The current endpoint, compiled executable and Rust architecture were not rebuilt. No real credential/grant/delegation, paid request, agent activation, accounting adjustment, push or deployment occurred. The protected primary remains stopped. Limited disk headroom remains; no files were deleted or cleanup undertaken. Existing four unrelated logo deletions remain unstaged.
