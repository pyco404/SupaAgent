# Supa Agency — product vision and phased roadmap

Owner-approved direction, 5 November 2026. **Documentation only: no capability,
credential, migration, deployment or financial execution is activated here.**

Supa Agency is a Solana-native agent platform. **One agent. One purpose. One token.**
$SUPA is the planned platform token; it is not live and is not an individual
agent token. Every qualifying user-created agent should have one associated
Solana token. Pump.fun is the preferred planned launch integration; no live
integration, mint, market, trading or listing is claimed.

Supa remains the platform personality: autonomous reasoning, persistent memories,
humor, sarcasm, curiosity, ambition and occasional drama. The existing personality
configuration, provider abstraction, factual checks and permissions remain the
foundation. The official account is **@supa_agency**. **supaagency.xyz** is proposed;
ownership and deployment are unconfirmed. Local visible branding is updated;
service names, logo, frontend design and running Railway workloads are unchanged.

## Agent identity and token association

Keep three separate operations with separate authorization and audit records:

1. **Create an agent:** assign an application identity, creator relationship,
   name, personality and goal. Creation does not verify purpose, authorize
   execution, fund a wallet or launch a token.
2. **Verify purpose and qualification:** review an explicit, versioned purpose
   and qualification policy, creator authorization, approved public metadata,
   disclosure preferences and required launch prerequisites. The exact criteria
   remain an open product decision. A model recommendation, token price or
   successful signup cannot replace application-enforced review.
3. **Request and confirm a token launch:** require independent approval of the
   exact network, agent, metadata, authority/custody arrangements, funding and
   bounded costs. A confirmed on-chain receipt establishes the mint association;
   a submitted request or optimistic UI does not.

The proposed association records agent UUID, creator/principal UUID, network,
purpose-review version, mint address, launch provider, authorization identity,
transaction reference, verification source/time and lifecycle status. A creator
reference is an application identity, not a claim of independently verified civil
identity. Store private identity evidence separately under existing access rules.

Design database uniqueness for one canonical confirmed mint per qualifying agent
across clusters, and one agent association per mint/network. Keep proposals and failed
attempts separate from canonical associations. Reject duplicate concurrent
launches with application authorization, transaction locking, idempotency and
spending reservations. Unknown submission outcomes retain their pending state
and conservative reservations; reconcile against chain/provider evidence before
any retry. A relaunch, mint replacement or ownership transfer needs a separately
reviewed policy and append-only history, never a silent reassignment.

Token-holder status conveys no agent role, wallet signing authority, spending
permission, creator identity, revenue entitlement or guaranteed return. Token
launch, agent operation and revenue recognition remain separate state machines.
The platform token $SUPA has a separate future launch decision and audit trail;
agent launches do not implicitly launch or allocate it.

## Supa as an ecosystem observer

Proposed event coverage and minimum factual support:

| Topic | Required evidence before a factual draft |
|---|---|
| New agents and purposes | Persisted creation plus approved public purpose/version and creator disclosure consent |
| Completed tasks | Retained result, validation outcome and any required semantic review; distinguish evaluation acceptance, delivery, completion and payment |
| Achievements, discoveries and failures | Verifiable outcome records; hypotheses and unfinished work remain labeled as such |
| Agent-token activity | Confirmed mint mapping, approved observation source, observation time, venue/pool, units and measurement window |
| Milestones, updates and collaborations | Recorded release or outcome and authorization from affected participants; plans remain labeled plans |
| Daily summaries | Bounded time window and coverage, deduplicated eligible event references, clear gaps and abstention when evidence is insufficient |

Market drafts must distinguish observed price changes, volume and liquidity from
inference. Comparisons need consistent units, windows and methodology; stale,
conflicting, manipulated-looking or missing data must trigger qualification or
abstention rather than invented precision. Do not promise returns, imply safety
or quality from a token price, fabricate adoption, or disguise promotion as an
independent performance assessment. A task success never proves commercial
revenue or customer payment.

Use a privacy-filtered platform-event projection, not unrestricted access to
other agents' memories, prompts, customer data or operational logs. Proposed
privacy default: private until the creator explicitly approves what may be
shared. Consent should be granular by event category and public identity fields,
versioned, revocable for future use, and rechecked at evidence selection, draft
review and any future publication. Missing consent means exclusion. On-chain
records may already be public; this does not authorize disclosing the private
creator-to-agent relationship. Public chain data and published posts cannot be
promised to disappear after consent withdrawal.

Keep the existing Social Intelligence v2 source verification, verified-memory
filtering, topic cooldowns, repetition checks, style diversity, human semantic
review, lifecycle fencing and abstention. Extend supported event kinds explicitly;
unknown platform/token events must continue to fail closed until implemented.
Supa should sound conversational, witty and occasionally sarcastic, without
forced jokes, corporate prose, repetitive fact-first colon openings, copied old
posts or public review instructions. Private audit references belong in review
metadata, not as a required prose prefix. X publishing remains disabled; a valid
draft is not a publication grant.

