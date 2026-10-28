---
target: web/src/app/page.tsx
total_score: 30
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 0
target_identity: "file:/home/pikoo/Desktop/supa-agent/web/src/app/page.tsx"
target_fingerprint: "sha256:7462c7f141ef743df7b179aec481a0f85d0acbafd3a902cc46cc7e5bc8604fb3"
target_path: /home/pikoo/Desktop/supa-agent/web/src/app/page.tsx
timestamp: 2026-10-28T11-38-41Z
slug: web-src-app-page-tsx
---
Method: implementation followed by independent Impeccable reviews (A: design, B: technical), actual REA captures and browser tests.

# Supa Agent usability refinement — 28 October 2026

The existing frontend is refined rather than replaced. All seven findings in
[the baseline audit](website-audit-2026-10-28.md) are addressed within the current
authenticated, read-only preview. Public registration, agent creation and real
economic execution remain planned. No records were added to make the website
appear active, and no financial, runtime or deployment operations were performed.

## Findings and outcomes

| Finding | Resolution | Evidence and limits |
|---|---|---|
| F01 — public journey unclear | Resolved within preview scope | Homepage explains the Solana-native platform, autonomous loop and current versus planned capabilities. Explore Agents leads to real saved profiles; How Agents Work links to the explanatory circuit. Public onboarding remains planned. |
| F02 — raw instructions and heavy cards | Resolved | Directory prioritizes name, lifecycle, highest-priority open goal, saved activity count, wallet network and profile link. Technical configuration is collapsed. Raw personality voice is stripped from browser API projections, not merely hidden. Planned concept forms and policy lists are disclosed on demand. |
| F03 — planned destinations look executable | Resolved | Trading, Token launch and Factory carry Planned labels on desktop and full-screen mobile navigation. Disabled concepts are initially collapsed; visitor-facing notices make their limits explicit. Contextual profile/activity/memory/economy/draft links preserve agent identity. |
| F04 — reload confused with chain freshness | Resolved | Reload saved records waits for every mounted read and reports partial failures honestly. Full UTC retrieval timestamps are distinct from Blockchain observation. Null runway uses a plain explanation; assumptions are in Calculation details. Mainnet activity is explicitly unavailable. |
| F05 — mobile empty-state density | Resolved within refinement scope | Shorter introductions, compact record controls, fewer repeated labels, side-by-side directory filters, hidden zero-data chart/search controls and collapsed concepts. Final 390×844 memory review places the empty heading at about y=832, down from about 1100 before the last refinement. Reading its full explanation still requires a short normal scroll. |
| F06 — secondary touch targets | Resolved in inspected scope | Header brand, text links, disclosures and contextual navigation have at least 44px-high targets. Actual mobile measurements and browser tests cover representative targets; this is not physical-device certification. |
| F07 — clipped selected names | Resolved | A complete wrapping identity sits immediately beneath the native select. Synthetic long-name tests use consistent directory/detail responses at 320px; valid unknown IDs never silently substitute another agent. |

## Visual system and motion

Preserved VT323 pixel typography, #090B0A background, #141816 surfaces, #29342D
borders, #F1F3EF text, #A4AFA7 secondary text and #23D6A0 mint. Existing shared
spacing and motion tokens are reused. There is no blue, purple, light theme, new
animation dependency, glass treatment or replacement dashboard.

The directory becomes an editorial record list rather than a grid of verbose
configuration cards. The explanatory circuit, financial chart/table, native
full-screen menu and desktop navigation remain working components.

Finite 180ms opacity/transform feedback accompanies confirmed retrieval and
expanded details. Existing route entrances, actual new-record entrances and
exact-value financial transitions remain. Only actual active lifecycle states
pulse; explanatory connections do not claim disconnected services are active.
Reduced motion disables entrances and disclosure/retrieval animation while
retaining final content, selection and status text. Retrieval feedback is not an
economic event or claim that the agent ran another cycle.

## API integrity and security

The same server-only, allowlisted GET facade remains in use. No browser-facing
credentials, additional mutation endpoint or permission grant was added.
Personality projections retain public traits and remove raw voice instructions
from both agent lists and profiles. Backend persistence and reasoning context are
unchanged. Technical configuration shows only identity, timing, tool limits and
public traits.

