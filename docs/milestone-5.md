# Milestone 5 — full-stack integration and private staging preparation

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
