# Milestone 8E — Independent backup and identity readiness

Status: preparation complete; independent recovery and identity verification remain
blocked on operator decisions. No primary startup, SQL connection, migration,
activation, credential installation, external transfer, deletion or push occurred.
This assessment follows the completed [8D cleanup](milestone-8d-disk-recovery.md)
and supplements the [8C procedures](milestone-8c-activation-blockers.md).

## Verified baseline and evidence

The offline onboarding guard passed with all 26 protected files intact. PostgreSQL
remains stopped. Schema **8**, epoch **prepared**, revision **0**, and absence of
grants are the last SQL-verified baseline, checked against existing evidence here;
this milestone did not query live database rows.

| Item | Exact identity |
| --- | --- |
| Primary PostgreSQL 16 cluster | `postgres-data/infrastructure-v1/data` |
| Database | `supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748` |
| Epoch | `fd2f52e0-33fc-4c0d-893a-7b08b26ab748` |
| Accounting namespace | `641457b2-5fd8-4b31-b85f-c58839626831` |
| Proposed owner, KI Daniel | `6fd863e1-745d-4a06-b68e-eea69e854a69` |
| Proposed custodian | `e8c6ade3-917f-4fe4-a092-f8a58eccb487` |
| Quarantined historical budget | `ad91e475-f1da-495a-8247-139643d2d8b1` |

The historical database remains **unrecovered**. Its $0.10 budget and unresolved
**$0.004048 hold** remain protected historical evidence: no reuse, refund, settlement
or reconstruction in this development epoch is authorized.

The approved Stage 1 dump remains 68,358 bytes, mode `0600`, at
`postgres-data/preflight/1d0e9efe-e66e-4e4f-afc4-478eee4f0833/database.dump`.
SHA-256 remains
`600d2bb19b52b4f17d4278459ba722ea5f1135df703b8efc1b0b3db0c8360671`.
The cluster directory is `0700`. Both reside on device `259:2`.

Private, ignored evidence: `artifacts/milestone-8e/2cbc98c5bf67/`, containing
`readiness.json`, `block-devices.json` and `mounts.json`; directory `0700`, files
`0600`. These contain inspection results and synthetic test results, not database
exports or keys. They are not staged for Git.

## Destination assessment

The visible physical storage consists of one non-removable 256,060,514,304-byte
NVMe disk, `nvme0n1`. Root, repository, primary data, existing backups and `/tmp`
share its ext4 partition `nvme0n1p2`. Its EFI partition is on the same disk.
`/mnt` and `/media/pikoo` contain no mounted destination directories. Snap loop
devices and virtual portal/GVFS mounts establish no independent recovery storage.
Mount permissions reported by this execution namespace do not establish host-wide
permissions or the topology of an unavailable external disk.

| Option | Assessment / required next step |
| --- | --- |
| Another local directory, partition or `/tmp` | Same disk; fails independent-disk recovery requirement. |
| Existing local backup | Verified hash and prior restoration evidence; same-disk protection only. |
| Separately attached physical disk | None visible. Operator must provide an approved mounted path and device; verify backing disks, permissions, capacity and encryption before copying. |
| Existing off-host storage | None configured or verified. Requires separate review of failure domains, access, deletion authority, encryption and restore access; no service created or contacted. |

`postgres-data/epoch-backup-policy.json` remains unset: `destination: null`,
`transfer_authorized: false`, `upload_enabled: false`. Read-only
`scripts/independent-backup-preflight.py` reports not ready. Its destination review
requires an owner-controlled `0700` directory, no symlink ancestry or ancestors
writable by others, and disjoint physical backing disks. Unknown/network/virtual
topology fails closed and requires separate review. No destination's actual
capacity, write access, filesystem support, encryption or restoration is verified.

## Prepared encrypted recovery procedure — requires separate approval

1. Approve the exact independent destination, retention, restore location and
   transfer inventory. Recheck mount/device identity immediately before every copy;
   reject an unmounted path falling back to the primary disk. Verify actual access,
   available bytes, maximum file size and owner-only storage permissions. Include
   room for retained generations and expanded restoration, not just compressed dumps.
2. Separately approve an externally controlled **OpenPGP encryption recipient**.
   An independent reviewer must verify the complete certificate fingerprint through
   an established channel, encryption capability, validity/revocation and recovery
   access. Record the approved primary certificate and encryption subkey binding.
   The Ed25519 onboarding proof keys below are not an approved encryption recipient.
   No recipient has been selected, imported or tested. GnuPG and OpenSSL are installed;
   tool presence is not proof of working encryption or decryption.
3. Approve either a copy of the existing reviewed snapshot or a fresh primary
   preflight and consistent logical backup. A fresh snapshot requires new permission
   to start the exact stopped cluster, verify SQL identity/schema/prepared state,
   roles and accounting, capture a custom-format dump, and stop it. Stop on drift.
   Do not copy a running data directory as a replacement for a consistent backup.
