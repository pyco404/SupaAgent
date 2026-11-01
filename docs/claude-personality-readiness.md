# Claude verification and Supa personality preservation

Prepared 2026-11-01, Africa/Lagos. No real provider authentication request, model generation, publication, primary-database startup, worker activation, push or deployment occurred.

## Authentication boundary

The key is now present in the private, ignored `.env.claude`, loaded from that file, with valid basic syntax and owner-only permissions. Checks never print key material. Configured workspace is `wrkspc_01FargC8PH6NDA25MHMRgz3F`; model is `claude-haiku-5-5`. Key presence and syntax do not prove authentication, workspace membership, generation access or available credit.

The previously requested separate approval for an authenticated external check remains outstanding. The exact next command, **prepared but not executed**, is:

```bash
python3 scripts/claude-local-credentials.py probe \
  --workspace-id wrkspc_01FargC8PH6NDA25MHMRgz3F \
  --model claude-haiku-5-5 --authorize-metadata-check
```

After explicit approval, this makes one model-metadata GET, with no generation, redirects or retries. Only sanitized results are reported. Matching metadata would establish access to that endpoint with the explicit workspace header; it does not independently verify billing balance, spending limits or every workspace role. No administrative key is requested. The reported credit balance remains unverified.

## Personality architecture and implemented compatibility

`config/supa.json` is the canonical new-agent configuration. Existing agents retain their own persisted `Personality`; the domain default is a fallback and is not byte-identical to the JSON configuration. Nothing rewrites existing agents or silently replaces their voice. Both providers now consume the **same supplied owned personality**, not independently maintained provider personas.

Inspection found OpenAI already implements Social Intelligence v2; Claude previously exposed only controlled reasoning and work. Implemented a default-denied `ControlledProvider::social` proposal interface and a Claude implementation. OpenAI delegates this interface to its existing social generator. This is preparation for separately authorized comparisons: **no API endpoint or recurring Claude Reasoner was enabled**. Callers must still verify provider access, reserve a separate authorized budget, establish owned context, enforce finite consent and lifecycle fencing, and persist/review results. The existing $0.005 work scope does not authorize social generations.

Both transports now use `social_payload` in `crates/providers/src/social.rs`:

- Identical personality, voice instructions and strict structured schema.
- Identical selected verified events and claims, four bounded memory excerpts and eight previous drafts. Existing domain context selection retains twenty drafts for application checks, six relevant verified memories and event/topic cooldown filtering.
- Same dry wit, sarcasm, ambition, curiosity, conversational tone and occasional drama; observations, questions and silence remain valid. Added explicit guidance against repetitive fact-first colon openings. Corporate language, forced jokes, stock templates and recycled metaphors remain discouraged.
- Memories are perspective, not authority; only supplied verified memory IDs may be cited. Events, goals, memories and personality text remain untrusted data, never permission or policy.
- Canonical claims stay in metadata; approved factual fragments appear naturally in text. Simulated economic labels, unknown-event rejection and unsupported-claim checks remain application-owned.
- Shared controlled-result validation rejects invalid claims, foreign memory references, review-instruction leakage, repeated drafts and cooled topics while preserving genuine provider usage for accounting.
- Empty context abstains without transport or usage. Voluntary model abstention retains usage. No automatic rewrite/retry or publishing tool exists.

The existing autonomous OpenAI runtime still receives parsed decisions and records accepted/abstained/rejected outcomes under its original persistence lock. Its ownership, history, lifecycle and permission revalidation remain intact. Early controlled-interface validation does not replace that audit path. Human-review/publishing-disabled metadata remains separate from public text.

Previous GPT drafts and authored examples are never hardcoded production outputs. The comparison fixture is read only by tests/the offline example. Prior GPT text is reference material, not a generation template.

## Offline comparison framework and measured results

Run without credentials, database access or provider calls:

```bash
CARGO_TARGET_DIR=target/claude-validation CARGO_PROFILE_DEV_DEBUG=0 \
  CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo run -p supa-providers --example social_comparison --locked --offline -j2
```

The example uses `crates/domain/tests/fixtures/social-comparison.json` and deterministic synthetic verified-event identities. It outputs source provenance, grounding/schema validity, supplied memory-reference counts, positive verified-memory checks, foreign-memory rejection, repeated-draft rejection, topic cooldown, opening diversity, colon flags, styles and valid empty-context abstention. Optionally pass one reviewed local JSON file of saved decisions to evaluate later actual outputs against these exact synthetic fixture contexts. Format is documented at the top of the example; invalid schemas/indices fail rather than falling back to fixture answers. This mode does not generate outputs or claim provider provenance automatically; retain and independently review original evidence.

