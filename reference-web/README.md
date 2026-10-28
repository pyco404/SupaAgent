# Supa Agent cinematic website

The approved dark, warm-ivory terminal experience now presents Supa Agent content.
The code introduction, typing hands, robot, radial portal, tilted wheel, organic
branches, terminal and ambient entity remain. The older Next.js operator console
in `web/` remains intact; the Rust workspace and database are unchanged.

## Local setup

Requires Node.js 24.22+ (native TypeScript loading), Chrome at
`/usr/bin/google-chrome`, and the existing console dependencies:

```sh
npm --prefix web ci
npm --prefix reference-web run dev
```

Open **http://127.0.0.1:3200**. Enter moves from `/` to `/site/#home`.
The server binds only to `127.0.0.1`. Nothing is deployed.

The server reads only `SUPA_BACKEND_URL`, `SUPA_API_TOKEN`, `SUPA_WEB_USERNAME`
and `SUPA_WEB_PASSWORD` from the existing owner-only `web/.env.local` file.
Shell values take precedence. It never loads the evaluation key or budget file.
`SUPA_SITE_ENV_FILE` can select another private, owner-owned mode-0600 file;
set it to an empty string for a deliberately disconnected site. Do not put
credentials in client files or commit private configuration.

Backend URL must be a credential-free loopback HTTP origin. Bearer tokens stay
on the Node server. No model, blockchain or publishing requests originate here.

## Sections and interaction

The wheel has Home, Agents, Intelligence, Memory, Economy, Social, Trading,
Token Launch, Factory and About. Trading, Pump.fun launches and Factory are
planned and non-executable. The $5 trading ceiling describes a future policy,
not current trading availability. SUPA notation does not imply a launched token.

Scroll, drag, touch or use arrows/j/k on the wheel. In the terminal these keys
scroll saved content; Escape returns to navigation. The mobile menu remains open
while browsing options and closes on activation, Escape or history navigation.
Its background is inert while open. Reduced motion bypasses intro/portal motion
and reveals complete section text immediately.

Click Command or type `:`. UI-only commands:

- `supa status`, `supa agents`, `supa inspect <exact name or UUID>`
- `supa intelligence`, `supa memory`, `supa economy`, `supa social`
- `supa capabilities`, `supa help`, `theme`, `q`

Up/down inside command input browses the last 50 commands in memory. No command
runs a shell or writes backend data. Commands are not persisted to browser storage.
Only the local dark-theme preference is stored.

## Public and operator records

Public `/api/status` exposes validated health. Public `/api/agents` deliberately
projects only ID, name, lifecycle and creation date. It never forwards goals,
memory, personality, model prompts, tool arguments or agent configuration.

Use **Operator sign-in** to unlock private read-only records with the existing
console credentials. This is an explicit browser Basic-auth challenge on
`/operator`; private API requests without authentication return a sanitized 401
without prompting public visitors. Auth is evaluated server-side using the
existing `web/src/lib/auth.ts` rules. Browser credentials use loopback HTTP only;
this local auth design is not a production account system. Close the browser's
operator session to clear its cached Basic credentials; no logout endpoint is
claimed.

The Node adapter imports the existing Zod contracts from
`web/src/lib/contracts.ts`, strips unknown fields, scrubs known/recognizable
credentials, limits responses to 2 MB, uses five-second upstream timeouts and
rejects redirects. All API operations are GET/HEAD. Unknown paths, queries,
non-local hosts/origins and every write method fail closed.

Intelligence shows saved goals, cycles, activity and authorization outcomes;
Memory labels hypotheses and verified observations separately; Economy keeps
simulated micro-USD, saved Devnet test assets and unavailable Mainnet separate;
Social shows original stored drafts, event/memory references and review history.
No draft is published or generated when viewed. Retrieved timestamps are not
blockchain observation timestamps. No polling or invented live activity is used.
Reload saved records explicitly updates the selected view.