4. Prepare an explicitly inventoried private recovery bundle. Include the dump,
   epoch/namespace, SQLx migration versions/checksums, schema and row inventories,
   ACL/read-only settings, audit/receipt evidence and historical quarantine evidence.
   A database-only dump does not recreate cluster roles or database configuration:
   include reviewed role/configuration templates without passwords or password
   hashes. Exclude dotenv files, live credentials, wallet material and keyrings.
5. Include an approved recovery copy of the **exact source revision and required
   migration tooling**, PostgreSQL/toolchain versions and restore instructions.
   Local unpublished commits and executables disappear with the primary disk; an
   older GitHub checkout cannot be assumed sufficient. Exporting this kit off-disk
   requires the same explicit inventory/transfer approval; this report authorizes
   neither a push nor an export. Review licensing/tool availability for offline recovery.
6. Encrypt within a separately approved private workspace to the verified recipient.
   Require trusted key binding, no automatic key retrieval, trust bypass, password
   arguments, overwrite flags or plaintext upload. Use new, uniquely named files.
   Keep decryption keys and recovery instructions independently controlled outside
   both the primary disk and the backup medium; no private keys go to the LLM/runtime.
7. Create an authenticated integrity manifest binding source identity, snapshot time,
   exact inventory, plaintext and ciphertext SHA-256 hashes, schema/checksums,
   accounting evidence and recipient fingerprint. An independently approved signing
   key or separately authenticated manifest channel is required; unauthenticated
   hashes alone cannot detect replacement of both backup and manifest. Verify the
   ciphertext after transfer and preserve an independently accessible manifest.
8. Have the authorized key controller decrypt a verification copy in a private
   disposable workspace. Verify plaintext hashes before restoring to an isolated
   PostgreSQL 16 cluster with a distinct path/database/socket/port, no public listener,
   no workers and no live credentials. A destination must support sufficient file
   sizes and private storage; prove this through the approved encryption/restore drill.
9. Verify SQLx checksums and expected schema 8, epoch/namespace, row counts/content
   invariants, constraints, effective ACLs, read-only defaults, audit records and
   quarantine evidence including the 4,048-microUSD hold. Reapply database-level
   safeguards explicitly. Restart only the disposable server; check fencing and
   confirm copied authorizations/leases cannot resume work. Do not migrate as part
   of backup verification. Compare source evidence without expecting normal startup
   WAL/control files to remain byte-identical if primary startup was separately approved.
10. Record results and stop disposable services. Retain existing backups and evidence.
    Plaintext removal, retention expiry and media unmounting require an approved
    handling plan; no new deletion is authorized here.

For primary-disk failure, use the independent encrypted bundle and authenticated
manifest on a replacement host with compatible tools. Restore first into quarantine,
validate the same invariants, and keep services read-only/offline. Credential recovery,
rotation, trust installation and cutover require new approvals. Do not silently replay
claims, restore active signing credentials, resume agents or refund unresolved holds.
The proposed kit and key recovery must be tested without relying on the lost disk
before declaring disk-failure recovery ready. No independent copy or drill exists yet.

## Owner and custodian verification checklist

Reviewed `scripts/verify-owner-evidence.py`. Neither root-controlled trust record
`/etc/supa-agent/onboarding-authority.json` nor
`/etc/supa-agent/migration-authority.json` exists. The proposal, display name and
separate UUIDs establish neither identity nor independent custody.

1. Establish an independent authorized reviewer and the acceptable identity-check
   channel. Record a review reference per principal without committing identity
   documents. Resolve who independently controls custodian authority: both proposals
   use KI Daniel's name, and two keys held by one person do not prove independent
   control. If separate accounts held by the same person are intended, explicitly
   resolve that policy before claiming the required independence is satisfied.
2. Each controller supplies only their Ed25519 **public** key. Independently verify
   its SHA-256 fingerprint over DER SubjectPublicKeyInfo, not the PEM file text:

   ```sh
   openssl pkey -pubin -in /PRIVATE/owner.pub -outform DER | sha256sum
   openssl pkey -pubin -in /PRIVATE/custodian.pub -outform DER | sha256sum
   ```

   Check command failures as well as digests. Compare through the established
   independent channel; self-submitted public keys/fingerprints are insufficient.
   Private keys, seed phrases, passwords and identity documents must not be sent
   through chat, committed or supplied to the agent.
3. Under separate trust-installation approval, the independent administrator anchors
   each principal/key fingerprint, identity-review reference, verified independent
   control, exact epoch and trust expiry in the fixed root-owned trust path. Neither
   the requesting service nor reviewer process may modify that file or its ancestry.
