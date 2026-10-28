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

## Remaining fidelity and release limits

1. The ASCII face/hands and contribution/branch topology are independent procedural approximations; the original filmed portrait and actor algorithms were not copied.
2. Project panes use concise public facts, previews and repository links. Full original README rendering, archive/log/editor flows and advanced ghost/worm mechanics are not reconstructed. The local worm toggle is simplified.
3. Only three dark palettes are implemented; this is not the reference's complete theme catalogue.
4. Captured GitHub counts are dated, not live statistics. A labeled snapshot replaces the original visitor counter. They must never be relabeled as Supa activity.
5. Remote assets may change or fail offline. Public reuse requires replacing them with approved assets or obtaining applicable permission; the profile repository license does not clear them.
6. Validation uses Chromium desktop/mobile emulation, not physical devices or all browsers. Original motion timing and source-level equality remain unproven.

The appropriate next step is review of these baseline captures and outstanding fidelity decisions before integrating Supa branding or product capabilities. This report does not mark exact reference equivalence complete.

## Git handling

All new eligible files are committed individually on `main` through `scripts/commit-one`, with one descriptive Conventional Commit per file and hooks enabled. Secret scanning and the one-file history audit are required before completion. The pre-existing root `.gitignore` edit is preserved and excluded from these commits. No archival branch, private evaluation files or credentials are added. No push or deployment is authorized by this task.
