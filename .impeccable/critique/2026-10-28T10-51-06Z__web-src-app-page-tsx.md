---
target: Supa Agent existing website, all nine views; homepage anchor
total_score: 27
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 1
target_identity: "file:/home/pikoo/Desktop/supa-agent/web/src/app/page.tsx"
target_fingerprint: "sha256:d6c8537605e2fd401c9204afeb27adc701c0b8c7462e32d7bb104d4318e88db1"
target_path: /home/pikoo/Desktop/supa-agent/web/src/app/page.tsx
timestamp: 2026-10-28T10-51-06Z
slug: web-src-app-page-tsx
closed: true
---
Method: dual-agent (A: `/root/design_review` · B: `/root/technical_audit`)

# Supa Agent website audit — 28 October 2026

Audit only. Application components, backend behavior, API integrations, financial
records and permissions were preserved. This report combines an independent
design review with a separately conducted static-detector and browser assessment.
Assessment A finished before detector findings were released into synthesis.

## Overall assessment and integrity verdict

**Preserve the existing visual identity.** Charcoal surfaces, VT323 pixel type,
mint selection, fine borders and the interactive survival diagram form a coherent,
product-specific engineering interface. The main improvement is the journey from
an ambitious public introduction into a sparse internal console, followed by
scanability and clearer availability/freshness language.

**Implementation integrity: pass within the inspected scope.** The source
detector returned zero findings; actual interactions support the read-only and
explanation-only labels. This does not certify accessibility, backend correctness
or availability of roadmap integrations.

The two scores measure different things: technical quality of the current console
is good; usability for the broader public audience is less complete. Public
onboarding and agent creation are product-maturity gaps, not broken supported
features. No P0 failure of the existing read-only workflow was found.

## Technical audit score

| Dimension | Score / 4 | Basis |
|---|---:|---|
| Accessibility | 3 | Good labels, focus, sampled contrast and reduced motion; manual assistive-technology coverage remains incomplete |
| Performance | 3 | Local font, bounded motion and modest measured transfers; no field or slow-device benchmark |
| Theming | 4 | Consistent approved dark tokens and pixel type; no blue, purple or light-theme surface observed |
| Responsive design | 3 | Responsive layouts and synthesized touch work; secondary target ergonomics can improve |
| Implementation integrity | 4 | Coherent product-specific system; static detector returned no findings |
| **Total** | **17/20 — Good** | Address the documented weak areas without replacing the identity |

## Usability critique score

Homepage persuasion and operator tasks were reviewed together against the public
and operator audiences in `PRODUCT.md`. All ten heuristics apply.

| Nielsen heuristic | Score / 4 | Main observation |
|---|---:|---|
| Visibility of system status | 3 | Lifecycle and stale evidence are explicit; saved-record reload freshness needs clarification |
| Match to users' language | 2 | Operator instructions, RPC terminology and `=> null` need public-facing explanations |
| User control and freedom | 3 | Clear navigation and menu Escape; weak contextual onward paths from profiles |
| Consistency and standards | 3 | Cohesive styling; future routes have equal navigation prominence |
| Error prevention | 4 | Reviewed UI remains read-only; disabled execution and economic environments are explicit |
| Recognition rather than recall | 3 | Labeled controls help; assembling one agent's story requires cross-page navigation |
| Flexibility and efficiency | 2 | Search/filter/table alternatives exist; long empty screens slow inspection |
| Aesthetic and minimalist design | 3 | Distinctive composition; configuration prose and repeated placeholder content add weight |
| Error recognition and recovery | 2 | Retry exists; generic explanation and raw codes provide limited operator guidance |
| Help and documentation | 2 | Useful architecture/accounting copy; contextual task help is sparse |
| **Total** | **27/40 — Acceptable** | Journey and information hierarchy need improvement for public use |

Scores are scoped judgments, not certifications. Error-recovery critique combines
source review with B's browser-only synthetic 503 test; the real backend was not
deliberately disconnected.

## Scope, architecture and browser method

Read `PRODUCT.md` and `.impeccable/config.json` before inspection. The saved
comp-first setting concerns future major presentation work; it was not used to
generate a redesign during this audit.

