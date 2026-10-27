# Milestone 3 — Solana-native observation and accounting

Supa can create an encrypted Devnet wallet identity, recover its public address, observe balances, verify finalized classic SPL-USDC receipts, monitor account history, and reserve operating allocations under independent policies. This is local development infrastructure. All demonstration blockchain evidence is mocked. Devnet tokens have no monetary value. No funds were spent, no transaction signed or broadcast, and no live OpenAI request was made during this milestone.

## Architecture and changed files

The workspace still contains exactly six crates:

| Crate | Milestone 3 responsibility |
|---|---|
| `domain` | `src/wallet.rs`: networks/mints, canonical base58 addresses/signatures, exact decimal conversion, payment proof validation, financial policy, safe treasury context |
| `providers` | `src/vault.rs`: encrypted Ed25519 PKCS#8 envelopes; `src/solana.rs`: bounded read-only HTTP RPC, balances, payment verification, simulation for operator review |
| `persistence` | `src/wallet.rs`: wallet ownership, observations, receipts, checks, cursors, reservations and audit transactions; `intelligence.rs`: emergency-stop wallet lock |
| `runtime` | `src/wallet.rs`: operator-only creation/recovery/verification/sync; `src/lib.rs`: optional treasury information in reasoning context |
| `capabilities` | Existing closed LLM registry retained; no wallet, signing, transfer or policy-editing tool added |
| `api` | `src/wallet.rs`: authenticated routes; `main.rs`: explicit configuration and optional read-only polling |

Other changes: migration `0004_wallet.sql`, Cargo manifests/lockfile, `.gitignore`, `.env.example`, README and architecture/API/future-integration documentation. Tests live in domain/provider wallet modules, `runtime/tests/wallet.rs`, and `api/tests/http.rs`.

Migration 0004 adds `wallet_identities`, `wallet_balances`, `wallet_token_accounts`, `verified_payments`, `payment_checks`, `wallet_cursors`, `operating_reservations`, and `financial_audit`. It does not alter previous balances, transactions, evaluation budgets or inference records. It was applied only to `supa_agent_test` during validation; the existing evaluator database remains at migration 3. Existing migration checksums are preserved. Server startup applies pending migrations to its configured database; review that database URL before starting it.

## Identity and custody

Each agent has at most one wallet, with a unique address, wallet UUID, human operator UUID, network and provenance. `IdentityOrigin::Agent { parent_agent_id }` reserves the future domain format, but both application registration and the database reject agent-created identities. Mainnet identity/mint constants can be identified; all operational wallet services require Devnet.

The vault generates Ed25519 keys using the OS-backed `ring` random source. PKCS#8 bytes are encrypted with ChaCha20-Poly1305 and a random 96-bit nonce. Authenticated associated data binds wallet UUID, network and public address. The vault directory must be private (0700); encrypted files and the separately configured master file must have no group/other permissions (normally 0600). Creation uses exclusive file creation and fsync. Recovery decrypts internally and checks the derived address against both the envelope and persisted identity; HTTP returns the address only. Secret buffers under our direct control use `zeroize`; complete elimination of library/stack copies is not guaranteed.

The LLM receives neither the vault, master path, private key, seed, RPC URL nor HTTP credentials. No API serializes key envelopes. There is no secret export, signing method or broadcasting method on the custody interface. Signing attempts are audited and denied even for the correct operator.

Back up the database, encrypted vault directory and master key independently with suitable access controls. Recovery requires the original master; keys cannot be regenerated from agent UUIDs. Loss of either vault or master means loss of custody. A crash after creating an envelope but before committing its identity can leave an orphan encrypted file. Never automatically delete it or generate a replacement for an already registered wallet; reconcile against a trusted backup manually.

This local process and its OS account are trusted. File mode/symlink checks are not a defense against root or another process with the same OS privileges; this is not HSM/KMS isolation or a production custody service. Backup restore, master rotation, file race hardening and independent security review remain production prerequisites.

## Read-only chain adapter and verified receipts

The RPC interface allows only genesis, balance, token accounts, account signatures, signature status, transaction reads and simulation. It has no send-transaction, swap or airdrop method. HTTPS is required except for local loopback HTTP tests. Redirects are disabled. Each HTTP attempt has a five-second timeout, a two-megabyte body limit and at most one delayed read retry. Remote error bodies and RPC URLs are never propagated into application errors. Paid OpenAI retry behavior is unchanged.

