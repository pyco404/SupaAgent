# Milestone 8F — Approved Railway backup infrastructure created

The operator explicitly approved one dedicated private Supa Agent project, its
staging environment and one private Amsterdam backup bucket in the confirmed
existing workspace. That creation is complete and metadata verification passed.
This report supersedes the uncreated-resource status in the earlier
[readiness report](milestone-8f-railway-backup-readiness.md). No backup is stored
in Railway yet, and no decryption or restoration readiness is claimed.

## Actual configuration

| Resource | Verified name / configuration | Identifier |
| --- | --- | --- |
| Existing workspace | `Daniel 's Projects` | `e319f68a-9897-49ed-b119-743f1f846aa0` |
| New private project | `Supa Agent`, `isPublic: false` | `99648d2a-69a1-4d8c-98d0-7f2a26543c9c` |
| Sole environment | `staging` | `ff0b2092-e42b-4a89-ae25-441ab95d92d8` |
| Storage bucket | `supa-agent-backups`, Amsterdam (`ams`) | `76923b39-b3a7-4390-8a0b-783d8a8113e9` |
| Stored data | **0 objects, 0 bytes** | Metadata-only verification |
| Application services | **0** | No database, API, frontend or worker |

No production environment or service was created. The live API supports
`ProjectCreateInput.defaultEnvironmentName`; using `staging` avoided an automatic
unused production environment. Project creation specified the exact approved
workspace, `isPublic: false`, `prDeploys: false` and no source repository. No
deployment, image or database template was attached.

The checksum-verified official Railway CLI 5.71.0 reused existing authentication.
Before creation, read-only discovery confirmed the exact workspace and no existing
Supa Agent project. The project was created once through the supported public
GraphQL API; its workspace, sole environment, privacy and empty service/bucket
inventories were checked before proceeding. The bucket was then created once
using the official CLI with explicit environment ID and `--region ams`.
Railway returned `staged: false`, `committed: true`. No unrelated staged changes
were applied. The repository itself was not linked to Railway; CLI linking used
the separate private directory `/tmp/supa-agent-approved-bucket-8f`.

Final metadata queries confirmed exactly one bucket belonging to this project,
the correct environment and region, empty storage, no services and the unchanged
workspace plan. Mutation results and verification metadata are preserved privately
under `artifacts/milestone-8f/provisioning/`, directory `0700`, files `0600`.
These records contain no S3 credentials and are excluded from Git.

## Privacy and credential boundary

[Railway buckets](https://docs.railway.com/storage-buckets) are private and require
S3 credentials; public buckets are unsupported. The project is independently
verified private through its API flag. No serving proxy, public policy, presigned
URL or application service was created. No data was uploaded.

**No S3 credential endpoint or CLI credential command was invoked.** Access keys,
secret keys, provider-generated S3 bucket name, endpoint, signing region and URL
style were not retrieved. The verified name above is the Railway display name;
do not infer the actual S3 name from it. That remaining metadata must be recorded
through an approved credential-access workflow later, without printing secrets.
No anonymous object-access or authenticated S3 connection test was performed.
Privacy is established here by the provider's private-bucket model and observed
resource configuration, not by a successful object-level access test.

No local bucket credentials, encryption keys, signing material, recovery keyring,
owner credentials or ownership grants were generated or installed. Railway manages
the provisioned bucket internally; this report makes no claim that provider-side
credential material does not exist. Treat future supplied access keys as bucket-wide
unless narrower permissions are demonstrated. A wrapper cannot remove the underlying
credential holder's deletion/overwrite authority.

## Billing and retention

Live workspace query verified the existing plan is **PRO**, with project and bucket
capacity available. The plan was not changed, and no upgrade, new subscription,
permission expansion or unrelated paid service was requested. Resource creation
succeeded under existing access without an upgrade requirement.

Current [bucket billing](https://docs.railway.com/storage-buckets/billing) is
**$0.015/GB-month**, with aggregate workspace usage rounded up to whole GB-months.
Bucket API operations and bucket egress are free; Railway service egress is separate.
The empty bucket has no stored-byte usage at verification. This is not a final
invoice or a statement that unrelated workspace charges are zero.

Future 23-snapshot retention estimates from the reviewed plan remain approximately
**$0.015/month** at 1–10 MiB per snapshot, or **$0.045/month** at 100 MiB per snapshot,
excluding plan fees, unrelated usage and taxes. Actual billing depends on measured
ciphertext sizes, retention time, credits and aggregate workspace usage.

The **$1/month bucket-only ceiling remains a monitoring/approval policy**. No
bucket-specific hard cap or monitor was configured. Proposed warning is $0.75;
at the threshold or uncertain accounting, stop new uploads and request review.
Retained data can continue accruing charges after uploads stop. Do not automatically
delete backups, increase the ceiling, change the plan or apply a workspace-wide
hard limit that could suspend recovery access.

Retention target remains **23 encrypted snapshots**; no schedule, snapshot or
deletion mechanism exists yet. Railway has no object versioning, object locks or
lifecycle rules. Future uploads must use unique snapshot identifiers, preserve
existing generations and require separate exact-object deletion approval.

## Local preservation and verification

No primary PostgreSQL startup, connection, migration, backup extraction, restore,
epoch activation, agent execution or financial operation was performed. The local
offline guard verifies all 26 protected files and the approved Stage 1 backup hash.
Last SQL-verified baseline remains migration **8**, prepared epoch
`fd2f52e0-33fc-4c0d-893a-7b08b26ab748`, namespace
`641457b2-5fd8-4b31-b85f-c58839626831`. Do not misrepresent an offline evidence check
as a new SQL verification.

Historical PostgreSQL remains **unrecovered**. Quarantined budget
`ad91e475-f1da-495a-8247-139643d2d8b1`, its $0.10 ceiling and unresolved
**$0.004048 hold** remain protected evidence. No release, refund, settlement, reuse
or import into new accounting occurred. Frontend routes, appearance, service URLs
and runtime controls were not modified. No GitHub push occurred.

Validation consists of actual provider-side creation results, read-only project/
environment/bucket/plan checks, JSON consistency checks, offline protected-state
verification, full one-file history audit and secrets scanning. No build or browser
test was needed: only the non-executing example inventory and documentation changed.

## Next steps requiring separate approval

1. Independently select and verify the OpenPGP encryption recipient, external recovery
   custody and manifest signing public key. Private keys remain outside Railway and
   the primary machine. None is assigned; do not generate them as a continuation.
2. Approve metadata/credential retrieval and secure installation for this exact
   bucket. Capture directly into private storage; never print or commit access keys.
   Then verify actual S3 identifiers, effective permissions and connectivity.
3. Approve reuse of the existing consistent Stage 1 archive and the exact encrypted
   export inventory, or separately approve primary startup for a fresh consistent
   backup. Current resource approval grants neither.
4. Separately approve encryption, authenticated manifests, upload verification and
   an isolated cloud download/decryption/disposable PostgreSQL recovery drill.
   Never restore over primary data; review plaintext handling and cleanup explicitly.

The creation approval is fulfilled. It is not an ongoing authorization to create
additional resources, retrieve secrets, upload data, deploy, migrate or activate
anything. Independent recoverability remains pending the approved key, backup and
recovery steps above.