Provider/model attribution is currently unavailable through the reused console
contracts. Empty histories remain empty. Historical GPT evaluation databases are
not merged or selected automatically. Wallet signing remains disabled.

## Validation and assets

```sh
npm --prefix reference-web run check
npm --prefix reference-web run build
npm --prefix reference-web run test:unit
npm --prefix reference-web test
```

The build writes ignored `dist/` **static assets only**; it does not package the
private adapter or credentials. A static host without the Node API has honest
disconnected states. Tests use disconnected port 3201 plus explicitly synthetic
read-only fixtures on 18082/3202. They never connect a database or paid provider.

See [SUPA-REPORT.md](SUPA-REPORT.md) for API evidence, browser captures, security,
parity gaps and independent critique. `REPORT.md`, `reference-data.json` and the
earlier screenshots are historical reference evidence, preserved unchanged.
The legacy snapshot is excluded from the server allowlist and fresh builds;
no reference account, statistics, remote screenshots or favicon is used by Supa.

The approved SVG robot retains its exact pixel silhouette. IBM Plex Mono is
self-hosted under the included [SIL Open Font License](public/fonts/OFL.txt).
All interface code and geometry are independently authored; no upstream website
scripts, styles or filmed visual assets are vendored.

## Code-built logo refinement

The large intro and portal robot now use
[`supa-agent-code.svg`](public/assets/supa-agent-code.svg): source-code characters
clipped to the **unchanged canonical pixel path**, including transparent eyes
and mouth. The intro retains a faint, lightly blurred treatment behind the
editor. The animated text entity also uses this mask; its characters change
without deforming the silhouette. Header/favicons retain solid pixels for small
sizes. This is decorative source text, not runtime activity or model output.

Syntax/build passed, and 15 focused Chromium tests passed across desktop, tablet
and mobile. Tests retain canonical solid-mask comparisons and verify the new
code texture, exact path, transparent cutouts, portal fit, reduced motion and
responsive overflow. REA rendered captures:
[desktop](evidence/2026-10-28/supa-code-logo-desktop.png) /
[mobile](evidence/2026-10-28/supa-code-logo-mobile.png).
Evidence IDs: `ev_2258aacc694b5ba8a3d5f7b4529303428b853c0b2b39ef63acdadec0272ccb27`
and `ev_29a92d8811fe5966e30a1a59a91f57b93892f4f9e97e303b74bf05bb2cff896d`.
Physical viewports were calibrated before each capture. REA reported missing
event coverage; screenshots do not establish complete runtime equivalence.


## Typing on every visit

Every section types its full content from top to bottom on each visit, return,
agent selection or saved-record reload. Shared navigation/controls stay usable.
Text follows the saved DOM order, including headings, record metadata, draft
wording and planned-feature explanations; no content is invented by the effect.
The terminal follows the typing cursor when already scrolled to the bottom and
respects readers scrolling back up.

**Show all text** completes the current section immediately and restores terminal
focus. Reduced-motion preference does the same, including a change made during
playback. Live announcements pause during typing; incomplete controls remain
inert until their row finishes. Switching pages cancels obsolete animation so
previous-page text cannot reappear. This user-requested behavior supersedes the
prompt-only typing described in the earlier transformation report.

Validation for page-wide typing: syntax/build passed, 12 focused checks passed,
and the full Chromium suite passed **87/87** with no failures. REA captures:
[desktop](evidence/2026-10-28/supa-page-typing-desktop.png) /
[mobile](evidence/2026-10-28/supa-page-typing-mobile.png).
Evidence: `ev_fdfc22538adb5b9e1dc7f95692dbc0f949ac43a2252ca4e40aa218d44176dc9f`
and `ev_31756c552d143fbe63ba2f35ae3063e5ec729393de2e86d77292960f2224b5a7`.
Physical viewports were calibrated before capture; REA's missing event coverage
means these captures are not a complete runtime-equivalence claim.

