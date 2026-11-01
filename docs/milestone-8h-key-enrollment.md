# Milestone 8H — Secure recovery-key enrollment handoff

**Status: waiting for human key custody and independent verification.** No real or
disposable key was generated in 8H, no key home was inspected, and no fingerprint
was enrolled. This is the human terminal handoff, not a completed key enrollment.
Read the [8G design and tested limits](milestone-8g-encryption-readiness.md) first.

## Checks completed

- Working branch `main`; no initial working-tree changes.
- Installed GnuPG, agent and gpgv: `2.4.4-2ubuntu17.6`; libgcrypt `1.10.3`;
  terminal pinentry `1.2.1-3ubuntu5`. Version/help inspection required no key access.
- Installed options include `--force-ocb`, `--cipher-algo`, interactive key/subkey
  generation, public-key inspection and secret-key export. These match the prepared
  AES-256/OCB, Ed25519 and cv25519 profile. Official
  [key-management syntax](https://www.gnupg.org/documentation/manuals/gnupg/OpenPGP-Key-Management.html)
  and [OCB options](https://www.gnupg.org/documentation/manuals/gnupg/OpenPGP-Options.html)
  were checked. Exact compatibility on an external recovery workstation is pending.
- Reviewed the existing 8G evidence: **15 passed synthetic tests**, with only loopback
  and zero IPv4 routes. Those are prior tests, not tests of the owner's future keys.
  The helper/test source remains unchanged. No key-generating suite was rerun in 8H.
- Offline primary guard passed: stopped cluster, all 26 protected files and approved
  Stage 1 archive intact. Schema 8 / prepared epoch are last SQL-verified evidence;
  no SQL connection was made here. Historical database remains unrecovered and the
  unresolved **$0.004048 hold** is unchanged.

The four approved fingerprint fields remain null in
[`config/railway-backup-plan.example.json`](../config/railway-backup-plan.example.json).
Recovery-copy status is unverified, and all real-operation permissions remain false.

## A. Human confirmation checkpoint

Before running the commands below, the owner must choose:

1. A separate trusted workstation with maintained GnuPG and private encrypted working
   storage. Confirm its AES-256/OCB and Ed25519/cv25519 support and run the 8G synthetic
   fixture there under the local security policy. No real private key belongs on
   this Supa Agent Ubuntu machine, Railway or a model-connected terminal session.
2. Two independently stored encrypted recovery-media locations. Two directories or
   partitions on one disk do not count. Verify actual mounts/devices, private access,
   capacity and offline accessibility; leave neither sole copy on the workstation.
3. The encryption controller, separate signing identity/controller and independent
   authorized fingerprint reviewer. Two certificates do not by themselves establish
   independent human custody or ownership. Application owner/custodian verification
   and grants remain separate from this key-enrollment process.
4. An explicit expiry/renewal policy. The commands use `1y` as a template only;
   select it deliberately and renew before expiry. Keep old decryption keys and
   historical public/revocation evidence. The current helper rejects expired keys;
   historical validation needs separate review, never an ignore-expiry shortcut.

**Stop here until machine, storage and review arrangements are confirmed.** Codex
does not execute the following private-key commands. An owner choosing to run them
does so directly in an unrecorded local terminal on the separate workstation.

## B. Interactive generation on the separate workstation

Replace `/PRIVATE` with the reviewed encrypted working-storage path. Use new key
homes and no shared, cloud-synced, wallet or application-credential directories.
Never type passphrases into chat, flags, environment variables or scripts. Use a
different strong, long passphrase for each identity through trusted pinentry.
Do not accept an empty-passphrase override; do not use batch/test-fixture settings.

```sh
umask 077
export GPG_TTY="$(tty)"
install -d -m 700 /PRIVATE/supa-encryption /PRIVATE/supa-signing /PRIVATE/review
gpg --no-options --homedir /PRIVATE/supa-encryption --pinentry-mode ask \
  --quick-generate-key 'Supa Backup Encryption' ed25519 cert 1y
gpg --no-options --homedir /PRIVATE/supa-encryption \
  --fingerprint --fingerprint --list-keys
```

Read and record the **complete encryption primary fingerprint** privately. Replace
the uppercase placeholder below with that public fingerprint, not a short key ID:

```sh
gpg --no-options --homedir /PRIVATE/supa-encryption --pinentry-mode ask \
  --quick-add-key FULL_ENCRYPTION_PRIMARY_FINGERPRINT cv25519 encr 1y
gpg --no-options --homedir /PRIVATE/supa-signing --pinentry-mode ask \
  --quick-generate-key 'Supa Backup Manifest' ed25519 cert 1y
gpg --no-options --homedir /PRIVATE/supa-signing \
  --fingerprint --fingerprint --list-keys
gpg --no-options --homedir /PRIVATE/supa-signing --pinentry-mode ask \
  --quick-add-key FULL_SIGNING_PRIMARY_FINGERPRINT ed25519 sign 1y
```

The expected structure is one certification-only Ed25519 primary and one cv25519
encryption subkey in the encryption home, plus a **different** certification-only
Ed25519 primary and Ed25519 signing subkey in the signing home. No wallet import.
Use the same explicit-home listing command for each home to display all fingerprints:

```sh
gpg --no-options --homedir /PRIVATE/supa-encryption --with-colons \
  --with-fingerprint --with-subkey-fingerprint --list-keys
gpg --no-options --homedir /PRIVATE/supa-signing --with-colons \
  --with-fingerprint --with-subkey-fingerprint --list-keys
```

An existing dedicated certificate can be selected instead only after equivalent
capability, separation, expiry, revocation, custody and independent review. Import
of any existing private key is a separate human decision on the external machine;
do not discover, import or repurpose personal keys automatically.

## C. Public-only handoff and independent fingerprint checks

After generation, export **public certificates only**, with new output paths:

```sh
gpg --no-options --homedir /PRIVATE/supa-encryption --armor \
  --output /PRIVATE/review/supa-encryption-public.asc \
  --export FULL_ENCRYPTION_PRIMARY_FINGERPRINT
gpg --no-options --homedir /PRIVATE/supa-signing --armor \
  --output /PRIVATE/review/supa-signing-public.asc \
  --export FULL_SIGNING_PRIMARY_FINGERPRINT
sha256sum /PRIVATE/review/supa-encryption-public.asc /PRIVATE/review/supa-signing-public.asc
```

Stop on an existing output; do not overwrite older evidence or keys. Share only
the public certificates and public fingerprint strings for review. Never share a
secret-key export, key-home archive, revocation material or passphrase with Codex.
The public handoff must contain a public certificate, not a file merely named public.

The reviewer inspects the public certificates on a separately trusted system, with
an isolated private inspection home and no automatic retrieval/import. For example:

```sh
gpg --no-options --homedir /PRIVATE/public-inspection --no-auto-key-retrieve \
  --no-auto-key-import --auto-key-locate clear --with-colons \
  --with-fingerprint --with-subkey-fingerprint --show-keys \
  /PRIVATE/review/supa-encryption-public.asc
```

Create that inspection home with mode `0700` first and repeat for the signing public
certificate. This inspection does not establish trust or install ownertrust.

Four checks must be independently recorded:

| Check | Required evidence | Current result |
| --- | --- | --- |
| Encryption primary | Full fingerprint, Ed25519 certification capability, controller identity/control, valid dates and no revocation | Pending |
| Encryption subkey | Full fingerprint, cv25519 encryption capability, valid primary binding, dates and no revocation | Pending |
| Signing primary | Different full fingerprint, Ed25519 certification capability, controller identity/control, valid dates and no revocation | Pending |
| Signing subkey | Full fingerprint, Ed25519 signing capability, valid primary binding, dates and no revocation | Pending |

Compare all four through an established independent channel, such as a previously
verified call or in-person review. Obtain controller identity/control confirmation
and reviewer evidence references. Compare certificate hashes too, so an amended
certificate is not silently substituted. Seeing the same fingerprint twice on
one machine is **not independent verification**. UIDs, names and proposed owner
records are not identity proof. Do not trust a keyserver or an unsolicited contact
merely because it supplies matching labels.

## D. Protected recovery copies, controlled by the owner

On the external workstation only, verify that `/MEDIA_A` and `/MEDIA_B` are actually
mounted independent encrypted media, not fallback directories on its local disk.
Use private directories enforcing `0700`/`0600`, new filenames, and a separate
location for passphrase recovery. Check the media identity immediately before export.
The exported secret packets must remain passphrase-protected; encrypted media adds
another protection layer. Neither Railway nor this Supa Agent host receives them.

For each approved medium, export both complete certificates directly to that medium:

```sh
gpg --no-options --homedir /PRIVATE/supa-encryption --pinentry-mode ask \
  --output /MEDIA_A/supa-encryption-secret.pgp \
  --export-secret-keys FULL_ENCRYPTION_PRIMARY_FINGERPRINT
gpg --no-options --homedir /PRIVATE/supa-signing --pinentry-mode ask \
  --output /MEDIA_A/supa-signing-secret.pgp \
  --export-secret-keys FULL_SIGNING_PRIMARY_FINGERPRINT
```

Repeat with the separately verified `/MEDIA_B`. Do not put a sole key copy on one
hardware device. Preserve the controlled `openpgp-revocs.d` recovery material,
approved public certificates, fingerprints and review records on the encrypted
media as well. Revocation certificates can disrupt future use; keep them controlled,
not in Git or chat. Agree who can revoke/renew each identity. Do not revoke anything
during enrollment.

Verify recovery copies on another approved isolated workstation using **synthetic
messages only**, with fresh `0700` homes and no original key homes or cached agent
passphrases. Secret import is done by the owner outside model-connected tools.
Prove that canceling the passphrase prompt cannot decrypt/sign, then prove success
through trusted interactive pinentry. A file hash or successful import alone is
not proof of passphrase protection or usable recovery. Restore from each media copy
and record only a sanitized witness result, public fingerprints and fixture hashes.

The later rehearsal must verify externally signed canonical manifests and ciphertext
hashes before decryption, require successful AES-256/OCB decryption and synthetic
plaintext equality, and reject wrong signatures, wrong pins and changed data.
The signer private key cannot substitute for the encryption recovery key; public
signing material suffices to verify old manifests. Real database data is unnecessary.

## E. Enrollment and proposed backup inventory

After the owner completes B–D, the public-only phase can inspect supplied public
certificate paths, compare the four independently attested pins, capabilities and
expiry/revocation, and run synthetic verification. Record the reviewer reference,
controller/custody confirmation, certificate hashes, review date, expiry/renewal
decision and recovery witness results in private local evidence. Then obtain
explicit approval to enroll those exact public values. A receipt supplied by the
requester is not independently trusted automatically.

Do not populate configuration merely from chat strings. Until the independent
review and enrollment decision, these fields remain null:
`approved_recipient_fingerprint`, `approved_encryption_subkey_fingerprint`,
`approved_manifest_signer_fingerprint`, `approved_manifest_signing_key_fingerprint`.
Even after enrollment, backup export, bucket credentials, upload and restoration
require their own authorizations; public-key enrollment does not grant application
ownership or activate agents.

Prepared locally, without copying/exporting source data:
`artifacts/milestone-8h/proposed-backup-inventory.json`, mode `0600` in `0700` directory.
It enumerates **32 exact existing files**, **388,820 source bytes**, each with an
archive-member path and SHA-256. The reviewed candidate comprises:

- Existing Stage 1 dump, manifest and verification report; no fresh database extraction.
- Prepared epoch and cluster identity plus reviewed PostgreSQL configuration for
  recovery reference, not automatic installation on a new host.
- Historical accounting/quarantine evidence, preserved separately from live balances.
- Migrations **1–8 only**, matching the archived baseline; no migration execution.
- Existing crypto/test/preflight recovery source and reviewed recovery reports.

Inventory SHA-256 at preparation:
`e1b272a04fba31dd749e59d057c1720d512ef9494838f7d992e1a72b4ef7d7fe`.
This private inventory is unsigned proposed membership, **not an approved or
authenticated backup manifest**. Sensitive-content review/export approval remains
pending. Rehash before approval and stop on drift. It excludes dotenv files, pending
credentials, private keys, wallets, Git metadata and unrelated artifacts. Approved
public certificates, sanitized enrollment evidence and generated internal manifest/
restore instructions must be listed in an amended inventory after enrollment.
It is a database-recovery candidate, not a complete application-source/runtime kit;
that broader recovery kit needs its own reviewed inventory and approval.

The exact target remains the provisioned private `supa-agent-backups` bucket in
`staging`, Amsterdam. No Railway credential request or resource modification occurred
in 8H. No backup was encrypted, signed or uploaded, and no plaintext restore was run.
The $1 monitoring policy and 23-snapshot target are unchanged. Historical $0.004048
remains quarantined; no refund, settlement, reuse or reconstruction is authorized.

## Stop condition and remaining decisions

This milestone is paused at the **human private-key setup checkpoint**, not marked
complete enrollment. Needed from the owner: confirmation of the separate workstation,
two independent encrypted storage locations, controller/reviewer arrangement and
chosen expiry policy. Later provide **public material only**, independent review
references and sanitized recovery-copy confirmation. Never provide private keys,
passphrases, seed phrases or personal wallet material.

No keys or recovery copies were available for actual enrollment verification, so
there are **zero verified production fingerprints**, no new real-key synthetic
test result and no real recovery-copy rehearsal result. Prior 8G synthetic evidence
remains the applicable automated validation. The helper is still capped at 16 MiB,
and real packaging/upload orchestration remains separately gated.

Before real backup work: independently approve and enroll public keys, review exact
export membership, separately authorize bucket credential access, then approve
encryption/signing/upload verification and a disposable restore. Fresh primary
startup needs separate approval. No migration, activation, deployment or push occurs
under this key-preparation milestone. Git secrets/history checks accompany this
single report commit; private inventory and evidence remain ignored.