- Next.js App Router: server root layout and homepage, validated dynamic section
  routes, agent profile route, shared client shell and console.
- `console.tsx`: resource retrieval, directory/profile/economy/memory/social
  surfaces and explicitly disabled research forms. History is bounded to the
  latest 100 records per resource, rather than a complete database archive.
- `visuals.tsx` and `motion.tsx`: interactive explanatory circuit, expense
  chart/table, progressive reveals and exact-value updates.
- `globals.css`: semantic palette, pixel typography, shared spacing, responsive
  rules and reduced-motion behavior.
- Server-only backend facade: allowlisted GET resources, schema validation,
  bounded responses and timeouts. Backend bearer credentials remain server-side;
  the local website uses operator authentication. UI inspection did not expose
  keys or enable a mutation path.

All nine views were inspected live on desktop 1440×1000 and mobile 390×844,
including full-page scrolling. B measured four representative routes at both
sizes, home at 320px, and a 720px reflow geometry. Parent screenshots separately
measure real page dimensions. These are Chromium emulation checks, not physical
device or native browser text-zoom certification.

The actual database contains one created verification agent with largely empty
histories. Screenshots retain that real state. No records were fabricated to make
the product look active. Only loading/error tests used browser-local interception,
clearly separated from actual API evidence.

REA tools used: `list_browser_targets`, `capture_web_screenshot`,
`inspect_web_page`, `capture_browser_scenario`. Independent assessments used
separate browser contexts/tabs. B also measured the accessibility tree, computed
styles, target dimensions and synthesized Chromium touch gestures.

## Approved direction and reference comparison

