# Supa Agent content and API transformation — 2026-10-28

The approved cinematic terminal frontend is now Supa Agent at
**http://127.0.0.1:3200**. The previous `web/` operator console is preserved.
No Rust files, database records, evaluation budgets or historical evidence changed.

## Implemented sections

| Section | Behavior |
|---|---|
| Home | Supa identity, tagline, bounded current capability, explicit platform vision, real API status and direct agent navigation |
| Agents | Actual identities/lifecycle/creation dates; selectable agent and exact-name/UUID inspection |
| Intelligence | Authorized saved goals, priorities, suggested progress, cycles, activity, tool authorization/status/failures |
| Memory | Authorized persisted lessons; hypotheses distinguished from verified observations; event/cycle references |
| Economy | Validated simulated micro-USD ledger, expenses/revenue/runway, separate saved Devnet wallet/balances/treasury/receipts |
| Social | Original stored drafts, styles, event/memory references and review outcomes; no publishing or generation |
| Trading | Planned, non-executable; future strict $5 maximum per trade |
| Token Launch | Planned Pump.fun capability; no launch action |
| Factory | Planned human/agent-created agents; authorization requirement, no creation action |
| About | Architecture, vision, current boundaries, official GitHub and SUPA brand/token distinction |

Replaced the reference account/logo/favicon, biography, project names/descriptions,
remote screenshots/wordmark, GitHub statistics, contribution counts, sample
commands, dated snapshot labels and demonstration notices in the running product.
The old factual snapshot and reference report remain historical files, are not
served by the application, and are excluded from the current static build.
CSP now permits only local images. No reference artwork is loaded.

The robot's exact SVG silhouette and warm muted palette remain. Code types from
top to bottom with synchronized hands. The radial portal still displays the
robot/SUPA AGENT before entry. The tilted scrolling wheel and organic branches,
ASCII entity, pointer feedback, terminal selection, history and theme switching
remain. The architecture branches and ambient entity are labeled decorative/UI
feedback rather than autonomous activity. The former contribution spark has no
invented data. Saved records render immediately; only prompt/command rows type.

## Real API evidence and privacy

`backend.mjs` imports the existing console's TypeScript Zod contracts and auth
functions. A new server-side allowlist issues only GET requests to loopback Rust
API routes. Public responses project just agent ID/name/lifecycle/created_at;
private goals, memory, activity, reasoning, finances, drafts and wallets require
existing operator Basic authentication. `/operator` is the explicit challenge;
public visitors are not prompted by ordinary private-view fetches.

The private mode-0600 environment file is read on the server only. Shell settings
retain precedence; only the four existing backend/operator configuration keys
are loaded. No OpenAI key or wallet secret is sent to the browser. Unknown fields
are stripped; known/recognizable credential values are scrubbed. Five-second
upstream timeouts, redirect rejection, bounded 2 MB responses, no-store headers,
local Host/Origin validation, CSP and traversal/static-file allowlists remain.
Queries, unknown routes and write methods fail closed. Model and shell execution
are absent from UI command handling.

Actual configured-backend verification returned HTTP 200 for health, agents,
and all 13 selected-agent resources: goals, memory, activity, cycles, actions,
financials, simulated-ledger, social, drafts, wallet, wallet/balances,
wallet/treasury and wallet/payments. Observed **one agent and five saved activity
records**; goals, memory, cycles, actions, ledger, social drafts and payments were
empty. Financial/wallet/balance/treasury bodies passed existing contracts; their
contents and credentials were not printed. No records were created or changed.

These are saved API observations, not a new Solana RPC/network verification or
proof of live GPT generation. Historical GPT evaluation databases were not merged
into this database. Populated histories and drafts are validated using explicitly
synthetic fixtures, never displayed as live achievements.

Financial values use exact BigInt scaling of validated safe integers. Simulated
USD and Devnet test assets remain separate; Mainnet is unavailable. Retrieval
time and observation time/slot are shown separately. Configured RPC versus mock
RPC evidence is distinguished; stale/reconciliation states are retained. Runway
assumptions are expandable, and missing estimates have a plain-language state.

## Browser evidence

REA tools used: `list_browser_targets`, `capture_browser_scenario`,
`compare_web_screenshots`. Screenshots and manifest are in
[evidence/2026-10-28](evidence/2026-10-28/supa-evidence.json).

