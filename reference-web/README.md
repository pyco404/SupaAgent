# Hancore reference baseline

A separate, independently authored reconstruction of the **current** [hancore.dev](https://hancore.dev/) experience observed on 2026-10-28. This is a local reference study, not an official upstream source port or a Supa Agent product release. Supa's existing `web/`, robot logo, Rust workspace, database and safety controls are preserved.

## Run locally

From the repository root, with Node.js and the existing frontend dependencies installed:

```sh
cd reference-web
npm run dev
```

Open **http://127.0.0.1:3200**. The preview binds to loopback only. `/` opens the code/ASCII introduction; Enter opens `/site/`. No authentication or backend connection is needed.

```sh
npm run check
npm run build
npm test
```

The build creates ignored `dist/` static output. Tests use the repository's existing `web/node_modules` Playwright installation and `/usr/bin/google-chrome`; they start their own loopback server on port 3201. On a fresh checkout install the existing frontend dependencies from the repository root with `npm --prefix web ci`. No additional animation or application dependencies were introduced. There is no deployment step.

## Implemented baseline

- Almost-black and warm ivory terminal presentation, locally hosted IBM Plex Mono, responsive header and persistent statistics footer.
- Authored code-wall opening, procedural ASCII entity and hands, pointer/keyboard response, and radial transition into the project browser.
- Vertical tilted project wheel, organic canvas branches, scroll/drag/click selection, arrows and j/k navigation, folders and fragment deep links.
- Contribution visualization based on a captured public GitHub snapshot, progressive terminal reveal, image browsing/expansion and repository links.
- Mobile project drawer with Escape/focus restoration; browser back navigation; three persisted dark palettes.
- Reduced-motion handling, cancellation of obsolete terminal reveals, hidden-tab animation suspension and restrained frame scheduling.

In the terminal, Escape returns focus to navigation; j/k and arrows scroll the text rather than changing projects. `:` opens a small allowlisted command interface (`help`, `theme`, `e <project>`, `q`). It cannot execute JavaScript or shell commands. The WORM toggle is a simplified visual interaction.

## Source and asset boundaries

The live site links [HANCORE-linux](https://github.com/HANCORE-linux). A bounded search of that public account's repositories did not locate an official website source repository or establish a site reuse license. The profile repository's MIT license is not permission for all website art; its asset notices exclude third-party material. Public delivered modules were inspected for behavior, but none of their scripts, stylesheets, README bodies or filmed portrait assets were vendored.

`app.js`, `scene.js`, `wheel.js`, markup and CSS are independently implemented. The introduction displays **this implementation's own public `app.js`**, not copied upstream code. The bio is paraphrased. `reference-data.json` stores factual public project/contribution data from [content.json](https://hancore.dev/site/content.json) and [github-snapshot.json](https://hancore.dev/github-snapshot.json), whose recorded snapshot timestamp is **2026-10-15T23:36:51.675Z**. These are Hancore snapshot statistics, not Supa statistics or live visitor counts.

Preview images, favicon and wordmark are referenced directly from public `https://hancore.dev` URLs. They require connectivity and may change or disappear. They are not copied into the asset directory; no redistribution rights are claimed. Before public reuse, replace these references with approved assets or obtain appropriate permission.

The independently obtained [IBM Plex Mono](https://github.com/google/fonts/tree/main/ofl/ibmplexmono) font is distributed under the included [SIL Open Font License](public/fonts/OFL.txt).

## Isolation and limitations

The local server allows only GET/HEAD for an explicit static-file allowlist and public assets. It rejects parent paths, dotfiles and symlink escapes. CSP restricts script and fetch connections to this origin and images to this origin plus hancore.dev. No analytics, visitor counter, Supa API, wallet, model API or financial operation is connected. Remote project links open the original public repositories.

This baseline does **not** establish exact visual or behavioral equivalence. The portrait, contribution topology, worm/ghost behavior, project text and theme catalogue differ. Full upstream archive/log/editor functionality is not reconstructed. See [the implementation and validation report](REPORT.md) for matched screenshots, measured differences and remaining work. Supa integration is intentionally deferred.
