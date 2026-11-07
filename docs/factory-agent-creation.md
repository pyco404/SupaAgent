# Factory: purposeful agent creation

## Current functional integration — synthetic local validation

The implementation reuses the existing signed synthetic sign-in, owner ACLs,
transactional Factory extension, purpose/consent model and optional avatar
pipeline. This development phase does not change the published website, Railway
configuration, API deployment, live database or existing security gates.

Owner-only agent details now have a dedicated `GET /factory/agents/{id}` Rust
endpoint and strict Next.js adapter at `/api/factory/agents/{id}`. Both resolve
through the creator-bound Factory projection with a current owner ACL. Missing,
other-owner and revoked IDs receive the same unavailable response. There is no
fallback to a generic accessible-agent list and no new table permissions. Detail
POSTs are unavailable; configuration is displayed read-only. Editing saved goals,
capabilities or execution authority is outside this paused-creation pilot.

`/my-agents/{id}` works even when avatar uploads are disabled. The My Agents
workspace always links to each saved agent's detail/configuration view; image
editing is offered only when the separately gated avatar pipeline is available.
Start, Pause, Resume, Stop and token launch stay disabled. Creation requires no
token, paid model or wallet. Requested capabilities are recorded for review;
autonomy remains disabled, enabled capabilities empty and spending fixed at $0.

Validation includes synthetic Sign In → Factory → review → create → My Agents →
owner details on desktop/mobile, direct detail API reads, unknown-ID/write
rejection, credential expiry, other-owner denial, revocation and actual database
restart. Avatar-free detail routing/adapter tests do not depend on the upload
flag. The existing PostgreSQL suite also verifies atomic rollback, concurrent
idempotency, schema/financial write denial and private/public image boundaries.

Run `SUPA_FACTORY_TEST_RAM=1 python3 scripts/validate-agent-factory.py` on a host
with sufficient `/dev/shm` capacity when disk space is limited. The opt-in runner
copies the existing build cache and creates only its own private RAM data
folder; it retains the strict newly generated `/tmp` socket target guard. It
stops and removes only its own disposable cluster/cache, preserving test logs
and browser screenshots in `artifacts/functional-factory`. No existing database
URL, token, account or database path is accepted as a test target.

Current disposable validation passed on PostgreSQL 18.3, including a real
stop/start and restricted runtime SQL. Desktop and mobile each created a
synthetic assistant, found its persistent ID in My Agents and read its detail
configuration. Desktop exercised upload/crop/replacement; mobile skipped upload
and received the deterministic default. Session/adapter tests (7), image-codec
tests (5), TypeScript, lint and public-boundary regressions passed. The first
browser attempt crashed on the constrained local host; after moving browser
temporary files to RAM and releasing the copied compiler cache, the complete
journey and restart checks passed. No assertion or security gate was removed.

Release still requires separate source synchronization, newly scanned API and
frontend images, private publication, reviewed synthetic database extensions /
EXECUTE grants, bounded synthetic account provisioning and controlled deployment.
Both Factory and avatar feature flags remain default-off. No public registration
or sensitive records may be introduced before independent encrypted-backup
recovery and security validation are complete. Image/container scans are release
gates; local test success does not replace them.

## Authenticated synthetic creation milestone — local, November 7, 2026

The existing local pilot implements **Sign In → Factory → review configuration
→ Create Agent → My Agents**. Its source is already present in the application;
the feature remains disabled by default and is not activated against the live
Railway database. The preview description below records the released,
non-creating interface. This section describes the reused foundation.

Reuse: existing named server-side credentials, Rust authentication middleware,
principal UUIDs and owner ACLs; `Agent`, personality, goal and capability models;
PostgreSQL storage; `PurposeRevision` validation; ecosystem private-consent
records; the existing Next.js host and approved terminal design. No wallet,
token, inference, scheduler or publishing integration is invoked.

New implementation:

