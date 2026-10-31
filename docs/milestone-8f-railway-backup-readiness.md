# Milestone 8F — Railway encrypted backup and recovery preparation

Historical preparation report. The subsequently approved project/staging/bucket
creation and current identifiers are recorded in
[the provisioning report](milestone-8f-bucket-provisioning.md). The remaining
encryption, credential-access, upload and recovery approvals are still outstanding.

## Outcome and approved planning choices

Preparation completed; **no independent cloud backup has been established yet**.
Railway authentication and read-only inventories succeeded. The operator selected
a **new dedicated project, `Supa Agent`**, with separate staging and production
environments. Neither existing project may be reused. The project is not created
or accessible in the inspected account. No project ID exists. Read-only account
verification identifies the candidate existing workspace as **`Daniel 's Projects`**,
ID **`e319f68a-9897-49ed-b119-743f1f846aa0`**; operator confirmation is still required.
The planned bucket display name is `supa-agent-backups`, placement region
**Amsterdam (`ams`)**. This approves the planning region, not resource creation.

The operator has selected a **$1/month bucket-only monitoring and approval ceiling**,
excluding Railway plan fees and unrelated services. No verified bucket-specific hard
limit is claimed. OpenPGP recipient, manifest signer and key-custody arrangements
remain unassigned. All execution permissions in
[`config/railway-backup-plan.example.json`](../config/railway-backup-plan.example.json)
are false. This is a review document, not runtime configuration or an authorization
record. The existing local backup policy and primary service configuration were
not changed. The website and Rust runtime remain unchanged.

## Railway verification and proposed structure

The official CLI was absent from PATH. Version **5.71.0** was downloaded to
`/tmp/supa-milestone-8f-cli/railway` without an installer or agent setup. Its release
archive SHA-256 matched GitHub release metadata:
`856dde75501fc1088065e5d980141ddc2340be81cfcef5ece8cd0e947ff9d709`.
Existing local authentication configuration is `0600`. No login, credential
retrieval, reset or installation was performed; account identifiers/tokens were
not printed. Read-only `whoami --json` and `list --json` succeeded.

Explicit-project/environment `status --json` requests inspected:

| Accessible project | Environment | Bucket inventory |
| --- | --- | --- |
| Gamine | production | Empty |
| agentic-economy | staging | Empty |
| agentic-economy | production | Empty |

These results establish the visible inventory at inspection time, not access to
every possible workspace. These projects remain excluded. Before any creation,
repeat discovery in the operator-approved workspace to avoid a duplicate Supa Agent
project and repeat bucket discovery in the exact approved environment. Existing
staged changes must be inspected; do not commit unrelated changes while adding a bucket.

Proposed structure, with nothing provisioned:

```text
Daniel 's Projects (e319f68a-9897-49ed-b119-743f1f846aa0; confirmation pending)
  Supa Agent (new dedicated project)
    staging
      supa-agent-backups: private bucket instance, ams (first proposed backup target)
      frontend / API / paused worker / PostgreSQL: deferred, separate deployment approval
    production
      separate bucket instance / credentials only if later explicitly approved
      all application services deferred
```

The initial resource scope is **staging backup infrastructure only**. Production
remains a future separation plan, not permission to create production resources.
The staging backup target is planned, not authorized for creation. Creating or
duplicating environments can create separate bucket instances: do not duplicate
resources, credentials or local development records into production. No remote
database or running service is needed for this backup-only stage. Do not apply the
existing four-service IaC graph to create a backup bucket.

