# Supa's personality and social intelligence

Supa is intelligent, curious and ambitious about independence. Its voice is conversational, with dry wit, gentle sarcasm and occasional drama about expenses. Honest observations, sincere questions and silence are as valid as jokes. Personality is configuration and never grants tool permissions, changes financial policy or authorizes publication.

This update preserves the six-crate architecture, existing OpenAI adapter, token/cost guard and legacy draft interface. No OpenAI call or account probe was made to assess the new implementation. New creative examples below are authored deterministic fixtures, not claimed GPT results. The earlier live responses remain unchanged.

## Social decision flow

The autonomous planner now sees `consider_tweet`, backed by the existing `DraftTweet` capability. It may choose no social action. The legacy `draft_tweet` tool remains supported for existing callers and old evaluator behavior, but is omitted from the live planner's suggested tool set. `consider_tweet` chooses verified experiences internally and supports `draft` or `abstain` decisions; no publishing tool exists.

1. Load owned persisted events and recent legacy/new draft history. Rank known events: failed/interrupted cycles first, then simulated revenue, execution/goal assessments, assigned goals/treasury and recurring costs. Ignore unknown event kinds and financial entries lacking the simulation flag. Candidate selection takes at most one event per topic and at most eight candidates, avoiding a flood of expense entries.
2. Exclude event IDs used by the last twenty drafts and topics covered by the last three drafts. Rejected outputs and abstentions do not count as public draft history. History is per agent and survives restarts.
3. Retrieve relevant verified memories with owned event sources, including older source events outside the latest event window. Retrieval is bounded: twelve relevance-ranked memories plus four oldest anchors provide up to sixty-four additional event IDs; at most six relevant verified memories enter the social context. Hypotheses remain available to the existing task planner but are excluded as social factual evidence.
4. If no candidate remains, persist `abstain/nothing_new` without invoking a provider or creating an inference reservation. Ordinary simulated cycle operating expenses still apply.
5. Otherwise, make one routine-provider decision under the existing durable inference guard. The model selects up to two candidate events, a style, up to three supplied memory IDs, and either writes a post or abstains. Prompt context includes eight previous draft texts and four bounded memory excerpts; application validation uses the full bounded history. Voluntary abstention still accounts for its inference request. No automatic rewrite is performed after validation rejection.
6. Lock the agent/cycle and revalidate the original candidate context, current source ownership, current draft history, permissions and lifecycle before persistence. A pause or emergency stop fences the pending draft. Rejected text is discarded; only the failure code is retained. Accepted and abstained decisions are recorded durably.

The six styles are `humor`, `observation`, `achievement`, `failure`, `question` and `reflection`. Achievement currently means only an evidenced completed reasoning cycle, not a commercial milestone. Failure style requires an actual recorded failed/interrupted cycle. The model may select `nothing_new`, `too_repetitive` or `conserve` rather than a post. Choosing not to post is a successful decision, not a provider failure.

## Evidence without audit-log prose

Legacy validation still requires canonical sentences such as “A simulated inference expense was recorded.” inside legacy tweet text. Its historical behavior is unchanged.

The new path keeps the canonical proposition in `claims.fact`, checks it against the owned persisted event, and verifies `claims.wording` against an application-controlled factual fragment. Examples are “simulated inference costs,” “a completed reasoning cycle” and “an operator-assigned goal.” The selected fragment must appear naturally in public text, case-insensitively. These are factual vocabulary constraints, not tweet or joke templates. No production joke bank or fallback tweet text exists.

Every selected event requires a matching canonical claim and approved public fragment. Economic fragments explicitly say simulated. Numeric embellishments, unsupported source IDs, unknown styles, missing simulation labels and known unsupported claims about payments, trading, transfers, publication, deployment, wallets or profitability are rejected. Style and memory references do not establish a new fact. The model is instructed to keep all other text subjective and never invent capabilities or achievements.

Review fields are application-owned columns: `review_status=requires_human_review` and `publishing_enabled=false`. They are separate from `data.text`. Public text containing review instructions or legacy audit openings is rejected. Claims, source IDs, style and disposition remain audit metadata rather than a prefix/suffix appended to the tweet.

Application checks cannot prove all natural-language semantics. As with the existing factual verifier, an otherwise valid sentence can still contain an unsupported assertion outside its declared claim. Human semantic review remains necessary; no output is published automatically. The validator is deliberately conservative and can reject harmless wording. This update preserves exact canonical event verification while changing only how its supported proposition is presented publicly.

## Repetition controls

The application compares twenty recent accepted drafts from both paths. It rejects equal normalized text, the same first four normalized words, strong word overlap in the remaining joke after removing factual fragments, reused event IDs, and topics in the three-draft cooldown. Normalization handles case and punctuation. Overlap uses 65% Jaccard similarity or 85% containment for at least five joke words. This detects lexical reuse, not arbitrary semantic paraphrases; the model also receives history and instructions to avoid recycled metaphors. Repetition failures persist as `social_repetition`; they do not trigger a paid retry.

## Old versus new

The old text below is an actual earlier GPT output. The new texts are **authored, validated test fixtures**, showing what the new contract accepts; they are not evidence of new live-model creative quality. Production never selects these fixture strings.

| Case | Before / accepted fixture |
|---|---|
| Earlier live expense draft | A simulated inference expense was recorded. My imagination has an operating-cost subplot; thrilling. Draft for human review. |
| New humor fixture | Ambition has expensive taste: simulated inference costs. I would like to speak to ambition's manager. |
| New observation fixture | Apparently simulated operating costs do not accept enthusiasm as a discount. |
| New achievement fixture | A completed reasoning cycle. I am allowing myself a very modest victory lap. |
| New failure fixture | After a failed reasoning cycle, confidence will be attending the meeting as an observer. |
| New question fixture | An operator-assigned goal: how ambitious is too ambitious when the next step still needs evidence? |
| New reflection fixture | A goal progress review feels healthier than congratulating myself for having a to-do list. |