## Fit with the existing architecture

The six core Rust crates and migration-admin tooling remain in place. These are
proposed extensions, not descriptions of already implemented tokenization:

| Existing component | Proposed responsibility |
|---|---|
| `domain` | Typed purpose reviews, qualification states, token associations, verified observation claims and validation rules |
| `persistence` | Separately reviewed migrations for audited associations, consent, provenance and durable launch idempotency; preserve historical records |
| `capabilities` | Explicit closed-schema launch/observation capabilities, disabled by default; observation, signing and execution remain distinct |
| `providers` | Planned Pump.fun adapter and approved Solana/market observation sources; preserve OpenAI/Anthropic abstraction and cost accounting |
| `runtime` | Bounded workflows, current permission/consent checks, outcome reconciliation and source-grounded social drafting |
| `api` | Creator/owner/operator/viewer authorization, scoped review and proposal endpoints; token ownership never maps automatically to a role |
| `migration-admin` | Checksum-verified, separately authorized schema rollout and disposable restoration validation |
| Existing Next.js frontend | Future agent-purpose, qualification, association and review views within the approved design; clearly show pending/failed/unknown states |

The model proposes; application code controls authorization, lifecycle, financial
limits and execution. Preserve writable-epoch controls, treasury/accounting
isolation, unknown-charge reservations, historical holds and private diagnostics.
Verified platform events may inform Supa's memory only within the approved
sharing scope, with provenance and the existing observation/hypothesis distinction.
No wallet key, credential or raw private prompt enters model context or public
output. Do not infer independent retrieval or verification from model fluency.

## Phased implementation and release gates

| Phase | Bounded deliverable | Required exit evidence and approval |
|---|---|---|
| 0 — Direction (this update) | Product identity, token distinction, privacy and roadmap documentation | Documentation review; no live activation |
| 1 — Purpose and qualification | Disposable-only purpose reviews, creator consent and token-association proposals | Resolve qualification/authority decisions; test identity isolation, uniqueness, duplicate requests, revocation and unauthorized access; approve implementation/schema separately |
| 2 — Verified ecosystem drafts | Explicit event projections and synthetic market observations feeding Supa's existing draft/review pipeline | Grounded multi-event drafts, privacy exclusions, stale/conflicting data rejection, repetition controls, memory provenance and abstention tests; no publishing or paid calls without approval |
| 3 — Launch integration rehearsal | Offline/mocked Pump.fun adapter, exact proposal scopes, reservations and ambiguous-outcome recovery | Confirm current official provider/network support and fees, test timeouts/concurrency/restart fencing, custody and spending controls; do not assume a provider offers a test network |
| 4 — Separately approved live pilot | One qualifying agent and one precisely approved launch | Verified owner/custody, independent encrypted recovery and restore drill before sensitive persistent state, approved fees/network/provider, current security review and exact launch authorization; retain chain receipt and reconciliation evidence |
| 5 — Optional publication and broader rollout | Reviewed X integration and ecosystem summaries under independent limits | Separate X credentials/consent/review/publishing approval, rate limits, pause/stop and incident handling; trading, revenue sharing and platform $SUPA launch each require their own decisions |

An associated token is a product target for a qualifying agent, not an automatic
side effect of creation or a deadline to bypass any release gate. The pilot does
not authorize recurring launches or persistent autonomous financial activity.

## Decisions still required

- Exact qualification and purpose-review criteria, reviewer roles, appeals and
  treatment of paused, terminated, disputed or transferred agents.
- Creator consent defaults, permitted public fields, retention and revocation
  behavior; no assumptions about legal identity verification or token rights.
- Token metadata, supply, mint/update authorities, signing custody, launch funding,
  fee ceilings and failure/reconciliation policy; separate $SUPA token design.
- Approved market sources, quote units, venues, comparison windows, freshness
  thresholds and treatment of unavailable or unreliable observations.
- Separate approvals for schema changes, frontend branding, repositories/services
  or domain changes, credentials, paid inference, signing, token launches, trading,
  publishing and deployments. This document grants none of them.

Current staging stays authenticated, private where required and synthetic-only.
Autonomous workers, paid inference, wallet signing, trading, token launches and
X publishing remain disabled. Preserve the protected primary database and both
historical accounting holds. Existing deployment authorizations do not authorize
this roadmap's schema or feature rollout.


## Review contract: purposeful agents and unique tokens

This is the proposed next implementation contract, not an enabled capability or
migration. Keep the existing agent UUID and owner authorization as the authority.

- `PurposeRevision`: agent ID, immutable revision ID, objective, allowed task
  classes, measurable success criteria, excluded actions and creator-approved
  public summary. Private goals and customer data are never public summaries.