Activity counts use the returned latest-100 activity window, explicitly labeled;
they are not fabricated all-time counts. A wallet network label requires an
actual wallet response. Unavailable wallet data is not treated as zero or as
Devnet by default. The directory initially mounts ten records, with an explicit
next-ten control, bounding its per-agent activity/wallet reads.

The shared records tracker includes nested reads, aborts obsolete requests and
retains same-path data during reload. A delayed wallet-payment read keeps the
button busy; a synthetic 503 produces a partial-failure message without a success
timestamp. Read completion is not a blockchain observation. Reload does not
query Solana, activate an agent or perform paid reasoning.

## Actual browser evidence

All nine views were inspected on desktop 1440×1000 and mobile 390×844: homepage,
directory, profile, economy, memory, social, trading, launch and factory.
[Evidence index](../artifacts/usability-refinement/2026-10-28/evidence.json)
records REA IDs, actual measured viewport sizes, PNG hashes and completeness.
Nineteen screenshots cover all eighteen route/viewport combinations plus the
open mobile menu. They show actual saved database state, including one created
agent and five saved activity events. No screenshot uses the automated fixtures.

| View | Desktop | Mobile |
|---|---|---|
| Homepage | [After](../artifacts/usability-refinement/2026-10-28/home-desktop.png) | [After](../artifacts/usability-refinement/2026-10-28/home-mobile.png) |
| Directory | [After](../artifacts/usability-refinement/2026-10-28/agents-desktop.png) | [After](../artifacts/usability-refinement/2026-10-28/agents-mobile.png) |
| Profile | [After](../artifacts/usability-refinement/2026-10-28/profile-desktop.png) | [After](../artifacts/usability-refinement/2026-10-28/profile-mobile.png) |
| Economy | [After](../artifacts/usability-refinement/2026-10-28/economy-desktop.png) | [After](../artifacts/usability-refinement/2026-10-28/economy-mobile.png) |
| Memory | [After](../artifacts/usability-refinement/2026-10-28/memory-desktop.png) | [After](../artifacts/usability-refinement/2026-10-28/memory-mobile.png) |
| Social | [After](../artifacts/usability-refinement/2026-10-28/social-desktop.png) | [After](../artifacts/usability-refinement/2026-10-28/social-mobile.png) |
| Trading | [After](../artifacts/usability-refinement/2026-10-28/trading-desktop.png) | [After](../artifacts/usability-refinement/2026-10-28/trading-mobile.png) |
| Token launch | [After](../artifacts/usability-refinement/2026-10-28/launch-desktop.png) | [After](../artifacts/usability-refinement/2026-10-28/launch-mobile.png) |
| Factory | [After](../artifacts/usability-refinement/2026-10-28/factory-desktop.png) | [After](../artifacts/usability-refinement/2026-10-28/factory-mobile.png) |

[Open mobile menu](../artifacts/usability-refinement/2026-10-28/mobile-menu.png).
Closed state is visible in every mobile route screenshot. Baseline captures are
preserved in `artifacts/impeccable-audit/2026-10-28/`. REA's same-size homepage
comparison identifies 111,648 changed pixels of 1,440,000 (7.75%); visual review
assesses the changed hierarchy and copy. Pixel difference is not a quality score.

REA used `list_browser_targets`, `capture_web_screenshot` and
`capture_browser_scenario`; final comparison used `compare_web_screenshots`.
The menu scenario completed its explicit open action and retained screenshot,
DOM and accessibility evidence, but did not retain requested page-error events.
The index records this incomplete event coverage rather than claiming a clean
console. Browser tests separately check route behavior and errors. REA's attach
window cannot reconstruct pre-attach events.

Independent A inspected and scrolled all nine views at both sizes, tested real
contextual links and reload timestamps, and rechecked memory/planned copy after
the last edit. B measured 24 route/viewport combinations at 320/768/1440px plus
mobile directory/economy, with no document overflow or unnamed sampled buttons.
B checked menu focus containment/restoration, Escape, Back, body-scroll release,
44px targets, unknown IDs, delayed and failed reads, normal/reduced motion and
the absence of private voice from the actual browser facade. B's REA namespace
became unavailable, so its measurements use Chromium/CDP; parent REA captures
and independent A renders provide the actual visual evidence.

## Updated Impeccable assessment

Design specificity remains strong; preserve the incumbent visual identity.
Implementation integrity passes in the inspected scope. A completed before B's
detector results were released to synthesis. The static detector returned zero
findings; it cannot determine product comprehension or replace visual review.
The existing CSP blocks the external detector overlay, so no overlay validation
or weakened CSP is claimed.