The live [reference website](https://aiengineeringfromscratch.com/) was inspected
again with REA. Supa shares its numbered editorial structure, fine-grid texture,
large technical headline and figure-led explanation. The reference provides a
clear learning entry path; Supa's equivalent entry path currently reaches an
internal development state. Improve that information journey rather than copying
the reference's content, blue palette, mixed serif body type, statistics or assets.

| Requirement | Observed result |
|---|---|
| Near-black background, restrained mint, no blue/purple/light theme | Preserved across inspected views and controls |
| Pixel typography | Consistent VT323, including data, forms and navigation |
| Dotted texture, thin borders, sharp geometry | Preserved; texture remains subtle |
| Generous editorial spacing | Effective on the homepage; excessive scaffolding in low-data mobile console states |
| Interactive technical diagrams | Four selectable stages update explanation and selected circuit node; never claim live execution |
| Purposeful animation | Finite entrances and transitions; reduced motion preserves visible state |
| Full-screen mobile menu | Correct native dialog with separators, active page and close control |

Typography is not the primary defect. Pixel text is readable at actual viewport
size; long full-page screenshot previews shrink it and must not be mistaken for
the UI's true scale. The largest type/content problem is publishing long voice
configuration as the directory summary. Small uppercase metadata also competes
with useful records when it accumulates across several strips and notices.

The most generic elements are the three disabled form layouts and repeated policy
checklists. Their availability labels are honest, but the repetition adds little
new information. The survival-loop figure and evidence-grounded voice are the
strongest authored elements and should remain.

## Prioritized findings

**Counts: P0 0; P1 1 conditional public-release gap; P2 5; P3 1.** No verified
WCAG AA failure is asserted from the sampled measurements. Findings F01–F05 came
from the independent design review; F06 adds measured technical evidence. F07 is
a visually verified polish observation. The detector found none of these product
or ergonomic issues; it is not a substitute for the review.

### F01 — P1 before public release: the introduction leads into an operator-only dead end

The homepage's “MEET THE AGENTS” leads to an “Operator view” with a created agent,
no goals/memories, and long configuration text. “EXPLORE THE REASONING” reaches
empty histories. A public visitor lacks an explanation of what is useful today
and an appropriate next step. This is an expected maturity gap, not a request to
enable agent creation or manufacture evidence.

**Recommendation:** State the current inspect-only promise near the CTA, guide
visitors through available persisted evidence, and define public/operator route
boundaries before offering public onboarding. Give genuine empty states a useful
onward path. **Workflow:** `$impeccable shape`, then `$impeccable onboard` or
`$impeccable clarify` after that scope is approved.

**Location:** `web/src/app/page.tsx:6`; `web/src/components/console.tsx:88`, `:102`,
`:119`. **Category:** journey/information architecture. **Standard:** product
readiness, not a claimed WCAG failure.
**Evidence:** [homepage](../../artifacts/impeccable-audit/2026-10-28/home-desktop.png),
[directory](../../artifacts/impeccable-audit/2026-10-28/agents-mobile.png),
[memory](../../artifacts/impeccable-audit/2026-10-28/memory-desktop.png).

### F02 — P2: agent cards expose instruction prose instead of a scan-friendly identity

The card prints the full persisted `personality.voice`, including behavioral
instructions. It occupies a narrow one-third desktop card, or a large fraction
of a 2,211px mobile directory. Counts and the inspect cue follow the long prose.
Visitors read a configuration block where they expect a concise agent summary.

**Recommendation:** Lead with name, lifecycle and verified counts; provide a clear
inspect affordance. Put the exact voice configuration under a labeled disclosure
on the profile. Use a wider sparse-list treatment where useful. Do not compose
fake agent speech or overwrite stored personality. **Workflow:** `$impeccable distill`.

**Location:** `web/src/components/console.tsx:88`, `:102`;
`web/src/app/globals.css:221`. **Category:** typography/hierarchy/content density.
**Standard:** usability. **Evidence:** [desktop card](../../artifacts/impeccable-audit/2026-10-28/agents-desktop.png),
[mobile card](../../artifacts/impeccable-audit/2026-10-28/agents-mobile.png).

### F03 — P2: unavailable research routes look equally important in navigation

Eight sibling navigation entries give Trading, Token launch and Factory the same
weight as usable records. The menu does not mark them planned or locked; their
status becomes clear after navigation. Each repeats a large disabled form and
similar six-item execution prerequisites. This is late disclosure and clutter,
not an executable or deceptive trading integration.

**Recommendation:** Keep routes accessible, label them before navigation, group
research separately, and progressively disclose repeated concept/requirements
detail. Preserve the full-screen menu and its large row targets. Static hollow
squares in prerequisites should not resemble actionable checkboxes.
**Workflow:** `$impeccable clarify`, `$impeccable distill`.

**Location:** `web/src/components/shell.tsx:9`, `:66`, `:108`;
`web/src/components/console.tsx:129`.
**Category:** navigation/cognitive load. **Standard:** usability.
**Evidence:** [mobile menu](../../artifacts/impeccable-audit/2026-10-28/menu-mobile.png),
[disabled trading concept](../../artifacts/impeccable-audit/2026-10-28/trading-desktop.png).

### F04 — P2: saved-record reload and chain freshness need clearer separation

“REFRESH RECORDS” re-fetches saved API records. The treasury later says
“STALE / REFRESH REQUIRES OPERATOR.” The control does not query Solana, and an API
connection does not establish a fresh network observation. The existing source
observation timestamp is useful and must remain.

**Recommendation:** Rename the control “Reload saved records,” show when retrieval
completed, and explain beside stale status that reloading does not query Solana.
Offer relevant operator guidance without adding a new observation/signing action.
Make completed/error feedback more actionable than a raw error code.
**Workflow:** `$impeccable clarify`, then a narrowly scoped `$impeccable animate`
for genuine retrieval completion feedback if needed.

**Location:** `web/src/components/console.tsx:8`, `:46`, `:81`, `:105`.
**Category:** system status/interaction feedback. **Standard:** usability and
financial-evidence clarity. **Evidence:** [economy](../../artifacts/impeccable-audit/2026-10-28/economy-desktop.png);
actual source timestamps and stale labels were inspected farther down the live page.

### F05 — P2: mobile console scaffolding delays useful records

Header, introduction, selector, connection strip, history window and notices use
most of the first viewport. The observed empty economy is 4,155px tall; memory is
2,228px. The long runway formula includes implementation notation `=> null`, and
the empty expense chart still offers chart/table controls. Important observation
freshness lies several screens below the top.

**Recommendation:** Preserve point-of-use simulation and Devnet labels, but make
shared metadata and empty panels compact. Lead with current state/latest evidence.
Explain unavailable runway plainly and put the exact formula under calculation
details. Suppress meaningless chart-mode choices while no expense data exists.
Keep generous homepage whitespace; do not compress the entire brand indiscriminately.
**Workflow:** `$impeccable distill`, `$impeccable adapt`.

**Location:** `web/src/components/console.tsx:80`, `:110`, `:119`;
`web/src/components/visuals.tsx:44`; `web/src/app/globals.css:363`.
**Category:** mobile layout/spacing/hierarchy. **Standard:** usability.
**Evidence:** [mobile first viewport](../../artifacts/impeccable-audit/2026-10-28/economy-mobile.png),
[metrics and formula](../../artifacts/impeccable-audit/2026-10-28/economy-mobile-records.png).

### F06 — P2: secondary links could have more comfortable touch areas

Measured text links are approximately 27.5px high; the mobile brand link is
71.4×34.8px. These exceed the 24px WCAG 2.2 AA target minimum in the sampled cases,
but are less comfortable than 44px ergonomic targets. This is not a verified
SC 2.5.8 AA failure.

**Recommendation:** Expand hit areas with block padding/minimum height while
retaining the quiet text-link appearance and avoiding overlapping targets.
**Workflow:** `$impeccable adapt`, then `$impeccable polish`.

**Location:** `web/src/app/globals.css:61`, `:85`.
**Category:** responsive accessibility/ergonomics.
**Evidence:** measured DOM bounds in the validation artifact and the
[mobile secondary CTA](../../artifacts/impeccable-audit/2026-10-28/home-mobile.png).

### F07 — P3: selected agent identity clips in the native select

The long verification-agent name and lifecycle do not fit in the desktop/mobile
selector. The control remains operable, but a visitor cannot read its full selected
identity without another cue.

**Recommendation:** Echo the full selected name/lifecycle nearby or adjust the
summary layout; preserve the native accessible control. **Workflow:** `$impeccable polish`.

**Location:** `web/src/components/console.tsx:80`;
`web/src/app/globals.css:124` (selector styling).
**Category:** visual polish/recognition. **Standard:** usability, not an asserted
WCAG failure. **Evidence:** [desktop](../../artifacts/impeccable-audit/2026-10-28/economy-desktop.png),
[mobile](../../artifacts/impeccable-audit/2026-10-28/economy-mobile.png).

## Accessibility, states, motion and performance

- Sampled sRGB text contrast: primary/background **17.68:1**, secondary/panel
  **7.91:1**, dim/panel **5.78:1**, mint/panel **9.55:1**, dark/mint **10.53:1**.
  Control-border/background is **4.23:1**. These are computed color-pair checks,
  not anti-aliased glyph-pixel measurements or a whole-site certification.
- Sampled visible form controls had labels; no unnamed buttons were found in the
  inspected accessibility tree. Banner, main and footer landmarks were present.
  A hidden modal heading in the raw DOM is not evidence of a visible heading-order bug.
- Skip-link focus was visibly mint. The mobile dialog filled 390×844, focused its
  Close control, locked document scrolling, closed on Escape and returned focus
  to MENU. The source includes a focus loop. A full assistive-technology session
  remains untested.
- Synthesized CDP touch opened/closed the menu and selected the diagram's Reason
  stage. No physical touch device or drag interaction is claimed.
- A two-second browser-local API delay produced a loading skeleton and status.
  A synthetic 503 produced “Console disconnected” with CHECK CONNECTION retry.
  Empty real histories remain explicitly empty; no mock content was placed in
  product screenshots. B's initial retry-selector mismatch was a harness false
  negative and was corrected, not reported as an application defect.
- Reduced motion produced zero running animations and automatic scroll behavior;
  final states remained visible. Normal mode has restrained, finite entrances,
  selected-stage transitions and data-driven updates. No animation defect requiring
  a new motion system was verified. Offscreen reveals appeared correctly after
  actual scrolling; blank offscreen regions in unscrolled full-page captures are
  not missing-content defects.
- A local navigation sample measured DOMContentLoaded 245ms and load 480ms.
  Ten JS transfers totaled 178,828 bytes; the font decoded to 17,668 bytes.
  Concurrent local audit browsers, caching and absence of device/network throttling
  prevent interpreting this as a production performance benchmark.
- A 720px reflow geometry passed. Doubling computed font sizes by injected DOM
  styles at 390px caused 665px content width; this is an artificial stress signal,
  not native text-only zoom or a proven WCAG resize violation. Native 200% text
  resizing, Safari/Firefox, physical devices and screen readers need follow-up.
- Existing 111-test results belong to the earlier implementation validation; the
  suite/build were not rerun or represented as fresh audit results. This audit's
  fresh checks are the detector, live browser measurements and recorded interactions.

## Cognitive load and emotional journey

The diagram's four stages and four financial cells form manageable groups.
Extraneous load comes from eight equally weighted destinations, repeated policy
lists, configuration instructions and cross-route recall of agent context.
Progressive disclosure should reduce those costs while keeping safeguards visible.

The tagline and circuit produce an effective initial peak. Empty operator records
then create a valley. The interface should explain that stage and provide a useful
next inspection step; inventing activity would undermine the product's strongest
trust feature. The current final impression emphasizes unavailable execution.

Persona checks:

- **Jordan, first-time visitor:** reaches internal configuration and unexplained
  lifecycle/RPC/runway notation; needs a truthful “what can I inspect today?” path.
- **Alex, operator:** must scan instructions and long empty screens to find current
  state; needs contextual profile-to-economy/memory/social navigation and clear
  retrieval versus observation timestamps.
- **Casey, mobile visitor:** menu targets are generous, but eight equal choices
  and deep empty screens cost attention; secondary text links have smaller hit areas.

## Improvement plan — recommendations only

1. **Public clarity:** `$impeccable shape` / `$impeccable clarify` for F01/F03.
   Define current promises and future-route labels without enabling planned features.
   A later major public landing redesign follows the saved comp-first workflow.
2. **Distill:** F02/F03/F05. Short verified directory summaries, disclosed voice
   configuration, compact low-data states and less repeated placeholder scaffolding.
3. **Clarify/harden feedback:** F04. Separate saved-record retrieval from source
   freshness, retain exact financial labels and give errors specific operator remedies.
4. **Adapt:** F05/F06/F07. Improve mobile information order, enlarge secondary hit
   areas and expose the full selected identity. Retest 320px, desktop and text zoom.
5. **Animate only meaningful feedback:** preserve the current finite diagram and
   exact-value motion. If F04 adds a completed-retrieval status, a short opacity
   transition may emphasize that actual result. No fake events, count-up amounts,
   animated disconnected services or additional continuous decoration.
6. **Polish last:** F07 plus type rhythm, control spacing and focus consistency
   after the information changes. Re-run REA, `$impeccable audit` and
   `$impeccable critique` against the approved scope.

Each recommendation is independently actionable. This audit authorizes none of
the fixes, route changes, execution capabilities, paid inference or deployment.

## Evidence archive

Nine REA PNGs plus two full-page independent-review directory captures are retained.
These screenshots show the current interface and relevant content; they are not
redesign mockups. Hashes, viewports, evidence IDs, detector result, interaction
measurements and limitations are recorded in
[validation.json](../../artifacts/impeccable-audit/2026-10-28/validation.json).

The source detector result is separately retained as
[detector.json](../../artifacts/impeccable-audit/2026-10-28/detector.json).
Its empty result has no false positives to resolve. Browser-overlay injection
was blocked by the existing CSP on three routes; no CSP allowance was added and
no user-visible overlay is claimed. Static scanning, REA inspection, screenshots
and measured interactions are the fallback evidence.

Audit and screenshot files receive separate Conventional Commits on `main`, with
secret scanning and the one-file audit. The pre-existing `.gitignore` modification
is unrelated and preserved. No push or production deployment is authorized.
