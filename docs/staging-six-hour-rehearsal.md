# Six-hour private synthetic staging preparation

Status: local preparation only. No publication, CI dispatch, registry access,
credential installation, resource creation or deployment is authorized by this
document. Existing functional evidence is CI run
[41679768139](https://github.com/pyco404/SupaAgent/actions/runs/41679768139).
Its image digests predate the Gosu change and must not be reused as new evidence.

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
a network-free non-root user/group switch. These executable checks are prepared
for CI; they have not run locally because Docker/Go are unavailable and disk
headroom is approximately 720 MiB. Local tests exercise the version guard only.

PostgreSQL 18.6-r7, libxml2-16 2.15.4-r2, ABI 16, LLVM, glibc, en_US.utf8,
UTF8/libc locale, XML/TLS, volume layout and entrypoint are unchanged.
CVE-2026-39824 remains visible as UNKNOWN: its affected function is in
`x/sys/windows`, while this candidate is Linux. No severity override or suppression
is introduced. Fresh image scanning, including lower severities, is still required.

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
The operator has not approved or created this package. Do not create it or test
credentials as a side effect of validation. Confirm package access, ownership,
private visibility, storage/transfer charges and publication scope first.

For an explicitly approved publication, use a separate manual workflow with
packages:write only for its publishing job, protected environment approval and
repository-bound GITHUB_TOKEN. Publish the exact locally scanned OCI artifact;
do not rebuild between scan and push. Retain OCI manifest/config digests and
verify the registry manifest after publication. Local Docker image IDs must not
be mistaken for a remotely retrievable manifest digest. If rebuilding is needed,
scan and exercise the new artifact before publication. Do not enable publication
in the existing read-only validation workflow.

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

1. Approve code push and exactly one focused validation run.
2. Review its actual scan/runtime evidence and residual findings.
3. Approve private registry destination/publication and read-only Railway pull
   credential provisioning; verify artifact digest and private visibility.
4. Approve the exact existing-project additive resource plan, six-hour/$1 policy,
   monitoring/shutdown responsibility, synthetic credentials, isolated schema-only
   migration administration, volume retention and authenticated frontend access.
5. Separately authorize deployment. No paid inference or agent execution is included.

References: [Gosu 1.20](https://github.com/tianon/gosu/releases/tag/1.20),
[Railway private registries](https://docs.railway.com/builds/private-registries),
[GHCR authentication](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry),
[Railway pricing](https://docs.railway.com/pricing).