Each verification pins the Devnet genesis hash, fetches signature status with transaction-history search, and fetches the transaction independently using `jsonParsed`, finalized commitment and maximum transaction version 0. It requires finalized success, matching signature/slot, successful transaction metadata, the canonical network USDC mint, classic SPL-token program, six decimals and wallet ownership. The exact positive wallet-owned token balance delta must equal parsed incoming transfers minus outgoing transfers, including inner instructions. Incomplete metadata, owner changes, unsupported programs, mint-only balance changes, internal transfers, nonpositive receipts and mismatched deltas fail closed. “Confirmed” alone is deliberately insufficient; finalization is required.

Only that proof can enter the receipt store. A unique `(wallet, signature, mint)` constraint and row locking credit it once. Identical processing returns `inserted: false` and records a duplicate audit event. Conflicting evidence for an existing signature is rejected without altering the original receipt. Failed/pending verification is durable but adds no revenue. Mock and configured-RPC evidence cannot be mixed in a wallet's records.

The RPC endpoint is a trust root, not a cryptographic light client or a multi-provider quorum. A malicious RPC operator could fabricate a consistent response. “Independent verification” here means fetching and checking finalized chain data independently of a caller's claim; it does not mean independent consensus validation. Token-2022, arbitrary token accounting, other transaction versions and unsupported parsed instructions are not financial-credit paths.

## Monitoring and recovery

`sync` first reads balances and discovers USDC token accounts, scans the wallet plus retained token-account addresses, verifies candidate signatures, persists scan progress, and refreshes balances again. Closed known accounts remain monitored. Each address is limited to two pages of 50 signatures per sync; a saved `before` cursor continues the backlog on the next sync. The completed watermark advances only after the scan reaches the old watermark/end. If a page fails or contains an unconfirmed candidate, that page is not acknowledged. Already committed receipts remain protected against replay after a crash. A sync has a 60-second deadline, at most 101 known addresses, and a shared in-process gate. A bounded backlog reports `scan_incomplete: true`.

Optional polling is explicitly configured, with a minimum 30-second interval and skipped missed ticks. It monitors at most the first 100 listed agents owned by the configured operator. Observation may continue for paused/suspended agents; it never reactivates them or spends their funds. Manual sync remains available. Multi-process cursor coordination, archival RPC retention guarantees, discovery of token accounts created and closed entirely between observations, more than 100 token accounts/agents, subscription delivery and transaction-history pagination through the API remain future work. Operators can submit a known missed signature for independent verification.

## Accounting and financial authorization

Amounts are signed 64-bit integer atomic units, nonnegative for balances and positive for receipts/reservations. SOL has nine decimals and USDC six. Human amount parsing uses checked integer arithmetic and rejects floats/exponents, signs, excess precision and overflow. RPC amounts are parsed from integer strings; UI floating-point values are ignored.

| Category | Source and treatment |
|---|---|
| Simulated microUSD | Existing independent per-agent simulated ledger and runway; always labeled simulated |
| On-chain observations | SOL lamports and classic SPL-USDC atoms, slot/time/network and evidence source; Devnet test tokens, no monetary value |
| Verified incoming USDC | Finalized, validated net-positive receipts only; never inferred just from observed balance |
| Operating reservations | Explicit human-authorized held USDC allocations; no transaction or service purchase |
| Inference cost | Existing `inference_calls.accounted_microusd` estimate/reservation evidence, exposed separately in treasury summaries |
| Infrastructure cost | Existing simulated operating expense ledger; no real infrastructure purchase occurs |
| USDC cash expenses | Both cash-expense fields remain zero because this milestone has no payment execution/settlement path; they are not estimates or an invoice journal |

Available Devnet operating capital is `max(0, min(observed USDC, verified receipts) - held reservations)` only when observation is under 120 seconds old, its slot covers the latest receipt and observed USDC equals tracked receipts. Otherwise capital is zero. Unexplained initial balances, missing receipts, external debits and stale snapshots require reconciliation rather than inventing revenue or resetting history. No exchange rate turns Devnet tokens into dollars or simulated funds. API charge estimates are not subtracted from token balances. Verified receipts cannot pay for OpenAI; the existing authorized USD budget and simulated survival gate still apply.

The separate financial policy defaults to locked, reservations disabled, zero caps and an empty destination allowlist. A new reservation requires the persisted owner, an active lifecycle, Devnet, explicit reservation enablement, unlocked policy, an allowed destination, a positive amount within per-transaction/daily caps and fresh reconciled available capital. Agent and wallet row locks serialize requests across service instances. Request UUIDs are immutable: same-payload replay is idempotent while eligible, conflicting reuse or reuse after release is rejected. Nothing is automatically released after restart.

