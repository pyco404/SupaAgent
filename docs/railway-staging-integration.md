# Railway staging integration review — 4 November 2026

Preparation only. **NO-GO for provisioning/deployment under the current $1
incremental monitoring policy until replica-limit and shutdown prerequisites are
resolved.** No Railway resource, variable, credential, domain or billing limit was
changed. The protected primary PostgreSQL remains stopped and historical accounting
holds remain untouched.

## Verified target and image

- Workspace: Daniel 's Projects, `e319f68a-9897-49ed-b119-743f1f846aa0`.
- Existing Supa Agent project: `99648d2a-69a1-4d8c-98d0-7f2a26543c9c`.
- Staging: `ff0b2092-e42b-4a89-ae25-441ab95d92d8`.
- Live workspace plan: **PRO**, independently read from the workspace API.
- No application services were present at review. Preserve the existing Amsterdam
  `supa-agent-backups` bucket; it is not rehearsal cleanup inventory.
- PostgreSQL: `ghcr.io/pyco404/supa-agent-postgres-infra-98c1006fcb1d@sha256:9409009a40c0929052306d068c14bef92f3ecc67662c453540e2c9b95ae18fc8`.

`config/railway-staging-image.json` records image acceptance separately from deployment
authorization. The authoring graph accepts only this exact image, verified private
repository association, anonymous denial, Linux/amd64 platform, 16 verified layers,
configuration digest and a hash of the retained independent verification report.
No fallback to the two public packages, a mutable tag or a generic PostgreSQL image
is permitted. The previous publication workflow remains failed; subsequent independent
read-only verification established image integrity and private access.

The image's recorded UNKNOWN Gosu advisory, CVE-2026-39824 in golang.org/x/sys v0.1.0,
remains visible. This integration is not a fresh vulnerability scan. Its use remains
limited to the accepted synthetic-only six-hour rehearsal.

## Registry authentication — prepare, do not create or install