The fixtures are in `crates/domain/tests/fixtures/social-comparison.json`, exercised by a test that validates all six styles and their source claims together with preceding draft history. Additional tests deliberately supply repeated openings/jokes/topics, foreign events, malformed simulation claims and unverified memories.

## Persistence, API and migration

Migration `0003_social.sql` creates an append-only `social_decisions` table and index. It does not rewrite agents, financial records or existing drafts. Each row records its agent, cycle, authorized action, decision/disposition, separate review metadata and any safe failure code. There is no published state. Accepted text and metadata live in this table; `/drafts` continues to expose legacy drafts. The new authenticated local `GET /agents/{id}/social` endpoint exposes new drafts, abstentions and rejected decision codes. The regular agent memory and activity endpoints remain unchanged.

The migration was tested in the separate `supa_agent_test` database. The existing `supa_eval` database was inspected read-only and deliberately left at its prior migrations; normal startup or a future authorized evaluator run applies the additive migration. Existing agents retain their persisted personality configuration; new agents use the updated default or `config/supa.json`, and the new social prompt supplies the current behavioral guidance. No frontend was replaced or redesigned.

## Optional live creative evaluation — not executed

The existing `live_eval` example accepts `--social`. Social setup initializes an explicitly simulated working treasury but does not inject hypothetical revenue to create variety. Only legitimately recorded setup, goal, cost and execution events are available; failures and commercial achievements are never manufactured. It runs three live planning cycles followed by five social opportunities rather than forcing five posts. Planning may also select a permitted social action. All requests, including abstentions and validation failures, use the same budget and reservation rules. It saves social decisions, style, source claims, usage and abstentions in a private attempt report, then suspends the evaluator agent. Automatic retries remain disabled in the evaluator.

Use non-paid preflight first, from the repository root:

```bash
env -u OPENAI_API_KEY -u SUPA_EVAL_DATABASE_URL -u SUPA_EVAL_OUTPUT \
  SUPA_EVAL_ENV_FILE=/home/pikoo/Desktop/supa-agent/.env.eval \
  SUPA_ALLOW_OPENAI=true \
  SUPA_LIVE_BUDGET_ID=ad91e475-f1da-495a-8247-139643d2d8b1 \
  SUPA_LIVE_BUDGET_MICROUSD=100000 \
  OPENAI_MODEL=gpt-6-luna SUPA_PLANNING_MODE=routine SUPA_MAX_OUTPUT_TOKENS=2048 \
  cargo run -p supa-runtime --example live_eval --locked -- --social --check
```

Only after separate explicit authorization, use the identical configuration and replace the final `--social --check` with `--social --run --authorize-paid`. Do not change the budget UUID or increase the ceiling. At this task's read-only baseline, the budget held 7,948 micro USD committed out of 100,000, including the preserved historical reservation. A future run must recheck current spending; no unused allowance is newly granted by this document.

An entirely abstaining evaluation can finish its protocol successfully, but `live_evaluation_success` stays false without an actual generated social draft. The report's creative-quality field always requires human review. Assess whether actual outputs are natural, distinct, appropriately funny or serious, and supported by their events; do not confuse schema validity or deterministic fixtures with a successful live creative evaluation. No live outputs from this new path exist yet.

## Validation results

Passed Cargo formatting, Clippy with warnings denied, **41 unit/example tests**, and **32 integration tests** (26 PostgreSQL/runtime, five local mock-provider, one HTTP workflow). New checks cover varied candidates, six styles, approved factual fragments, canonical claim matching, simulation labels, repeated jokes/openings/topics, memory provenance, foreign evidence, abstention, legacy-history retrieval after reconnect, insufficient treasury, paused in-flight work and the authenticated social-history endpoint. The stricter public-text checks also reject explicit unsupported capability and real-income claims. All model fixtures use local mock servers or deterministic providers.

Commands: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`, `cargo test --workspace --all-targets --locked --offline`, and the same test command with `-- --ignored` and the separate test database URL. A non-paid `live_eval --social --check` succeeded in selecting social mode while leaving paid readiness false. The new Responses schema follows the [official strict structured-output contract](https://developers.openai.com/api/docs/guides/structured-outputs); provider wire tests exercise it without real API calls.

Read-only before/after checks on `supa_eval` showed the original budget UUID unchanged, ceiling 100,000 micro USD, committed 7,948 micro USD, and 17 inference rows. No new API request, real financial operation, publishing or deployment occurred. See [machine-readable validation summary](../artifacts/social/validation.json). Git metadata is still unusable and was preserved; commits could not be created.

## Social v2 live preflight follow-up

A fresh non-paid preflight passed formatting, Clippy, 42 unit/example tests (including a regression preventing social setup revenue injection), and 32 integration tests. The scoped configuration is ready; explicit paid authorization remains absent. PostgreSQL and existing migration checksums were verified read-only. The original budget remains at 7,948 of 100,000 micro USD committed, leaving 92,052 micro USD. No new API request or financial record was created. Migration 3 remains pending in supa_eval and is applied by normal evaluator startup before any model request.

The live protocol creates a fresh agent, so live memory evidence will come from its own completed planning cycles; older/cross-agent memories are not imported. Event/style coverage and voluntary live abstention must be reported as observed, not guaranteed. Empty-candidate abstention is already covered offline and intentionally avoids a model call. See [the current preflight record](../artifacts/social/v2-live-preflight.json). No v2 live drafts, tokens or creative-quality assessment exist yet.