- `/sign-in` accepts only explicitly provisioned synthetic accounts. It checks
  the live backend credential, role and deadline before issuing a signed,
  HttpOnly, Secure, SameSite=Strict cookie. Sessions last at most 15 minutes;
  the backend's existing credential deadline can expire sooner. Credentials
  never enter localStorage, public HTML, cookies, URLs or logs. There is no
  registration endpoint, implicit owner claim or permanent session. The cookie
  admits only the Factory adapter and owner-management pages, not `/operator`
  or the older console APIs. The only extra anonymous staging-host resources
  are sign-in HTML and the reviewed stylesheet/logo/font asset allowlist.
- `/factory/create` collects bounded name, description, purpose, personality,
  goals, requested capabilities and explicit private-storage consent. A readable
  review precedes submission. Autonomy is fixed to disabled and spending to zero.
  Requested capabilities are review intent, **not enabled permissions**. All
  launch and lifecycle controls remain disabled. A token is not required.
- `/api/factory/agents` is a same-origin, bounded JSON server adapter. It maps the
  authenticated session to its existing Rust credential; it does not accept
  browser-supplied identity, role or owner flags. Responses use strict schemas.
  No generic URL, arbitrary backend method or credential forwarding is exposed.
- Rust `/factory/agents` requires the explicitly enabled pilot and a named
  synthetic Owner-role credential. This role scopes creation of that principal's
  own records; it does not provision real users or grant ownership of old agents.
  Anonymous, service-only, viewer and expired credentials are rejected. Epoch
  write protections remain intact. The dedicated main-process pilot branch
  starts no scheduler, recovery writer, wallet integration or worker monitor.
- A single SQL transaction creates a private paused agent, owner ACL/audit,
  lifecycle/activity evidence, goals, private purpose, revoked/empty public
  disclosure consent and durable submission receipt. Purpose qualification stays
  `pending_review`; creation does not fabricate independent approval. There is
  no public listing, balance, financial entry, memory or token association.
- Creator/request-key idempotency survives restarts. Conflicting reuse fails.
  One creator advisory lock serializes concurrent replays and the 100-submission
  pilot quota. The browser locks a submitted configuration and reuses its key
  after an ambiguous response; it does not retry writes automatically. My Agents
  can be checked before a manual retry. No private drafts are stored in browser
  persistence.
- My Agents lists only records created through this pilot and still owned by
  the authenticated creator. Shared access, token ownership and existing staging
  records never populate it. It displays saved configuration, paused state and
  pending qualification. Start, Pause, Resume, Stop and token launch are disabled.
  Existing agent mutation endpoints reject pilot-managed agents; a database
  trigger also freezes their underlying agent state until separately reviewed.

`deployment/agent-factory.sql` is an explicit extension, outside the unchanged
13 migrations. It requires the existing reviewed `ecosystem-social.sql` extension.
Its fixed-search-path SECURITY DEFINER functions allow only new private creation,
owner-bound projection and a managed-agent check. PUBLIC has no schema, table or
function rights. The dedicated test login has only feature-schema usage,
EXECUTE on those functions and SELECT on migration/epoch/access metadata needed
by existing checks. It has no direct table-write, migration, role-management,
financial or accounting privileges. Functions trust only the principal supplied
by the authenticated server, not a browser claim. This is an application-service
database role, not a separate database login or grant for every website visitor.

Validation: `python3 scripts/validate-agent-factory.py` creates a new socket-only
PostgreSQL 18 cluster, applies the original migrations and explicit extensions,
tests the restricted role and real Axum authentication, then runs Chrome desktop
and mobile through Next.js → Rust → PostgreSQL. It restarts PostgreSQL and checks
replay persistence, owner isolation and revoked-ownership denial. It also covers
concurrent identical/different submissions, conflicting keys, invalid inputs,
blocked lifecycle operations, raw SQL denials, zero balances and atomic rollback
after an injected late database failure. The fixture is stopped and disposed;
sanitized logs and screenshots remain in `artifacts/functional-factory`.

`node scripts/test-factory-session.mjs` covers signature tampering, duplicate
cookies, expiry, account revocation, default-off behavior, exact same-origin
writes, sign-in assets and exclusion of operator/private APIs. Existing public
exploration and landing regressions remain required, alongside Rust, TypeScript,
lint, history and secrets checks. Local browser success does not constitute a
new image vulnerability scan or production-security certification.