Baseline results for **authored fixtures, not model outputs**:

| Check | Result |
| --- | --- |
| Event grounding/schema acceptance | 6/6 |
| Acceptance with preceding history | 6/6 |
| Unique first-four-word openings | 6/6 |
| Style diversity | 6 styles |
| Verified-memory reference acceptance | 6/6 synthetic additions |
| Foreign-memory rejection | 6/6 |
| Repeated draft / topic cooldown checks | 6/6 each |
| Empty-context abstention validity | 6/6 |
| Colon within first five words | 2/6; a diagnostic, not a blanket rejection |
| Original fixture memory references | 0; no invented usage |
| Provider requests / charges | 0 / $0 |

Humor, naturalness and personality-quality scores are deliberately **null**. The report supplies a human 1–5 rubric for voice, rhythm, appropriate wit/sincerity, diversity and semantic grounding. Matching prompts or passing lexical checks cannot establish creative quality. No live Claude-versus-GPT quality comparison or new live drafts exist. The historical GPT example remains labeled reference evidence and excluded from new-output measurements.

## Preserved live work scope and isolated database

The work plan is unchanged, including every task byte, evidence reference, ID, nonce, budget and expiry:

- Scope SHA-256: `b11a1cb723e32742e23988b897504dd524e026390e027a0def5398ae44eb931b`.
- Input SHA-256: `613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`.
- Independent budget `3bae1525-7e83-4220-9682-d30d06cc06f2`: **$0.005 total, $0 committed**; one `claude-haiku-5-5` request, **$0.004 ceiling**, conservative $0.003536 reservation not yet incurred.
- Token bounds remain 24,192 input / 1,024 output. Only `inspect_input`, `extract_evidence`, `check_requirements`; work memory remains disabled.
- Agent `1d892448-ce83-4e38-a2b9-fc166411b15a` paused; job `fd408604-98d9-48e4-a2b0-c4cd92a3a3e0` created, unpaid, simulated, revision zero.
- Expiry **2026-11-02 13:30:58.948063 WAT**. Approximately 23.3 hours remained during verification. Recheck immediately before any execution; never silently extend it.

The exact existing disposable database was started only for read-only verification, then stopped. Identity/system identifier, scoped ownership, zero-spent budget, original job input/expiry, absence of calls/reservations/cycles/actions/schedules/economic entries and all twelve migration checksums passed. No rows were inserted or updated. Primary PostgreSQL stayed stopped and unqueried; all **26 protected primary/backup/accounting hashes** matched the starting baseline. Historical **$0.004048** remains quarantined and unchanged. The old social report's archived-budget commands are historical, not authorization to reuse that budget.

The separately protected paid command remains in [the workspace readiness report](claude-workspace-readiness.md). It has not been run. Authentication approval does not approve that work request, a social comparison request or continuous execution. Social-only prompt changes do not alter the work prompt, work plan or work digest.

## Validation and limitations

- Workspace Rust tests: **89 passed, 0 failed, 89 opt-in tests ignored**.
- Provider tests with local HTTP mocks plus example test: **36 passed**, zero external provider requests. The cross-provider mock test compares exact prompt/context/schema bytes and checks valid draft, voluntary abstention, unsupported claim, foreign memory and review leakage; usage survives rejection.
- Credential tests: **6 passed**. Evaluation scope/budget/replay tests: **9 passed**.
- Offline example regression passes, including verified/foreign memory, repetition, cooldown and abstention metrics.
- Formatting, diff whitespace checks and provider Clippy with warnings denied passed. No frontend files or browser interactions changed; prior approved frontend validation remains applicable.
- Secrets and full-history audits are run before/following individual commits. Local raw evidence stays ignored under `artifacts/claude-personality/`; keys, private logs, databases and generated artifacts are not committed.

Natural-language semantic grounding and humor still require human review; lexical overlap does not detect every paraphrase. Actual provider access/billing and live personality quality remain unverified. No paid requests, historical financial changes, primary activation, recurring worker, signing, transfers, trading, launches, X publishing or Railway deployment were enabled. No push is authorized.
