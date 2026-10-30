# Milestone 5 — full-stack integration and private staging preparation

> Current status: see [revalidation on 2026-10-30](#revalidation-on-2026-10-30).
> The original results below are historical. Scoped authentication has superseded
> the original shared-token staging recipe. No staging deployment is active.

The approved experience now runs through a Next.js server with the existing Rust
API and PostgreSQL. No redesign, logo change, deployment, paid inference, signing,
transfer, trade, publication or token launch was performed.

## Integration inventory

| View | Data and implementation | Access / limitations |
| --- | --- | --- |
| Intro, Home, About | Existing code typing, Supa code mascot, portal, keyboard/hands, engineering diagrams; API health and identity count | Artwork/topology is explicitly decorative, not observed runtime activity. Product vision is editorial content. |
| Agents | Persisted identity UUID, name, created timestamp and lifecycle | Public identity projection excludes goals, memory, personality, config and capabilities. |
| Intelligence / profile | Agent record, lifecycle, creation time, saved wallet network/address, goals and suggested progress, reasoning cycles, authorization outcomes, tool status and activity | Operator only; history endpoints are bounded pages, not lifetime totals. Model progress is an assessment rather than proven completion. |
| Memory | Saved lessons, verified observations versus hypotheses, event/cycle references | Operator only. No reclassification of hypotheses into facts. |
| Economy | Simulated microUSD balance/revenue/inference/operating expenses and ledger, runway assumptions, Devnet identity/balances/treasury/receipts | Exact integer scaling; saved observation time and retrieval time remain distinct. Null runway is explained. Mainnet unavailable. |
| Social | Original stored draft wording, style, provenance, review disposition, rejection/abstention reasons | Operator only; no draft generation or X publishing occurs on page visits. |
| Trading, Token Launch, Factory | Existing planned/non-executable disclosures and previews | No execution API is exposed by the frontend. |

Existing record integrations were reused. Added profile summaries, 15-second
read-only polling and a Next.js host for the existing HTML/CSS/JS/assets. Polling
pauses in hidden tabs, during typing, navigation drawers and utility/picker
interactions. Unchanged text is retained. A reader focused in a record is not
interrupted by a background rerender; changed private content appears after focus
leaves that record and a subsequent poll runs. No polling operation starts cycles,
generates drafts or changes lifecycle.

Local `reference-web/server.mjs` remains loopback-only. The original Next.js
operator console is preserved. `SUPA_SITE_MODE=staging` selects the approved
experience at the Next.js root through server rewrites. Its assets are served
verbatim rather than reimplemented as a different React interface.

## Permissions and worker separation

The staging gateway accepts independently configured visitor or operator Basic
credentials over an exact configured HTTPS origin. Visitors can read only API
health and the minimal identity projection. Private read APIs independently check
operator authentication. Host/origin checks reject unrelated domains and
cross-origin requests. Backend forwarding allows loopback by default and explicit
Railway private service hostnames only in staging, with no redirects, bounded
response sizes, schema validation and credential redaction. Tokens remain server
side. The frontend exposes no mutation methods.

The API has a distinct minimal `/ready` database readiness route; `/health` and
agent endpoints retain bearer authentication. Staging disables the embedded API
scheduler and rejects paid providers. The separate `supa-runtime` executable
starts paused by default, creates no provider while paused, and requires an
explicit launch setting to enable offline scheduling. OpenAI is intentionally
unavailable in this staging worker; enabling paid autonomy needs a separate
budgeted authorization workflow. Existing live evaluation tooling is preserved.
Both processes handle SIGTERM. Recovery touches expired leases and does not
reactivate paused, suspended or terminated identities.

Saved wallet reads no longer need vault access. Optional
`SUPA_WALLET_READ_OPERATOR_ID` scopes identity, balances, treasury, payment and
payment-check GETs to the persisted owner UUID. Wrong owners are denied. All wallet
mutations still require the vault-backed service; the staging image disables it.
Tests remove the vault and service before proving this separation.

Basic authentication is suitable for this restricted staging foundation, not a
completed public account/role system. It lacks per-user sessions, MFA, role
administration and a dedicated login/logout UX. Add rate limiting and account
authentication before a broad public release. Private service URLs must remain
unpublished; a bind opt-in does not itself configure Railway's network exposure.

## Actual end-to-end evidence

`scripts/staging-smoke.mjs` requires a dedicated local `*_test` PostgreSQL database,
generates temporary credentials internally and launches actual Rust API, Next.js,
worker and Chrome processes. It never loads evaluation configuration or creates
real financial records. This run used a new `supa_m5_test` database, leaving the
evaluation and Devnet databases separate.

The synthetic agent completed **one deterministic/offline cycle**, persisted
**two memories** and **one successful tool action**, and incurred **1100 simulated
microUSD** of inference/operating expenses. Visitor private reads and frontend
POSTs were rejected. The API was stopped and restarted: lifecycle remained paused,
memory/cycle counts and saved financial values were unchanged. The standalone
worker was healthy while globally paused. These are working process/database
integrations, not live GPT intelligence evidence.

A separate real worker-process regression used an isolated PostgreSQL schema to
complete at least three autonomous offline cycles, persist permitted Remember
actions and lessons, and record simulated expenses. After explicit pause and
SIGTERM, a fresh worker without its enable flag retained the paused lifecycle,
cycle/memory counts and treasury. Its temporary schema was cleaned up without
selecting or changing other agents.

See [system-check.json](../artifacts/milestone-5/system-check.json) and the
reproducible check:

```sh
cargo build -p supa-api -p supa-runtime --bins --locked --offline
npm --prefix web run build
# Point TEST_DATABASE_URL only at a disposable local database ending in _test.
node scripts/staging-smoke.mjs
```

The script defaults to a fresh temporary evidence directory; historical reports
are not overwritten. `SUPA_STAGING_EVIDENCE_DIR` can select a new review directory.
An optional loopback `SUPA_REA_CDP_PORT` keeps the owned browser/services available
for REA inspection until SIGTERM. It does not expose the application publicly.

## Real Devnet versus mocked evidence

Two independent `devnet_verify --check` processes accessed the configured Devnet
RPC using the existing dedicated identity. The observed genesis was
`EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG`. Finalized canonical mint
`4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU` had six decimals. Wallet
`4imA97URQuuFYbSKNum5dWCqUmRNenmscEcVpAju5S7U` had **0 SOL lamports and 0 USDC
atoms**. The first balance observation was at slot 516130218; the second process
report is [devnet-reconnect.json](../artifacts/milestone-5/devnet-reconnect.json).
Observations were persisted in the dedicated Devnet database, without changing
receipt totals or simulated balances.

No actual token receipt was verified: no suitable funded test transfer and
signature were available or authorized. Existing mock RPC/PostgreSQL tests prove
mint/network/owner checks, finalization handling, duplicate/concurrent observations,
idempotent credit, failure recovery and signing denial. They are not actual chain
receipt evidence. The configured RPC is trusted; no light client or provider
quorum was added. Existing vault limitations remain documented in Milestone 3A.

## Browser evidence and focused quality review

Actual PostgreSQL-backed screens were checked at **1440×1000, 768×1024 and 390×844**.
Intelligence, memory, economy and social rendered saved data/empty states without
horizontal overflow or page errors. Real staging intro typing, portal entry,
terminal utility controls and full-screen mobile navigation were exercised.
Separate deterministic browser fixtures cover loading, disconnected, empty and
error recovery, permissions, all ten views, scrolling, pointer/keyboard/touch
controls, reduced motion and existing animations.

Screenshots:

- [Desktop intelligence](../artifacts/milestone-5/desktop-intelligence.png)
- [Desktop economy](../artifacts/milestone-5/desktop-economy.png)
- [Tablet intelligence](../artifacts/milestone-5/tablet-intelligence.png)
- [Mobile intelligence](../artifacts/milestone-5/mobile-intelligence.png)
- [Mobile intro](../artifacts/milestone-5/mobile-intro.png)
- [Mobile navigation](../artifacts/milestone-5/mobile-navigation.png)

REA `list_browser_targets`, `capture_web_screenshot` and passive `inspect_web_page` observed desktop
intelligence, tablet memory, mobile social and mobile navigation. Content hashes,
viewport sizes and Evidence IDs are in
[rea-evidence.json](../artifacts/milestone-5/rea-evidence.json). The installed
Impeccable detector ran against the loopback CDP render and returned zero findings.
The approved typography, muted palette, symbol-only header, diagram geometry and
motion were preserved. Screenshots are state evidence; automated interactions
provide behavior evidence. Chrome emulation is not a physical mobile-device test
or a full accessibility certification. Terminal microtype remains an intentional
approved aesthetic with readability limits; this task did not redesign it.

A sampled homepage loaded one application script entry and approximately 299 KB
of decoded same-origin resources. There is no new client animation dependency.
This is a local observation, not a Lighthouse/Core Web Vitals or Railway latency
measurement. Keyboard focus and reduced motion were checked; no new global motion
was introduced.

## Validation and release boundary

Commands and machine-readable results are summarized in
[validation.json](../artifacts/milestone-5/validation.json), with Rust details in
[rust-tests.txt](../artifacts/milestone-5/rust-tests.txt).

Rust formatting, Clippy with warnings denied, and all 104 workspace tests passed
including opt-in database/socket tests. Next.js build, ESLint and TypeScript
checks passed. Its preserved console passed 144 browser tests. The adapter passed
10 unit tests and infrastructure passed six SDK-backed offline tests. The first
experience run passed 122/123: one desktop reduced-motion button assertion was
transiently out of sync. All six focused follow-up tests passed; the subsequent
full suite passed all 126 tests, including polling at all three viewport sizes.
No application behavior was weakened to suppress the assertion.

Compiled browser bundles were scanned for known local secrets and credential
patterns: 26 bundles, zero findings. The repository scanner's eight self-tests
passed. Candidate/staged/history scans and the mandatory one-file history audit
are release gates. The unrelated root `.gitignore` edit remains unstaged.

The current, SDK-validated Railway IaC, Dockerfiles, environment-variable table,
private networking, startup/migration order, fresh PostgreSQL storage and recovery
plan are in [railway-staging.md](railway-staging.md). No local evaluation secrets,
wallet envelopes, simulated treasury records or test credentials belong in those
services. Docker is unavailable here, so container builds and image scans remain
unverified. The SDK's PostgreSQL helper selects its PostgreSQL 18 image; local
database validation used PostgreSQL 16. Validate migrations and persistent volume
behavior against the actual staging image before deploying. Railway account
configuration, fresh shared variables, exact approved
domain, private connectivity, backup/restore and deployed readiness still need
verification in an explicitly approved staging workflow. No push or deployment
has been performed.

## Revalidation on 2026-10-30

### Outcome and preserved boundaries

The full stack operates in an isolated test environment: PostgreSQL 16, the Rust
API, a globally paused worker, Next.js serving the approved experience, and Chrome.
This is not activation of the prepared development epoch or a Railway deployment.
The primary cluster stayed stopped. Its offline control-file fingerprint, pending
owner files, development intent and archived accounting evidence remained unchanged.
The historical database remains unrecovered. The quarantined budget
`ad91e475-f1da-495a-8247-139643d2d8b1`, $0.10 ceiling and unresolved $0.004048 hold
were not imported, refunded, reused or reconciled. Independent backups remain unset.

All twelve checked visual source/asset files match baseline
`ecf6b163f375aa459eac3ed23e618357ddbf901b` byte for byte. All ten sections, logo,
intro, portal, typing hands, branch motion, wheel, typography and layouts are
preserved. No visual application component was edited.

### Current integrations

| Section | Genuine integration | Static, simulated or planned boundary |
| --- | --- | --- |
| Home | Authenticated API health and public identity projection | Architecture diagram is decorative; product vision is editorial. |
| Agents | Persisted identity, UUID, lifecycle and creation time | Public projection excludes private instructions, goals and memory. |
| Intelligence | Scoped profile, goals, cycles, actions, execution health, authorization history and bounded offline controls | Live GPT remains disabled in staging. A heartbeat or authorization is not proof of execution. |
| Memory | Scoped saved observations, hypotheses and event/cycle references | Hypotheses remain unverified; no invented lessons on page load. |
| Economy | Scoped simulated financials and ledger; saved wallet identity/observation endpoints | Hypothetical USD is not real money. Absent wallet records remain unavailable. |
| Social | Scoped stored drafts, provenance and review status | Empty data remains empty. No draft generation or X publishing on visits. |
| Trading | Navigation and explicit availability disclosure | Planned; no executable trading API. |
| Token Launch | Navigation and explicit availability disclosure | Planned; no Pump.fun launch action. |
| Factory | Navigation and explicit availability disclosure | Planned; no child-agent creation. |
| About | Existing identity diagram and explanatory content | Editorial explanation, not an observed runtime. |

Existing read-only polling retains the 15-second cadence and pauses around hidden
tabs and active editing/navigation. Database and HTTP permission tests cover
private memory, execution history, social records, financial data and wallet reads.
No browser identity/role field chooses an upstream Rust principal.

### Implementation changes

The staging IaC still used the original shared-token recipe. That token now has
only public-service scope in Rust, so it could not operate the scoped console.
The corrected plan supplies `SUPA_OPERATOR_CREDENTIALS` to the API and
`SUPA_WEB_OPERATORS` to Next.js through separate secret references. Each operator's
Basic account maps to its own server-side Rust token. Public-service, visitor and
operator credentials cannot be shared. Legacy shared operator authentication is
disabled specifically in staging; local compatibility is preserved.

Staging preflight validates bounded JSON arrays, unique non-nil UUIDs, allowed
roles, token/password formats, matching frontend/backend operator tokens and
separation from visitor credentials. Diagnostics report configuration names,
not values. The API rejects missing/ambiguous staging credentials, paid providers,
controlled inference, embedded scheduling, wallet execution and legacy owner
configuration before database access. Existing ownership and per-tool checks remain.

The smoke harness now creates only synthetic test principals and uses the current
finite schedule authorization contract. Its offline panel selector is explicit
because Intelligence now contains three separate control panels. Test credentials
are generated in memory, local dotenv loading is disabled, and no real owner
identity, grants, configuration profiles or spending authorizations are installed.

### Actual end-to-end demonstrations

The final fresh test database persisted two test identities:

- One explicitly authorized offline cycle: two memories, two tool-action records,
  1,100 simulated microUSD of expenses, then explicit pause.
- The bounded offline rehearsal: three cycles, six memories, eight action records,
  3,300 simulated microUSD of expenses and 6,700 simulated microUSD remaining.
  It finished paused with scheduling disabled.

The frontend performed the rehearsal against the actual Rust API and PostgreSQL.
Replay returned HTTP 409 without another identity, cycle or charge. The public
service token and an unrelated principal could not read private memory, even with
forged owner headers. Visitors could read the minimal directory but not private
records. Arbitrary frontend mutation routes were rejected. After API restart,
paused lifecycle, cycles, memories and simulated accounting persisted unchanged.
The separate worker reported readiness while globally disabled. No paid request,
transfer, trade, publication, token launch or child agent occurred.

The social and wallet rendering tests use explicit synthetic fixtures. Rust tests
verify persisted social review and read-only wallet contracts. The final full-stack
database had no wallet observations or social drafts to invent; its UI showed
unavailable/empty states. These are distinct from the live chain observations below.

### Real Devnet versus mocked verification

Read-only calls to the public Solana Devnet endpoint succeeded at
`2026-10-30T19:35:34Z`. The genesis matched
`EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG`. The canonical USDC mint
`4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU` had the expected token-program
owner and six decimals. The documented public wallet
`4imA97URQuuFYbSKNum5dWCqUmRNenmscEcVpAju5S7U` held zero SOL lamports at slot
516881507 and zero USDC atoms at slot 516881509, using finalized reads.

These observations were not written into the historical or prepared database and
did not require custody files. No actual token receipt was verified: suitable
funding and a specifically authorized transfer/signature were not available.
Mint/owner/network validation, finalization, receipt idempotency, concurrency,
restart recovery and signing denial remain covered by mocked-RPC/database tests.
They do not establish that a new real receipt occurred. GPT inference tests use
mocked providers; this review made no live paid inference request.

### Backup, restoration and service plan

After shutting down the owned smoke services, a private custom-format dump of
the disposable database was restored into a different, newly created test
database. All non-system schemas, data, constraints, indexes and sequence state
matched the source fingerprint. The source fingerprint stayed unchanged:
`8798b2996d7418bdb318be3df4bd2b1fb9b4e6446df6f2a5845fc73d4bd81bbd`.
The dump was mode 0600 inside a private directory. Both the disposable cluster and
primary cluster were stopped afterward. This same-disk test is not proof of
Railway volume recovery or protection from disk loss. No private dump was uploaded.

The four-service plan remains Next.js, Rust API, paused worker and fresh PostgreSQL
with a persistent database volume. Only the frontend may later receive an approved
HTTPS domain. API/worker/PostgreSQL remain private, with no public TCP proxies.
Use a new isolated staging environment, never the unrecovered history or prepared
local epoch. Startup/migrations on an actual staging database still require approval;
runtime image rollback does not reverse database migrations.

The exact environment table, migration order, health checks, backup requirements,
and rollback boundaries are maintained in [railway-staging.md](railway-staging.md).
Required configuration groups are:

- Frontend: `SUPA_SITE_MODE`, exact `SUPA_SITE_ORIGIN`, visitor username/password,
  `SUPA_WEB_OPERATORS`, public-service `SUPA_API_TOKEN`, private `SUPA_BACKEND_URL`,
  and `PORT`.
- API: private `DATABASE_URL`, `SUPA_PRIVATE_STAGING=true`,
  `SUPA_OPERATOR_CREDENTIALS`, the matching public-service token, bind/port,
  `SUPA_PROVIDER=offline`, and explicit false settings for API scheduling, wallets,
  `SUPA_ALLOW_OPENAI` and `SUPA_CONTROLLED_INFERENCE`.
- Worker: private database URL, private bind/port, offline provider,
  `SUPA_WORKER_ENABLED=false`, bounded poll interval, paid/controlled inference false.
  It receives no API/operator secret through the IaC definition.

No actual staging identity, secret or account resource was created. A real
administrator must choose and approve staging principals/roles, configure fresh
secret values and explicitly grant any future agent access. Do not use the
pending local KI Daniel credentials for this new environment.

### Browser and focused design evidence

The final passive REA screenshots, after restoring the owning browser's viewport
and reduced-motion settings, show real PostgreSQL-backed screens:

| Capture | Viewport | REA Evidence ID |
| --- | --- | --- |
| [Desktop Intelligence](../artifacts/staging-revalidation-2026-10-30/rea-desktop-intelligence.png) | 1440×1000 | `ev_6a19b628b4778b4a4d3dd0d606ded8d4ff79a44ab76d610e5b4c01a0aea2f32a` |
| [Tablet Economy](../artifacts/staging-revalidation-2026-10-30/rea-tablet-economy.png) | 768×1024 | `ev_819fed93e80d1f84b9dd255e347e5b9dae4a98dd335634c2619e73e4bc1d56d3` |
| [Mobile Home](../artifacts/staging-revalidation-2026-10-30/rea-mobile-home.png) | 390×844 | `ev_ed6ef6ae3d3e8512799223ebbaf86884c51f31a135d3f3ca42cc386b021f0f65` |

REA itself executed the terminal `supa memory` navigation
(`ev_68093a53ded299b7d90e4f2fe5ad025fa1264a16fd472f8ce3103abd57e1d04a`) and
mobile navigation open/Escape-close checks
(`ev_ed369d810c8cccf712db7326171bf23f89037fd99263d0f766a1899a25635b04`). Every
declared interaction step completed. Attached-session event coverage is incomplete;
it cannot establish absence of earlier browser errors. Transient scenario screenshot
emulation was inconsistent, so those screenshots are not the final viewport evidence.
The final passive captures follow independently verified CSS viewport measurements,
reduced motion, readiness and no horizontal document overflow. Chromium emulation
is not a physical-device test or accessibility certification.

The Impeccable detector reported four warnings: a real 3.5:1 status-label contrast
issue and three possible SVG/wheel-label overlaps. The latter were not reproduced
as painted occlusion in the final desktop capture and remain unsuppressed. These
are existing visual concerns, not claimed regressions; no design change or broad
ignore rule was introduced. The contrast issue needs a narrowly approved
accessibility pass before claiming WCAG AA compliance.

Focused audit: accessibility 2/4, performance 3/4, theming 4/4, responsive behavior
3/4, implementation integrity 4/4: **16/20, provisional Good**. The product-specific
system is preserved. This is a bounded review, not a complete-site certification or
Lighthouse/Core Web Vitals result. Recommended later workflows: targeted
`impeccable harden` for verified contrast/accessibility concerns, then
`impeccable polish`, preserving the approved identity.

### Validation results

| Check | Result |
| --- | --- |
| Rust workspace excluding migration-admin, including opt-in PostgreSQL/socket tests | 121 passed; zero failed or ignored |
| Migration-admin unit/security tests | 3 passed; 1 specialized Stage 2 clone test intentionally not rerun |
| Final API staging-policy unit tests | All 4 passed; includes malformed/duplicate identities and unsafe launch flags |
| Real process startup gates | Seven unsafe staging settings rejected before database access |
| Adapter, workflow and epoch unit tests | 24 passed |
| Railway SDK graph/security checks | 6 passed, no account requests |
| Railway TypeScript authoring check | Passed |
| Next.js production build | Passed with sanitized environment and local dotenv loading disabled |
| Frontend ESLint and TypeScript | Passed |
| Original console browser suite | 144 passed |
| Approved experience browser suite | 153 passed |
| Rust formatting and all-target Clippy with warnings denied | Passed |
| Compiled browser secret scan | 26 JavaScript bundles, zero findings |
| PostgreSQL backup/restore and protected-file fingerprints | Passed |

The initial restricted build failed to parse TypeScript CLI configuration; the
sanitized host build succeeded. Initial smoke attempts exposed stale shared-token,
unauthorized manual-cycle and ambiguous panel-selector assumptions. The harness
was corrected to current contracts and the final fresh-database run passed. No
permission assertion or application security control was weakened to pass tests.

Full reruns and additional checks were limited to changed paths or observed
failures. The specialized migration-only clone test requires its dedicated Stage 2
port/storage profile and was not invoked against this differently isolated cluster.
No primary migration or epoch transition was performed.

### Git, evidence retention and remaining release blockers

Generated screenshots, JSON, logs, database files and backups remain local and
untracked. The local evidence directory is
`artifacts/staging-revalidation-2026-10-30/`; it contains system, Devnet, startup,
backup, preservation, REA and scanner results. It is intentionally absent from the
public repository. Source and documentation changes use individual Conventional
Commits on `main`; the final secrets scan and one-file history audit are retained
there after completion. No push, history rewrite, deployment or account apply is
authorized by this review.

Private staging still requires an explicit deployment decision, verified account
and environment, approved named principals, fresh secret provisioning, reviewed
domain/origin, Docker image build and image scan, least-privilege database runtime
roles with an approved migration procedure, actual PostgreSQL-image validation,
and a Railway-volume backup/restore drill. Docker is unavailable in this environment;
matching Dockerfile source copies pass native-platform locked Cargo metadata checks,
which is not a container build. Local tests use PostgreSQL 16, while the selected
Railway helper image must be verified separately. Image digest pinning, database
resource limits, monitoring, Basic-auth rate limiting and MFA/session-based access
remain operational/security follow-ups. Never import local evaluation secrets,
treasuries or test credentials. Paid inference, signing/transfers, trading,
token launches, X publication, child agents and worker enablement stay disabled.
