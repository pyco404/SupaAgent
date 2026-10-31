# Milestone 8G — Backup encryption, manifest authentication and key readiness

Preparation and synthetic cryptographic tests completed. **No real backup was
encrypted, signed, uploaded or restored. No real key or fingerprint is approved.**
The primary PostgreSQL cluster remains stopped; existing data and historical
accounting evidence remain protected. This milestone does not activate credentials,
issue ownership grants or authorize further infrastructure changes.

## Infrastructure recheck

Read-only Railway GraphQL metadata and `bucket info --json` reconfirmed the
[provisioning report](milestone-8f-bucket-provisioning.md):

| Resource | Identity |
| --- | --- |
| Workspace | Daniel 's Projects — `e319f68a-9897-49ed-b119-743f1f846aa0` |
| Private project | Supa Agent — `99648d2a-69a1-4d8c-98d0-7f2a26543c9c` |
| Sole environment | staging — `ff0b2092-e42b-4a89-ae25-441ab95d92d8` |
| Private bucket | supa-agent-backups — `76923b39-b3a7-4390-8a0b-783d8a8113e9`, `ams` |
| Stored data / services | 0 objects, 0 bytes, 0 application services |

No resource was changed, no S3 credentials were requested and no object requests
were made. Private access remains Railway's private-bucket model, with the project
API's private flag verified; anonymous object access was not tested. Actual S3 name,
endpoint, signing region and access keys remain unretrieved.

## Encryption and signing design

Selected preparation profile: **`gnupg-openpgp-aes256-ocb-v1`**. GnuPG performs
public-key encryption using a dedicated Curve25519 ECDH encryption subkey and
**AES-256/OCB authenticated encryption**. A separate Ed25519 certificate/signing
subkey signs a canonical ciphertext manifest with SHA-256 or stronger SHA-2.
Certification primaries and operational subkeys have separate full fingerprints.
No wallet keys, application tokens or owner-verification keys are reused.