| Usability heuristic | Before / 4 | After / 4 |
|---|---:|---:|
| Visibility of system status | 3 | 3 |
| Match to users' language | 2 | 3 |
| User control and freedom | 3 | 3 |
| Consistency and standards | 3 | 4 |
| Error prevention | 4 | 4 |
| Recognition rather than recall | 3 | 3 |
| Flexibility and efficiency | 2 | 2 |
| Aesthetic and minimalist design | 3 | 3 |
| Error recognition and recovery | 2 | 3 |
| Help and documentation | 2 | 2 |
| **Total** | **27/40** | **30/40 — Good** |

Technical score improves from **17/20 to 18/20**: accessibility 3, performance 3,
theming 4, responsive design 4, implementation integrity 4. These are scoped
judgments, not accessibility or production certifications. A's initial two P2
observations about planned copy and memory density were fixed and visually
rechecked. Remaining combined priorities: P0=0, P1=0, P2=0, P3=1 (generic tab
titles). The final small fixes do not justify artificially increasing the score.

First-time visitors now have an honest entry path; operators retain evidence and
contextual links; mobile readers face less default scaffolding. Navigation still
has five available and three planned destinations, and five agent-context links.
Grouping reduces competition but does not satisfy a strict four-choice heuristic.

## Accessibility, performance and limitations

Native controls/dialog/disclosures, visible mint focus, explicit current routes,
live retrieval status and disabled concept inputs are preserved. Automated checks
cover keyboard navigation, Escape, Back, focus restoration, reduced motion,
mobile widths, overflow, loading/error/empty states and exact financial values.
The fixed palette's primary, secondary and mint text all exceed 4.5:1 on the two
approved dark backgrounds. Borders are decorative separators; contrast of a
separator is not substituted for required control/focus contrast.

No animation dependency was added. Motion is finite and limited to opacity and
transform where introduced. Directory rendering is bounded; reload retains saved
data instead of replacing it with a full loading screen. There is no new field
performance, slow-device, Lighthouse or layout-shift certification. The host
experienced memory pressure during concurrent browser validation; the final
suite uses one worker. The nine-document narrow-width sweep has a 90-second
test budget instead of a single-document timeout; no assertions or route coverage
were removed and automatic test retries remain disabled.

Remaining limitations: the local authenticated preview is not complete public
onboarding; all browser tabs retain the same generic title; discoverable public
task help, saved views and expert history navigation remain limited. The real
dataset has only one agent and mostly empty histories. Multi-agent/long-name,
populated tables and error recovery use explicitly synthetic browser fixtures,
not fabricated persisted evidence. Screen readers, physical touch devices,
alternate engines, text-only zoom and large-agent load were not certified.

## Changed files and verification

Application changes: `web/src/app/page.tsx`, `web/src/app/[section]/page.tsx`,
`web/src/app/globals.css`, `web/src/components/{console,records,shell,visuals}.tsx`
and `web/src/lib/contracts.ts`.

Tests: `web/tests/{console,contracts,security,styling-motion,usability}.spec.ts`
and `web/tests/fixtures.ts`. Browser preparation: `scripts/rea-web-view.mjs`.
Local skill cache exclusion: `.impeccable/.gitignore`. Documentation, critique
history and the evidence directory record the result. All are separate files for
Git purposes; braces here are only a compact inventory.

Commands: `npm run build`, `npm run typecheck`, `npm run lint`,
`npm test -- --workers=1` from `web/`; `./scripts/git-local diff --check`,
`python3 scripts/scan-secrets.py` and `python3 scripts/git-policy.py audit` from
the root. Exact final test statistics are retained alongside browser evidence.
Rust is unchanged and its tests are not rerun for this frontend-only refinement.

Each eligible changed file is committed separately through `scripts/commit-one`
on `main`; the helper verifies the one-file staged index and runs policy hooks.
The unrelated existing root `.gitignore` edit is preserved and excluded from this
work's commits. No push, squash, rebase, force push or deployment is authorized
or performed. Historical audit/evaluation evidence, secrets and database records
remain preserved. New pushes require separate explicit approval.

Questions skipped: the user already selected public clarity and all seven findings;
the requested implementation is complete. Generic tab titles are a minor future
polish task, not a reason to replace this visual identity.
