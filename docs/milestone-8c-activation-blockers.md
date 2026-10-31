# Milestone 8C — environmental and identity blockers

Status: **preparation complete; operational blockers remain unresolved**. No primary
startup, SQL connection, migration, deletion, credential creation, grant, activation,
upload, push or deployment occurred. Website and runtime code are unchanged.

## Disk capacity and proposed cleanup

The inspection on 2026-10-31 found one 256,060,514,304-byte physical NVMe disk,
`nvme0n1`. The primary ext4 filesystem on `nvme0n1p2` has 249,792,131,072 bytes
of capacity. At the recorded capacity sample, unprivileged available space was
**358,850,560 bytes (342.2 MiB)**. `df` rounds usage to 100%. Inodes were only
23% used; this is a byte-capacity problem. `/tmp` shares this filesystem.
Root-reserved ext4 blocks are not counted as available operational capacity.
Other applications can change these measurements; measure again before approval.

Read-only allocated-size inventory (rounded; overlapping rows must not be added):

| Location | Allocated bytes | Assessment |
| --- | ---: | --- |
| `target/` | 12,396,113,920 | Regenerable builds; contains validated executables that must be retained |
| `target/debug/deps` | 9,331,408,896 | Proposed cleanup candidate, only after explicit approval |
| `target/debug/examples` | 2,456,821,760 | Proposed cleanup candidate, only after explicit approval |
| `web/.next/cache` | 154,025,984 | Optional regenerable frontend cache; insufficient alone |
| `postgres-data/` | 8,825,688,064 | Excluded: databases, backups and recovery/evaluation evidence |
| `postgres-data/stage-two` | 6,646,849,536 | Disposable-origin clusters, but retained evidence; not approved for deletion |
| `artifacts/` | 137,154,560 | Excluded: historical validation evidence |
| `~/.npm/_cacache` | 1,753,722,880 | Optional download cache; approval and permission needed outside workspace; offline rebuild tradeoff |
| `~/.npm/_npx` | 2,011,303,936 | Optional cached tool environments; inspect pinned-tool dependencies before approving removal |
| `~/.cargo/registry` | 764,559,360 | Retain for offline builds; do not clear to recover space |
| `~/.cache` | 7,152,168,960 | Mixed application data/toolchains; no blanket cleanup recommendation |
| `/tmp` | 542,625,792 | Mixed processes and other-project validation directories; not verified disposable or safe to remove |

A hard-link-aware inode inventory of **only** `target/debug/deps` and
`target/debug/examples` identified **11,730,452,480 bytes (10.93 GiB)** of regular-file
blocks whose links are all inside those candidates. Another 413,310,976 allocated
bytes have links outside them and are deliberately excluded from that reclaim estimate.
This is an estimate, not space already reclaimed. No files were deleted.

The recommended cleanup proposal for a separate decision is:

1. Stop/confirm absence of active Cargo builds and test processes.
2. Approve removal of only `target/debug/deps` and `target/debug/examples`.
3. Preserve all five top-level executables: `target/debug/supa-api`,
   `supa-runtime`, `supa-migration-admin`, `supa-migration-eleven-admin` and
   `supa-migration-twelve-admin`. Their sizes and SHA-256 hashes are recorded locally.
4. Preserve all Git metadata, source, Cargo registry, databases, backups, dotenv
   files, credentials, wallet files, artifacts and historical evidence.
5. Recheck retained executable hashes and actual free space afterward. These two
   candidates should exceed the capacity gate below if filesystem use stays stable.
   Removing build dependencies will require recompilation for future development;
   it does not invalidate unchanged retained executable bytes.

Do not run blanket `cargo clean`: it would also remove the pinned migration
executable. Its current SHA-256 is
`42c5a4dfb860900beb8e66c02199bc63dec2c82b6e934c71125877f4e05a6ac5`.
Any future rebuild requires renewed executable-digest review before migration
approval; an old signed plan cannot be reused for a changed binary.

### Capacity gate before primary preflight

Use the following conservative **local operational policy**, not a PostgreSQL
vendor minimum or a guarantee against unbounded concurrent disk use:

