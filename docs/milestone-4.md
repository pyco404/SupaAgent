# Milestone 4 — Website foundation and Devnet verification

Implemented and validated locally on October 28, 2026. The existing six Rust crates, migrations, custody mechanism, runtime permissions and historical evaluation evidence are preserved. This milestone adds `web/` and local preview/inspection helpers. No production infrastructure or paid inference was used.

## Completed pages and API integration

| Page | Implemented behavior | Authenticated Rust reads |
|---|---|---|
| `/` | Original editorial homepage, required tagline, four-stage interactive survival diagram and actual connection/agent count | `health`, `agents` |
| `/agents` | Search, lifecycle filter, identity cards and profile navigation | `agents` |
| `/agents/{id}` | Personality, lifecycle, goals, model progress estimates, capability boundary and activity provenance | Agent detail, `activity` |
| `/economy` | Exact unit conversion, simulated financial metrics/runway, expense chart/table, Devnet treasury, observed balances and verified receipts | Agent detail, `simulated-ledger`, wallet `balances`, `treasury`, `payments` |
| `/memory` | Search and evidence filters, hypotheses versus verified observations, source references, selectable cycles and authorization/execution records | `memory`, `cycles`, `actions` |
| `/social` | Exact persisted public draft wording, styles, abstentions, rejected decisions with text withheld, provenance and separate legacy archive | `social`, `drafts` |
| `/trading` | Disabled Jupiter trading concept; no prices, quotes or swap path | None |
| `/launch` | Disabled Pump.fun concept; no upload, token creation or launch path | None |
| `/factory` | Non-executable parent/child illustration and disabled creation fields | None |

Every connected console page uses persisted identities. No fallback agents, balances, earnings, drafts, trades or achievements are invented. A missing balance is unavailable, not zero. Model progress is explicitly a suggestion, separate from operator-confirmed goal completion. The app does not expose private chain-of-thought.

The local API was exercised through the actual Next.js proxy: **16 authenticated read resources returned HTTP 200**. An unauthenticated frontend request returned 401; an authenticated frontend POST returned 405; an unapproved signing GET path returned 403 before reaching Rust. The direct wallet policy read remained locked with reservations disabled. See [actual API evidence](../artifacts/milestone-4/actual-api.json).

## Design and browser evidence