### Remaining approvals and release work

1. Review and approve the exact local application commits for synchronization.
2. Review a frontend/API build pinned to the new revision. The infrastructure
   frontend recipe must package `reference-web/factory-create.html` as well as
   `my-agents.html`; update asset expectations, source/recipe pins and runtime/
   browser gates. Do not silently reuse the old image or recipe checksum.
3. Separately authorize image builds/scans, retained-archive private publication
   and controlled backend/frontend deployment. HIGH/CRITICAL and secret findings
   continue to block release. Existing immutable images are unchanged.
4. Separately approve installing the explicit extensions and minimal function
   grants in an isolated synthetic staging target. No live database, permissions,
   migrations or fixtures changed in this milestone. Public registration and
   unrestricted user-data storage remain unavailable.
5. Separately approve bounded synthetic account provisioning/renewal and setting
   `SUPA_SYNTHETIC_FACTORY_ENABLED=true` on the dedicated API and frontend pilot.
   Keep existing private networking, verified database TLS, origin checks and
   all financial/provider/publishing controls. No activation is implied here.
6. Run one controlled end-to-end demonstration. Independent purpose approval,
   public listing consent, lifecycle execution, nonzero budgets and token launches
   each require further reviewed implementation/authority. They are not needed
   to create a private paused synthetic agent.

## Agent appearance — optional local pilot

The appearance step extends the same authenticated synthetic journey. Both
`SUPA_SYNTHETIC_FACTORY_ENABLED=true` and `SUPA_AGENT_AVATARS_ENABLED=true` are
required; defaults remain off. No public registrations, real accounts, live
uploads, storage provisioning, deployments or production database changes are
implied by this implementation.

- **Upload Image:** PNG, JPEG or WebP, at most 5,000,000 actual streamed bytes.
  The browser provides a square crop with zoom and horizontal/vertical position,
  preview, explicit acceptance, replacement and removal. Filename/path strings
  are not storage identifiers. Client checks are conveniences; server checks are
  authoritative. SVG, unsupported formats, content-type mismatches, malformed or
  truncated data, animations, invalid crop coordinates, dimensions above 4096
  and inputs above 16,777,216 pixels are rejected. Server-side Sharp 0.35.5 fully
  decodes, auto-orients, crops, resizes to 512×512 sRGB PNG and strips all ancillary
  PNG chunks. Originals, EXIF, ICC data and user filenames are not retained.
- **Generate with AI:** visibly unavailable. No provider call, generation endpoint,
  budget, regeneration loop or paid inference is enabled. Name, purpose,
  description and personality must not leave the platform for generation until
  its service, consent, cost and authorization gates are separately configured.
- **Use Default Avatar:** deterministic identicon from a versioned SHA-256 seed.
  Image-free creation remains the original creation transaction and consumes no
  upload quota. Existing records without an image render their default on read;
  they do not require an external service, payment, stored upload or token.
  The default seed is the persisted submission UUID, keeping the pre-creation
  preview and subsequent profile appearance consistent.

My Agents displays private avatars and links to authenticated individual profiles.
Owners can upload, replace or revert their profile image, with a revision check
that rejects concurrent/stale updates. Removing a staged image deletes only an
unattached asset belonging to that uploader. Removing a saved profile avatar
means explicitly saving a default revision; historical versions needed for audit
or token-image snapshots remain private and retained. Image editing never updates
frozen agent execution/configuration records.

### Storage and trust boundary

`deployment/agent-avatars.sql` is a separately installed extension, not a change
to the 13 automatic migrations. It requires the existing Factory and ecosystem
extensions. Normalized PNG bytes use private PostgreSQL `bytea` records with
SHA-256 integrity checks; no bucket, public filesystem path or new credentials
are created. The application login receives schema USAGE and EXECUTE on only
`agent_avatars.command` and `agent_avatars.create_agent`; it receives no direct
image table, financial, migration or role-management write permissions.