For conservative daily caps, committed allocations include every reservation created since 00:00 UTC (including released ones), plus all older unresolved holds. Releasing a reservation restores available capital but cannot reset today's cap. There is no settlement path that could turn released allocations into off-ledger spending. Policy changes require the human owner. Emergency stop atomically suspends/fences reasoning and locks the wallet policy; resume alone does not unlock it. Personality and LLM tool permissions cannot change these controls.

`simulate_for_review` accepts bounded base64 wire bytes, pins Devnet, uses signature verification disabled with recent-blockhash replacement, requires a successful simulation and limits compute to 200,000 units. It returns sanitized slot/unit evidence with `signing_authorized: false`. It is a provider helper, not a public signing capability. No signing path exists, so successful simulation never leads to signing. A future signer still needs wire-transaction parsing, destination/mint/fee validation, fresh simulation, current permission checks and atomic policy reservation. The planned $5 trading ceiling remains documentation only.

Every reached reservation/policy/release/signing decision is audited with actor, agent/wallet, fixed operation/outcome, request UUID where relevant and timestamp. Receipts, duplicates, verification failures and sync results are durable. Unauthenticated HTTP requests are rejected by middleware before domain operations; malformed bodies never become financial operations. Private prompts, credentials, arbitrary remote errors and key material are absent from financial audit data. Model-visible wallet events use fixed safe codes; social canonical facts were not expanded to fabricate wallet achievements.

## Local configuration

Wallet support is disabled by default. No master key or wallet was installed in the user's environment during this implementation. Use a development database, the existing private API token and an operator UUID. Create custody material only when intentionally setting up a local wallet:

```bash
# Run once; exclusive creation refuses to replace an existing master.
umask 077
mkdir -p .supa-secrets
chmod 700 .supa-secrets
python3 - <<'PY'
import os
fd = os.open('.supa-secrets/wallet-master', os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
with os.fdopen(fd, 'wb') as f:
    f.write(os.urandom(32))
    f.flush()
    os.fsync(f.fileno())
PY
```

Set these exported values or load a private ignored `.env.wallet` in the shell before starting `supa-api`. The server does not automatically load dotenv files. Never put raw master bytes in environment variables.

```dotenv
SUPA_WALLET_ENABLED=true
SUPA_WALLET_OPERATOR_ID=<stable-human-operator-uuid>
SUPA_WALLET_VAULT_DIR=/absolute/project/path/.supa-vault
SUPA_WALLET_MASTER_FILE=/absolute/project/path/.supa-secrets/wallet-master
SUPA_SOLANA_RPC_URL=<your-devnet-https-rpc-endpoint>
# Optional; omit for manual observation only:
# SUPA_WALLET_POLL_SECONDS=60
```

Use mode 0600 for `.env.wallet`; `.env.*`, `.supa-vault/` and `.supa-secrets/` are ignored. Do not overwrite `.env.eval` or change its budget configuration. Leave `SUPA_PROVIDER=offline` unless separately authorizing paid reasoning. Wallet creation and public recovery require no chain request; only sync/verification or explicit polling contacts RPC. Mainnet is rejected even with a configured endpoint. The API still binds only to loopback.

## Authenticated API contract

All routes below have prefix `/agents/{id}/wallet`, require the existing bearer token and the configured trusted operator matching the wallet owner. This is a single-operator local API; actor UUIDs never come from untrusted request bodies.

| Method / suffix | Contract |
|---|---|
| `GET /` | Public identity and owner/provenance metadata |
| `POST /` | Create human-owned Devnet identity, or return existing owned identity |
| `POST /recover` | Validate vault recovery and return public address only |
| `GET /balances` | Last observed `WalletBalance` or null; includes evidence source/network/slot |
| `POST /sync` | Scan verified history and reconcile; returns scanned/new-payment counts, backlog flag and treasury |
| `GET /payments` | Up to 100 newest verified receipts |
| `POST /payments` | Body `{"signature":"<base58-signature>"}`; fetch and verify chain proof, return `inserted` |
| `GET /payment-checks` | Up to 100 latest durable verification statuses |
| `GET /treasury` | Exact atomic amounts, estimates in separate microUSD field, freshness and reconciliation flags |
| `GET /policy` | Independent persisted spending policy |
| `POST /policy` | Strict `SpendingPolicy` object; changes reservation policy only, never enables signing |
| `GET /reservations` | Up to 100 newest operating allocations |
| `POST /reservations` | `request_id` UUID, `destination` base58, integer `amount_atoms`, `purpose` (`inference` or `infrastructure`) |
| `POST /reservations/{key}/release` | Owner releases a bookkeeping allocation; no funds transfer |
| `GET /audit` | Up to 100 newest sanitized financial decisions |
| `POST /sign` | Always 403; audited `denied` or `milestone_signing_disabled` |