Installed tooling is **GnuPG 2.4.4**, Ubuntu package **2.4.4-2ubuntu17.6**,
libgcrypt **1.10.3**. `--force-ocb` is supported locally and documented in the
[GnuPG OpenPGP options](https://gnupg.org/documentation/manuals/gnupg/OpenPGP-Options.html).
The helper requires GnuPG status confirming cipher 9 / AEAD 2 (AES-256/OCB), not
just a configured flag. Synthetic decryption independently confirmed the same mode.
This is a tested GnuPG OCB profile, **not a claim of universal OpenPGP/RFC 9580
interoperability**. Run the fixture on the independently controlled recovery machine
and validate its exact tool version before approving real backup encryption.

The [offline helper](../scripts/backup-crypto.py):

- Uses explicitly supplied owner-only `0700` homes with no symlink ancestry;
  ignores ambient configuration and inherited cloud/GPG/application settings.
- Requires public-only encryption/verifier homes, exact full primary/subkey pins,
  usable capabilities and nonexpired/nonrevoked certificates. This preparation
  profile accepts the specified 255-bit EdDSA/ECDH algorithms only.
- Disables automatic key retrieval/import and ambient recipients. Only after exact
  pin checking does it give GnuPG a per-invocation certificate trust anchor;
  it never installs ownertrust or uses broad `--trust-model always`.
- Forces OCB, AES-256 and no compression, suppresses original embedded filenames,
  checks process exit and encryption status, and bounds in-memory plaintext to
  **16 MiB**. Larger sources need separately reviewed streaming orchestration.
- Authenticates a bounded canonical manifest using a public-only verifier, pinned
  primary and signing key, SHA-2 and binary detached signature. Expired/revoked/error
  signature statuses fail closed. It verifies ciphertext length and SHA-256 before
  any decryption or restoration may occur.
- Requires the expected UUID and key bindings from an independent approved
  inventory; it does not accept a replacement snapshot as the expected one.

**Fingerprint parameters are cryptographic pins, not proof of operator consent.**
There is no production sealing/signing/decryption/upload CLI. Running the helper
without arguments only reports its preparation status. The test harness invokes
its bounded library primitives with synthetic fixtures. Real orchestration and
trust installation remain separate approval boundaries.

GnuPG can emit provisional plaintext before a final authentication failure.
Never pipe decryption directly into `pg_restore`, and never accept `DECRYPTION_OKAY`
alone. Require the externally pinned manifest signature/hash first, complete
decryption exit success and explicit OCB status, then verify internal plaintext
inventory before opening a restore transaction. Upstream
[2.4.6 release notes](https://dev.gnupg.org/T7030.html) include a fix concerning OCB
status reporting; the installed Ubuntu package has security backports, but that
specific fix was not established by the package changelog review. The preparation
therefore requires independent hashes and successful exit codes, and actual corrupt/
truncated fixtures returned nonzero. Check maintained package/security support and
rehearse the approved recovery toolchain before real use; no software upgrade was run.

## Exact human key actions — NOT executed

First approve the dedicated key-generation/import plan and independent controllers.
Perform these steps on a trusted **separate recovery workstation**, preferably on
encrypted storage with an offline or hardware-backed custody arrangement. Do not
run them on this primary Ubuntu host, in Railway or through Codex. Avoid terminal
recording, shell tracing and automatic cloud sync of key directories.

1. Create two private key homes on that workstation. The following are templates;
   replace `/PRIVATE` with reviewed encrypted storage, not a public or shared path:

   ```sh
   umask 077
   install -d -m 700 /PRIVATE/supa-encryption /PRIVATE/supa-signing
   gpg --no-options --homedir /PRIVATE/supa-encryption --quick-generate-key \
     'Supa Backup Encryption' ed25519 cert 1y
   gpg --no-options --homedir /PRIVATE/supa-encryption --fingerprint --fingerprint
   ```

   Set a strong passphrase through the trusted interactive pinentry. Do not put it
   in flags, environment, scripts or chat. Never use the test harness's empty
   passphrase or unprotected fixture-key settings for real keys.
2. Read the complete encryption primary fingerprint and use it to add the exact
   encryption subkey:

   ```sh
   gpg --no-options --homedir /PRIVATE/supa-encryption --quick-add-key \
     FULL_ENCRYPTION_PRIMARY_FINGERPRINT cv25519 encr 1y
   gpg --no-options --homedir /PRIVATE/supa-signing --quick-generate-key \
     'Supa Backup Manifest' ed25519 cert 1y
   gpg --no-options --homedir /PRIVATE/supa-signing --fingerprint --fingerprint
   gpg --no-options --homedir /PRIVATE/supa-signing --quick-add-key \
     FULL_SIGNING_PRIMARY_FINGERPRINT ed25519 sign 1y
   ```

   Verify both primary and subkey fingerprints, expiry, role capabilities and
   algorithms. Use `--with-colons --with-fingerprint --with-fingerprint --list-keys`
   with each explicit home for machine-readable review. Select expiry/rotation
   deliberately; this template's one-year lifetime is not an approved policy.
   Preserve old decryption keys and historical public/revocation/trust records when
   rotating. Expiry does not erase old ciphertext, but this helper deliberately
   rejects expired verification certificates. Renew/review before expiry; recovery
   with expired or revoked certificates needs a separately reviewed historical
   signature policy, not an ignore-expiry flag or automatic trust bypass.
3. Export **public certificates only** to a private review directory, with explicit
   output files. For example, after confirming the full primary fingerprint:

   ```sh
   gpg --no-options --homedir /PRIVATE/supa-encryption --armor \
     --output /PRIVATE/review/supa-encryption-public.asc \
     --export FULL_ENCRYPTION_PRIMARY_FINGERPRINT
   gpg --no-options --homedir /PRIVATE/supa-signing --armor \
     --output /PRIVATE/review/supa-signing-public.asc \
     --export FULL_SIGNING_PRIMARY_FINGERPRINT
   ```

   Create the private review directory first; use new paths and never overwrite an
   existing export. An existing dedicated certificate may be selected instead, but
   must satisfy the same capabilities, separation, validity and independent checks.
4. Have an independent authorized reviewer compare **all four full fingerprints**
   through an established separate channel, verify controller identity and actual
   key control, and record the approved role, expiry, certificate hash and evidence
   reference. A display name, certificate UID, short key ID or fingerprint delivered
   alongside an unverified certificate is insufficient. Importing a public key
   alone is not trust. Do not designate the synthetic test fingerprints as trusted.
5. Under explicit approval, preserve protected secret-key exports and revocation
   material on **two independently controlled offline encrypted media locations**.
   Keep passphrase recovery separate from the media and from Railway credentials.
   An external controller can export to a reviewed destination with:

   ```sh
   gpg --no-options --homedir /PRIVATE/supa-encryption \
     --output /APPROVED_OFFLINE_MEDIA/supa-encryption-secret.pgp \
     --export-secret-keys FULL_ENCRYPTION_PRIMARY_FINGERPRINT
   ```

   Repeat for the separate signing certificate in its own approved custody path.
   Verify passphrase protection and backup completeness externally. The output is
   secret even when protected; never send it to Codex, upload it to Railway or
   stage it in this repository. For hardware-backed keys, prove an independent
   recovery route before relying on the device; a nonexportable device cannot be
   treated as a backup of itself. Preserve revocation certificates as controlled
   material and document who may use them.
6. Test recovery on another isolated machine using disposable messages before
   approving real backup use. Public-certificate import into an explicitly isolated
   public-only encryption/verifier home is permitted only after fingerprint review.
   Secret-key import occurs solely on the approved external recovery machine, under
   human control. Recheck full fingerprints after import; do not trust filenames.

Required production pins are currently **all unassigned**:

| Role | Configuration field |
| --- | --- |
| Encryption primary | `approved_recipient_fingerprint` |
| Encryption subkey | `approved_encryption_subkey_fingerprint` |
| Signing primary | `approved_manifest_signer_fingerprint` |
| Signing subkey | `approved_manifest_signing_key_fingerprint` |

The four fields must be independently approved; values observed in tests are not
substitutes. Owner/custodian application identity verification remains separate.

## Backup format, retention and verification procedure

Proposed complete snapshot, with a fresh canonical random UUID per capture:

```text
snapshots/<uuid>/<uuid>.pgbundle.gpg
snapshots/<uuid>/manifest.json
snapshots/<uuid>/manifest.sig
```

The encrypted plaintext bundle will contain the approved consistent custom-format
dump, internal SHA-256 inventory, snapshot provenance, schema/SQLx checksums,
epoch/accounting namespace, ACL/read-only recovery templates and separately
quarantined historical accounting evidence. No live credentials, wallet material
or unrelated artifact directories belong in the export inventory. Archive creation
and path-safe extraction are not implemented by the crypto helper and require a
reviewed future wrapper. Bounded extraction must reject traversal, symlinks, duplicate
paths and unexpected members before invoking PostgreSQL tools.

The **outer manifest** contains only the UUID, opaque ciphertext filename, byte
count, ciphertext SHA-256, capture time, profile and public-key fingerprint bindings.
It has a strict field schema, bounded size, duplicate-field rejection and canonical
JSON encoding (sorted keys, compact separators, ASCII escaping, final newline).
An external signing controller signs those exact bytes with the approved signing
subkey. Detailed database inventories remain encrypted. The public verification
certificate and trusted snapshot inventory must be available outside the bucket.

After separate approval, upload only ciphertext and authenticated manifests under
a new UUID prefix. No overwrites or mutable `latest` pointer. Treat incomplete sets
as incomplete. Download the ciphertext after upload and verify its full SHA-256
against the externally pinned signature; an S3 ETag is not a substitute. Metadata
and credential access, anonymous-denial checks and actual S3 round trips remain
unperformed here. Future signing/upload/restore orchestration is still required.

Retention remains **7 daily + 4 weekly + 12 monthly = 23 snapshot sets**, with no
schedule or automatic deletion enabled. Unique names do not provide object locks.
Bucket-wide credentials can still overwrite/delete data; Railway has no versioning,
object-lock or lifecycle protection. Keep an independently authenticated inventory
and consider a separately approved offline copy. Apply the $1 monitoring policy;
retained data continues billing when new uploads stop. Deletion or budget increases
require separate explicit approval.

For a disposable restore, first verify the exact approved snapshot and fingerprints,
signature, ciphertext length/hash and completed AEAD decryption. The external key
controller uses a private encrypted workspace and checks internal hashes before
extracting the approved bundle. Restore only into a fresh socket-only PostgreSQL 16
cluster, never over the primary. Compare schema 8, migrations 1–8/checksums, row/state
and catalog evidence, ACL/read-only defaults, prepared epoch and accounting namespace.
Preserve the separate historical hold evidence without importing it as live balance.
Verify restart fencing, stop the disposable server and retain sanitized drill results.
Review plaintext/swap handling: unlinking or shredding on an SSD does not guarantee
erasure. Key disposal on an approved encrypted temporary volume is preferable.

## Recovery after losing the Ubuntu machine

Recovery requires independently accessible encrypted snapshots, pinned signing
public certificate and snapshot inventory, encryption secret primary/subkey backup
and passphrase, Railway account/2FA recovery, reviewed source/migration/toolchain
recovery kit, and the restore instructions. Preserve them outside the failed disk;
do not rely on an unpublished local Git commit or a sole hardware key. Reacquire
bucket access under the separate approved credential process. Manifest verification
does not require the signing private key, and the signing key cannot decrypt backups.

Restore first into isolated read-only state, revalidate evidence, and seek fresh
credential/cutover approvals. Do not resume copied agent leases, reuse old operational
authorizations or refund accounting reservations automatically. This design protects
against local disk loss; a single Railway account/provider remains an additional
failure/deletion domain until a separate approved copy exists elsewhere.

## Existing-script coverage and limitations

| Existing/prepared component | Capability and boundary |
| --- | --- |
| `preflight-development-epoch.py` | Reviewed consistent dump/restoration and invariants; starts primary, so not executed here and requires separate approval. |
| `independent-backup-preflight.py` | Local physical-disk independence/permissions; does not validate cloud storage or upload. |
| `backup-crypto.py` | New offline public-key encryption and manifest/ciphertext verification primitives, 16 MiB limit; no real-data execution CLI. |
| `test-backup-crypto.py` | Disposable key generation, external-role signing, corruption rejection and recovery rehearsal, synthetic data only. |
| Real archive/sign/upload/decrypt/restore orchestration | Not enabled or approved. Must bind real independently verified pins, exact export inventory and operation-specific approvals before use. |

## Validation and remaining approvals

Network-free tests ran inside `unshare --user --map-root-user --net`; temporary
GnuPG agents used Unix sockets only. **15 tests passed**: actual AES-256/OCB round
trip; ciphertext corruption/truncation; altered manifest; wrong/bad signatures;
short/wrong pins; wrong decryption key; snapshot/path substitution; duplicate,
unknown/noncanonical JSON; signed downgrade/limit rejection; role/capability and
revoked/expired metadata rejection; private-home/secret-material rejection;
randomized encryption; fresh-home recovery; actual expired certificate; ignored
ambient recipient configuration. Revocation metadata rejection uses synthetic
listing records, not a real operator-key revocation exercise. Test homes/agents
were cleaned up; no real secrets or private test keys enter logs or Git.

An initial ordinary-sandbox invocation failed during disposable fixture key
generation; the completed suite used private GnuPG agents in the isolated network
namespace. The final isolation check observed only loopback and zero IPv4 routes.
Sanitized results are in `artifacts/milestone-8g/synthetic-crypto-tests.json` (ignored,
mode `0600`). Reproduce with `unshare --user --map-root-user --net -- python3
scripts/test-backup-crypto.py`; local policy must permit temporary Unix sockets and
user/network namespaces. Do not substitute real key homes if isolation is unavailable.

Local offline protection checks passed, preserving all 26 protected files, approved
backup hash, stopped primary and migration-8/prepared evidence. Historical database
is still **unrecovered**, including quarantined budget
`ad91e475-f1da-495a-8247-139643d2d8b1` and its unresolved **$0.004048 hold**. No SQL
comparison or real database restoration was performed in this milestone. Free space
was about **10.42 GiB** at inspection; retain the 10 GiB recovery reserve and recheck
before any larger operation. No frontend/backend build was needed or performed.

Remaining approvals, separately scoped:

1. Human generation/selection of dedicated real keys, independent verification of
   all four full fingerprints, external recovery copies and exact trust installation.
2. Real bucket metadata/credentials retrieval and secure storage/connectivity tests.
3. Exact backup/export inventory and reuse of the consistent Stage 1 snapshot, or
   separate primary-startup approval for a fresh consistent snapshot.
4. Reviewed real-data packaging, encryption and external signature workflow, then
   upload verification and an isolated decryption/disposable database recovery drill.

No protected operation follows automatically from successful synthetic tests.
Production pins remain null, all operation-authorizing flags remain false, and no
push, deployment, paid inference, worker activation or financial activity occurred.