The Next.js server is the image decoder/normalizer. A domain-separated HMAC
receipt binds the normalized-byte checksum, image kind, request UUID and current
owner API credential using the existing private service-authentication secret.
The Rust API independently verifies this receipt and the byte checksum before
admitting bytes. An ordinary owner bearer token cannot bypass normalization by
asserting that arbitrary bytes are safe. The receipt is not sent to the browser
or retained in SQL. This is a trusted processor boundary, **not** a claim that
Rust independently decodes the original image or that a compromised application
service/database administrator is an untrusted website visitor.

Private image reads recheck ownership, including revocation. Public reads require
an approved current avatar revision **and** current, non-revoked creator consent
for public `agent_created` disclosure **and** an explicit public purpose summary.
The public directory/profile projection contains only canonical ID, name,
creator-approved summary and avatar revision. It never falls back to private
agent lists, goals, personalities, memories, financial records or staging data.
Unlisted/revoked profiles and images return unavailable; replacing an approved
avatar requires a fresh image approval. Public image approval alone cannot
publish an agent. Public/private PNG responses use fixed MIME, `nosniff`,
no-store caching and no user-controlled filenames. Existing staging-host and
cross-origin restrictions remain enforced.

Limits: two concurrent decoder slots per frontend process, bounded upload reads,
five-second codec timeout, ten authenticated upload attempts/minute and thirty
image-management commands/minute per process, plus a durable twenty accepted
uploads/hour per creator. PostgreSQL serializes quota/update checks per creator,
limits storage to 20 MiB/100 assets per creator and retains at most fifty avatar
revisions per agent. Idempotent retries do not create another stored asset.
Temporary unbound uploads expire after one hour, become unreadable, and are
removed on the next upload/cleanup call. Explicit removal/cleanup never deletes
bound versions or token snapshots. No new worker or scheduled task is activated.

### Token image preparation

An owner must save an actual image revision, inspect it and explicitly confirm
its checksum for future token use. A separate immutable token-image record pins
that revision, exact stored PNG bytes and SHA-256. It remains unchanged after
later avatar replacement, default reversion, visibility changes or restart. A
second confirmation cannot silently overwrite the first. A virtual default must
first be saved as an image version before confirmation. This confirmation does
not publish the image, qualify an agent, authorize a launch, generate a mint
signer, transfer rights or execute Pump.fun. Original-mint permanence, uniqueness
and separately authorized successor history remain untouched. Future exceptional
successor-image policies need their own review rather than rewriting a snapshot.

### Evidence and release requirements

`node scripts/test-avatar-images.mjs` tests the actual codec, all three input
formats, orientation, square crop pixels, metadata removal, deterministic
identicons, malformed/SVG/animated/oversized inputs, stream limits/timeouts and
bounded processing. `python3 scripts/validate-agent-factory.py` additionally
exercises authenticated image receipts, private/public reads, explicit consent,
replacement, CAS/idempotency, quota exhaustion, temporary cleanup, atomic image
attachment and token-image preservation through a real PostgreSQL 18 restart.
Large normalized PNGs exercise the image-specific body limit without broadening
legacy request limits. Default creation still succeeds when image quotas are
exhausted. Real desktop/mobile Chrome runs cover upload/crop, image-free creation,
My Agents/profile display, replacement, default reversion and token-image
confirmation. An isolated GET-only hostname adapter tests approved public
profiles against the real local backend, without contacting live domains.

Evidence is retained under `artifacts/functional-factory` and
`artifacts/avatar-tests`. TypeScript, lint, session/origin/public-boundary tests,
secrets scanning and one-file Git audits remain required. These local checks do
not replace new API/frontend container vulnerability and embedded-secret scans.

The prior milestone's synchronization/build/publication/deployment approvals
remain pending. A future reviewed frontend recipe must include both new HTML
assets (`factory-create.html`, `public-agent-profile.html`), the pinned Sharp
runtime dependency and the new server modules. Review the exact source/recipe
pins and image/profile browser gates; do not reuse an old artifact or checksum.
Separately approve installing the image extension/function grants in the isolated
synthetic target and enabling its flag after scanned images are accepted. Keep
AI generation, agent execution, token launches, wallet signing and publishing
disabled. No live Railway/backend/database or DNS changes occurred here.

## Previously released preview

Local frontend preparation only. This does not change the running website,
Railway configuration, database, grants or execution capabilities.