Route paths have no trailing slash in actual requests (`/wallet`, not `/wallet/`). Policy JSON fields: `locked`, `reservations_enabled`, `per_transaction_atoms`, `daily_atoms`, `destinations` (array of allowed addresses). Insufficient policy/ownership returns 403, invalid payment/input 400, unconfirmed 409, unavailable RPC/vault/database 503. Missing bearer token returns 401. No secret is included in error responses.

Future Next.js/TypeScript/Tailwind work must preserve the approved dark/neon-teal cinematic identity. Show separate simulated microUSD, Devnet test tokens and provider USD estimate panels. Display network, mock/configured source, freshness and reconciliation prominently. Keep publishing, trading, Mainnet payments and agent factory disabled. No replacement dashboard was built.

## Validation and evidence

Commands (offline providers and mocked chain RPC only):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --all-targets --locked --offline
# Explicitly select the existing disposable local test database:
TEST_DATABASE_URL=postgresql://pikoo@127.0.0.1:55432/supa_agent_test \
  cargo test --workspace --all-targets --locked --offline -- --include-ignored
# Human-readable controlled demonstration:
TEST_DATABASE_URL=postgresql://pikoo@127.0.0.1:55432/supa_agent_test \
  cargo test -p supa-runtime --test wallet --locked --offline \
  mocked_devnet_demo_identity_receipts_duplicate_restart_and_signing_denial -- --ignored --nocapture
```

The test database uses existing local trusted authentication; the example URL contains no password. Tests add uniquely identified test records, never truncate existing records. Do not point `TEST_DATABASE_URL` at an evaluation or production database.

Final results: formatting passed; Clippy with warnings denied passed; the full suite passed **91 tests, zero failures, zero ignored** (including 35 PostgreSQL integration tests and six local HTTP mock tests). The default offline suite passed **50 tests, zero failures, 41 intentionally ignored** opt-in tests. The separately captured demonstration passed one test. Machine-readable evidence is recorded in `artifacts/milestone-3/validation.json`; raw test transcripts are alongside it. The demonstration checks offline Ed25519 identity creation, mocked 2 SOL test balance, one finalized mocked 1.5 USDC test receipt, exact duplicate rejection, reconnection with new Store/WalletService/vault objects, and rejection of both unauthorized and owner signing. This is component restart/reconnection evidence, not a machine power-loss or production backup-restore exercise.

Additional tests cover network/mint/owner/decimals validation, exact amounts and overflow, unconfirmed failures, incomplete/mismatched transaction proof, RPC timeout plus bounded retry, sanitized errors, failed simulation, policy caps, concurrent reservations, idempotency conflicts, lifecycle suspension/termination, emergency locking, stale/unreconciled capital, mock/actual source separation, preserved scan cursors after page failure, authenticated HTTP routes and model-context separation. All prior runtime, social, OpenAI wire-contract and recovery tests remain included; OpenAI responses are local mocks.

Read-only verification of `supa_eval` confirms budget `ad91e475-f1da-495a-8247-139643d2d8b1` remains capped at 100000 microUSD ($0.10), committed 10299 microUSD ($0.010299), with 24 succeeded calls and one unknown historical reservation. No budget, reservation or old inference record was reset. No new paid requests were made.

## Git and next-phase readiness

`git status` reports “not a git repository.” `.git` is still the original empty read-only placeholder. No metadata was deleted, moved, replaced, initialized or force-pushed; no commit could be created and no main branch could be verified.

Safe recovery recommendation: create a separate checkout of the authoritative upstream on `main`, preserve this entire working tree and original `.git`, then review/copy source changes into that separate checkout while excluding credentials, vault material and build artifacts. Verify changes and commit there using Conventional Commits (suggested title: `feat(wallet): add devnet identity and verified payment accounting`). Do not run `git init` over the existing placeholder. Recovery has not been performed without an upstream location and permission to change filesystem locations outside this workspace.

The next phase can configure an approved Devnet RPC and separately approved test funding, then repeat observation and receipt checks against actual Devnet. Actual-chain behavior remains unvalidated here. Real expense settlement, safe isolated signing, parsed transaction authorization, production authentication/custody, independent chain trust, multi-process monitoring and backup recovery require additional implementation and review. Mainnet transfers, Jupiter trading, X publishing, child agents and deployment remain disabled.

Primary protocol references: [Solana transaction RPC](https://solana.com/docs/rpc/http/gettransaction), [signature status](https://solana.com/docs/rpc/http/getsignaturestatuses), [cluster environments](https://solana.com/docs/references/clusters), and [Circle USDC contract addresses](https://developers.circle.com/stablecoins/usdc-contract-addresses).