4. At review time, generate a fresh canonical UUID nonce and purpose-bound challenge
   for `owner-provisioning-review-only`, this epoch and these exact two principals.
   Require integer timestamps, no future issuance, expiry after now and validity
   at most 900 seconds. Do not create a challenge now that would expire before review.
5. Each controller signs identical canonical JSON bytes externally: sorted keys,
   compact separators and ASCII escaping, as implemented by `canonical()` in the
   verifier. Collect two different Ed25519 public keys and their 64-byte signatures.
   No signing material is placed in Supa Agent.
6. An unprivileged reviewer invokes the existing verifier with the challenge, public
   keys and signatures. It checks the fixed root trust, exact fields/bindings,
   fingerprints, signatures and expiration. A successful review explicitly returns
   `execution_authorized: false`, zero grants and no created credentials.
7. Separately approve credential provisioning with exact principal, role, scope,
   expiry and verified evidence digest. Consume the nonce once in immutable
   provisioning audit. Repeating proof review itself is not an authorization or
   a one-time grant. Owner, custodian issuance and executor controls remain separate.

Missing: independent reviewer decision, verified identity references and custody,
approved public fingerprints, root trust, fresh signed challenge and separate
provisioning authorization. No real keys, credentials, trust records or grants were
created, and no identity was marked verified.

## Capacity protection

At the recorded sample, unprivileged available space was **11,308,658,688 bytes
(10.53 GiB)**. Primary cluster allocation is 345,739,264 bytes; apparent size is
344,755,786 bytes. Root-reserved blocks are excluded. Host activity changes free space.

Retain the existing operational gate:

```text
C = max(cluster allocated bytes, cluster apparent bytes)
B = verified dump bytes
required free = max(10 GiB, ceil(1.25 * (2C + 3B + 2 GiB)))
```

Current gate: **10,737,418,240 bytes (10 GiB)**, leaving only **571,240,448 bytes
(0.53 GiB)** above it. This is a planning reserve, not a guarantee for arbitrary WAL,
rewrite, retention or database growth. Recalculate before work; monitor during work;
stop launching new work when measured capacity cannot cover the approved operation.
Do not run builds concurrently with backup/restoration/migration. Use at least the
same recovery gate on an independent destination hosting expanded restoration,
plus measured retention requirements. Backup-only capacity needs a separate inventory.

8D recovered 11,068,469,248 allocated bytes of Rust intermediates. Using that observed
size with 25% contingency, a full rebuild while preserving the recovery reserve
needs approximately **24,573,004,800 bytes (22.89 GiB)** free initially. This is an
empirical estimate; a changed toolchain/workspace can require more. Current capacity
passes the recovery floor narrowly but fails this full-build budget. Further cleanup,
disk expansion or separately approved build storage is needed before such a rebuild.
No additional files were deleted and no Rust compilation was run.

## Validation and remaining approval boundaries

- Offline primary/protected-evidence guard: passed; approved dump SHA-256 unchanged.
- Unset destination check: correctly reported not ready, no transfer.
- Storage topology unit tests: **5 passed** (synthetic devices, partition sharing,
  mapper backing disks, unknown storage, insecure/symlink destinations).
- Owner proof unit tests: **6 passed**, using temporary synthetic test keys only;
  covered binding, expiry, self-attestation rejection, tampering and missing trust.
  These are not verification of the proposed real owner or custodian.
- No independent encryption/decryption, destination write or restore test occurred.
  Prior same-disk PostgreSQL restoration evidence remains limited to its tested scope.
- No frontend/backend logic changed; no build, Rust suite or browser run was needed
  for this documentation/storage assessment. Approved website remains unchanged.
- Git and secrets gates are run for this single report commit; local artifacts remain
  ignored. The targeted secrets scanner is a gate, not proof against every possible secret.

Separate future approvals, in order:

1. **Storage and encryption:** exact independent mounted destination, verified full
   recipient fingerprint, manifest authentication, recovery-key custody and retention.
2. **Backup drill:** exact export inventory and transfer; separate explicit primary
   startup/fresh snapshot approval if needed, isolated decryption/restoration and shutdown.
3. **Identity/trust:** independent evidence and public-key bindings, then explicit
   trust installation and scoped credential provisioning; no implicit owner grant.
4. **Primary migration:** fresh preflight, verified independent restoration, exact
   8-through-12 migration plan/checksums and independently signed migration approval.
5. **Operational activation:** separately approved epoch cutover/ownership and one
   finite offline agent session. Neither backup nor migration grants execution rights.

Paid inference, funding/Devnet recipient assignment, transfers, trading, launches,
X publishing, deployment and pushing remain outside these approvals. The Devnet
recipient remains unassigned. Primary activation is not ready; a narrowly scoped
backup preflight may be considered only after storage/key decisions and fresh
capacity verification, with no implication of migration or activation permission.