Main navigation: Home, Agents, Intelligence, Memory, Economy, Social, Trading,
Factory, About. Home and Agents link prominently to Factory. Old `/token-launch`
links lead to Factory; launch-related domain and backend work remains intact.

Factory presents the intended sequence, with every unfinished action disabled:

1. **Create Agent:** name, purpose, description and personality. Authenticate
   before accepting input; do not collect private details in the public preview.
2. **Configure:** capabilities, goals, permissions, autonomy and spending limits.
   Begin paused with zero authority and no paid or financial execution.
3. **Purpose Qualification:** validate purpose, creator identity, consent and
   approvals. Qualification grants neither execution nor launch permission.
4. **Agent Token (optional):** skipping must preserve normal agent creation.
   Planned Pump.fun integration requires separate authorization, signatures and
   verified confirmation. The platform's planned $SUPA mint is separate. Keep
   permanent original-mint uniqueness, immutable history and separately disclosed
   successors; no implicit transfer of holder balances, claims or permissions.
5. **My Agents:** authenticated owner management at `/my-agents`. The current
   placeholder loads no records and has disabled Start/Pause/Resume/Stop buttons.
   Staging visitor credentials do not grant this operator-backed route access.
   Authentication is a gate, not proof of agent ownership.

Agents remains an honest public directory: no fabricated entries, balances,
tokens, achievements or activity. Supa Agent is introduced as the flagship
personality without presenting its editorial introduction as a verified live
profile. Future listings require creator consent and sanitized public evidence.

## Minimum implementation sequence

- Reuse the Rust `/agents` create handler and `create_owned` transaction after
  reviewing its request contract, principal roles, paused defaults, input bounds
  and idempotency. Existing `/agents` listing returns *accessible* agents, not
  necessarily only owned agents. Do not label it "My Agents" without an
  authoritative owner filter; `/directory` also needs explicit public-consent
  filtering before being used on public pages.
- Add authenticated account onboarding and owner-scoped frontend API adapters.
  Existing synthetic operator/visitor credentials are not general user accounts.
  Require same-origin requests, backend authorization and explicit ownership on
  every read/write. Never trust a browser-provided creator ID or owner flag.
- Connect purpose/consent persistence and qualification using the existing
  `crates/api/src/ecosystem.rs` handlers; do not infer creator consent from token
  ownership. Keep partially completed configuration recoverable and auditable.
- Integrate owner-scoped lists/details and separately authorized lifecycle
  controls using the existing lifecycle/pause/resume/emergency-stop handlers.
  Review finite budgets, worker-disabled behavior, revocation, stale revisions,
  concurrency and restart recovery before enabling any execution.
- Validate against disposable PostgreSQL and synthetic identities: cross-owner
  denials, unauthorized writes, duplicate submissions, zero balances, paused
  creation, no token required, and privacy of goals/memories/credentials. Public
  browsing must continue making no backend reads or writes.

Creation, lifecycle execution, token launches, wallet signing, paid inference,
trading and X publishing remain unavailable in this release. No backend grants or
database migrations are implied by the navigation change. Synchronization, image
validation, publication and deployment each remain subject to approval.

## Local verification and release

Run `node reference-web/tests/public-exploration.test.mjs` and
`node reference-web/tests/public-landing.test.mjs`, then run
`node web/node_modules/@playwright/test/cli.js test --config=reference-web/factory.playwright.config.mjs`
with Node 24.22.1. The isolated Next.js development fixture uses only synthetic
credentials, maps browser traffic to loopback and never contacts Railway or a
database. It covers anonymous, invalid, staging-visitor, viewer and operator access,
cross-origin rejection, disabled actions and zero public backend requests/writes.
Run TypeScript and lint checks in `web`, plus the relevant existing navigation
and layout browser regressions. Screenshots are retained under
`artifacts/factory-flow-20261106`.

Before a separately approved image validation, update the infrastructure browser
expectations from ten sections to these nine, retain a compatibility check for
old `/token-launch` links and require `/my-agents` authentication. Do not reuse
the old image or skip the existing vulnerability, secrets and boundary gates.