| Section | Desktop 1440×1000 | Mobile 390×844 |
|---|---|---|
| Home | [capture](evidence/2026-10-28/supa-home-desktop.png) | [capture](evidence/2026-10-28/supa-home-mobile.png) |
| Agents | [capture](evidence/2026-10-28/supa-agents-desktop.png) | [capture](evidence/2026-10-28/supa-agents-mobile.png) |
| Intelligence | [capture](evidence/2026-10-28/supa-intelligence-desktop.png) | [capture](evidence/2026-10-28/supa-intelligence-mobile.png) |
| Memory | [capture](evidence/2026-10-28/supa-memory-desktop.png) | [capture](evidence/2026-10-28/supa-memory-mobile.png) |
| Economy | [capture](evidence/2026-10-28/supa-economy-desktop.png) | [capture](evidence/2026-10-28/supa-economy-mobile.png) |
| Social | [capture](evidence/2026-10-28/supa-social-desktop.png) | [capture](evidence/2026-10-28/supa-social-mobile.png) |
| Trading | [capture](evidence/2026-10-28/supa-trading-desktop.png) | [capture](evidence/2026-10-28/supa-trading-mobile.png) |
| Token Launch | [capture](evidence/2026-10-28/supa-token-launch-desktop.png) | [capture](evidence/2026-10-28/supa-token-launch-mobile.png) |
| Factory | [capture](evidence/2026-10-28/supa-factory-desktop.png) | [capture](evidence/2026-10-28/supa-factory-mobile.png) |
| About | [capture](evidence/2026-10-28/supa-about-desktop.png) | [capture](evidence/2026-10-28/supa-about-mobile.png) |

Additional actual captures: [typing intro](evidence/2026-10-28/supa-intro-desktop.png),
[portal](evidence/2026-10-28/supa-portal-desktop.png),
[mobile menu](evidence/2026-10-28/supa-menu-mobile.png),
[terminal navigation](evidence/2026-10-28/supa-command-desktop.png),
[disconnected](evidence/2026-10-28/supa-disconnected-desktop.png).

REA exercised opening the menu and keyboard selection, terminal Command →
`supa memory` → Enter, normal-motion typing/hands and Enter → portal, plus a real
configuration-disabled disconnected server. REA schema rejected an initial
literal fill argument before executing actions; it was corrected to the declared
literal-value object. No failed inference or financial action occurred.

REA capture detachment resets device emulation. Final captures therefore measured
physical dimensions with the owned browser before **each** scenario, instead of
trusting crop dimensions. The manifest records these measurements, artifact hashes
and Evidence IDs. Final Home/Menu captures include the last 44px select adjustment;
other mobile section snapshots precede that small control-height refinement.

Historical approved desktop baseline versus current Home: Evidence
`ev_711d9caee460b0104e4396e975e3b6dd3cd87795a411ba39c913a6087d70f039`,
1,440,000 compared pixels, 178,308 changed (12.3825%) at threshold zero. This
quantifies changed content/diagram space, not semantic similarity or equivalence.
Visual inspection confirmed retained palette, wheel, branches and terminal world.
REA reports incomplete coverage because event capture is unavailable; no complete
console/network capture, exact behavioral equality or production certification is
claimed. Playwright separately covers page errors and request-origin restrictions.

## Validation results

- `npm --prefix reference-web run check`: passed.
- `npm --prefix reference-web run build`: passed; six application files and six local assets.
- `npm --prefix reference-web run test:unit`: **9 passed**, zero failures.
- `npm --prefix reference-web test`: **78 passed**, zero failures/skips/retries,
  across desktop, tablet and mobile.
- Final focused checks after copy/touch refinements: **18 passed**, zero failures.
- Impeccable source detector: **0 findings and 0 advisories**. The obsolete
  terminal-demo undersized-text exception was removed, rather than extended.
- Rust was not rebuilt or changed, as requested. The existing console is unchanged.

Unit coverage includes public projection, private reads before any upstream call,
route/origin/query restrictions, schema stripping, unsafe amounts and incorrect
networks, sanitized upstream failures, oversized bodies, secret redaction, all
reused resource contracts and exact unit calculations. Browser coverage includes
all ten views, sign-in, original drafts/review states, private memory exclusion,
agent selection, command history/no JavaScript execution, escaped hostile text,
loading/empty/error/recovery, no external paid calls, read-only paths and secret
file blocking. The synthetic fixture was corrected to expose the real financial
route; authenticated tests explicitly enter through operator sign-in.

Existing regression checks retain exact robot pixel masks at 24/48/96px, slow
intro progress, keyboard motion/rest, opaque branded portal, single navigation,
restart/history cancellation, wheel scrolling/clicks, reduced motion, theme
persistence, stale terminal cancellation and responsive overflow checks.

## Independent Impeccable critique