```text
C = max(primary cluster allocated bytes, primary cluster apparent bytes)
B = size of the most recent verified custom-format dump
required free = max(10 GiB, ceil(1.25 × (2C + 3B + 2 GiB)))
```

`2C` allows additional migration/rewrite space and a separate restoration while
preserving the original cluster. `3B` budgets fresh plaintext, encrypted and
verification copies conservatively. The additional 2 GiB is WAL/temporary-work
headroom, with 25% contingency. The 10 GiB floor also reserves operating-system
headroom. Recalculate when data, retention, migrations or restore placement changes;
stop if projected WAL/growth exceeds this allowance. No Rust/container build should
run concurrently with a capacity-limited database preflight.

Measured `C = 345,739,264` bytes; apparent size was 344,755,786 bytes.
The approved dump is 68,358 bytes (small because this is a fresh development epoch,
not recovered history). Required free space is therefore **10,737,418,240 bytes
(10 GiB)**, with a sampled shortfall of **10,378,567,680 bytes (9.67 GiB)**.
Use the same 10 GiB free-space floor on the independent restore destination if it
will also host the disposable PostgreSQL restoration. Retention of multiple larger
backups requires additional measured capacity; compressed dump size alone is not a
safe restore-space estimate.

**Primary startup/backup/restore preflight should not be approved at current capacity.**
After separately approved cleanup, remeasure before considering that approval.

## Independent backup storage

`lsblk` and `findmnt` show no suitable independently mounted physical disk in the
inspection environment. The only non-loop disk is `nvme0n1`; EFI partition
`nvme0n1p1` shares it and is not an independent backup. Snap loop images, tmpfs,
`/tmp` and repository bind mounts are not independent physical storage. No candidate
was found beneath `/mnt` or `/media/pikoo`. Those mount-parent paths are not writable
in the current execution profile and are not configured private destinations.
An inaccessible/unexposed host mount cannot be claimed as verified storage.

The approved destination remains **unset**, with transfer/upload authorization
false. No external service was contacted or created. Consequently destination
capacity, owner-only access and encryption cannot be marked verified. No outside-disk
backup or outside-disk restoration has occurred.

Required operator action: supply an approved, locally mounted path on independently
controlled physical storage. This milestone does not authorize acquiring resources,
mounting a device, creating a destination or transferring private files. A future
read-only review must run the existing `scripts/independent-backup-preflight.py
--destination /APPROVED/private-supa-backups`, verify owner-controlled 0700 access,
reject symlinks/writable ancestors and same-disk partitions, and measure free space.
The target must remain writable in the approved execution profile. For restoration
there, require at least the capacity gate above, not merely room for the dump.

### Encrypted backup and restore procedure — prepared, not executed

GnuPG is installed; no keyring was initialized, key imported, credential created,
encryption performed or private file copied. The OpenPGP encryption recipient must
be independently verified and capable of decryption under external operator control.
It is distinct from the Ed25519 owner-verification signing keys.

Following separate storage and primary backup/preflight approval:

1. Verify the destination device, capacity and access. Approve retention and an
   independently checked OpenPGP encryption-recipient fingerprint, or separately
   approve encrypted removable storage. Keep decryption material off the backup
   disk and out of the LLM, repository, logs and application environment.
2. Start only the reviewed primary cluster under the existing guarded preflight;
   verify its identity, schema 8, prepared revision 0, read-only settings and unchanged
   accounting/roles. Stop immediately on drift. Take a fresh consistent custom-format
   dump using the existing local socket/admin procedure.
3. Prepare a private, explicitly inventoried evidence bundle containing the dump,
   schema/checksum/epoch/namespace/ACL/audit manifest and required historical quarantine
   evidence. Exclude credentials, dotenv files, keyrings and wallet secrets. Bind
   plaintext hashes and the reviewed migration executable to the approval evidence.
