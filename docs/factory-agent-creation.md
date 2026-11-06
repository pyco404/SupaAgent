# Factory: purposeful agent creation

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
