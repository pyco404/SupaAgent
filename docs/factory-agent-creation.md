# Factory: purposeful agent creation

## Authenticated synthetic creation milestone — local, November 7, 2026

The local pilot implements **Sign In → Factory → review configuration → Create
Agent → My Agents**. It is disabled by default and has not been pushed, built
into release images, deployed or applied to a live database. The historical
preview description below documents the currently released interface.

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