4. Encrypt to the independently verified OpenPGP recipient using a separately approved
   private keyring directory. GnuPG batch encryption must fail if key trust/capability
   cannot be established. Do not bypass trust checks, enable automatic key retrieval,
   put passwords in command arguments, or use overwrite flags. Use a newly named
   destination file with owner-only access; never replace an existing backup.
5. Verify ciphertext hashes at the approved destination. Preserve an authenticated
   manifest binding ciphertext and plaintext hashes, epoch, source and capture time.
   A hash by itself is not proof that a replacement manifest is authentic.
6. The operator decrypts a verification copy under their own key control in a private
   disposable workspace; no private key is supplied to this coding agent or runtime.
   Verify the plaintext hashes before restoring into a separately named socket-only
   PostgreSQL cluster/database. Never restore into the primary database.
7. Verify rows, SQLx checksums, epoch/namespace, quarantine evidence, effective ACLs,
   migration receipts and audit history. Reapply database-level read-only defaults
   (not supplied by a database-only dump). Verify restart fencing and that copied
   operational claims cannot resume. No credential activation or worker startup.
8. Record restore evidence and confirm source/protected hashes unchanged; stop the
   primary and disposable postmasters. Temporary plaintext retention/removal and
   independent-media unmounting need the approved retention procedure. No deletion
   is implied by this document.

Existing same-disk restoration evidence from 8B remains valid for what it tested;
it provides no protection against loss of the NVMe disk. An independent copy and
its restore test remain prerequisites before primary migration unless the operator
explicitly reviews and changes that backup requirement.

## Identity evidence still required

Owner: **KI Daniel**, proposed principal
`6fd863e1-745d-4a06-b68e-eea69e854a69`.
Custodian: proposed principal `e8c6ade3-917f-4fe4-a092-f8a58eccb487`;
its proposal still says `proposed_unverified` and every verification/grant/install
flag remains false. Both proposals use the same display name. Neither that name
nor separate UUIDs establishes independent control.

Missing: an independently verified human-to-principal binding, verification method
and reference for each role, evidence of independent custodian control, independently
anchored public-key fingerprints, the root-controlled onboarding trust record, and
fresh purpose-bound owner/custodian signatures. No root trust record is installed.
The dormant private owner material remains uninstalled and proves no identity.

Exact owner actions for a separate verification ceremony:

1. Confirm the intended owner principal and designate an independently recognized
   verifier using an established out-of-band channel. Agree who independently
   controls custodian issuance authority; do not self-attest separation or mark the
   existing proposal verified. Resolve any ambiguity in the two KI Daniel proposals.
2. Select externally controlled Ed25519 signing keys for each role under a separately
   approved key-management process. Provide only public keys. Never send private keys,
   seeds, recovery phrases, API tokens or identity documents through this chat.
3. Have the independent verifier authenticate the person/role and compare each
   public key's SHA-256 fingerprint over SPKI DER through the established channel.
   Record the method/reference and actual custody evidence; a supplied key or email
   alone is insufficient. Keys must differ, but differing keys alone prove no custody.
4. Separately approve an administrator installing the reviewed public trust anchors
   at `/etc/supa-agent/onboarding-authority.json` with root-controlled ancestors and
   non-group/world-writable permissions. Do not turn a proposal into a trust anchor.
5. At the ceremony, obtain fresh canonical JSON from the verifier: version 1,
   purpose `owner-provisioning-review-only`, exact epoch and both principal UUIDs,
   random UUID nonce, integer issue/expiry timestamps and a maximum 15-minute lifetime.
   No challenge is created now because it would expire before the real ceremony.
6. Owner and custodian each sign the **same canonical bytes** on their own controlled
   systems. Return only public keys, challenge and detached signatures to the verifier.
   Signatures prove key control, not human identity or permission to activate.
7. Run the existing `verify-owner-evidence.py` review with the independently installed
   trust record. Reject missing, mismatched, expired or altered evidence. Valid proof
   produces no credentials, grants, migration approval or execution authorization.
8. Obtain a new provisioning approval for exact role/login/credential UUID/expiry
   bindings. The trusted administrator must consume the nonce once in the immutable
   provisioning audit and retain the evidence digest; repeated proof review never
   grants access. Issuer credentials remain independent of runtime executor credentials.

