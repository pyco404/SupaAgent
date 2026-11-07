# Cinematic terminal artwork — local review

This is a visual enhancement of the existing Supa Agency interface, not a new
navigation architecture. The wheel's dimensions, orbit, tilt, selection and
branch attachments are unchanged. The logo, intro, terminal, keyboard/hand
interaction, footer, deep links and security boundaries remain in place.

The original artwork collection uses graphite, aged ivory ceramic, muted olive
metal and near-black space. Images are editorial concepts, never evidence of
running agents, earnings, trading, memories or launches. No Hancore assets or
source were copied. The reference is composition inspiration only:
https://hancore.dev. The current source and the user's explicit preservation
requirements govern layout and interaction.

## Placement and inventory

| Section | Original visual | Larger composition |
| --- | --- | --- |
| Home | Interconnected sculptural AI cores | A shared orbit |
| Agents | Curious ivory/olive robotic portrait | Supa Agent |
| Intelligence | Folded neural reasoning lattice | Paths of thought |
| Memory | Layered ceramic memory archive | What stays with us |
| Economy | Bounded loops and resource conduits | Resources, with boundaries |
| Social | Sculptural signal horns and filaments | A signal worth sending |
| Trading | Abstract observation instrument | Observe before acting |
| Factory | Precision manipulators and an agent core | A purpose takes shape |
| About | Exploded connected architectural layers | Built with intent |

Each section has a square 320px WebP thumbnail, 480px and 960px WebP content
variants, and a 960px AVIF variant. The 36 assets total 1,014,290 bytes; the nine
thumbnails total 83,200 bytes. Exact paths, sizes, source hashes and output
SHA-256 values are in `cinematic-artwork-inventory.json`. Original PNG masters
are retained privately under `artifacts/cinematic-review/source`; tool originals
also remain in the built-in generator's output directory. No stock images,
external image CDN, trackers or new runtime dependencies were introduced.

The existing diagram slot carries the larger image only in public editorial
views. Home/About retain their original branching canvas underneath. Terminal
text stays in its existing scroll region; operator records do not gain a new
decorative panel. The mobile wheel remains a full navigation drawer. Responsive
image sizing, asynchronous decoding and lazy thumbnail loading limit transfer;
section imagery loads only as its section is selected. The image slot reserves
its dimensions before decode to avoid layout shifts. AVIF has a WebP fallback.

The public terminal body is 14px/1.7 on desktop and 13px/1.7 on mobile, with
20px/18px headings. Palette and monospace font are preserved. The only new motion
is a short already-visible image reveal; reduced-motion removes it. Existing
animation controls and reduced-motion behavior are retained.

## Supa Agent and creator avatars

`supa-agents-*` is an editorial flagship portrait concept. It is not a verified
live listing, a creator's uploaded image, a token image or a new canonical logo.
It can be reused as a future explicitly approved flagship profile/activity
portrait after identity/consent review. This patch does not assign it to a
persisted agent or override creator avatars. The existing optional upload,
deterministic-default and disabled AI-generation flow remains intact, as do
immutable token-image snapshots and original-mint history. Public profile images
continue using the separately consent-gated backend projection.

## Production and exact prompts

Generated with the built-in `image_gen` tool: one separate generation per scene.
The tool was available; no API-key fallback or paid application provider was
enabled. Offline Sharp 0.35.5 optimization strips metadata and emits the retained
variants. `scripts/prepare-cinematic-artwork.mjs` regenerates derivatives from
the retained masters; it performs no image generation or network requests.

Home prompt:

> Use case: stylized-concept. Create one original square cinematic illustration for Supa Agency's Home section. Subject: a sparse ecosystem of distinct artificial-intelligence cores connected by delicate organic ivory signal filaments, suspended in a near-black archive space. One central sculpted computational core with subtle branching satellites; tangible ceramic, graphite and aged brass surfaces. Art direction: exquisite macro photographic 3D sculpture, restrained retro science-fiction, muted olive and warm ivory highlights, extremely dark deep shadows, fine grain, a little moss-green light, no blue or purple, no saturated neon. Composition: centered subject with generous near-black margins; clearly readable as a small square thumbnail and richly detailed at 768px. Background #080908. No letters, numbers, logos, charts, coins, token symbols, UI, watermarks, screenshots or claims of activity. This is original concept art for the platform, not a real network visualization.

Shared prefix for each of the remaining eight prompts, followed by its subject:

> Use case: stylized-concept. Make one original square cinematic artwork for Supa Agency, matching a coherent nine-part editorial art collection. Detailed photographic 3D sculpture in deep near-black #080908 space, graphite, aged ivory ceramic, muted olive metallic surfaces, subtle moss-green light, deep shadows and fine analog grain. Restrained retro science fiction. Centered iconic subject, generous dark margins; strong silhouette for a tiny square card and beautiful material detail for a larger content image. No blue, purple, saturated neon, lettering, logos, watermarks, actual charts, numbers, coins, screenshots or UI. This is illustrative concept artwork, never a depiction of actual platform activity.

