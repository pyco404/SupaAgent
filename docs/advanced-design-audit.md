# Advanced design audit and refinement

Date: 2026-10-31. Target: `reference-web/index.html`, served at `http://127.0.0.1:3200`. The approved Hancore composition, atmospheric palette family, pixel robot, ten sections and animation architecture remain the visual authority. PRODUCT.md contains an older palette reference; this pass honors the user's current approval rather than treating that difference as permission to replace the design.

## Method and verdict

Read PRODUCT.md, `.impeccable/config.json`, and Impeccable audit, critique, polish, distill, animation and craft-floor guidance. The configuration requests the existing composition path and has no detector ignore rules. DESIGN.md and a current target-specific surface brief are absent. No configuration or design-system rewrite was performed.

Two independent read-only assessments were completed: A assessed design and usability without detector findings or prior critique snapshots; B ran the source detector and reviewed implementation risks without A's conclusions. Root then verified rendered behavior with REA and Chromium. A subsequently reviewed the actual after screenshots and source. This is an independent review plus browser evidence, not a claim that the source detector establishes visual quality.

**Verdict:** strongly authored and product-specific. The code-formed mascot, terminal interaction, branching navigation, illustrated keyboard and honest saved-record terminology remain distinctive. The useful improvements are clearer control and less redundant work, rather than additional spectacle. No remaining P1 issue was identified in this scoped follow-up review; that is not accessibility certification.

### Heuristic assessment

Scores are review judgments, not intelligence or production-performance metrics.

| Nielsen heuristic | Before /4 | After /4 |
| --- | ---: | ---: |
| Visibility of system status | 3 | 3 |
| Match with real world | 3 | 3 |
| User control and freedom | 3 | 3 |
| Consistency and standards | 3 | 3 |
| Error prevention | 3 | 3 |
| Recognition rather than recall | 2 | 3 |
| Flexibility and efficiency | 2 | 3 |
| Aesthetic and minimalist design | 3 | 3 |
| Error recovery | 2 | 3 |
| Help and documentation | 2 | 2 |
| **Total** | **26/40** | **29/40** |

The improvement reflects visible text access, stronger keyboard control and visible lookup recovery. No score of 4 is justified by the limited source/screenshot review. The technical audit judgment is 16/20: accessibility 3, performance 3, theming 3, responsive behavior 3, implementation integrity 4. Physical-device, assistive-technology and cross-browser certification are outside this run.

## Prioritized findings and decisions

| Priority | Finding / location | Complexity | Performance impact | Outcome |
| --- | --- | --- | --- | --- |
| P1 | Main reading-pane focus ring used `--line`, only 1.664:1 against the default background (`style.css`) | Small | None | Use the existing warm foreground token and a small text inset; verify all palettes. |
| P1 | Global character-only shortcuts had no off switch (`app.js`) | Small | Negligible | Add a persistent UI-only opt-out inside existing utilities; preserve defaults, arrows and component-focused controls. |
| P1 usability | Long typed views gate later controls, but text reveal was hidden inside commands (`index.html`, `app.js`) | Small | Neutral or beneficial when used | Move the existing transient Show all text action to the identity toolbar. Default typing remains. |
| P2 | Picker searched only one character and repeated initials always selected the same option (`app.js`) | Small | Negligible | Add 700ms accumulated-prefix search and repeated-letter cycling. |
| P2 | Failed `supa inspect` only updated a screen-reader announcement (`app.js`) | Very small | Negligible | Show the same recovery message in the terminal and scroll it into view; retain announcement. |
| P2 | Entity code glyphs were repainted at about 30fps although their texture changes every 1.8s (`scene.js`) | Small–medium | Reduces redundant glyph work | Repaint only when its visible state changes, retaining the current renderer and exact mask. |
| P2 | Two themes shared the same background, breaking the established theme-cycle contract (`app.js`) | Very small | None | Give the second theme a distinct near-black shade within the same approved family. |
| P2 | A 3px mascot entrance translation caused the pointer endpoint to drift during startup (`style.css`) | Very small | Neutral or lower | Retain the atmospheric opacity entrance, remove the unnecessary spatial movement. |
| P2 remaining | Empty/disconnected states separate recovery explanation from Reload/sign-in in utilities | Small | Negligible, explicit reads only | Proposal: reuse only the relevant recovery action beside its notice; no automatic retries or new agent creation. |
| P2 proposal | Mobile blueprint competes with the record viewport | Medium | Potentially lower canvas work | Any collapsing, resizing or content-order change needs a separate visual proposal and approval. |
| P2 proposal | Richer activity-aware entity cues from authenticated execution-health records | Medium | Prefer existing reads; no fabricated liveness | Defer until fresh execution evidence and its unavailable/stale states can be validated. Saved lifecycle alone remains insufficient. |
| P3 proposal | Optional audio | Medium | Extra asset/context/control cost | Not implemented. Consider only opt-in, silent-by-default, locally hosted sounds with independent mute and volume controls; no automatic playback. |
| P3 | Further startup/entity choreography or motion-token consolidation | Small–medium | Could add work without usability benefit | Defer. Existing portal timing and state meanings are preserved. |