## Offline baseline and migration integrity

The protected offline check passed again: primary stopped; 26 protected hashes
unchanged. This is not a fresh SQL inspection. The last verified SQL baseline is:

| Field | Reviewed value |
| --- | --- |
| Database | `supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748` |
| Epoch | `fd2f52e0-33fc-4c0d-893a-7b08b26ab748` |
| Accounting namespace | `641457b2-5fd8-4b31-b85f-c58839626831` |
| State/revision/schema | `prepared` / `0` / `8` |
| Original historical database | Unrecovered |
| Quarantined budget | `ad91e475-f1da-495a-8247-139643d2d8b1` |
| Archived ceiling / unresolved hold | $0.10 / **$0.004048**; no reuse, refund or settlement |

The approved 68,358-byte dump remains mode 0600 and has SHA-256
`600d2bb19b52b4f17d4278459ba722ea5f1135df703b8efc1b0b3db0c8360671`.
SHA-384 checksums of migrations **1–8 match the protected primary-backup manifest**;
all **1–12 are byte-identical to reviewed 8B commit
`8eead0c936cea63cf121975d4642b940eb2ee187`**. No migrations were edited or executed.

Future migration remains the dedicated independently signed **8→12** profile,
including 9–12, with exact database/epoch/namespace, expected revision/schema,
backup/executable digests, expiry and scope. Existing schema-12 disposable tests,
rollback-on-interruption and restoration checks were not repeated in this milestone.
After a successful commit, there is no approved in-place down migration: preserve
migrated evidence and restore a verified backup into a separate database under a new
rollback approval, or review an additive forward fix. Never overwrite primary state
or replay old operational claims after restoration/restart.

## Separate approval checklist

1. **Cleanup:** approve only the named build-output candidates and retained-binary
   hash safeguards. Remeasure; require at least 10 GiB free before primary startup.
2. **Backup/storage and primary preflight:** designate and verify independent storage,
   encryption recipient and retention; explicitly approve the existing-cluster
   identity check, fresh dump, encrypted transfer, isolated restoration and shutdown.
   This approval authorizes no migrations, credentials, grants or agents.
3. **Primary migration:** only after backup verification, approve the exact independently
   signed 8→12 migration plan. No credential provisioning or epoch execution follows
   automatically. Stop on any baseline, checksum or identity mismatch.
4. **Provisioning:** only after independent owner/custodian evidence, approve the exact
   trust/binding/role/credential-expiry installation. Custodian issuance authority and
   runtime credentials are separately controlled; no blanket execution permit.
5. **Offline activation:** separately approve agent creation/ownership, finite offline
   control (goals, simulated seed/budget, interval, expiry and ceilings), then the
   unused execution admission and host. Test pause/revocation/emergency stop and
   stop at the ceiling. No renewal, paid inference, signing or financial permission.

Independent identity evidence is required before provisioning; it is not a reason
to grant access during a backup-only preflight. At current disk capacity **primary
preflight is not yet safe to recommend for approval**. After approved cleanup and
storage review, it can be reconsidered as its own bounded operation. Migration,
provisioning and offline execution remain separate later decisions.

## Evidence and work performed

Local ignored evidence: `artifacts/milestone-8c/4354582a203d/inspection.json` and
`retained-binaries.json`. They contain capacity, checksum and executable-hash
metadata; no credentials or database contents are committed. Read-only discovery
used `df`, inode counts, `du`, `lsblk`, `findmnt`, file metadata, hashes and the
existing stopped-primary guard. No independent destination could pass permission
or capacity verification because none exists in the visible environment.

Resolved here: storage topology and capacity uncertainty, exact cleanup proposal,
capacity gate, encryption/restore procedure and owner action list. **Actual free
space, independent storage and independent identity verification remain operator
blockers.** No already-passed Rust/frontend/browser suites or disposable SQL
rehearsals were repeated. The only tracked change is this readiness report; normal
one-file commit hooks, secrets scanning and the complete main-history audit apply.