- **Agents:** Subject: Supa Agent's original flagship robot portrait. A unique nonhuman curious observant robot with a tapered ivory ceramic faceplate, two small horizontal green optical slits, dark exposed fine mechanical neck, asymmetric olive temple plates and a delicate braided sensor filament behind the head. Three-quarter bust, face turned slightly toward camera, poised thoughtful personality. Avoid human faces, skulls, helmets copied from franchises, armor or weapons. Same silhouette must be usable as a profile avatar.
- **Intelligence:** Subject: a suspended folded neural reasoning sculpture, intricate ivory branching pathways threading through thin translucent olive planes and a graphite central decision core. Delicate topology, not a human brain or a generic glowing sphere. Cinematic macro depth.
- **Memory:** Subject: an elegant vertical memory archive, layered ivory ceramic data wafers arranged like archival leaves inside an open graphite monolith, fine olive fiber connections and one softly lit exposed memory wafer. Tangible futuristic storage, not books with text.
- **Economy:** Subject: a restrained sculptural economic network of interlocking dark mechanical loops and ivory conduits, three grounded nodes linked by slim olive routing filaments. Suggest carefully governed resource circulation. No coins, currency, arrows showing profit, balances, charts or real transactions.
- **Social:** Subject: a constellation of delicate ceramic signal horns and thin branching communication filaments meeting across a dark void, asymmetrical organic radio-like forms, small ivory highlights and subdued olive transmitters. Abstract communication with no speech text or social logos.
- **Trading:** Subject: a precise dark market-observation instrument, graphite fins of differing heights behind a fine etched ivory measurement lattice and olive optical lens, strongly architectural abstract data sculpture. No literal charts, numerical values, profits, coins or activity claims.
- **Factory:** Subject: an intimate robotic assembly atelier, two elegant graphite precision manipulators assembling a small unlit ivory artificial-agent core above a dark plinth; fine cables and muted olive rim light. Cinematic tangible craft, no factory personnel, no mass production, no text.
- **About:** Subject: a refined exploded architecture sculpture, layered graphite frames and ivory structural plates connected by thin organic olive fibers, one central empty aperture, precise and coherent system anatomy. Suggest identity and thoughtful engineering, not a real system status diagram.

## Review and release boundaries

Desktop/tablet/mobile rendered previews and the contact sheet are retained under
`artifacts/cinematic-review`. These local screenshots are the representative
mockups for visual approval, not a claim that the live website has changed.
See the browser and asset test reports there for measured results and limitations.
Local browser timing is not a WAN, Safari or production performance certification.

Final local validation: 33 visual/navigation/atmosphere tests passed across
desktop, tablet and mobile; six real Next.js artwork/Factory/My Agents boundary
tests passed across desktop and mobile. The asset/provenance suite passed three
tests, the existing session suite passed six, and TypeScript and ESLint passed.
Measured text contrast was at least 8.03:1 at all three sizes, with no missing
image alt attributes, horizontal overflow or private requests in public browsing.
All artwork checksums and the 1.25 MB complete-collection budget passed.

The broader legacy run is not fully green. Its outdated tenth-card expectation,
old token-launch text/route expectation, ambiguous agent button selector and
typing-timing assumptions reproduce on the unchanged committed baseline
`66a55dc9f5d41acb7af97a0b0db5ef8ab73f1aef`. They were not changed to make this
visual patch pass. Full logs are retained as `browser-regressions.log`,
`unchanged-baseline.log` and `unchanged-carousel-baseline.log` under the review
artifacts. The new tests' focus-target assumption and the loopback fixture's
canceled-image/teardown race were corrected; final focused runs pass. The fixture
only excuses an explicitly aborted obsolete decorative-image response, never an
active image or private request, and blocks any unmapped external traffic.

Initial browser profiles exhausted local disk space; memory-backed temporary
profiles resolved those crashes. A redundant disposable-database validator also
could not initialize a fresh cluster under that constraint; it cleaned up its
own stopped fixture. No fresh database-validation pass is claimed for this visual
change. No backend/database code was modified. Local pattern/known-secret checks
are not fresh container vulnerability or production security certification.

The only public authorization extension admits the exact nine artwork stems and
their named WebP/AVIF variants on the approved public host for GET/HEAD. Uploads,
private API routes, management routes, arbitrary files, other hosts and mutations
remain excluded. Public browsing must make no private backend requests or writes.
No environment flags, credentials, database objects or running services change.

Release requires review of these local visuals, separate source synchronization
approval, a reviewed frontend recipe including all 36 assets, one separately
authorized build/browser/security scan, exact-archive private publication and a
separately authorized frontend-only deployment. Existing execution, paid
inference, financial, token-launch and X-publishing controls remain disabled.