Method: dual-agent (A: `/root/supa_product_design_review`;
B: `/root/supa_product_technical_review`). A completed independently before B's
detector output entered synthesis. A then rechecked the implemented fixes live.

| Nielsen heuristic | Final /4 | Finding |
|---|---:|---|
| System status | 3 | Saved/retrieved, unavailable and empty states are distinct |
| Real-world language | 3 | Product purpose visible; terminal remains an intentional idiom |
| User control | 3 | Back/Escape/commands; prompt motion does not hide records |
| Consistency | 3 | Warm terminal visual system preserved |
| Error prevention | 4 | Read-only, authenticated detail, planned operations non-executable |
| Recognition | 3 | Direct agent action; rotated wheel still takes effort to scan |
| Efficiency | 3 | Selection/history/keyboard paths; immediate saved-record display |
| Minimalism | 3 | Diagram reduced without removing it; more useful terminal space |
| Error recovery | 3 | Reload and sign-in; honest disconnected state |
| Help/documentation | 3 | Command help and current capability explanation |
| **Total** | **31/40** | Initial independent review: 25/40 |

Design specificity: the warm terminal, organic branching wheel and authored
animation remain distinctive; no generic dashboard/card redesign was introduced.
A's revised dimensions: craft 8/10, specificity 7/10, hierarchy 7/10, accessibility
7/10; overall 7.3/10. These are reviewer judgments, not an accessibility certificate.

P1 fixes: useful Home content now fits; a direct agent action is initially visible;
API errors no longer imply an empty population; long records are immediate;
mobile arrows no longer dismiss the drawer or lose focus. P2 fixes: semantic h2
headings, full-width mobile agent selector, compact footer time, 44px mobile
select and Supa-first source comments. No remaining P1 from the independent
reinspection. Rotated/scaled neighboring labels are retained as approved; full
accessible labels, active labels, touch, keyboard and command alternatives remain.

Jordan now sees a direct Explore action; Alex can read records without waiting;
Casey has a full-width 44px select and persistent menu browsing. Sam has heading
semantics, visible focus, inert background/focus looping and reduced motion, but
real assistive-technology testing remains outstanding.

Detector B initially reported an overridden legacy stripe declaration as one
advisory; the unused declaration was removed. Nothing was suppressed. Existing
undersized text suppression was removed. B's attempted executable browser overlay
was blocked by CSP; no user-visible overlay is claimed and CSP was preserved.
Reviewers' temporary servers and isolated browsers were stopped. Target:
`reference-web`; critique ignore list absent. Questions skipped: implementation
scope was already explicitly authorized. Source/browser evidence is the fallback
for the unavailable injected overlay.

## Accessibility, performance and parity limits

No document overflow at widths 320, 390, 768 or 1440px in tested Chromium layouts;
320×568 was also independently inspected. Visible focus, label/select association,
semantic headings, keyboard shortcuts/history, Escape, browser back, mobile focus
loop/inert content, long-text wrapping and reduced-motion cancellation were tested.
New functional DOM typography is at least 11px; neighboring scaled wheel cards
remain an intentional density compromise. Canvas entity/source labels are
decorative or terminal contexts. Screen readers, 200% zoom and other browser
engines have not been certified.

No new animation package or runtime dependency was added. Font/assets are local;
wheel RAF settles, hidden-tab/reduced-motion handling remains. One local browser
sample read 235,497 decoded resource bytes and DOMContentLoaded around 145ms;
this is not a production performance benchmark. Large histories are bounded by
response size but not paginated or virtualized.

Remaining feature-parity gaps: provider/model attribution is not available in the
reused read contracts; no lifecycle mutations, funding, wallet syncing/signing,
publication, trading, launches or factory actions are added. Saved records reload
explicitly rather than streaming. Rich historical economic charts in the old
console remain there; the new terminal presents exact ledger records. Public
memory/draft sharing needs a future explicit publication/redaction policy. Public
account roles, production auth/TLS/hosting and independent frontend packaging
remain future work. Node imports the existing console contracts/dependencies,
so the original `web/` cannot yet be removed. No data was fabricated to fill
empty current histories, and no evaluation database was selected automatically.

## Git and safety

All eligible files use individual Conventional Commits on recovered `main` via
`scripts/commit-one`, which checks exactly one explicit staged path per commit.
The full scanner and history audit are run before/after the commit sequence;
final counts and HEAD are reported in the completion message. The unrelated root
`.gitignore` edit is preserved unstaged. Original Git metadata and historical
commits/evaluation artifacts remain intact. No push or deployment is authorized.
No paid inference, X publishing, signing, transfers, trading or token launch ran.