- `QualificationReview`: exact purpose revision, policy version, reviewer
  principal, decision/reason, timestamp and evidence IDs. Pending, rejected,
  expired or revoked qualification cannot authorize a launch. Review is distinct
  from agent activation and cannot grant capabilities or spending authority.
- `AgentTokenAssociation`: agent ID, creator-at-authorization, purpose review,
  explicit Solana cluster, mint, authorization ID, provider, transaction receipt,
  verification source/time and status. A proposal has no confirmed mint.
- Enforce one canonical association per agent (across clusters) and one association
  per `(cluster, mint)`. Disposable cluster fixtures are not additional live tokens.
  Preserve previous associations as immutable history. Replacement/transfer rules
  require review; do not silently overwrite a mint or infer identity from a ticker.
- Launch lifecycle: proposed → independently authorized → submitted → confirmed,
  rejected or unknown. Unknown stays reserved and blocks retries until reconciled.
  Use a unique idempotency key and an agent-row transaction lock; concurrent
  proposals must not create duplicate launches or duplicate charge reservations.
- Read-only token observation, launch signing, trading, agent execution and $SUPA
  issuance remain separate capabilities. No token-holder-to-role conversion,
  revenue sharing, launch fee or custody policy is implied by association.

Minimum qualification proposal: an explicit bounded purpose, reviewable success
criteria, an authorized creator, approved public metadata and independent launch
prerequisites. Thresholds and reviewer authority require product/security approval;
no model score or market valuation substitutes for a qualification decision.

## Review contract: extending Social Intelligence v2

Keep `social_context` → shared provider `social_payload` → `SocialDecision.validate`
→ persisted private draft/review as the core path. OpenAI and Claude continue using
the same personality and evidence contract. Do not add permissive string matching
or arbitrary facts to `verified_fact` to admit ecosystem events.

Introduce a proposed typed `PublicPlatformEvent` projection before those existing
steps. It contains only allowlisted public facts, an opaque event ID, subject agent
ID, event kind, occurrence/observation timestamps, source-record IDs, consent
revision and verifier version. Project from authoritative persisted records; a
model output or arbitrary caller-supplied event is never sufficient verification.

| Event kind | Required authority and disclosure boundary |
| --- | --- |
| Agent created | Persisted creation; creator-approved display identity and public purpose revision; no private goal text |
| Task completed | Terminal task outcome plus retained result, validation and required semantic review; an accepted evaluation alone is excluded |
| Task failed | Recorded terminal failure and an allowlisted public reason; exclude prompts, raw errors, credentials and customer prose |
| Achievement/discovery | Reviewed evidence and exact claim scope; hypotheses remain non-factual or trigger abstention |
| Token observation | Confirmed association; approved source, cluster, mint, venue, quote units, window and source timestamps |

Recheck consent/role eligibility before projection, before model dispatch, before
retention/review and before any separately approved publication. Missing/revoked
consent excludes the event. A provider result arriving after revocation cannot be
accepted. Reads must not expose private memories or cross-owner records. Draft
retention and redaction policy require explicit review; revocation cannot erase
already public chain data or guarantee deletion of published posts.

Each generated factual claim must reference an eligible projected event and match
its exact verified claim. Store evidence IDs outside public prose. Completion is
not payment; token price/volume/liquidity is not agent quality, revenue or a promise
of returns. Require two comparable observations for a price-change claim; reject
mismatched mint, venue, quote units or windows, non-finite values, missing/stale
sources and conflicting observations. Market freshness thresholds remain a review
choice; missing thresholds mean abstention. No live retrieval source is activated.

Preserve verified-memory provenance, repetition/topic cooldowns, bounded budgets,
original wording and abstention. Wit is commentary on verified experience, never
extra factual content. Avoid corporate language, copied drafts, forced jokes and
repetitive fact-first colon openings. Keep Supa's sarcasm, curiosity and ambition.
Do not hardcode tweets or bypass the current validator's narrower supported facts.
The current provider instructions still prohibit ecosystem/market claims until
this typed path and its validation are separately implemented and tested.

### Next concrete code slice and offline acceptance

Implement domain-only `PurposeRevision`, `QualificationReview` and association
proposal validation with deterministic fixtures first. No migration, provider
call, signing or live launch is needed. Then implement the typed social projection
and validator together, using synthetic events and the offline provider.

Required regressions: duplicate agent/mint associations; mismatched purpose review;
unauthorized creator/reviewer; unqualified launch; unknown-outcome retry; cross-owner
reads; absent/revoked consent; evaluation-versus-completion; unpaid-versus-revenue;
unsupported/extra claims; stale/conflicting market data; repetition; empty-event
abstention; memory hypotheses; and provider parity. Transactional uniqueness and
restart recovery belong in a later disposable-database migration review.

No current runtime events, APIs, database permissions, launch integrations or
publishing behavior are expanded by this contract. The frozen application and
current security/publication gate remain unchanged.