### Dense source-character composition

The large code robot now consists of 60 full, syntax-colored source lines rather
than a short repeating pattern. The text is sampled exclusively from the public
frontend's own `app.js`; no environment, private memory or model context is used.
There is no solid filled mascot underneath. The original even-odd mask preserves
the exact silhouette and empty eye/mouth areas. The intro treatment is larger,
clearer and only lightly softened, matching the supplied text-portrait reference
while retaining the Supa robot shape. Small solid icons remain readable.

Syntax/build and 12 focused desktop/tablet/mobile tests passed. Actual REA intro
captures: [desktop](evidence/2026-10-28/supa-source-mascot-desktop.png) /
[mobile](evidence/2026-10-28/supa-source-mascot-mobile.png).
Evidence IDs: `ev_43f68ad850a393c0c07c45589fde7409cb9b2871c3a4e0f63dd6e7c6a922e287`
and `ev_064bd67f7ad47146cb32e88edc0a9f9ae53e559e051f29703dc10ba10e550cea`.
Viewports were calibrated before each capture; event coverage was unavailable.

### Clear workspace toolbar and current Home logo

The workspace toolbar now contains only the agent selector. Reload saved records,
Show all text and Operator sign-in are available through the footer's SUPA AGENT
command button (or the `:` keyboard shortcut). The panel has a close button and
supports Escape; reload and text reveal close it after use. The Home navigation
card uses the latest source-character robot SVG instead of the solid icon.

Syntax checks, static build and **42 focused browser tests passed** across desktop,
tablet and mobile, including relocated controls, typing, navigation, permission
boundaries and loading/error recovery. REA inspected the actual local page at
1440×1000 and 390×844 with no horizontal overflow:
[desktop](evidence/2026-10-28/supa-clean-toolbar-desktop.png) /
[mobile](evidence/2026-10-28/supa-clean-toolbar-mobile.png).
Evidence: `ev_e9e9589badbdce47aa9989d29dd50c93341ef196e7afb46741359781fceaf624`
and `ev_864d57d6bf7e3ed3d76cabfe3f8e27c9df6d137963f1358c6b15ea114b683bb8`.
These screenshot/DOM/accessibility captures lack event coverage; interaction
assertions come from the browser tests, not a complete REA runtime trace.

### Pointer tether

A viewport-sized, non-interactive canvas connects the code mascot to the mouse
with a warm-ivory dashed line and cursor box. The former pointer listener stored
coordinates without rendering them. The tether now follows those coordinates,
hides on pointer exit/window blur, touch input and portal transitions, and
is removed with the scene. Reduced motion follows deliberate pointer input
without adding easing or a continuous animation loop.

Syntax/build and 12 focused desktop/tablet/mobile browser tests passed, including
actual painted-line pixels, endpoints, reduced motion, input pass-through, touch
suppression, keyboard commands, responsive scenes and navigation focus. REA
captured the real local viewport after Playwright moved the mouse:
[desktop](evidence/2026-10-28/supa-pointer-tether-desktop.png) /
[mobile with mouse](evidence/2026-10-28/supa-pointer-tether-mobile.png).
Screenshot evidence IDs: `ev_82a9679c4f85dc421a408991370fa36dee5b9ce3e7a53e48f5c5158a169dd033`
and `ev_b3ed94db5acd6b0a59dbc2f03382d614fbe5e70cafeb5002f14ff0818b23cbbb`.
REA screenshots establish appearance; automated browser assertions verify behavior.

### Animated decorative branches

The architecture canvas now gently sways around its fixed root, with small
highlights following the existing cubic branches. The warm terminal palette,
shape, statistics and explicit “not runtime activity” label remain intact.
This is decorative motion, independent of transactions, balances or agent events.
The footer command panel includes a keyboard-accessible Pause/Play diagram toggle.
Reduced motion renders a stable diagram; animation time freezes offscreen, during
portal transitions and in hidden tabs, without catching up on return. It reuses
the existing capped canvas frame loop and adds no animation dependency.