[The requested reference](https://aiengineeringfromscratch.com/) was inspected live with REA before frontend source implementation. Its editorial hierarchy, spacious columns, thin dividers, figure labels and technical presentation informed the layout. Its prose, curriculum and illustrations were not copied.

Supa uses near-black `#0b0d0c`, restrained mint `#a8edcf`, neutral borders, dotted grids and locally hosted VT323 pixel typography throughout. The SVG mascot, survival loop and recorded-expense visualization are original. The font license is retained at [`web/public/fonts/OFL.txt`](../web/public/fonts/OFL.txt). There are no external font, analytics or illustration requests. The automated palette check rejects blue accents.

REA 6.8.0 / Chromium 146.0.7680.177 tools actually used:

- `binary_session`: installation identity and registered capability discovery.
- `capture_browser_scenario`: live reference inspection, original diagram interaction, mobile menu navigation, Memory search and hypothesis filtering.
- `list_browser_targets`: selection of the exact authenticated loopback target.
- `inspect_web_page`: DOM, accessibility and observed console inspection.
- `capture_web_screenshot`: final actual rendered screenshots of all nine pages at three measured viewport sizes.

Final REA evidence covers **27 successful views**: desktop 1440×1000, tablet 768×1024 and mobile 390×844. The preparation helper checked actual `innerWidth`, `innerHeight` and document scroll width before each capture. Thirteen representative PNGs are retained, with source evidence IDs and verified SHA-256 hashes in [the REA manifest](../artifacts/milestone-4/rea-evidence.json).

| Screenshot | Desktop | Mobile / tablet |
|---|---|---|
| Homepage | [Desktop](../artifacts/milestone-4/home-desktop.png) | [Mobile](../artifacts/milestone-4/home-mobile.png), [tablet](../artifacts/milestone-4/home-tablet.png) |
| Directory | [Desktop](../artifacts/milestone-4/agents-desktop.png) | All viewports recorded in REA manifest |
| Agent profile | [Desktop](../artifacts/milestone-4/profile-desktop.png) | [Mobile](../artifacts/milestone-4/profile-mobile.png) |
| Economy | [Desktop](../artifacts/milestone-4/economy-desktop.png) | [Mobile](../artifacts/milestone-4/economy-mobile.png) |
| Memory | [Desktop](../artifacts/milestone-4/memory-desktop.png) | All viewports recorded in REA manifest |
| Social drafts | [Desktop](../artifacts/milestone-4/social-desktop.png) | All viewports recorded in REA manifest |
| Trading | [Desktop](../artifacts/milestone-4/trading-desktop.png) | All viewports recorded in REA manifest |
| Token launch | [Desktop](../artifacts/milestone-4/launch-desktop.png) | All viewports recorded in REA manifest |
| Factory | [Desktop](../artifacts/milestone-4/factory-desktop.png) | All viewports recorded in REA manifest |

Visual corrections: the 320px heading/grid initially overflowed and was corrected; a browser fixture incorrectly replaced a null balance and was fixed. REA attached-scenario viewport settings initially cropped a desktop layout. Those captures are explicitly discarded as responsive evidence. The final workflow measures/resizes the owned page with Playwright and captures it using REA's read-only screenshot tool. A failed preview navigation/cleanup was diagnosed, the local preview restarted, and the browser recovered before final captures. A non-unique Memory wait selector was corrected. No unchanged failing browser operation was accepted as a pass.

REA screenshot evidence is point-in-time, not a claim of visual equivalence to the reference. Attached scenario event coverage was reported incomplete; zero retained page-error events is not proof of full error coverage. Automated browser tests separately assert that no uncaught JavaScript errors occur. `rea doctor` reported optional native analysis engines/unrelated client registrations missing; the registered browser tools operated successfully. No unrelated engine installation or client setup was attempted.

## Local commands

Use Node.js 22.18+; validated here with Node.js 24.22.1 / npm 11.19.1. The Rust/PostgreSQL requirements remain in [local setup](local-setup.md).

```bash
npm ci --prefix web
npm run build --prefix web
./scripts/web-local
```

Open `http://127.0.0.1:3000`. The launcher uses the existing private `.env.devnet` and dedicated `supa_devnet` database, verifies that all agents are inactive and unscheduled, forces the offline provider, and starts the Rust API on `127.0.0.1:8080`. It does not load `.env.eval`, OpenAI credentials, wallet polling or paid evaluation configuration. Existing `web/.env.local` is preserved; if absent, the launcher creates unique credentials with mode 600. Read the username/password privately in a local editor. Do not paste them into chat, screenshots or commits. Stop the launcher with Ctrl-C.

For an unconnected design preview, leave server variables unset and run `npm run dev --prefix web`. The development server binds loopback and shows explicit disconnected states. For a manually connected existing API, copy `web/.env.example` to `web/.env.local`, set the server-only bearer token/base URL, and configure a unique operator password of at least 24 characters. Never use `NEXT_PUBLIC_` for secrets. Use `127.0.0.1` consistently rather than mixing origins.

`build`/`dev` use Next.js's webpack path because this environment blocks the default Turbopack CSS worker's internal socket. Rebuild with the preview stopped, then restart; do not replace `.next` assets under a running preview.

```bash
# Final frontend checks; Chromium is already installed at this path locally.
npm run typecheck --prefix web
npm run lint --prefix web
npm run build --prefix web
npm test --prefix web

# Existing Rust checks, including disposable database integrations.
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
TEST_DATABASE_URL=postgresql://pikoo@127.0.0.1:55432/supa_agent_test \
  cargo test --workspace --all-targets --locked --offline -- --include-ignored

# Observation only: no signing, airdrops or generation requests.
cargo run -p supa-runtime --example devnet_verify --locked --offline -- --check
```

Browser tests launch isolated loopback test services on ports 3100, 3101 and 18081, explicitly override private configuration, and use clearly synthetic fixtures. They never connect the Rust test suite to `supa_eval`. Playwright's executable path can be adapted in `web/playwright.config.ts` for another local Chromium installation.

To reproduce REA inspection after starting the real local preview:

```bash
node scripts/rea-web-session.mjs
# In another terminal: prepare a measured page, without printing credentials.
node scripts/rea-web-view.mjs home 390 844
```

Use `list_browser_targets` at `http://127.0.0.1:9222`, restricted to `http://127.0.0.1:3000`, then `inspect_web_page` and `capture_web_screenshot` against that exact target. For interactive scenarios, match the measured viewport and explicitly request only the necessary captures. Basic authentication is supplied internally from the ignored file, never inline in REA arguments or URLs. Stop the browser helper with Ctrl-C; its private profile is ignored. The profile helper targets this workspace's existing Devnet verification agent.

## Actual Devnet verification

The existing observation-only verifier returned success against `https://api.devnet.solana.com`:

- Full genesis: `EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG`.
- Devnet USDC mint: `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU`, initialized six-decimal mint at finalized commitment.
- Existing public wallet: `4imA97URQuuFYbSKNum5dWCqUmRNenmscEcVpAju5S7U`.
- Observed slot **515987213**, **0 SOL lamports**, **0 USDC atoms**, no token accounts.
- Persisted observation at **2026-10-28T07:59:43.965046Z**, reconciled; later reads correctly show stale after the existing freshness window expires.
- Verified incoming receipts: **0**. No actual transaction was obtained, funded, signed or verified in this milestone.
- Database migrations 1–4 are applied successfully. Repeated local API restarts retained the same identity and observed slot without adding receipts or revenue.

These are real configured-RPC balance/mint observations, not mocked Devnet receipts. [Actual API evidence](../artifacts/milestone-4/actual-api.json) retains the persisted observations after restart. Confirmation regressions, wrong networks/mints, unconfirmed payments, RPC failures, malicious metadata, duplicate/concurrent receipt observation, monitor pagination/reconnect and signing denial were exercised through deterministic Rust tests. They do not establish that a real incoming transaction has been observed. That remains blocked on a separately approved external testing workflow.

## Validation and security results

- Rust formatting and Clippy with warnings denied: passed.
- Rust unit, HTTP, provider, database and restart/recovery tests: **97 passed**, no ignored tests in the explicit full run.
- TypeScript, ESLint with zero warnings, production webpack build: passed.
- Responsive Chromium suite: **66 passed**, zero retries, skipped or flaky tests; desktop/tablet/mobile plus 320px minimum checks.
- Browser coverage: navigation, diagram controls, agent search/profile, simulation and mock-evidence labels, chart/table toggle, memory filtering/provenance, exact draft wording/abstention/rejection handling, disabled future forms, loading/empty/error/disconnected states, authentication, path/method permissions and origin/Host checks.
- Runtime and full frontend dependency audits: **zero reported vulnerabilities** after replacing the initially vulnerable Zod pin with 4.6.7.
- Generated client assets: **27 files scanned**, zero matches for known local credentials.
- Targeted repository secret scanner: extended to include local frontend credentials, browser profiles, build output and trace exclusions; eight scanner self-tests passed. Commit hooks run scanning for every individually staged file; a final full scan and one-file history audit are required before reporting Git completion.

The Next.js server-only proxy accepts a closed GET resource allowlist, validates UUIDs and response schemas, strips unknown fields, rejects unsafe integers/inconsistent receipt proofs, enforces a five-second timeout and two-megabyte response limit, refuses redirects, and pins the backend to a loopback IP. Upstream error bodies are never forwarded or logged. Local Basic authentication protects the connected site; absent operator credentials fail closed. Host/origin checks reject cross-origin/rebinding requests. Mutations are not forwarded. The backend bearer, wallet master and OpenAI keys never enter browser context.

This is a single-operator loopback console, not public hosting infrastructure. HTTP Basic authentication assumes local HTTP access only. CSP permits the inline script/style behavior needed by the current Next.js application; production nonce-based hardening, TLS, multi-user sessions, rate limits and deployment review remain future work. No runtime signing, spending, trading, X publishing, token launch, child-agent creation or deployment was enabled. Existing evaluation budgets and historical spending records were not used or reset.

## Limitations and Git workflow

The actual dedicated Devnet agent is `created`, unscheduled and has no goals, memory, cycles or social history. Those live empty states are intentional. Populated component behavior is verified with explicitly synthetic test-only fixtures, never product fallback data. Histories retain the backend's latest-100 window without pagination. Refresh is operator-driven; the console does not run reasoning or wallet polling. Public hosting, real receipt confirmation and executable economic integrations need separate approval and implementation.

Work is on recovered `main` using `scripts/git-local`; the original read-only `.git`, historical archive and previous 133 individual commits are preserved. Every eligible changed file receives its own descriptive Conventional Commit through `scripts/commit-one`, including each screenshot and report artifact. No bundling, squashing, rebasing, force-pushing or archival-branch push is permitted. The prior initial-push authorization is recorded accurately in `AGENTS.md`; **Milestone 4 has no push authorization**, so these new commits remain local. Final commit count/SHA, clean status, one-file audit and secret-scan result are reported after committing.
