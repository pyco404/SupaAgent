# Current Hancore baseline: implementation and validation

Date: 2026-10-28. Scope: separate `reference-web/` implementation. The user confirmed **the current live website** as the target. This supersedes assumptions about an older opening. No Supa product features were added at this stage.

## Result

The working local baseline reproduces the main current-site structure: code/ASCII introduction, radial entry transition, terminal project browser, vertical tilted cards and branches, contribution visualization, terminal reveal, keyboard controls, image navigation, persistent footer and responsive mobile project drawer. It is independently authored and retains visible differences. It is ready for review as a baseline; it is not an exact upstream port.

The existing frontend, approved robot logo, Rust services, migrations and historical financial/evaluation records were not edited. The preview is http://127.0.0.1:3200. No deployment or push is included.

## Discovery and provenance

REA's registered tools were inspected before use. Actual tools used were `list_browser_targets`, `inspect_web_page`, `capture_web_screenshot`, `capture_browser_scenario` and `compare_web_screenshots`. The live root and `/site/` were observed, including normal-motion entry and keyboard selection. Source discovery inspected public delivered modules to understand structure, not to import their code.

The live site links the official [HANCORE-linux GitHub account](https://github.com/HANCORE-linux). A bounded inspection of its 46 public repositories and likely website names did not find the deployed site's source repository. [The profile repository](https://github.com/HANCORE-linux/HANCORE-linux) has an MIT license, but no website source was found there; its asset notices reserve third-party imagery. This search does not prove that no source or license exists elsewhere. No site-specific redistribution authorization was established.

The implementation therefore uses original code and procedural visuals. Remote previews/favicon/wordmark remain remote references, not vendored assets. The separately acquired IBM Plex Mono font includes its OFL license. Public project facts and daily counts are saved with the upstream snapshot timestamp. The original visitor/analytics systems were deliberately not reproduced: the footer explicitly identifies captured GitHub data. Original Cloudflare analytics and `/api/seen` are absent locally.

## Architecture and preservation

| Files | Responsibility |
| --- | --- |
| `index.html`, `style.css` | Terminal composition, warm ivory palette, responsive geometry, focus styles |
| `app.js` | Navigation/history, project panes, allowlisted commands, progressive reveal, preference handling |
| `scene.js` | Own-code intro, procedural face/hands, transition, contribution and entity canvases |
| `wheel.js` | Tilted project list, selection, organic connections, scroll/drag/key input |
| `reference-data.json` | Dated public reference facts; no Supa financial claims |
| `server.mjs`, `build.mjs` | Loopback read-only preview and isolated static build |
| `playwright.config.mjs`, `tests/baseline.spec.mjs` | Responsive interaction and isolation regression checks |
| `public/fonts/` | IBM Plex Mono and license |
| `evidence/2026-10-28/` | Actual REA PNG captures, evidence IDs, comparison and test summaries |

No new npm dependency was added. The baseline uses browser-native ES modules, CSS and canvas rather than replacing Supa's existing Next.js app. Static build output, raw Playwright traces/results, environment files and browser profiles are ignored. The public preview serves neither reports nor arbitrary workspace files.

## Matched browser evidence

All screenshots below were captured from actual rendered sites with REA, at matching physical viewport sizes. Static comparisons requested reduced motion to stabilize captures; the interaction pair explicitly used normal motion. Reference content is public external evidence, not a local claim of authorship. Click a filename to inspect the PNG.

| View | Reference | Local |
| --- | --- | --- |
| Desktop introduction, 1440 × 1000 | [Reference](evidence/2026-10-28/ref-intro-desktop.png) | [Local](evidence/2026-10-28/local-intro-desktop.png) |
| Desktop projects, 1440 × 1000 | [Reference](evidence/2026-10-28/ref-projects-desktop.png) | [Local](evidence/2026-10-28/local-projects-desktop.png) |
| Mobile introduction, 390 × 844 | [Reference](evidence/2026-10-28/ref-intro-mobile.png) | [Local](evidence/2026-10-28/local-intro-mobile.png) |
| Mobile projects, 390 × 844 | [Reference](evidence/2026-10-28/ref-projects-mobile.png) | [Local](evidence/2026-10-28/local-projects-mobile.png) |
| Mobile drawer open, 390 × 844 | [Reference](evidence/2026-10-28/ref-drawer-mobile.png) | [Local](evidence/2026-10-28/local-drawer-mobile.png) |
| Normal entry then ArrowDown, 1440 × 1000 | [Reference](evidence/2026-10-28/ref-interaction-desktop.png) | [Local](evidence/2026-10-28/local-interaction-desktop.png) |

Measured desktop geometry guided the centered 1008px composition, 78px header and footer around y=758 at a 1000px viewport height. Mobile uses a 350px composition within a 390px viewport, with the contribution panel before terminal text and a collapsible project wheel. Intermediate tablet and narrow 320px layouts were exercised by browser tests.

REA/CDP calibration exposed an important limit: attaching/detaching can reset caller media emulation, and a requested screenshot viewport alone did not always resize the underlying page. Final captures first prepared the physical viewport, then applied matching explicit scenario environment and settled the page. Earlier mismatched calibration captures were discarded. The `measurement` fields in the manifest describe pre-scenario preparation; final drawer/selected states are demonstrated by actions and screenshots.

The normal-motion scenario waited for the identity's GitHub link, pressed ArrowDown on the project wheel, and captured the selected Shibumi-Shell pane. Every action completed on both sites. Identity readiness in this observation took about 10.4s on the reference and 8.7s locally; these are single-run timings, not a performance benchmark or proof of identical choreography. Local progressive reveal cancels when selection changes and completes immediately under reduced motion.

REA returned screenshots, DOM/accessibility artifacts and completed actions, while marking event coverage incomplete. These runs therefore do not establish full console/network coverage or runtime equality. No video/frame-by-frame equivalence is claimed. Local browser tests separately check request origins and page errors in their exercised flows.

### Exact pixel comparisons

REA compared equal-sized PNGs at channel threshold zero:

| View | Changed pixels |
| --- | ---: |
| Desktop introduction | 9.61% |
| Desktop projects | 13.34% |
| Mobile introduction | 18.37% |
| Mobile projects | 27.41% |
| Mobile drawer | 28.24% |

All five comparisons report **different**. These percentages are exact pixel differences, not a similarity score: shared dark background can dominate the metric, while text, procedural topology, cursor phases and imagery can produce many differing pixels. Semantic layout and motion cannot be inferred from the ratio. Evidence IDs, dimensions and hashes are in [evidence.json](evidence/2026-10-28/evidence.json).

## Validation results

- `npm run check`: passed JavaScript syntax checks for application and harness modules.
- `npm run build`: passed; produced ignored static output.
- `npm test`: **39/39 passed**, across desktop 1440×1000, tablet 768×1024 and touch-emulated mobile 390×844. Zero skipped, failed, flaky or retried tests. Final run duration: 89.592 seconds.
- Responsive tests additionally exercised 320px width and both routes without horizontal page overflow.
- Verified intro/Enter/back; wheel arrows and j/k; terminal scrolling without project changes; Escape focus; allowlisted command input; direct/nested hashes and reload; palette persistence; live reduced-motion changes; mobile drawer; pointer wheel and card clicks.
- Added regression cases for reducing motion during terminal reveal and switching projects before the previous reveal completes. Both pass on all three viewport projects.
- Isolation tests verify read-only methods, CSP, blocked parent/dotfile/config requests, absence of model/Solana/backend/analytics requests in tested flows, and no application page errors in those flows.

The sanitized [validation.json](evidence/2026-10-28/validation.json) records test names/outcomes and file hashes. Rust checks were not rerun because no Rust/backend files changed. No paid model calls, wallet operations or database writes were made.

Accessibility improvements include semantic links/buttons, selected listbox options, visible focus, Escape restoration, status announcements and reduced-motion handling. During progressive reveal the terminal is marked busy and live announcements are suspended until complete. These are functional checks, not an accessibility certification. Reference-sized 12px text and some small controls remain; physical-device, screen-reader and cross-engine reviews are outstanding.

### Design-hook typography triage

The post-completion detector reported nine `undersized-ui-text` findings in `reference-web/index.html`. Each was reviewed against the matched reference captures and the user's explicit confirmation to reproduce the current live site faithfully:

| Finding | Classification and disposition |
| --- | --- |
| 8px keyboard hints | Intentional compact terminal-footer legend; retained for this reference demo |
| 9px REPOS | Intentional terminal statistics label; retained |
| 9px STARS | Intentional terminal statistics label; retained |
| 9px FOLLOWERS | Intentional terminal statistics label; retained |
| 9px WORM | Intentional terminal actor-status label; retained |
| 10px asleep | Intentional reference-scale terminal actor control; retained with its existing keyboard access |
| 9px EYE | Intentional terminal actor-status label; retained |
| 9px GHOST | Intentional terminal actor-status label; retained |
| 7px CAPTURED GITHUB DATA | Compact local provenance label within the same footer composition; retained, with the dated source explained in accessible footer metadata and this report |

No application styling was changed by this triage. The sanctioned demo exception is persisted through `impeccable hooks ignore-value` for **this rule in this HTML file only**. Other design rules and the existing Supa frontend remain in scope. This preserves intentional reference presentation rather than claiming the small sizes meet a general production usability standard. Reconsider the small footer type when adapting the baseline into the public Supa product.

The detector also reported an advisory about repeating stripes. Those stripes are confined to a theme-preview thumbnail, representing the preview's subject rather than decorating page surfaces. The advisory remains unsuppressed; no page-level stripe treatment was added.

## Remaining fidelity and release limits

1. The ASCII face/hands and contribution/branch topology are independent procedural approximations; the original filmed portrait and actor algorithms were not copied.
2. Project panes use concise public facts, previews and repository links. Full original README rendering, archive/log/editor flows and advanced ghost/worm mechanics are not reconstructed. The local worm toggle is simplified.
3. Only three dark palettes are implemented; this is not the reference's complete theme catalogue.
4. Captured GitHub counts are dated, not live statistics. A labeled snapshot replaces the original visitor counter. They must never be relabeled as Supa activity.
5. Remote assets may change or fail offline. Public reuse requires replacing them with approved assets or obtaining applicable permission; the profile repository license does not clear them.
6. Validation uses Chromium desktop/mobile emulation, not physical devices or all browsers. Original motion timing and source-level equality remain unproven.

The appropriate next step is review of these baseline captures and outstanding fidelity decisions before integrating Supa branding or product capabilities. This report does not mark exact reference equivalence complete.

## Follow-up: slow top-to-bottom opening code

The requested opening now reveals characters from the first line downward, in both desktop panes and the single mobile pane. Its typing budget is 30ms per character with eight additional character intervals (240ms) between lines. Completed lines stay in place; the previous periodic line rotation is removed. A caret follows the active line. The sequence starts again on page load/source initialization, pauses while the tab is hidden, and leaves Enter navigation available throughout. Reduced motion reveals all text immediately and keeps it complete when normal motion is restored.

Actual REA normal-motion captures at 1440×1000 showed approximately two partially revealed lines after the early wait and seven lines after the later wait, with lower rows unpainted. Evidence IDs: `ev_ac2c19e34aec6542bef94dd9f8085f9e9369cc00f7a4a26b5017475b863f1bc3` (1.8s requested wait) and `ev_4c5037229bc03864f4a91ed961ceb26f7c57aa164f02c90661ac2bfd4d2082fd` (8s requested wait). These are sampled browser observations, not frame-exact timings. The earlier committed screenshots and hash manifest remain historical baseline evidence, not refreshed artifacts for this change.

A new browser regression verifies gradual character/row progress and immediate completion under reduced motion across desktop, tablet and mobile. The first full run passed 41/42 cases; one existing desktop navigation case exceeded its total timeout amid slow startup and browser commands. Its trace recorded the expected selection change. A complete rerun was performed after closing the independent inspection browser, with automatic retries still disabled.

The final complete rerun passed **42/42 tests**, with no failed, skipped, flaky or automatically retried cases. Syntax checks, static build and whitespace validation also passed. The original 39-test validation JSON above documents the earlier baseline; the latest full result is in the ignored local `test-results/results.json`.

## Follow-up: screenshot-matched single editor

The opening now follows the user's supplied current-site screenshots: one wide editor instead of mirrored columns, a file heading and numbered rows, muted token coloring, an active-line highlight and caret, and one continuous mode/file/cursor status bar. The status switches from INSERT to NORMAL when the slow top-to-bottom reveal completes. The displayed source path and cursor refer to this implementation's own public code; the terminal-style command beneath is decorative and executes nothing.

The larger face is independently generated from code glyphs shaped around eyes, nose and mouth, with an eye box and dashed guide to the actual pointer position. Hands use articulated finger landmarks, palm connections and bounding boxes above a keyboard layout. All geometry is procedural and pointer-driven; there is no webcam, face capture or biological tracking. These are approximations of the supplied screenshots, not copied portrait assets or a claim of exact visual equality.

REA inspected the rendered desktop (1440×1000) and mobile (390×844) composition with reduced motion for stable captures. Both scenario actions completed; event coverage remained incomplete, so no complete runtime-equivalence claim is made. The mobile status bar, header and footer fit the viewport without horizontal page overflow. The previous Supa website and project-browser components remain separate.

| Updated capture | REA evidence ID | PNG SHA-256 |
| --- | --- | --- |
| [Single editor, desktop](evidence/2026-10-28/intro-single-editor-desktop.png) | `ev_d10846eb0756232cdd479f07c86d4537ad37340a9e964e4a90c3b75f0b44a767` | `8e13cf0397340bc754b62c6481650a76d212fcf5cafa01cfc16573f23cd8efce` |
| [Single editor, mobile](evidence/2026-10-28/intro-single-editor-mobile.png) | `ev_896bb33344ece4fae943cfeb12c5d84ad32d115c9babd6cfbc0252b6aa3d143b` | `0a2917a41a3584af68be504ea5ba2bc99f79cf919ca24f7f7998be54521d9f84` |

The captures predate only the nonvisual screen-reader description added to clarify that the geometry is procedural. Earlier paired screenshots and their comparison metrics remain historical baseline evidence. The new regression checks a single mode label, true source filename, cursor position, status-bar bounds, theme interaction and status dismissal when entering the project browser on each viewport.

Validation for this follow-up passed: syntax checks, static build, whitespace checks, and **45/45 browser tests** across desktop, tablet and mobile, with zero failed, skipped, flaky or retried cases. The focused design detector reports zero failures; the previously documented theme-thumbnail stripe advisory remains unchanged.

## Follow-up: radial scene before entry

Enter now presents an opaque, full-screen procedural radial scene before navigating to the existing `/site/` home/project browser. A centered black disk, thin warm-ivory rim, restrained surrounding light and 1,500 moving streaks reproduce the supplied image's composition. The scene lasts 2.4 seconds; its final phase expands the disk before the destination appears. It is rendered locally in canvas rather than loading or briefly flashing a static external image. The intro cannot show through the transition.

The destination is held until animation completion. Repeated Enter requests are ignored while entry is pending. An explicit view/history change cancels the pending animation and invalidates its navigation attempt, preventing a stale promise from redirecting the user afterward. Reduced-motion users enter immediately; enabling reduced motion during the scene completes entry without waiting for the remaining motion.

REA exercised actual Enter clicks and sampled the intermediate scene at desktop 1440×1000 and mobile 390×844 with normal motion. Both scenarios completed their actions. These are sampled frames, not proof of frame-exact equivalence to the reference. The earlier baseline and editor captures remain historical evidence.

| Transition capture | REA evidence ID | PNG SHA-256 |
| --- | --- | --- |
| [Desktop radial scene](evidence/2026-10-28/enter-portal-desktop.png) | `ev_4e09bef2fa5241dad85fd35cd9f3b691d750089bd5b3f2116611cdeb4290ed9e` | `0b9bfb8e1d0860bcabd5ada70ff10fab8307f0f654f39dc70251162dbbd7836f` |
| [Mobile radial scene](evidence/2026-10-28/enter-portal-mobile.png) | `ev_c68c0b03009589d3cae11c12b6538ef587c890b1394353e7e8360959b3d5fe8d` | `9032a069613b45f26d937d16472528b0be58aec9f8ef44b29f862efeb2d8be18` |

The added regression checks opaque canvas pixels (including a black center), the intro remaining selected before entry finishes, exactly one history entry after repeated Enter, browser-back restoration, immediate completion when reduced motion is enabled, and cancellation when a popstate arrives mid-transition. The popstate cancellation is a controlled browser event test; ordinary back navigation is tested separately.

Validation passed: syntax checks, static build, whitespace checks and **51/51 browser tests**, with zero failed, skipped, flaky or automatically retried cases. The six added viewport cases cover the transition flow and cancellation. Candidate file scans report zero secret findings, and both PNG hashes were verified before committing.

## Follow-up: centered keyboard and typing fingers

The intro keyboard is enlarged to 700px on the 1440px desktop capture and 350px at a 390px mobile viewport. It is horizontally centered below the editor, with visible key outlines, staggered rows and a space bar. Both hands now rest over the home-row keys rather than floating to either side.

The finger motion shares the editor's reveal clock: approximately every five character intervals it selects a visible source character, maps punctuation to a key, highlights the key and bends the corresponding finger toward it. Fingertips descend and recover over a smooth press cycle. This is a decorative representation of the code reveal, not a claim of one physical keystroke per displayed character. Finger animation and highlights stop when the reveal is complete or reduced motion is enabled; the hands remain resting on the keyboard. No camera or external animation dependency is introduced.

REA captured the actual normal-motion scene on desktop and mobile. The images verify scale, centering and hand placement; animation is additionally checked by comparing rendered pixels in the keyboard area over time and verifying they stay unchanged after reduced motion is enabled.

| Capture | REA evidence ID | PNG SHA-256 |
| --- | --- | --- |
| [Desktop typing keyboard](evidence/2026-10-28/typing-keyboard-desktop.png) | `ev_968ab4be14b880791d2e778c3cc65d778c3ae8ae142debdcb5790b0bfd10932c` | `901a89f161047df5fb7483694bc4abb486595ff7b94ddfa7c2fde26a9c7b0c14` |
| [Mobile typing keyboard](evidence/2026-10-28/typing-keyboard-mobile.png) | `ev_6c5b88e54aa4feeb0f5c2f55f8b3db110e4090d00e8c850b2b148acbd06234fe` | `2fc8ba3a100c1ad3ed920e7216f7186e167ee1932b33c99dbd34b8889fe0bd76` |

Validation passed: syntax checks, static build, whitespace checks and **54/54 browser tests**, with zero failed, skipped, flaky or retried cases. The added regression checks keyboard width, horizontal centering and vertical bounds, visible pixel changes during typing, and unchanged keyboard/hand pixels under reduced motion. Candidate secret scanning reports zero findings; both screenshot hashes match the recorded values.

## Git handling

All new eligible files are committed individually on `main` through `scripts/commit-one`, with one descriptive Conventional Commit per file and hooks enabled. Secret scanning and the one-file history audit are required before completion. The pre-existing root `.gitignore` edit is preserved and excluded from these commits. No archival branch, private evaluation files or credentials are added. No push or deployment is authorized by this task.