Syntax/build and 15 focused browser tests passed across desktop, tablet and mobile.
Tests compare actual painted pixels, verify pause/resume and offscreen freezing,
and confirm reduced motion remains static. Command access, clean workspace,
pointer interaction and responsive fit also pass. REA screenshots of the local
running animation: [desktop](evidence/2026-10-28/supa-animated-branches-desktop.png) /
[mobile](evidence/2026-10-28/supa-animated-branches-mobile.png).
Evidence IDs: `ev_fa5cff7184049adec7499deb8a52ddcc57308a75c321a31c9ffa52279a9dd2a1`
and `ev_d2b78c22bb05680b24dad08fddbd1e8cb617e16c2c9866e43c0cdea23410a228`.
Screenshots establish appearance; browser pixel comparisons establish motion.

### Opening-screen mouse pointer

The same dotted mascot-to-mouse tether and small outlined cursor box now appear
on the typing intro. Its origin follows the code robot's responsive bounds;
entering the portal suppresses it and entering the workspace switches its anchor
to the workspace mascot. It never intercepts input and remains suppressed for
touch input. Pointer-driven updates also work with reduced motion.

REA inspected the rendered intro with a mouse at desktop and mobile widths:
[desktop](evidence/2026-10-28/supa-intro-pointer-desktop.png) /
[mobile with mouse](evidence/2026-10-28/supa-intro-pointer-mobile.png).
Evidence IDs: `ev_3015f07e664b3bbfcf92266cbe66cadeb8b5f23000794201ffd2ddcb1c6bad08`
and `ev_64be3207f30818c79d6104a8644b3ab2d63d30a6cd9c4f68d118b8754592a12d`.
Screenshots establish appearance; browser tests check responsive anchor positions,
painted pixels, touch suppression and transitions between intro and workspace.
Syntax/build and all 15 focused desktop/tablet/mobile browser tests passed.

### Keyboard/mouse hand coordination and text tether

The right hand now uses an outlined mouse while the pointer moves, then returns
to the keyboard after 700 ms of inactivity. The left hand stays at the keyboard.
Both hands follow the existing screen-typing clock; no camera or device tracking
is involved. The keyboard keeps identical bounds across the handoff. Touch input
does not activate the mouse pose, and reduced motion keeps typing gestures still.

The workspace now has a non-interactive input illustration in its own row below
saved-record status. It uses the same renderer as the intro. A second dotted
line follows the actual typing position: canvas cursor coordinates on the intro,
and a DOM Range around the current rendered character in the workspace. A
focused command field uses its visible caret position. Offscreen text endpoints
are omitted; navigation clears stale text references, and the portal hides both
lines. The pointer line continues to mark the actual mouse position separately.
No saved record, inference, transaction or model context is generated or changed.

Syntax/build checks passed. The initial 27 focused browser checks passed; after
stabilizing keyboard geometry, all nine geometry/input/responsive checks passed
again across desktop, tablet and mobile. These check real hand transitions,
keyboard bounds, typing synchronization, text endpoints and reduced motion.
REA inspected the running local page:
[mouse pose on desktop](evidence/2026-10-28/supa-input-hands-mouse-desktop.png) /
[keyboard pose in mobile workspace](evidence/2026-10-28/supa-input-hands-keyboard-mobile.png).
Evidence IDs: `ev_d570b8980a82cea3c3b2b5d17c94d9be433337cccab67f69f6e8847b84dc31e9`
and `ev_241c4f433207faf60ee7c42adfc6cec87ac6c31b5273042060fd6c5359f20660`.
The mouse capture uses controlled CDP movement; the keyboard capture follows its
cessation. Screenshots establish appearance; automated browser tests verify timing.