[Railway private registries](https://docs.railway.com/builds/private-registries)
require Pro, which the selected workspace already has. For GHCR, Railway accepts
a GitHub access token and handles the username. Use a separately approved, expiry-bound
**personal access token (classic), read:packages only**, whose owner already has read
access to the specific private package. Package scopes on classic PATs are not
per-package restrictions; owner access can cover other packages. Do not claim narrower
scope than actually granted. No write:packages, delete:packages or new repo scope is
needed for this granular package. A dedicated read-only identity would require its
own approved account/package access grant.

Do not install the existing interactive GitHub CLI credential in Railway, copy its
other scopes, reuse the expired Actions GITHUB_TOKEN, or reuse S3/model credentials.
The owner must provision the approved credential through a private channel. Never
put it in chat, Git, ordinary service variables, image layers or logs. After separate
approval, install it in the PostgreSQL service's Source → Registry Credentials only,
verify a private by-digest pull, and record expiry/revocation responsibility. No token
has been generated, rotated, retrieved or installed for this Railway integration.

## Proposed graph and resource review

Four compute services, one replica each, Amsterdam `europe-west4-drams3a`:

| Service | Port / readiness | Required boundary |
| --- | --- | --- |
| supa-web | 3000 / /healthz | visitor/operator authentication; separately approved HTTPS access |
| supa-api | 8080 / /ready | private network; distinct restricted runtime login; no runtime DDL |
| supa-worker | 8081 / /ready | private network; offline, disabled, zero autonomous cycles |
| supa-postgres | private 5432 | exact image above; TLS; separately scoped migration administrator |

The graph additionally declares one `supa-postgres-data` volume, proposed **1000 MB**,
mounted at `/var/lib/postgresql/data`, with no automatic backup schedule for disposable
fixtures. The live plan's unspecified-volume default is **50,000 MB**; do not accept
that default silently. Confirm the explicit size in the eventual additive provider
plan. Storage and persistence do not establish independent recovery.

Application shutdown grace is 60 seconds; PostgreSQL receives 120 seconds. These are
supported SDK `deploy.drainingSeconds` fields. The installed SDK also exposes
`deploy.limitOverride.containers.{cpu,memoryBytes}` and `volume.config.sizeMB`.
Its schema accepting numbers does not prove provider acceptance of every value.

| Service | Earlier RAM target | Earlier CPU target |
| --- | --- | --- |
| Frontend | 512 MiB | 0.5 vCPU |
| API | 256 MiB | 0.25 vCPU |
| Disabled worker | 128 MiB | 0.125 vCPU |
| PostgreSQL | 512 MiB | 0.5 vCPU |

The live PRO entitlement reports `containers.minReplicaCpu = 4` and
`containers.minReplicaMemoryBytes = 4000000000`, with maximums 24 vCPU and
24,000,000,000 bytes. The public documentation does not resolve the precise effect
of those minimum fields on small custom limits. **Earlier small caps are unverified**;
they are not installed or represented as enforced in the authoring graph. Obtain
provider confirmation/read-only planning evidence before accepting them. Do not
silently round up or represent these ceilings as minimum charged allocations.
Startup and workload stability at the small targets have not been measured.

No domain/TCP proxy is requested by the graph. Do not attach trysupaagent.xyz, alter
DNS, expose API/PostgreSQL, or generate a frontend domain by implication. The exact
authenticated access mechanism and origin remain a separate approval requirement.
The three application services currently reference GitHub main; freeze the separately
approved application revision and disable automatic deployments before launch. Their
Railway rebuilds are not asserted byte-identical to retained CI application images.

## Refreshed cost decision

Read-only CLI usage and live workspace-plan evidence were retained locally.
Billing period ends 18 November 2026, 16:34:07 UTC.

| Workspace measure | USD |
| --- | ---: |
| Current usage | 18.407073 |
| Forecast usage | 33.581035 |
| Shared hard limit, unchanged | 40.00 |
| Current headroom | 21.592927 |
| Forecast headroom | 6.418965 |

[Current Railway container pricing](https://docs.railway.com/pricing/plans):
RAM $0.000231/GB/minute, CPU $0.000463/vCPU/minute, volume
$0.000003472222222/GB/minute, egress $0.05/GB. Service builds and image storage/pulls
are not separately charged by Railway under the documented build policy. GitHub
registry/Actions billing is separate and must not be silently assumed covered.

- Earlier assumed average usage (aggregate 0.75–1.5 decimal GB RAM, 0.05–0.2 vCPU)
  gives $0.07–0.16 compute for six hours; this is an assumption, not a staging metric.
- If all earlier small caps were accepted and sustained, six-hour compute would be
  about **$0.352**. A full 1 GB volume for six hours adds **$0.00125**; 1 GB of egress
  adds **$0.05**, totaling about **$0.403** under those conditional assumptions.
- A sustained four-service scenario at 4 GB and 4 vCPU each is about **$3.997** compute.
  This is a usage scenario, not a minimum bill or an installed limit.
- With no confirmed custom caps, four services at the reported 24 GB/24 vCPU maxima
  would be about **$23.985** compute in six hours, exceeding forecast headroom.
- Retaining 1 GB after shutdown costs up to roughly $0.15 per 30-day month; a volume,
  snapshot or backup can continue billing when compute is stopped. Retention needs
  an approved deadline or separately approved deletion. The earlier 5 GB option is
  not the current proposal.

The $1 value is an **operator monitoring/approval policy**, not an enforced cap.
The shared $40 hard limit can affect unrelated workloads and must not be changed.
Costs below $1 are plausible at low consumption, but not bounded sufficiently by
the current supported-limit evidence. Therefore deployment is **NO-GO** under the
current envelope. Recheck usage immediately before any later approval. Do not
upgrade the plan, enlarge the envelope, provision a probe service, or test resource
limits through a real deployment to resolve this preparation blocker.

## Six-hour startup and shutdown procedure — for separate approval

These control-plane command signatures were checked against installed CLI 5.71.0
and live read-only schema introspection. There are no rehearsal services to stop
now; end-to-end live shutdown remains a rehearsal acceptance test, not a claimed
completed operation.

1. Resolve resource/cost limits. Name a launch/shutdown operator and fallback;
   assign an independent deadline reminder before launch. Capture exact project,
   environment, four service IDs, deployment IDs and one volume ID in a session
   inventory. Exclude the existing bucket and unrelated projects/resources.
2. Approve credentials, additive provisioning, bootstrap and deployment separately.
   If provisioning implicitly launches workloads, deployment approval must already
   cover that operation. Start the clock with the first billable rehearsal workload,
   not after health checks. Record T0 and an absolute T0+6h deadline.
3. Bootstrap only a fresh synthetic staging database, approved schema-only migrations
   and SQLx checksums using separate administration. Supply restricted API/worker
   logins; never import the primary dump, historical ledgers, retained evaluations,
   actual owner records, wallets, model keys or private memories.
4. Start PostgreSQL, then the restricted API, disabled worker and frontend. Verify
   TLS, /ready and /healthz, unauthorized/role boundaries and zero cycles. Require
   provider/scheduler/signing/publishing/trading/launch restrictions to remain off.
   Finite synthetic agent activity requires its own authorization; none is included.
5. Check incremental usage every ten minutes and at startup/shutdown. Propose early
   warning at $0.50 and begin shutdown at $0.75, or earlier on abnormal usage. Billing
   lag prevents treating those triggers as a guaranteed $1 ceiling.
6. Begin normal shutdown by T0+5h50m: close frontend access, disable automatic source
   deployments/image updates, prevent new requests and verify no active task leases.
   Remove frontend, API and worker deployments, then PostgreSQL gracefully. Use the
   inventoried deployment IDs; cancel queued/building deployments as well. Do not
   treat disabled worker scheduling, a failed health check or an idle process as
   proof that its compute billing stopped.
7. Read all deployment states until none of the four services has Active, Deploying,
   Building, Initializing, Queued or replacement workloads. Ensure terminal
   Removed/Cancelled states, stopped replicas and no pending source-triggered run.
   Escalate to the fallback operator before T0+6h if this cannot be established.
8. Review volume, backup and service inventory for remaining charges. With separate
   explicit cleanup approval, delete only the inventoried synthetic volume and
   unused rehearsal services. Otherwise retain the volume with a cost/deletion
   deadline; do not claim all billing ended. Preserve the backup bucket and all
   unrelated resources. Revoke only approved rehearsal credentials, record final
   usage and check delayed billing afterward. Restart requires new approval.

Supported commands to prepare with exact IDs; **do not execute now**:

```sh
railway deployment list --project PROJECT_ID --environment ENVIRONMENT_ID --service SERVICE_ID --json
railway api 'mutation($id: String!) { deploymentRemove(id: $id) }' --var id=DEPLOYMENT_ID
railway api 'mutation($id: String!) { deploymentCancel(id: $id) }' --var id=QUEUED_OR_BUILDING_DEPLOYMENT_ID
railway volume --project PROJECT_ID --environment ENVIRONMENT_ID list --json
# Only after separate approval to discard this synthetic volume:
railway volume --project PROJECT_ID --environment ENVIRONMENT_ID delete --volume VOLUME_ID --yes --json
```

Do not rely solely on `railway down`: it removes only the latest successful
deployment and does not establish absence of concurrent/queued workloads or remove
volumes. See [deployment lifecycle](https://docs.railway.com/deployments/reference),
[CLI down](https://docs.railway.com/cli/down) and
[volume billing](https://docs.railway.com/pricing/plans).

## Exact remaining approvals and prerequisites

1. Provider-supported resource settings and a sufficiently bounded cost plan;
   neither the $1 policy nor the shared $40 hard limit is changed here.
2. Private-channel creation and service-specific installation of read-only GHCR
   credentials, plus fresh synthetic runtime/migration credentials and their expiry.
3. Exact additive four-service/one-volume provisioning plan, storage retention,
   schema-only bootstrap and authenticated frontend access mechanism.
4. One six-hour deployment at reviewed source/image identities, with named operators,
   monitoring, verified shutdown responsibility and separately authorized cleanup.
5. Any source push remains separately approved. No code was pushed in this review.

Synthetic test state must be disposable and reconstructible. Independent encrypted
backup upload, verified encryption/signing identities, off-machine recovery keys
and a restoration drill remain mandatory before persistent sensitive data is used.

## Focused validation

All 13 Railway authoring tests passed, including exact image identity, fail-closed
attestation checks, one-replica service boundaries, explicit volume sizing/mounting
and graceful shutdown settings. The focused TypeScript check and whitespace check
passed. No container rebuild or full application-suite repeat was needed.

An optional local runtime-profile check stopped with `disposable_profile_psql_failed`
while reading the retained disposable fixture, before its separate cluster creation
or startup. That fixture was not restarted. This is an unavailable local runtime
check, not new container-execution evidence. The prior successful CI validation is
preserved. Live Railway startup, custom resource-limit acceptance and end-to-end
shutdown remain unperformed pending separate approval.
