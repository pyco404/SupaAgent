# Three-column spacing refinement

Local-only review, 7 November 2026. Preserve the approved Supa Agency palette,
type, illustration assets, wheel behavior, content, authentication and capability
gates. This change does not publish or deploy the website.

## Structural changes

- `.editor` now owns three explicit Grid tracks: a responsive 190–230px wheel,
  a flexible terminal capped at 620px, and a 120–176px decorative ASCII column.
  Both horizontal gutters use one `clamp(16px, 2vw, 32px)` token. The bounded
  1120px frame provides breathing room without an unbounded text measure.
- Move the existing decorative entity canvas one level outward, into that third
  track. Its DOM identity, accessibility state and animation code are unchanged.
  It no longer relies on absolute/fixed coordinates inside or above the terminal.
- Align wheel, terminal pane and visualization containers at the editor's top.
  Existing terminal header and illustration hierarchy remains inside the center.
- Keep terminal content as a native, focusable overflow region with `min-height:0`
  and bottom scroll padding. Reserve 16px between its bottom and the footer.
- Put status, note and keyboard in explicit project grid rows. The animated
  keyboard remains below content in normal flow instead of covering the viewport.
- Below 701px, preserve the navigation drawer and stack the small decorative
  visualization above the terminal. Compact heights reduce illustration/portrait
  height. A 600px content floor permits ordinary document scrolling on very short
  landscape screens instead of clipping controls behind a fixed footer.

No artwork, text, font, palette, backend adapter, authentication logic or execution
controls changed. No new runtime dependency or animation implementation was added.

## Matching captures and measurements

`scripts/test-terminal-layout.mjs --before` captured the original source before
edits. The default invocation applies strict after-layout checks. Both use the
same eight viewport sizes, reduced-motion setting, section routes and image
decode readiness. Before mode records geometry rather than accepting the old
layout as correct. Evidence lives privately in `artifacts/terminal-spacing`.

| Viewport | Center width before → after | Equal new gutters | Factory footer clearance before → after |
| --- | --- | --- | --- |
| 1440×1000 | 532 → 620px | 28.8px | 0 → 16px |
| 1366×768 | 532 → 620px | 27.3px | 0 → 16px |
| 1280×600 | 532 → 620px | 25.6px | −6 → 16px |
| 1024×768 | 718 → 567.8px | 20.5px | 0 → 16px |
| 768×1024 | 462 → 364.5px | 16px | 0 → 16px |
| 390×844 | 350 → 350px | Stacked | 0 → 16px |
| 375×667 | 335 → 335px | Stacked | 0 → 16px |
| 844×390 | 538 → 432.7px | 16.9px | −216 → 16px |

At 1440px the old effective gutters were 16px and 32px. At 1024px the old
visualization overlapped the terminal by 124px. All after-layout desktop/tablet
columns align within one pixel, use equal gutters and remain separate. All eight
sizes reach the final Factory links inside the scroll region with footer
clearance, no horizontal overflow and no private requests or writes.

The retained before/after PNGs use names such as
`before-desktop-agents.png`, `after-desktop-agents.png`,
`before-mobile-agents.png`, `after-mobile-agents.png`, and
`before-compact-factory-end.png` / `after-compact-factory-end.png`.
Complete rectangles and scroll measurements are in
`before-measurements.json` and `after-measurements.json`.

The public [Hancore reference](https://hancore.dev) was inspected read-only at
1440×1000. `reference-composition.png` records its rendered wheel, bounded center,
separate ASCII visualization and lower status/keyboard composition. The first
captures preceded its entrance completion; only the final visible-navigation
capture is comparison evidence. Its assets and source were not copied. The
reference informs spatial balance, not pixel identity or Supa product behavior.

## Validation and limits

- Eight viewport pairs, each covering Agents and long Factory: 16 after-layout
  checks plus matching initial/end-of-scroll screenshots. All passed.
- 36 cinematic, responsive, atmosphere, clean-route, history and skip-link browser
  regressions passed across desktop, tablet and mobile.
- 12 targeted branch, pointer/text tether, keyboard/hand and footer-flow browser
  regressions passed, including pause and reduced-motion behavior.
- Six existing session/authentication tests passed; anonymous operator, My Agents
  and private API endpoint checks still return 401.
- Layout detector reports no findings. Node syntax and whitespace checks passed.
  The existing short-screen architecture assertion now accepts the intentional
  112px compact illustration slot; it retains footer/keyboard reachability checks.

Initial browser regression trace writes exhausted the local disk and one test
timed out. Diagnostics are retained in `initial-regression-diagnostics.json` and
`browser-regressions.log`. Moving temporary traces to memory-backed storage made
the unchanged assertions and timeouts pass; no application fallback or test
suppression was introduced. Final logs are `final-browser-regressions.log`,
`animation-layout-regressions.log`, `auth-regressions.log` and the after-layout
measurement report. These are local Chromium checks, not Safari, production
performance or formal accessibility certification.

## Review scope

Changed source: `reference-web/index.html`, `reference-web/style.css`, one existing
layout regression in `reference-web/tests/baseline.spec.mjs`, the new local capture
runner and this report. Existing unpublished avatar, token and visual work remains
intact. Synchronization, image validation/publication and deployment retain their
separate approval requirements. No live service, DNS, credential or database was
changed, and no disabled capability was enabled.