Railway documents bucket regions `sjc`, `iad`, `ams`, `sin`; placement cannot be
changed after creation. The actual S3 name has a generated suffix. After approved
creation, record the actual name, endpoint, URL style and signing region securely.
S3 signing region can be `auto`; do not substitute `ams` blindly. Read-only CLI
help and the [bucket command reference](https://docs.railway.com/cli/bucket) were
reviewed. `railway init` creates a project and links its working directory; any
future approved CLI creation should use a private preparation directory, not
silently change this repository's target.

## Costs, permissions and retention limitations

Current [bucket billing](https://docs.railway.com/storage-buckets/billing):
**$0.015 per GB-month**, workspace aggregate averaged over time then rounded up
to whole GB-months. Bucket API calls and egress are free. Railway service egress
is separate; bucket access uses the public network, so use verified HTTPS.

Measured candidate archive/evidence inventory: **263,469 bytes**, including the
68,358-byte dump and selected existing accounting/preflight evidence. This was
a size inspection only, not an approved export inventory or disclosure review.
Allow **1 MiB per snapshot** for this small current dataset, manifests and encryption
overhead; actual ciphertext size is unmeasured. Proposed retention is **7 daily,
4 weekly, 12 monthly snapshots**, at most 23 independently named snapshot sets
after the policy reaches steady state. No schedule or deletion was enabled.

| Scenario | Retained ciphertext allowance | Bucket storage/month, if sole workspace bucket usage |
| --- | --- | --- |
| One initial snapshot, 1 MiB | 0.00105 decimal GB | Approximately $0.015 after rounding |
| 23 snapshots, 1 MiB each | 0.02412 GB | Approximately $0.015 |
| 23 snapshots, 10 MiB each | 0.24117 GB | Approximately $0.015 |
| 23 snapshots, 100 MiB each | 2.41172 GB | Approximately $0.045 |

These estimates exclude subscription, taxes, other workspace usage and optional
services. Existing aggregate usage can change the incremental rounded charge.
[Plan pricing](https://docs.railway.com/pricing/plans) lists Hobby at $5/month and
Pro at $20/month; an existing suitable plan may avoid a new subscription. Actual
account billing eligibility, credits, usage and plan were not verified. Free/trial
availability and hard spending limits can suspend backup retrieval; choose recovery
access intentionally. The proposed $1 bucket-only ceiling requires no plan change.
Actual billing eligibility and any plan fees still need review before creation.

The $1 ceiling is a monitoring/approval policy. Proposed warning: $0.75 actual or
forecast monthly bucket spend; at $1 or uncertain accounting, pause new uploads and
request operator review. No monitor is deployed. Check retained ciphertext sizes,
month-to-date usage and projected rounded GB-months before each future upload;
provider billing remains authoritative. Isolate this bucket's estimate from other
workspace charges. Pausing uploads does not stop charges for retained data, and no
automatic deletion, budget increase or plan upgrade is authorized. Railway's
workspace hard usage limit is not established as a bucket-specific $1 cap and can
suspend recovery access while stored data continues billing. Do not configure it
as a substitute without separate approval and verified scope.

The future Railway administrator needs explicit workspace permission to create the
dedicated project, manage its environments/buckets and view billing. A designated
backup operator needs access to the selected bucket credentials and S3 operations;
recovery custodians need independently controlled decryption/signature-verification
material. Confirm these effective capabilities in the selected workspace rather
than inferring authority from CLI authentication. No application owner/custodian
grant follows from Railway membership, and no Railway access was granted here.

[Bucket capabilities](https://docs.railway.com/storage-buckets) do not provide object
versioning, object locks, lifecycle configuration or server-side encryption. Treat
provider credentials as **bucket-wide** unless a narrower effective permission is
demonstrated. No prefix-scoped, write-only or deletion-proof credential is claimed.
Use a dedicated backup bucket, minimal project membership and a separately controlled
backup operator. Do not inject its credentials into the API, frontend or worker.
Railway credential display commands reveal secrets: future approved retrieval must
capture directly into private storage without terminal output, shell evaluation,
debug logging or Git. An S3 client is not installed; its later approved installation
and configuration must use an isolated workspace and explicit credentials/endpoint.

Use opaque UUID snapshot prefixes, preserve existing generations, verify absence
before a write and permit one coordinated uploader. Do not overwrite or maintain
an unauthenticated mutable `latest` pointer. Client-side precautions are not an
object lock: another bucket-wide credential holder can still overwrite/delete
objects. Conditional create support has not been tested and is not assumed.
Deletion needs a separately approved exact-object retention plan after successful
restoration. No automatic pruning. Keep the signed inventory and pinned verification
key independently accessible; Railway alone cannot provide immutable retention.

## Encryption and key setup requirements

Chosen method for approval: **client-side OpenPGP**, using maintained GnuPG,
an independently verified encryption certificate and integrity-protected encryption.
No recovery key, keyring or encrypted backup was created. GnuPG/OpenSSL availability
is verified; actual approved-key encryption and decryption remain untested.

The operator must select an existing suitable external certificate or generate one
on a trusted **separate machine or hardware-backed setup**. Use maintained tooling,
supported modern algorithms, a strong passphrase and documented expiry/revocation.
Interactive `gpg --full-generate-key` on that external machine is one option; it
is not a command to run here. Confirm encryption capability, not merely signing.
Consult [GnuPG operations](https://www.gnupg.org/documentation/manuals/gnupg/Operational-GPG-Commands.html)
for public-key encryption and detached-signature verification.

Required before use:

1. Export only the public certificate. An independent reviewer compares the complete
   primary fingerprint and encryption-subkey binding through an established channel,
   confirms controller identity, validity/revocation and records the approval. A
   fingerprint sent alongside an unverified certificate is insufficient by itself.
2. Preserve the passphrase-protected private recovery material off Railway and off
   the primary machine, with an independently stored recovery copy and revocation
   instructions. Document authorized custodians and test access without the source
   machine. Avoid one irreplaceable hardware device or one offline copy as the only
   recovery path. Keep keys separate from backup credentials and backup media.
3. Independently approve a manifest signing key and pin its full fingerprint/public
   verification key off the primary disk. Signing happens under external key control;
   no private signing or decryption key goes to this coding agent or runtime. These
   approvals do not verify the proposed owner/custodian or grant application roles.
4. Import only approved public material into a private encryption workspace after
   approval. Verify full fingerprints, require trusted binding, disable automatic
   key retrieval and fail closed on mismatch/expiry. Do not bypass trust, accept
   prompts automatically or put passphrases in command lines/environment/logs.

Put detailed schema, row/accounting inventories and provenance **inside the encrypted
bundle**. Authenticate a minimal outer manifest with an external detached signature:
format version, random snapshot ID, opaque object keys, sizes, ciphertext SHA-256,
capture time and approved cryptographic bindings. No credentials, row data, private
memories or sensitive database details in object names/metadata or the outer manifest.
Hash plaintext inside the encrypted inventory. A hash or ETag alone does not
authenticate a manifest. The signing key must be pinned independently of the bucket.

## Backup source and stopped-state preservation

Primary PostgreSQL 16 remains stopped at
`postgres-data/infrastructure-v1/data`. The offline guard passed all **26 protected
files**. Last SQL-verified baseline is schema **8**, prepared revision **0**, database
`supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748`, epoch
`fd2f52e0-33fc-4c0d-893a-7b08b26ab748`, namespace
`641457b2-5fd8-4b31-b85f-c58839626831`. This milestone performed no SQL queries.

The approved existing Stage 1 custom dump is
`postgres-data/preflight/1d0e9efe-e66e-4e4f-afc4-478eee4f0833/database.dump`,
SHA-256 `600d2bb19b52b4f17d4278459ba722ea5f1135df703b8efc1b0b3db0c8360671`.
The recorded manifest's migrations 1–8 match current source SHA-384 checksums.
Prior restoration matched source state/catalog and ended stopped. This is an
existing consistent snapshot, not a newly captured or newly cloud-restored backup.
Approve reuse explicitly if acceptable; no primary startup is needed for that path.
For freshness, approve a new guarded Stage 1 preflight separately: the reviewed
`scripts/preflight-development-epoch.py` starts PostgreSQL, so it was **not run**.

Historical database remains **unrecovered**. The archived budget
`ad91e475-f1da-495a-8247-139643d2d8b1` has a $0.10 ceiling and unresolved
**4,048-microUSD ($0.004048) hold** preserved in quarantined historical evidence.
Do not claim a fresh-epoch dump contains recovered historical financial rows.
The approved encrypted inventory must preserve that separate evidence without
creating, reusing, releasing, refunding or settling the old budget.

## Approved-operation execution plan — not executed

1. Obtain exact workspace/project/environment creation approval, `ams` bucket scope,
   billing plan/ceiling and retention decision. Create only named resources; verify
   project/environment IDs and absence of duplicates immediately before creation.
2. Under separate credential approval, verify actual bucket S3 identifiers, endpoint,
   signing region, effective scope and HTTPS connectivity. Keep credentials `0600`
   in private directories outside Git; never print them or generate presigned URLs.
3. Approve public encryption/signing fingerprints, independent recovery custody and
   the exact source snapshot/export inventory. Package only reviewed data and
   recovery instructions. Include reviewed role/read-only templates without secrets:
   a database dump does not capture every cluster-level role/configuration setting.
   Preserve an independently approved copy of the exact source/migrations/tooling
   required after disk loss; local unpublished commits are not recoverable from an
   older GitHub checkout. Do not upload arbitrary artifact directories.
4. Encrypt locally and externally sign the minimal ciphertext manifest. Verify
   fingerprints, signatures, local hashes and capacity before transfer. No new
   plaintext staging archive is necessary if packaging streams directly to encryption.
5. Upload ciphertext to a new UUID prefix, then the authenticated manifest/signature;
   an incomplete set is never a completed snapshot. Preserve earlier generations.
   Download ciphertext and compare full SHA-256 to the signed manifest; do not use
   a multipart ETag as a content hash. Object upload/integrity tests remain pending.
6. Verify anonymous requests cannot retrieve objects; stop on unexpected success.
   Do not create public policies, serving proxies or presigned URLs. Do not log
   signed URLs, authorization headers or secret client errors. Credentials stay
   outside Supa Agent application services.
7. For the separately approved drill, download to a private isolated location,
   verify the externally pinned manifest signature and ciphertext hashes, then let
   the external key controller decrypt. Bound extraction sizes and reject unsafe
   archive paths/symlinks; do not execute restored code or dump SQL before provenance
   verification. Restore only into a distinct socket-only PostgreSQL 16 test cluster.
8. Compare schema, checksums, state/catalog digests, row inventories, constraints,
   effective ACLs, epoch/namespace and accounting/quarantine evidence against the
   verified snapshot baseline. Reapply database-level read-only defaults, verify
   restart fencing, leave agents/credentials inactive and stop the disposable server.
   Preserve any failure evidence; never restore over the primary or apply migrations.
9. Prepare disposable plaintext storage on an approved encrypted temporary volume
   whose key can be destroyed after the drill. Unlink/shred on the primary SSD or
   copy-on-write storage cannot guarantee sanitization; plan swap/temp spill too.
   Delete only explicitly approved disposable paths after evidence capture, never
   old backups or historical evidence. No plaintext was created/deleted here.

## Capacity, tests and readiness

Recorded available space: **11,210,252,288 bytes (10.44 GiB)**; varies with host use.
Keep at least **10 GiB** operational recovery headroom per 8E and recalculate for
actual expanded restoration, ciphertext copies, retention and WAL/temp growth.
The small current backup fits the planning allowance, but only about 0.44 GiB is
above the reserve. No full Rust rebuild, Docker installation or concurrent build
is appropriate under the 8E capacity budget. No additional cleanup was performed.

| Validation | Actual result |
| --- | --- |
| Railway CLI release checksum / authentication | Passed |
| Visible projects and bucket inventory | Read-only queries passed; selected new project absent |
| Existing dump hash / migration checksums 1–8 | Passed |
| Offline primary/protected evidence guard | Passed; primary stopped |
| Stage 1 safety unit tests | 5 passed, synthetic temporary state |
| Storage topology unit tests | 5 passed, synthetic devices |
| Approved recipient encryption / manifest signature | Not run; keys unassigned |
| Bucket creation / credential scope / S3 connectivity | Not run; resources forbidden |
| Upload / anonymous denial / cloud download / restoration | Not run; approvals outstanding |
| Fresh primary backup / live accounting comparison | Not run; primary startup forbidden |
| Frontend/Rust rebuild / browser validation | Not needed; no UI/runtime changes |

Private ignored evidence: `artifacts/milestone-8f/preparation/readiness.json`, mode
`0600` in `0700` directory. It records selected planning settings, account inventory
IDs, source hash and checks without credentials. Only this report and the example
plan enter Git, in separate Conventional Commits. Full history/secrets gates run
before completion; no push is authorized.

Remaining risks: bucket-wide deletion/overwrite power, no immutable retention,
unverified recovery-key custody, billing-related recovery suspension, narrow local
capacity and future shared-provider failure if the application also moves to Railway.
This would be independent of today's local disk, not independent of Railway or
its account administrators. A later separately approved offline/provider-diverse
copy can address that additional failure domain.

**Approval boundary:** confirm the exact candidate workspace and approve creation
of only the new Supa Agent project, staging environment and private `ams` backup
bucket under the $1 monitoring policy. If a default production environment is created
by Railway, leave it empty; do not create a production bucket or any application
service. Stop if creation requires an unapproved plan change or unrelated deployment.
Independently approve encryption/signing public keys and custody, snapshot reuse or
fresh-preflight permission, exact export/upload inventory and disposable drill.
No keys or credentials should be sent in chat. Primary preflight can be considered
as a separate request after those decisions and a fresh capacity check; operational
cutover is not ready and no migration/activation approval is implied.
