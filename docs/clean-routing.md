# Clean website routing

The approved experience uses pathname navigation through its existing browser
History API and the existing Node/Next.js hosts. No Rust endpoint, database,
permission, credential, financial control, or animation implementation changed.

| Section | Canonical path |
| --- | --- |
| Home | `/` |
| Agents | `/agents` |
| Intelligence | `/intelligence` |
| Memory | `/memory` |
| Economy | `/economy` |
| Social | `/social` |
| Trading | `/trading` |
| Token Launch | `/token-launch` |
| Factory | `/factory` |
| About | `/about` |

## Browser behavior

The cinematic introduction remains at `/`. Enter opens Home at the same path
using a history state entry; Back returns to the introduction. A Home workspace
entry survives refresh through its history state. Opening `/` in a fresh tab
starts the approved introduction. Other section paths enter their workspace
directly. The brand returns to the introduction.

Wheel and terminal navigation push a new entry only when the pathname changes.
Back/Forward restores selection without pushing another entry. Background
polling does not write history. Existing query parameters are retained during
section navigation. The skip-to-workspace fragment remains an accessibility
anchor; it is not a section route.

`/site` and `/site/` still serve the entry document so its client can read the
fragment, which is never sent to an HTTP server. The client replaces that legacy
entry with the corresponding clean pathname and preserves its query string.
This avoids a legacy redirect in the Back stack. An unknown legacy section
navigates to a local not-found path. `/home` redirects to `/`; section trailing
slashes normalize to clean paths. Unknown public paths return an HTTP 404 and a
minimal terminal-style page with a Home link.

Page titles and canonical metadata follow the selected section. The Enter link
and authenticated operator redirect use clean paths. Absolute asset and API
paths are unchanged. Planned capability labels remain in the wheel and content;
navbar page names remain concise.

## Hosting

`reference-web/server.mjs` serves the ten exact paths and legacy entry paths.
The existing Next.js experience route handler serves the same paths in private
staging mode through the existing rewrites and authentication proxy. Normal
Next.js application routes outside that mode are unchanged. The static build
exports each section entry and the not-found asset; the Dockerfile explicitly
includes that asset. No hosting resources were created or deployed.

## Reproducible validation

- `npm --prefix reference-web run check`
- `npm --prefix reference-web run build`
- `npm --prefix reference-web run test:unit`
- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run build`
- From `reference-web`: `../web/node_modules/.bin/playwright test`
- `node scripts/test-experience-routing.mjs`
- `node --test .railway/railway.test.mjs`
- `python3 scripts/scan-secrets.py`
- `python3 scripts/git-policy.py audit`

The production Next.js smoke test starts an ephemeral loopback host with random,
process-only synthetic visitor/operator credentials, no backend, and no database
or provider. It checks all ten paths and refreshes on desktop/mobile, legacy
migration, queries, public authentication, the operator redirect, assets, and
404s. It shuts its host and browser down afterward.

Routing regressions cover direct entry, refresh, matching selected cards,
canonical metadata, query retention, legacy URLs, history traversal, terminal
commands, and keyboard wheel navigation. Older Home-entry regressions retain
legacy URLs intentionally to exercise backward compatibility.

REA desktop evidence: `ev_7e5e5a3aeef0a30307c85bbd1e8f09e1cff4ab0a63bed0cd9057f03c591e4db0`.
REA mobile evidence: `ev_46c9a1ffac1eb2eb7d8aed48600ea0563c2e601426ea9411e005ceb15e4a199f`.
Both scenarios completed with no page exceptions. The primary disconnected shell
still returns expected 503 responses for status/directory reads; this work does
not start its database or imply restored records. REA desktop exercised terminal
navigation from Memory to Social; mobile exercised legacy migration and drawer
keyboard navigation from Memory to Economy.

Generated screenshots, browser logs, and Next.js smoke results stay local under
ignored `artifacts/clean-routing/`; regression reports remain ignored under
`reference-web/test-results/`. None belong in GitHub.

## Validation results (2026-10-31)

- Static frontend check/build: passed; ten section entries, legacy entry, and
  not-found document exported.
- Frontend adapter/unit tests: 28 passed.
- ESLint and TypeScript: passed. The production Next.js build passed after
  allowing its TypeScript subprocesses to run outside the restricted sandbox;
  the initial restricted build could not parse the subprocess output.
- Focused route/carousel run: 30 passed across desktop, tablet, and mobile.
- Full 222-case browser regression run: 219 passed; three palette tests still
  assumed that navigating to the same `/` URL always resets Home to the intro.
  Updated the setup to use the real brand/Enter controls and wait for Home.
- Final routing/accessibility run: 18 passed across all three viewports,
  including the three corrected palette tests and three added skip-link cases.
  Together these runs cover all 225 unique browser cases without an unresolved
  failure. The broad run and focused rerun are separate evidence, not a claim
  that one 225-case invocation ran green.
- The skip-link regression checks view preservation, focus, refresh,
  Back/Forward, query retention, and navigation from Home to Memory.
- Production Next.js private-staging smoke: passed for all ten routes at
  1440px and 390px, including refresh and authentication isolation.
- Railway configuration/Dockerfile assertion suite: passed. This was a static
  configuration check, not a new container image build or deployment.
- REA desktop/mobile interaction scenarios: completed, with no page exceptions.
  Expected disconnected API 503 responses remain visible and were not hidden.

The first focused attempt also caught an incorrect test expectation that unknown
API endpoints return a public-page 404; the existing API correctly rejects them
with 403. The test now preserves that API boundary. A carousel timing assertion
was sensitive to a concurrent production build; all 30 focused tests passed
when run without that heavy build. Neither observation required loosening
application permissions or changing the approved animation.
