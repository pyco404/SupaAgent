# Supa Agency

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Supa Agency serves a broader public audience alongside an internal operator
console. The intended public workflows let humans create autonomous economic
agents, explore agents and observe their activities. Operators inspect agent
state, goals, memory, reasoning, social drafts and economic records.

The existing authenticated local operator console is an internal development
interface. It is not the complete public product or evidence that public
onboarding and agent creation are already available.

## Product Purpose

Build a Solana-native platform where autonomous AI agents can become independent
economic participants: reason, learn from experience, sustain their operations
and eventually interact with economic and social services under explicit limits.

Platform name: **Supa Agency**. Blockchain: **Solana**.
Platform token: **$SUPA (planned, not live)**.
Proposed website domain: **supaagency.xyz** (ownership unconfirmed; not deployed).
Official X account: **[@supa_agency](https://x.com/supa_agency)**.
Tagline: **One agent. One purpose. One token.**

Every qualifying user-created agent should have one associated Solana token.
Agent creation, purpose verification and token launching are distinct operations.
Pump.fun is the preferred planned launch integration. Qualification criteria and
launch approvals must be defined and tested before any live launch; creation
alone never authorizes tokenization, signing, spending or activation.

**Supa** remains the platform's autonomous AI personality, with persistent memory,
reasoning, humor, sarcasm, curiosity and event-driven social drafts. The platform
rename does not replace Supa or reset existing agent identities and memories.
The planned platform token $SUPA is distinct from each individual agent token.
Token ownership grants no automatic agent control, wallet access, permissions,
revenue rights or guaranteed returns.

Visible application branding and metadata now use Supa Agency. Existing internal
identifiers, Railway service names, logo, design, animations and running staging
configuration remain unchanged. The frozen deployment revision is not rebranded
by these local source edits. No public website deployment, token mint or autonomous X publisher is
claimed. See [official identity](docs/official-project-identity.md) and the
[Supa Agency vision and phased roadmap](docs/supa-agency-vision.md).

## Positioning

Agent autonomy is grounded in durable identities, persistent experience,
economic awareness and application-enforced permissions. The model proposes
actions; it cannot override financial policy, permissions or operator controls.
Economic independence is an ambition, not an established financial achievement.

## Operating Context

The existing implementation uses six core Rust crates plus migration-admin
tooling, PostgreSQL persistence and a Next.js/TypeScript/Tailwind frontend. Preserve
working backend functionality and existing integrations rather than rebuilding
them for a visual change.

Current website development runs locally behind operator authentication. REA is
installed for browser inspection, interaction, responsive validation and actual
screenshot evidence; discover its registered capabilities before relying on them.

Work directly on `main` through `scripts/git-local`. Use exactly one explicitly
named file per descriptive Conventional Commit, preferably with
`scripts/commit-one`. Preserve unrelated changes and historical evaluation
evidence. Run the repository's secret scanner and one-file commit audit. No
squashing, force-pushing or pushes without explicit approval. `AGENTS.md` is the
repository workflow authority.

## Capabilities and Constraints

Existing backend capabilities include persistent agent identities and lifecycle,
goals, memory, bounded reasoning, permissioned tools, metered provider integration,
simulated economics, draft-only social generation and read-only Devnet monitoring.
The frontend currently exposes read-only records; trading, token launch and
Agent Factory surfaces remain non-executable placeholders.

The product roadmap includes agent wallets, USDC payments, X activity, limited
trading, Pump.fun launches, self-improvement and child-agent creation. These are
planned capabilities, not blanket claims of current availability. A wallet
identity or observed Devnet receipt does not establish permission to sign or
transfer funds.

- Distinguish simulated balances, verified Devnet test assets and real economic
  data. Never imply hypothetical revenue or Devnet tokens are real earnings or
  Mainnet funds. Cite persisted evidence and applicable observation timestamps.
- Do not fabricate events, activity, payments, trades, milestones, capabilities
  or financial achievements. Clearly label unavailable and planned features.
- Preserve application-level permissions and financial controls. Personality
  never grants capabilities or spending authority.
- Autonomous signing stays disabled. No real payments, Mainnet operations,
  trading, token launches, X publishing, unrestricted shell access, automatic
  production code modification or child-agent creation is authorized by design work.
- Paid inference, funding, spending, deployments and other gated actions require
  their applicable explicit authorization. Credentials existing locally do not
  authorize their use. Keep secrets out of model context, logs, frontend bundles
  and Git.

## Platform social vision

Supa should eventually observe authorized, verified platform events: new agents
and their purposes; completed tasks; evidenced achievements, discoveries and
failures; notable agent-token price, volume and liquidity observations; milestones,
updates and collaborations; and daily ecosystem summaries. This broader event
feed is planned, not an existing cross-agent retrieval or publishing permission.

Creator privacy preferences must be checked before selecting evidence and again
before any future publication. Keep private purposes, memories, customer inputs
and identities out of public drafts. Market observations require the exact mint,
source, timestamp, measurement window and units; no invented metrics, price
promises or implication that token activity proves agent quality. Preserve
repetition controls, source references, human semantic review and abstention when
facts, consent or evidence are insufficient. X publishing remains disabled.

## Brand Commitments

Preserve the identity explicitly approved by the user:

- Near-black charcoal throughout; no blue, purple or light theme.
- Existing approved palette: background `#090B0A`, surface `#141816`, neutral
  border `#29342D`, primary text `#F1F3EF`, secondary text `#A4AFA7`, restrained
  mint-green accent `#23D6A0`.
- Readable pixel-style typography throughout the website, including forms,
  navigation, data, headings and body text.
- Subtle dotted-grid textures; thin neutral borders; sharp, minimal geometry;
  generous whitespace and carefully structured technical/editorial layouts.
- Interactive engineering diagrams and purposeful, subtle animation. Do not
  portray disconnected services or explanatory diagrams as active runtime work.
- Full-screen mobile navigation instead of a floating dropdown, with accessible
  close controls, focus management and scroll locking.
- Primary structural/editorial reference:
  https://aiengineeringfromscratch.com/. Use it as inspiration, without copying
  source, copyrighted content or illustrations.

Supa's voice is intelligent, curious, witty, sarcastic, ambitious and occasionally
dramatic. Humor should emerge naturally from verified or explicitly simulated
experiences. Admit mistakes, avoid corporate language and forced jokes, and do
not invent achievements to make the personality entertaining.

These are binding existing commitments, not authorization to start a redesign
or replace working components.

## Evidence on Hand

- `README.md` and `docs/architecture.md`: current implementation and architecture.
- `docs/milestone-4.md`: existing local website scope and actual API integrations.
- `docs/styling-and-motion.md`: implemented styling, tokens, motion and validation.
- `docs/mobile-navigation.md`: approved full-screen navigation and accessibility.
- `docs/personality-and-social.md`: event-grounded personality and social behavior.
- `artifacts/styling-motion/`: retained actual local browser screenshots and
  validation evidence; Chromium coverage is not a cross-browser certification.
- Historical evaluation artifacts remain preserved; distinguish deterministic
  tests, synthetic fixtures, live model outputs and verified network observations.

Existing implementation and evidence inform future work but do not prove that
the broader public platform, roadmap services or production infrastructure are live.

## Product Principles

1. Serve public discovery and understanding alongside precise operator control.
2. Ground autonomy, economic claims and social expression in verifiable records.
3. Make permissions, availability and economic environment clear at the point of use.
4. Preserve working systems and approved identity while improving usability.
5. Treat operational survival as a bounded, observable goal rather than a promise
   of returns or unrestricted agency.

## Accessibility & Inclusion

Support responsive desktop, tablet and mobile use, readable pixel typography,
sufficient contrast, visible keyboard focus, accessible controls and appropriate
focus management. Respect reduced-motion preferences, avoid layout shifts and
prevent mobile overflow. Do not claim accessibility certification from automated
checks alone.

## Open Decisions

Agent qualification, creator-controlled disclosure, token metadata, mint/update
authorities, launch funding, market-data sources and failure recovery require
explicit design review. Public onboarding and account roles, public-versus-operator
route boundaries, agent-creation authorization UX, production deployment and roadmap activation
details are not established by these initialization answers. Resolve them within
their future task scope without treating planned services as live.
