# Six-hour private synthetic staging preparation

Latest review (4 November 2026): [Railway staging integration](railway-staging-integration.md)
supersedes the earlier pending-image, cost and unspecified-volume proposals below.
The private image now passes independent read-only verification and is pinned in
the proposed graph. Provisioning/deployment remain unauthorized and NO-GO under
the current unresolved replica-limit/$1 monitoring assessment. This document's
earlier publication approvals do not authorize a further publication attempt.

Status: local preparation only. No publication, CI dispatch, registry access,
credential installation, resource creation or deployment is authorized by this
document. Existing functional evidence is CI run
[41679768139](https://github.com/pyco404/SupaAgent/actions/runs/41679768139).
The focused Gosu 1.20 candidate subsequently passed CI run
[41685032319](https://github.com/pyco404/SupaAgent/actions/runs/41685032319) at
source revision `2be2597522e9631145d9d872409ace2fb791ec1a`. No new CI run is
needed for this document-only acceptance preparation.

## PostgreSQL candidate

`deployment/postgres-wolfi.Dockerfile` now builds Gosu 1.20 from upstream commit
`29b939aa5bd4dc588af765b8248f6e39150d2565`. The commit archive SHA-256 is
`2b70da8a5480fafedc16bb1fc91b572f064d03df736cbd91a616af2f1b66ab51`.
The separately downloaded release-tag archive has a different checksum because
its archive root differs; the Dockerfile uses the immutable commit archive.

The inspected public go.mod requires `moby/sys/user v0.4.1`, fixing
CVE-2026-61801, and `x/sys v0.1.0`. The build checks those module identities,
module checksums, Gosu version and privilege drop. The existing pinned Go 1.27.3
toolchain is retained instead of using the older upstream release binary.
The runtime harness additionally checks the actual binary version and performs
a network-free non-root user/group switch. The focused run passed these checks:
Gosu 1.20 / Go 1.27.3 / linux/amd64 dropped to UID 70. Locale, XML/TLS, restricted
API access, authentication, migrations, restart and restoration passed. No local
container rebuild was performed. Local tests exercised the version guard only.

PostgreSQL 18.6-r7, libxml2-16 2.15.4-r2, ABI 16, LLVM, glibc, en_US.utf8,
UTF8/libc locale, XML/TLS, volume layout and entrypoint are unchanged.
CVE-2026-39824 remains visible as UNKNOWN: its affected function is in
`x/sys/windows`, while this candidate is Linux. No severity override or suppression
is introduced. The focused PostgreSQL scan returned zero HIGH/CRITICAL/MEDIUM/LOW
findings and one UNKNOWN. A future new artifact still requires fresh scanning.

## Residual security disposition: limited operator acceptance

The operator explicitly accepted the listed residual findings only for the proposed
six-hour synthetic-only rehearsal and authorized fresh private PostgreSQL publication.
That decision grants no production/sensitive-data use, Railway pull credentials,
provisioning, deployment or persistent agent operation. Findings remain unsuppressed.
Record actual session times and deployed artifact identities before a rehearsal;
no session has started. This is not application-owner identity verification or a grant.

The PostgreSQL UNKNOWN is **CVE-2026-39824**, `golang.org/x/sys v0.1.0`, reported
fixed in v0.44.0. The official Go advisory identifies `x/sys/windows` function
`NewNTUnicodeString`; Linux is outside that documented affected platform. This is
a platform-reachability assessment, not a severity downgrade, suppression or proof
that all Gosu code is safe. Gosu is used during startup to drop privileges; protect
the image, entrypoint, user/group files and mounted paths. CVE-2026-61801 is no
longer reported after the verified user-module update.

Application evidence: frontend has 41 MEDIUM / eight LOW findings; API and worker
each have 36 MEDIUM / eight LOW. API evidence is the focused run; frontend/worker
evidence is the prior full run. Each contains the same 29 distinct CVE identifiers;
counts include duplicate GCC-family package associations. No fixes were returned
for these findings by those scanner reports. That is not proof that upstream fixes
or future compatible packages do not exist.

| Exact package and installed version | Findings | Runtime exposure, mitigation and residual risk |
| --- | --- | --- |
| `gcc-14-base`, `libgcc-s1`, `libgomp1`, `libstdc++6`, each `14.2.0-19` | Five MEDIUM associations per package: CVE-2026-102010, -114690, -114691, -115455, -95619 | Compiler-family metadata and runtime support libraries are present. Presence does not establish a vulnerable symbol is called. Do not assume all compiler/runtime advisories are unreachable. Keep non-root processes, immutable image files, no untrusted native binaries or compilation tools, and restrictive tool permissions. Conditional corruption/availability risk remains; per-symbol reachability was not proved. |
| `libatomic1 14.2.0-19` (frontend only) | Same five MEDIUM associations | Node's native runtime support; no arbitrary native plugin loading. Same compiler-family reachability limitation applies. |
| `libc6 2.41-12+deb13u4` | 14 MEDIUM / eight LOW | Native runtime functions are used by all applications. Crafted parsing/string/locale/pattern inputs can matter depending on the affected function. Authentication, bounded application inputs, memory/CPU monitoring and non-root operation reduce exposure; they do not eliminate memory-safety or denial-of-service risk. Exact exploit reachability is not established. |
| `zlib1g 1:1.3.dfsg+really1.3.1-1+b1` | MEDIUM CVE-2026-27171 and CVE-2026-85091 | Compression library is installed. Reachability through specific application compressed-input paths has not been established. Do not introduce arbitrary archive uploads or decompression tasks; preserve bounded requests and resource limits. Residual denial-of-service/corruption risk cannot be dismissed. |

The libc MEDIUM identifiers are CVE-2026-18374, -19499, -19542, -5435, -6238,
-6368, -6791, -77117, -80489, -84243, -8674, -86805, -89092 and -95818.
Its LOW identifiers are CVE-2010-4756, CVE-2018-20796, CVE-2019-1010022,
CVE-2019-1010023, CVE-2019-1010024, CVE-2019-1010025, CVE-2019-9192 and
CVE-2026-97399. Full package/version/CVE rows remain visible in the CI scan logs.
No finding or failing gate is removed by this disposition.

The accepted boundary is the following narrow rehearsal scope, not deployment consent:

> I accept the listed lower-severity findings and incomplete independent recovery
> solely for one six-hour, synthetic-data-only rehearsal in the named staging
> environment, using subsequently reviewed artifact digests. No HIGH/CRITICAL or
> secret finding may be waived. The database must contain only rebuildable fixtures
> and disposable credentials. Stop on scope drift, unexpected exposure, new severe
> findings, a $1 incremental spending threshold or the six-hour deadline. This
> acceptance expires at shutdown and authorizes no persistent sensitive staging,
> publication, credential installation, provisioning or deployment by itself.

Record the approving operator, timestamp, session start/end, actual image digests
and evidence references when this separate decision is made. A changed image,
finding set, duration or data classification requires review; no automatic renewal.

## Focused CI, after separate push and dispatch approval

The existing manual workflow defaults to full validation. Its new focused option
runs only the API image job, which builds the PostgreSQL candidate and an API
compatibility container. It retains scans, installed dependency/XML inventory,
locale, XML/TLS, non-root execution, 13 migrations/checksums, authentication,
restricted database permissions, restart/shutdown and schema/data restoration.
It does not rerun the frontend/worker images or the already-passed native Rust suite.

```sh
gh workflow run staging-validation.yml --repo pyco404/SupaAgent --ref main \
  -f validation_scope=postgres-candidate
```

Do not execute before approval. The workflow retains contents:read permissions,
has no registry credentials or publish step and never pushes an image. A passing
build does not prove a private registry pull or Railway deployment works.

Local validation passed: 44 CI/security regressions, 12 Railway graph/guard tests,
15 synthetic TLS-entrypoint tests, five independent-storage guards, four schema
metadata tests and 15 secret-scanner self-tests (95 checks total). TypeScript
type checking, workflow YAML parsing, public-source dependency/checksum inspection
and diff whitespace checks passed. Synthetic TLS tests use test doubles, not real
recovery keys. No Rust rebuild, container build or full application suite was run.

## Private registry publication and digest enrollment

Proposed destination only: private `ghcr.io/pyco404/supa-agent-postgres`.
The operator has now authorized fresh validated publication to this exact private
package. No package has been created or image published by this preparation.
Confirm package access, ownership, private visibility and publication scope in the
separately dispatched publishing job; no Railway operation is included.

Read-only permission review found repository admin/push access and a read-only
default workflow-token policy, with PR-review approval disabled. These do not prove
packages:write or read:packages authority. Exact package metadata lookup returned
HTTP 404; that can mean missing or inaccessible and is not proof of either absence
or private visibility. No token/header was inspected, no registry login attempted
and no credential created/retrieved. Package ACL and publisher/puller capability
remain explicit checks in the authorized publishing run. Local CLI scope inspection
returned gist/read:org/repo/workflow and no read:packages or write:packages. Do not
broaden this credential. The new publisher requests an ephemeral GITHUB_TOKEN with
packages:write for its single job; declared permission and a scope grant do not prove
effective publication permission until the validated registry copy succeeds.

The historical CI build digest is
`sha256:fec85dee966ee525d07c59399f8bff24b017cf8e80fd31382fd35dbf6879d0bc`.
The would-be reference under the proposed destination is
`ghcr.io/pyco404/supa-agent-postgres@sha256:fec85dee966ee525d07c59399f8bff24b017cf8e80fd31382fd35dbf6879d0bc`.
It is NOT published, pullable or approved. The run's artifact inventory returned
zero artifacts. The harness retained images only on a disposable hosted runner;
no OCI/archive copy is available in the reviewed evidence. A digest is not a backup
of its contents. Therefore exact publication of that historical image cannot be
promised, and the real deployment reference remains unassigned/null.

For an explicitly approved publication, use a separate manual workflow with
packages:write only for its publishing job, exact owner/SHA dispatch and a
repository-bound GITHUB_TOKEN. No protected GitHub environment is created or claimed;
adding an independent environment reviewer would require its own configuration.
Publish the exact locally scanned OCI artifact;
do not rebuild between scan and push. Retain OCI manifest/config digests and
verify the registry manifest after publication. Local Docker image IDs must not
be mistaken for a remotely retrievable manifest digest. If rebuilding is needed,
scan and exercise the new artifact before publication. Do not enable publication
in the existing read-only validation workflow.

Minimum publication sequence to approve, not execute now:

1. Confirm the exact GHCR repository, owner, private package ACL, publisher identity,
   reviewer and package costs. A public source repository does not establish that
   its container package is private. No registry or protected environment has been
   created by this preparation.
2. Since the old artifact is unavailable, explicitly authorize a fresh build and
   focused validation at an exact source commit. Export one canonical OCI artifact
   and record its manifest, config and layer digests. Disable unsolicited provenance
   or platform changes during copying, or account for them in the reviewed manifest.
3. Run the existing candidate checks and scans against the exported image contents.
   Bind the scan to the manifest/config/layers used by runtime probes. Preserve only
   the image archive and sanitized evidence needed for publication. Never export the
   harness working directory, credential env files, temporary database volumes or
   synthetic login material. Do not upload a private image as a publicly accessible
   artifact in the public source repository.
4. If publication occurs in a later job, use approved private artifact storage and
   verify its hashes on retrieval. If that storage is unavailable, perform validation
   and the explicitly approved publication on the same controlled runner. Rebuilding
   after validation invalidates artifact identity and requires revalidation.
5. After separate publication approval, use a registry copier with digest preservation
   (for example Skopeo --preserve-digests). Stop if a media-type conversion changes
   the manifest. A unique staging tag may transport the image; it is never the
   deployment reference. Keep publish authentication out of argv, logs and image
   layers; use short-lived job credentials and private auth files.
6. Read back the registry manifest and compare its SHA-256 and config/layer descriptor
   digests with the canonical validated artifact. Reject a manifest-list versus
   platform-manifest mix-up. Verify by-digest retrieval and actual private visibility;
   anonymous access must not retrieve the private manifest or blobs. Stop on a
   mismatch or unexpected visibility, without enrolling or deploying the image.
7. Present the actual registry-qualified digest and new evidence for image enrollment.
   Do not fill the local approved-image record with the historical CI config/image
   identifier or invent a registry digest before this comparison passes.

Digest-preserving copy semantics are documented in the upstream
[Skopeo copy manual](https://github.com/containers/skopeo/blob/main/docs/skopeo-copy.1.md).
The local `.github/workflows/postgres-publication.yml` now prepares this operation.
It is not on origin/main and has not run. Source push remains explicitly forbidden
without separate operator approval; publication authorization does not override it.

## Implemented publication path and source-push boundary

The manual workflow accepts one exact expected main SHA, restricts dispatch to the
repository owner on the hosted runner and refuses reruns (run attempt must be 1).
Checkout credentials are not persisted. Only this job requests packages:write;
the existing validation workflow stays read-only and cannot publish. A requested
registry push/pull token scope is tested before build, without exposing tokens.
An existing public package or inconclusive metadata/permission check stops the job.
A new package uses GitHub's documented private-by-default first publication and
must pass post-publication private metadata and anonymous-pull denial checks.

The existing API compatibility harness builds PostgreSQL once, scans the saved
image archive, runs its existing security/runtime/restore checks, and only then
writes a private candidate record. The record binds source/run/config ID, filesystem
diff IDs and archive SHA-256. Working state and archive stay in runner-private
temporary storage; runtime env files are removed, not exported.

`scripts/publish-postgres-image.py` rechecks the record/archive, preserves the
validated config bytes and verifies every uncompressed filesystem-layer hash while
creating canonical gzip OCI blobs. This is representation conversion, not a build
or filesystem change. Its canonical manifest digest is distinct from a Docker
config/image ID; the exact registry manifest is computed before upload. It uses
Skopeo --preserve-digests, compares the read-back raw manifest, downloads the
published config/layers and hashes every blob. It also verifies private package
metadata and a genuinely anonymous pull attempt. All findings are retained; the
publication gate allows only the explicitly accepted PostgreSQL UNKNOWN advisory
at its reviewed package/version/severity. Any new finding requires review and is
not hidden or suppressed. Anonymous registry access is checked before upload too,
including when GitHub's package API returns an ambiguous 404.
There is no build, fallback image, deletion or Railway operation in the publisher.

Registry login uses only the job token over stdin and a 0600 temporary auth file,
removed on success or failure. Transport errors never emit tokens, headers, raw
provider prose or tool stderr. HTTP redirects are rejected for authenticated
metadata requests. Ambiguous copy/verification failures record review-required and
never retry or claim success; an image may already exist and needs read-only review.

After separate source-push approval, the already-authorized publication can be
dispatched once (replace the placeholder with the reviewed final main SHA):

```sh
gh workflow run postgres-publication.yml --repo pyco404/SupaAgent --ref main \
  -f expected_source_sha=EXACT_APPROVED_MAIN_SHA
```

The fresh immutable reference cannot be filled until that remote job actually
succeeds. No local Docker build, package-token retrieval or registry login occurred.
Network-free synthetic archive/registry tests cover tampering, wrong ownership/
source/run, failed scans, duplicate/path/symlink archives, changed config/layers,
public/inaccessible packages, manifest mismatch, credential redaction and ambiguous
copy recovery. Local validation passed 21 publication tests, the existing 44 CI
security regressions and 15 secret-scanner self-tests, plus Python/workflow parsing.
These are mocked evidence, not successful live publication evidence. No full Rust
suite, Docker image build, registry login or remote publication run was repeated.

Railway GHCR pulls require separately installed read:packages credentials and
the supported plan. Private registry support requires Pro; recheck the actual
subscription before proceeding, without upgrading automatically. Do not reuse
S3, application, OpenAI or Anthropic credentials, and never embed registry tokens
in IaC, Docker arguments, Git or diagnostic logs. Credential requirements are
documentation-verified only; no current token scope or package ACL is claimed.

After publication and explicit image approval, fill `config/railway-staging-image.json`
with approved/private flags, the complete `ghcr.io/...@sha256:...` reference,
source revision, successful official scan-run URL and zero HIGH/CRITICAL/secret
findings. Lower-severity findings remain in the scan evidence and require a
documented security disposition. All fields are currently unapproved/null.
This is reviewed configuration, not cryptographic attestation or deployment consent;
the reviewer must independently bind these fields to the actual artifact and scan.

IaC uses the SDK's typed `database()` custom-image path, with no generic PostgreSQL
fallback. It fails before returning a graph on missing/unapproved image evidence,
mutable tags, foreign repositories, invalid digests or wrong target IDs. The proposed
GHCR repository is deliberately the only accepted repository. Another registry
requires a separate reviewed configuration/test change. No runtime environment
flag can substitute an image reference. Offline tests use synthetic references,
not published or approved images. Actual registry pull and Railway product-plan
compatibility remain unverified until their separately approved steps.

Frontend/API/worker publication is not included in the PostgreSQL package proposal.
Their current IaC builds from GitHub main rather than pulling retained CI artifacts.
Before deployment, freeze the approved source revision and disable automatic
redeployments. Review build costs and resulting image evidence; these are not proven
byte-identical to previous CI images. If the operator requires immutable registry
images for every service, approve those additional packages/publications separately.

## Exact proposed rehearsal and cost boundary

- Workspace: Daniel 's Projects, `e319f68a-9897-49ed-b119-743f1f846aa0`.
- Existing project: `99648d2a-69a1-4d8c-98d0-7f2a26543c9c`; staging environment:
  `ff0b2092-e42b-4a89-ae25-441ab95d92d8`. IaC rejects other project/environment IDs.
- Amsterdam service region `europe-west4-drams3a`; retain existing backup bucket.
- `supa-web`: one replica, port 3000, /healthz, separate visitor/operator gate,
  exact approved HTTPS origin. No domain is currently requested by IaC; domain
  creation/access arrangements require explicit approval before browser access.
- `supa-api`: one private replica, port 8080, /ready, offline provider and restricted
  runtime role; public-service and named operator credentials remain distinct.
- `supa-worker`: one private replica, port 8081, /ready, enabled=false, zero cycles.
- `supa-postgres`: one private PostgreSQL 18 candidate, digest-pinned custom image,
  persistent mount /var/lib/postgresql/data. Propose 5 GB volume capacity; confirm
  exact capacity and retention charges in the provider plan before approval.
- New schema-only staging database; separate migration administration and distinct
  restricted API/worker logins. No blanket financial UPDATE or runtime DDL grants.
- No local dotenv imports, primary data, real owner grants, wallets, provider keys,
  real memories, retained evaluation outputs or historical accounting imports.
- Both paid providers, controlled paid work, scheduling, wallets and external
  publishing remain disabled. Test credentials expire no later than session end;
  agent activation remains a separate approval and is outside this rehearsal.

Proposed initial resource ceilings, not yet installed or validated under load:

| Service | Replicas | RAM ceiling target | CPU ceiling target |
| --- | --- | --- | --- |
| Next.js frontend | 1 | 512 MiB | 0.5 vCPU |
| Rust API | 1 | 256 MiB | 0.25 vCPU |
| Disabled worker | 1 | 128 MiB | 0.125 vCPU |
| PostgreSQL | 1 | 512 MiB | 0.5 vCPU |

These total 1,408 MiB and 1.375 vCPU. Confirm Railway accepts the exact granularities
and that startup fits; do not silently round limits upward or enlarge them after
OOM failures. The current IaC does not set these numeric ceilings. Include their
exact supported provider settings in the resource plan before provisioning approval.
An inability to enforce the proposed settings is a review boundary. There is no
autoscaling or second replica in the proposal. A smaller 1 GB synthetic-only volume
may be selected instead of the earlier 5 GB proposal if the operator approves it and
the actual database sizing fits; neither volume has been created.

For browser access, choose either a separately approved access-controlled HTTPS
frontend origin or an approved private access mechanism. A password-gated public
HTTPS origin is not a network-private endpoint. API, worker and PostgreSQL receive
no public domains or TCP proxies. Do not create a frontend domain by implication.

Proposed maximum duration is six hours, incremental spending policy $1, with an
operator check at launch, hourly and at shutdown. These exported IaC policy values
are not a Railway TTL or a provider-enforced per-service spending cap. Nominate a
shutdown operator and record start/deadline before deployment. At six hours (or
the cost threshold, unexpected spend or unsafe behavior), stop only newly approved
rehearsal compute: frontend, API/disabled worker, then PostgreSQL gracefully.
Verify stopped replicas and disabled automatic redeployment. Never stop unrelated
workspace workloads or delete the retained volume automatically. Arrange separate
approval for eventual volume deletion/retention; storage continues billing after
compute stops. If reliable shutdown monitoring is unavailable, do not start.

Last read-only cost review: current $17.55, forecast $33.62, shared hard limit $40,
forecast headroom $6.38. Recheck at approval. Using aggregate average 0.75–1.5 GB
RAM and 0.05–0.2 vCPU, six hours compute is approximately $0.07–0.16. A retained
5 GB volume is approximately $0.75/month; remaining-period storage, low egress and
startup variability must fit the $1 policy. Actual memory, CPU, billing delay,
unrelated usage, registry and build charges remain uncertain. Allocation is not
measured consumption; do not enforce untested low memory caps that cause OOM loops.
Full-month continuous operation is outside the proposed authorization. Do not
raise the shared limit or claim the forecast is guaranteed.

At the proposed aggregate resource ceilings, using decimal-GB RAM billing and a
30-day arithmetic month, six-hour compute is roughly $0.35; this is not a measured
provider charge or account hard cap. A 1 GB retained volume adds approximately
$0.15/month; the earlier 5 GB option adds $0.75/month. Up to 1 GB of service egress
adds $0.05. Under average-use assumptions above, six-hour compute remains roughly
$0.07–0.16. Together these support a short-session $1 monitoring envelope, but build
costs, package storage/transfer, startup peaks, taxes, billing delay and unrelated
workspace consumption are not independently bounded by it. Registry/GitHub charges
need their own approval and must not be silently shifted into or outside the policy.
Retaining 5 GB for a full month plus ceiling-level compute and egress exceeds $1;
approve a specific volume size/retention deadline or do not start. No automatic
volume deletion is authorized. Recheck $6.38 forecast headroom before launch and
leave a safety margin; the shared $40 limit can interrupt unrelated workloads.

Record a named shutdown operator, an independent deadline reminder and a fallback
operator before launch. At shutdown: close frontend access and prevent new requests;
stop frontend, API and disabled worker; confirm no outstanding connections; stop
PostgreSQL gracefully; verify all four deployment processes stopped, no replacement
deployment is pending and automatic redeploys remain disabled. Preserve the volume,
record final usage/status and revoke only the rehearsal's credentials through the
approved mechanism. Database-resident tokens may require revocation before the
database is stopped. Never use an environment-wide delete, remove unrelated services
or mistake worker enabled=false for zero compute usage. Any later restart is a new
session requiring approval, not a continuation or an automatic retry.

## Synthetic-only recovery decision

A short, explicitly disposable technical rehearsal can be considered separately
without importing protected records. Only invented test identities/content are
allowed; use unrelated new credentials, and treat the entire database as rebuildable.
The volume tests restart persistence, not independent recoverability. If it is lost,
recreate synthetic fixtures only; do not represent them as recovered history.
An explicit synthetic-only risk acceptance may permit this limited rehearsal
before real recovery enrollment. It does not approve persistent sensitive staging.

Before introducing real identities, private memories or financial records: appoint
an independent reviewer; arrange trusted key-generation equipment and independent
encrypted recovery storage; establish separate OpenPGP encryption/signing identities;
verify all four primary/subkey fingerprints through an authenticated independent
channel; verify off-machine recovery copies; approve exact backup inventory and
credential access; upload ciphertext and signed manifests; download, verify, decrypt
and restore into a disposable database using independent recovery material.
Verify schema, audit/accounting invariants and restoration. Never restore over the
primary. No keys, S3 credentials or real records are accessed in this preparation.

The protected primary stays stopped at its existing baseline. Historical holds
$0.004048 and $0.003536 are not modified, imported, refunded or settled.

## Required separate decisions

1. **Residual-risk acceptance:** approve the bounded wording above with exact evidence,
   eventual artifact identities and the synthetic-only recovery exception. This grants
   no operational permission and expires at session shutdown.
2. **Registry publication:** approve only the named private PostgreSQL package,
   fresh canonical build/scan/runtime validation necessitated by lost CI artifacts,
   reviewed publisher workflow/permissions, package charges and exact artifact copy.
   Require digest/visibility verification. It grants no Railway pull credentials or
   service creation. A code push/CI dispatch needs explicit scope if required.
3. **Railway pull authentication:** separately approve creating/installing a scoped,
   expiry-bound read:packages credential with access to that package. An owner uses
   a private credential channel, not chat. Confirm Pro support without an automatic
   upgrade, private by-digest pull and revocation plan. No S3/application/model token
   reuse, broad write/delete scope, resource creation or deployment is included.
4. **Resource provisioning:** approve the additive plan in the exact existing project
   and staging environment, the four services, numeric resource settings, volume
   capacity/retention, fresh synthetic roles/credentials, schema-only migrations and
   access mechanism. No local dump imports, primary operations or resource deletion.
   If provider provisioning starts deployments automatically, stop: that combined
   operation also requires decision 5 before execution, not an inferred approval.
5. **Six-hour deployment and shutdown:** approve exact source/image identities,
   session start/deadline, $1 incremental monitoring scope, nominated operators,
   authenticated access and stopping only the newly provisioned rehearsal compute.
   Include startup/readiness/authentication checks and emergency stop, but no agent
   activation, paid calls, real data or financial actions. After six hours there is
   no implied renewal. Separate volume deletion and persistent-staging/recovery
   approval remain necessary.

Decisions 1 and 2 are now explicitly authorized within their limited scope; actual
publication awaits the separately required source push and a successful manual run.
Decisions 3, 4 and 5 remain unauthorized. Do not combine them into a blanket approval.

References: [Gosu 1.20](https://github.com/tianon/gosu/releases/tag/1.20),
[official Go advisory](https://pkg.go.dev/vuln/GO-2026-5024),
[Railway private registries](https://docs.railway.com/builds/private-registries),
[GHCR authentication](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry),
[Railway pricing](https://docs.railway.com/pricing).