The two accessibility findings follow [W3C character shortcut guidance](https://www.w3.org/WAI/WCAG22/Understanding/character-key-shortcuts.html) and [focus visibility](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html) / [non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html). A shortcut can remain available in its focused component while the global version is disabled. Automated checks do not prove complete WCAG conformance.

## Implemented interaction details

Show all text appears only while a workspace view types. It uses the existing finish/cancel behavior, immediately restores all received content and focuses the terminal when activated by keyboard. It shares the existing Agent row instead of adding another toolbar row. During that brief interval the picker is narrower and its existing accessible name/ellipsis behavior remains. In reduced motion the bypass is hidden because content is already complete. No financial or inference consent is bypassed.

The new utility button reports Global character shortcuts on/off through its visible label and `aria-pressed`. Only a local preference is stored. Default behavior is unchanged; when disabled, global letter/punctuation actions are suppressed outside the focused wheel and terminal. Arrow keys, Escape, ordinary typing in fields, explicit command controls and component-local navigation remain available. No backend preference or permission was introduced.

Picker search preserves native backing state, viewport containment, Enter selection, Escape/focus return and Tab dismissal. Hover uses the existing mint/charcoal material without adding motion. Failed inspection uses existing saved identities only; no identity, status or activity is fabricated.

The entity cache invalidates on texture offset, interference eligibility, lifecycle settling/state, visible input caption, decorative worm setting, code/font/resize redraw and theme changes. Reduced-motion output remains still; the rare code irregularity and saved-lifecycle semantics are preserved. No new loop, rendering dependency, offscreen bitmap, API request or long-lived sensitive data cache was added. Existing branches, pointer tether, hands, portal and diagram pause/intersection behavior remain intact. A meaningful normal-mode canvas cost remains in the rest of the scene.

Distillation here means moving the immediate reveal action out of hidden utilities, not deleting sections or controls. Consistent feedback, search and focus improve the existing interactions without new decorative layers, sound, glow, flashing or claims of consciousness.

## Browser, detector and screenshot evidence

REA before desktop: `ev_cc33f4af33699896b50b9393c48de9030412e47399da138315b0b1f7ba1e6573`. Before tablet navigation: `ev_8bc14fce20da29a165321bcafb3e7d8782970d4d9a345677431adbd9297fdd9a`. Before mobile drawer/About: `ev_4b6cb0e8c857db42b18386eec7b4c4c04c964b1fa7bb35f3fb3a84dc2f634c76`.

REA after desktop reveal/failed-inspect recovery: `ev_b654f5ecd2d15313b630d77bace72ce4359efbdc987712b13825ae484fde75c3`; all seven recorded steps completed. After tablet navigation/preference toggle: `ev_9c66f2b7933da8691c4de60c91f2a8b87473eb3a4ef88357de1a7e7856de80f2`. After mobile drawer/About/preference toggle: `ev_ab835f175403d0ad595faf3b92bdd10dfa7a6ce0865eefc5d2989d48540a5e69`. Final desktop capture: `ev_ae94e1ea3a8246a02cfb88859696c62aa552b2cd3339238a1a0bb2cc97bcd2f5`.

One early after attempt reused the pre-edit attached document and was discarded; successful evidence forced a fresh navigation. Attached scenario event coverage is incomplete. It does not prove absence of pre-attach events or constitute a complete network trace. Query parameters used for local review do not modify API contracts.

The source detector returned zero findings. The actual rendered CDP scan identified cramped terminal padding; an 8px inset resolved it. A later scan flagged three planned labels as overlapping. These are rotated axis-aligned bounding-box false positives: actual unrotated local name/planned rectangles are each 13px high with a 1px gap, under the same affine transform. Source geometry, screenshots and existing full-label tests confirm the separate text runs. The labels and required planned-state wording were preserved. No detector overlay was injected, no CSP was weakened, and no detector live server was started.

Local-only matched screenshots are under `artifacts/advanced-design/`: `before-` and `after-` desktop 1440×1000, tablet 768×1024 and mobile 390×844, for both introduction and Home. Additional focus and typing captures show the actual refined controls. Screenshots use reduced motion for stable comparisons except the typing demonstrations. The primary API remains stopped; disconnected/unavailable states in actual-site captures are genuine. Tests use explicitly synthetic records for connected/authorization scenarios, not live agent activity.

## Validation results

- **183 unique browser cases have passing latest results**, covering desktop, tablet and mobile. `coverage.json` reconciles the actual test listing against case-by-case logs; zero missing cases and zero unresolved failures.
- The combined run completed 173 cases before external termination (exit 143). It was not a green uninterrupted full-suite run. Six earlier failures identified duplicate theme backgrounds, two startup pointer-anchor timing mismatches, and three instances of an obsolete no-toolbar-button DOM assertion. The theme and startup issues were corrected; the toolbar contract now explicitly permits only the transient reveal action and asserts it is hidden in reduced motion. A bounded mobile completion batch passed 12/12 and the corrected regression batch passed 12/12. Together these provide the full 183-case current coverage.
- An earlier targeted run had three stale test-locator failures because the toggle's name changes on activation. The test now uses a stable control locator; all desktop/tablet/mobile shortcut cases subsequently passed. Earlier attempts remain in local evidence, not silently discarded.
- The suite exercises all ten sections, public/private read boundaries, explicit consent and uncertain-action handling with synthetic fixtures, keyboard focus and navigation, custom picker containment, text reveal/cancellation, reduced motion, pointer/typing tethers, canonical logo mask, scrolling and responsive geometry. No primary agent or financial state was mutated.
- **28 frontend unit tests**, syntax checks and the final static frontend build passed. Existing Node module-type/color-environment warnings remain non-failing. Rust/Next.js source is unchanged and those compilers were not rerun for this presentation-only pass.
- The actual terminal focus contrast changed from **1.664:1 to 14.024:1** in the default palette; tests verify at least 3:1 across all three palettes. The focus inset is 8px. The immediate reveal target meets at least 24×24 CSS pixels and uses a 44px minimum height. Keyboard activation restores terminal focus.
- Reduced-motion canvas pixels remain still. Memoization tests bound redundant glyph work and verify explicit invalidation on theme and resize. These are browser-emulated device classes in Chromium; physical touch hardware, speech input and manual screen-reader testing remain unverified.

## Performance

The final experiment ran after the regression runner and commit hooks finished: three approximately 1.5-second windows per variant in headless desktop Chromium, same host and viewport. Original JS/CSS were served through browser interception from baseline `403935887335233303781f78ae00a712142c4d08`, with shared current HTML. No checkout, production request, benchmark dependency or backend activation was involved.

| Median measured work per ~1.5s window | Before | After |
| --- | ---: | ---: |
| Main-thread task time | 0.973s | 0.676s |
| Script time | 0.307s | 0.132s |
| Task time / elapsed window | 64.7% | 44.9% |

That is roughly 31% less sampled task work and 57% less script work in this small local experiment. This is directional evidence, not a standardized benchmark or a phone battery/FPS claim. An earlier concurrent-regression sample was noisier and is not used for the final comparison. The underlying normal-mode scene still has meaningful CPU cost; further hand/tether/intro dirty-render profiling is a separate opportunity. No additional animation loop or dependencies were introduced.

## Remaining limits and next decisions

No backend, API permission, accounting, credential, wallet, official logo or financial-control code changed. No database was activated, no worker started, and no paid inference, transaction, cloud resource or deployment occurred. The primary frontend continues on loopback 3200 with its existing backend configuration.

The priority backlog now concerns contextual recovery and mobile reading depth. More immersive portal choreography, a redesigned entity, adaptive blueprint geometry, global search, rearranging the ten sections and optional sound remain proposals, not silently implemented changes. Keep the current palette/layout unless a separate proposal is approved. Profiling on physical low-end hardware and manual screen-reader/speech-input testing remain useful follow-ups.

## Git and run notes

Seven implementation/report files are committed separately with unique Conventional Commit subjects, directly on `main` through `scripts/commit-one`. Each helper invocation verifies exactly one explicitly named staged path and runs the existing secrets gate. No commits were bundled, rewritten, squashed, rebased or force-pushed. The full history and secrets audit results are retained in the local evidence directory; no artifact, dotenv, credential, database or sensitive log is included in these commits.

The current critique snapshot is retained locally through Impeccable, separately from this implementation report. No earlier matching snapshot existed for the target, so none was silently closed. Lower-priority proposals remain open. Snapshot and browser logs/screenshots remain local and untracked; the report and implementation files are committed individually. Temporary review browser cleanup is required; the user-facing frontend remains available. No push or deployment.

Questions skipped: the user supplied a complete direction and authorized low-risk implementation; substantial visual and audio proposals are deferred rather than implemented.
