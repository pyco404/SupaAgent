# Mobile navigation overlay

Implemented October 28, 2026. Only the navigation shell, its styles and related browser tests changed. Other website components, backend services, credentials, financial records and historical evidence remain unchanged. The pre-existing untracked `note.txt` is outside this change and is preserved.

The mobile menu is now a full-screen native modal dialog at widths up to 640px. It uses the existing near-black background, pixel typography, restrained mint accents, vertically spaced links and thin neutral separators. The logo stays at the top-left, with a labeled 48px close button at the top-right. The active page is identified by mint text, an arrow and `aria-current="page"`.

The dialog makes the background inert, focuses its close button on opening, traps Tab/Shift+Tab within navigation, locks the underlying page and restores focus and scrolling on close. Escape, close-button selection, navigation selection, route changes, browser Back and resizing into desktop navigation close the overlay. Opening/closing animations take 180ms with an 8px/6px movement; reduced-motion preference removes animations. On shorter screens, the overlay itself scrolls and its header/close button remain sticky.

## Validation

- Production build, TypeScript, ESLint with zero warnings, and whitespace checks passed.
- **87 frontend tests passed**, with no retries, skipped or flaky tests. These include the existing frontend suite and new navigation cases for keyboard focus, Escape, Back, selection, active-page marking, body scroll restoration, desktop resize, short screens, and reduced motion.
- Automated overlay dimensions: 320×568, 390×844 and 640×800. All navigation items remain reachable without horizontal overflow.
- Installed REA tools used: `list_browser_targets`, `capture_browser_scenario`, `capture_web_screenshot`, and `compare_web_screenshots`.
- REA interacted with the authenticated real local frontend: opened the menu, dismissed it with Escape, selected Memory, verified automatic closure, reopened it and verified the active-page highlight.
- Actual measured REA captures at **320×640**, **390×844** and **640×800** cover open and closed states. Checks confirmed full viewport bounds, body scroll lock, focus on the close button and no blue accents.
- The 1440×1000 desktop homepage was compared with the previous committed screenshot using REA: **0 changed pixels out of 1,440,000**. Desktop navigation is visually unchanged.
- Screenshot bytes were verified against their REA SHA-256 hashes. Full evidence IDs, results and limitations are in [validation.json](../artifacts/mobile-navigation/validation.json).

| Screen | Closed | Open |
|---|---|---|
| 320×640 | [Closed](../artifacts/mobile-navigation/closed-320.png) | [Open](../artifacts/mobile-navigation/open-320.png) |
| 390×844 | [Closed](../artifacts/mobile-navigation/closed-390.png) | [Open](../artifacts/mobile-navigation/open-390.png) |
| 640×800 | [Closed](../artifacts/mobile-navigation/closed-640.png) | [Open](../artifacts/mobile-navigation/open-640.png) |

Screenshots are point-in-time Chromium observations. Safari and Firefox were not run. REA attached-session viewport settings alone do not establish responsive layout dimensions, so the existing local preparation helper and measured page dimensions were used before final read-only captures. Incomplete REA event coverage is not treated as proof of complete console/error coverage.

Reproduce locally with `npm run build --prefix web`, `npm test --prefix web`, then `./scripts/web-local` and the existing `scripts/rea-web-session.mjs` / `scripts/rea-web-view.mjs` helpers. Stop the preview before rebuilding. Credentials are loaded privately from the existing ignored file and never supplied inline to REA.

Every changed source, test, screenshot and report file is committed separately with a unique Conventional Commit message on `main`. No push is authorized by this task. No paid inference, X publishing, wallet operation, trading or deployment is performed.
