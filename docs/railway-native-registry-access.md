# Railway-native private-image verification

Use the existing encrypted per-service registry credential. Do not retrieve it,
copy it to a local client, request it in chat, or prompt for it on each release.
Railway decrypts registry credentials at image-pull time:
https://docs.railway.com/builds/private-registries

## Adoption

1. Retain independent publication evidence: exact immutable GHCR reference,
   manifest/configuration/layer integrity, explicit Private visibility, private
   infrastructure repository association, and anonymous-access denial. Native
   deployment success does not prove package privacy or association.
2. Remove the local 15-minute **registry token-input receipt** precondition from
   frontend replacement controllers. Do not remove authentication-session expiry
   checks, independent repository-association receipts where required by the
   publisher, publication approvals, or the exclusive one-attempt intent journal.
3. During one separately authorized deployment, let Railway use its installed
   credentials. Never attempt a second deployment to test credentials. Do not
   change `registryCredentials`, fall back to another image, or weaken health checks.
4. Use `scripts/railway-activation-state.py` via the existing observation monitor.
   Its `deployment_ready` delegates to `native_image_access`: exact approved
   project/environment/service/deployment identity, immutable image reference,
   SUCCESS, not stopped, matching image and SHA-256 metadata are mandatory.
   Inventory checks and two consecutive health/security passes remain mandatory,
   followed by full HTTP/authentication and browser acceptance.
5. A confirmed failed/crashed deployment fails immediately. Transient read errors
   and pending/unknown states never count as success; only read observations are
   retried within the existing 300-second readiness deadline. An exhausted
   deadline or critical failure invokes only the separately reviewed frontend
   recovery procedure. Never log raw deployment metadata, registry errors or secrets.

The local operational controller at
`artifacts/ten-page-release-20261106/deployment/controller.py` now consumes this
guard without reading a registry credential receipt. Its completed attempt journal
is preserved: **do not rerun it**. A future release must bind a new journal,
approved target and current recovery image, and retain all remaining acceptance
checks. Operational artifacts remain untracked; the shared guard, tests and this
procedure are version controlled. Synchronize these commits only with approval.

## Limits of the evidence

Railway documents pull-time credential use, not a separate non-deploying test-pull
API. The supported approach establishes exact-image deployment availability at
deployment time, not before deployment. A cached image may start even if upstream
credentials have since expired; do not call that a fresh authentication test or a
guarantee that a later uncached pull will succeed. Metadata also does not establish
whether a failure was specifically authentication, networking or registry outage.
Report an image-access/deployment failure unless independent sanitized evidence
establishes the cause. If a fresh upstream authentication test is mandatory before
any deployment, this native approach cannot satisfy that stronger requirement;
stop rather than exporting the stored credential or pretending it was verified.

No new credential, Railway setting, image or deployment is needed to adopt the
local verification policy. Credential renewal remains a separate approved action
only when actually required. Current validated 10-page image remains unchanged:

```
ghcr.io/pyco404/supa-agency-frontend-infra-0c1e6e456ac8@sha256:a2a16b8ddefe2c82f82536beb7f5667ef2451958993c8228baacae58f8466cf2
```

Offline checks:

```
python3 scripts/test-railway-activation-state.py
python3 scripts/test-railway-observation.py
python3 scripts/scan-secrets.py
python3 scripts/git-policy.py audit
```

Transport tests bind only a local loopback fixture; they do not access Railway.
