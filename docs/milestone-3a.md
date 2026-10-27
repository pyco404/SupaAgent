# Milestone 3A — Git recovery and real Devnet pre-funding verification

Git is recovered through separate local metadata, and real Solana Devnet connectivity, chain identity, canonical USDC mint, encrypted wallet recovery and balance persistence are verified. No actual incoming payment has been obtained or claimed. Funding requires the user's separate approval, so the payment-receipt portion is deliberately pending.

## Git recovery

The original `.git` is an empty directory owned by `pikoo:pikoo` with mode 0555. It was not deleted, moved, rewritten, chmodded or replaced. The official remote `https://github.com/pyco404/SupaAgent` was reachable and returned no branches or tags; there was no upstream history to merge or overwrite.

A new private `.git-recovery/` directory (0700) holds independent Git metadata for this same working tree. `main` is prepared locally, with the official repository configured as `origin`. Use:

```bash
./scripts/git-local status
./scripts/git-local log --oneline
./scripts/git-local remote -v
```

Ordinary `git status` still sees the original unusable `.git`; the wrapper is required in this workspace until a separately approved migration to conventional metadata. No global Git configuration was changed. The complete Rust workspace, all four migrations, documentation and existing evaluation artifacts remain in place and are included in the local source snapshot. The snapshot uses an explicitly local coding-agent author identity rather than leaking a global personal email.

A local pre-commit hook runs the staged-content/history secrets scan. A pre-push hook unconditionally rejects pushes pending explicit user approval. The hook was tested directly; no push or push dry-run was attempted. Hooks live in ignored metadata and are not automatically installed by a future clone. No GitHub write occurred.

`.env.eval`, `.env.devnet`, `.env.wallet`, `.supa-secrets/`, `.supa-vault/`, the new metadata directory and common private-key formats are ignored. Files under `.aws`, `.codex` and `.agents` are excluded. Only `.env.example` is eligible for tracking among `.env*` files.

### Secrets scan

`python3 scripts/scan-secrets.py` scans staged blobs and every reachable commit tree for protected paths, OpenAI/GitHub/AWS token formats, private-key headers, credential-bearing URLs, high-entropy credential assignments, Solana private-key arrays and encrypted wallet envelopes. It also compares content against actual local secret values read internally from the environment/private dotenv files, and against raw/hex/base64 forms of the local master material. No matched values are printed; findings contain rule/path only. Six independent ignore checks are required. Seven scanner self-tests passed.

Initial findings were synthetic test credentials, scanner self-test strings and exact matches to passwordless local database URLs. No genuine credential was found in the proposed snapshot. Test fixtures now construct the same synthetic inputs without embedding complete credential-looking literals. Passwordless database URLs are classified as configuration; real password-bearing URLs remain scanned. No blanket source-directory exemptions or secret suppressions were added. The final scan passed with zero findings before commit; Git's pre-commit hook repeats the scan. A further reachable-history scan is performed after commit.

No third-party secrets scanner is installed. This is a targeted local gate, not a guarantee that every unknown secret format or semantic disclosure can be detected. Public push remains blocked for review and separate approval. See `artifacts/milestone-3a/secrets-before-commit.json` for the final recorded pre-commit findings/counts.

## Real Devnet verification

The official public endpoint `https://api.devnet.solana.com` was selected. It responded successfully to two independent process runs. This proves observed connectivity, not an availability SLA; the public service is rate-limited and can require a separately configured provider for sustained use. No provider account or paid RPC subscription was created.

The first real genesis read exposed a Milestone 3 bug: `SolanaNetwork::genesis()` held a truncated CAIP-style identifier rather than the full genesis hash. Mock RPC fixtures reused that constant, so the original 91 tests did not catch it. The actual RPC returned:

```text
EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG
```

Both Devnet and Mainnet full-hash constants were corrected against [Solana's genesis configuration source](https://github.com/solana-labs/solana/blob/master/sdk/src/genesis_config.rs). A new regression test uses an independent literal, accepts the full actual Devnet response and rejects the former truncated value. Mainnet remains operationally disabled; no Mainnet RPC was contacted.

The adapter now also supports read-only `getAccountInfo` to validate the exact canonical Devnet USDC mint, classic token-program ownership, non-executable mint account, initialized state and six decimals. It does not trust token names/symbols. Actual finalized RPC metadata matched [Circle's published Devnet mint](https://developers.circle.com/stablecoins/usdc-contract-addresses):

```text
4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU
```

The cluster/endpoint and test-token semantics are documented by [Solana](https://solana.com/docs/references/clusters). Devnet tokens have no monetary value and are separate from Mainnet assets.

## Dedicated wallet and database

A new `supa_devnet` database and a restricted `supa_devnet` login role were created on the existing localhost PostgreSQL server (port 55432). The role is not superuser and cannot create databases/roles or replicate. Existing databases and credentials were not reset. The new role owns only the dedicated database; a read-only privilege check confirmed it cannot INSERT, UPDATE or DELETE the evaluation budget table. Local PostgreSQL uses existing trusted authentication; this remains a local development environment rather than production authentication isolation.

Private `.env.devnet` (0600) persists these settings across terminal sessions:

- `SUPA_DEVNET_DATABASE_URL` — dedicated localhost database only.
- `SUPA_SOLANA_RPC_URL` — public Devnet RPC.
- `SUPA_WALLET_MASTER_FILE` — private raw 32-byte master file path.
- `SUPA_WALLET_VAULT_DIR` — encrypted vault directory.
- `SUPA_WALLET_OPERATOR_ID` and `SUPA_DEVNET_AGENT_ID` — stable identity references.

A fresh OS-random master was created with exclusive creation under `.supa-secrets/` (0700); its file is 0600. A separate encrypted Devnet envelope lives under `.supa-vault/devnet/` (0700 directory, 0600 envelope). No prior key or configuration was overwritten. No key bytes, seed or key fingerprint were printed or committed. `.env.eval` was neither loaded by the Devnet command nor changed.

The dedicated agent remains `created`, scheduler disabled, with no LLM tool capabilities. Its public wallet address is:

```text
4imA97URQuuFYbSKNum5dWCqUmRNenmscEcVpAju5S7U
```

Actual finalized observations were **0 SOL lamports and 0 USDC atoms**, with no token accounts. The mint itself was independently read and validated. A new process reconstructed the store/service/vault, recovered the same public address, re-read Devnet and persisted another balance observation. Read-only database verification confirmed one wallet, `configured_rpc` evidence, migration 4, locked wallet policy, zero verified receipts, zero simulated ledger entries and zero inference calls.

Evidence files:

- `artifacts/milestone-3a/devnet-check.json` — first actual chain/mint/balance check.
- `artifacts/milestone-3a/devnet-reconnect.json` — second process check.
- `artifacts/milestone-3a/devnet-database.json` — durable database facts.
- `artifacts/milestone-3a/evaluation-budget.json` — unchanged evaluation-budget evidence.

The existing evaluation budget remains `ad91e475-f1da-495a-8247-139643d2d8b1`, cap 100000 microUSD ($0.10), committed 10299 microUSD ($0.010299), with all 25 historical inference records preserved. No new model request, financial reservation or budget was created.

## Observation-only command

The new example reuses the existing vault, RPC, wallet service, migrations and persistence; it introduces no crate or dependency:

```bash
# Completed once: create/reuse local agent and encrypted wallet, no network calls.
cargo run -p supa-runtime --example devnet_verify --locked --offline -- --setup

# Completed twice: read-only Devnet calls and local persistence, no funding/signing.
cargo run -p supa-runtime --example devnet_verify --locked --offline -- --check

# Prepared, NOT executed: use only a signature from a separately approved external test payment.
cargo run -p supa-runtime --example devnet_verify --locked --offline -- \
  --verify '<approved-devnet-transaction-signature>'
```

The command automatically reads the private `.env.devnet`; it does not export secrets into process-global configuration or initialize a reasoner. Conflicting shell configuration is rejected rather than overriding the file silently. Database parsing and a server identity query require a local database named exactly `supa_devnet`; evaluator, test and remote database URLs are rejected. No funding, airdrop, signing or paid-generation command mode exists. Failures identify the stage and a fixed safe error code without dumping configuration or remote bodies.

The prepared `--verify` workflow independently fetches finalized transaction/status evidence, inserts a receipt once, reconnects through a fresh pool/service/vault, re-verifies the same signature and asserts that the duplicate cannot increase verified receipts. Its result includes network/genesis, canonical mint, signature, finalized status and exact treasury totals. It has not been run against an actual payment.

## Financial and confirmation safeguards

Simulated microUSD, Devnet SOL/USDC observations, finalized receipts and estimated provider dollars remain in their existing independent ledgers/units. No observed balance alone becomes revenue. Receipt storage preserves network, mint, signature, slot, exact atom amount and evidence source; `payment_checks.status=verified` means the verifier required finalization. The command's evidence additionally records the explicit `confirmation_status=finalized` and full genesis hash. No Mainnet asset path, signing, settlement or trading permission was enabled.

Concurrent observation tests use independent service instances and database pools. Exactly one insert succeeds, the other is a duplicate, and total verified atoms stay unchanged. When later evidence for an already recorded payment is unconfirmed/invalid or conflicts with stored proof, the application now locks the wallet policy and makes its prior balance observation stale. It keeps historical receipt/check evidence and financial records intact, emits a fixed audit outcome and requires human review. Restored finality does not automatically unlock the policy. RPC transport failures alone do not invent new credits, and stale capital remains unavailable.

These are deterministic tests for confirmation/slot/failure regressions, not evidence that a real finalized transaction reorganization occurred. The configured RPC remains a trusted source; this is not a light client or a multi-provider quorum. Devnet ledger resets, historical pruning and unavailable past signatures may require human reconciliation. The monitor and history bounds from Milestone 3 still apply.

## Validation

Formatting and Clippy with warnings denied passed. The full offline/local integration run passed **97 tests, zero failures, zero ignored**, comprising 55 default offline tests, 36 PostgreSQL integration tests and six local HTTP mock tests. The default suite passed 55 with 42 intentionally ignored opt-in tests. Seven Python secrets-scanner self-tests passed separately.

Commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --all-targets --locked --offline
TEST_DATABASE_URL=postgresql://pikoo@127.0.0.1:55432/supa_agent_test \
  cargo test --workspace --all-targets --locked --offline -- --include-ignored
python3 scripts/scan-secrets.py --self-test
python3 scripts/scan-secrets.py
```

Coverage includes the previous 91 tests plus independent full-genesis validation, malicious mint metadata, confirmation/failure/slot regressions, concurrent duplicate observations, unavailable signing/funding CLI modes and dedicated-database validation. Existing wrong-mint/owner/decimals checks, malformed proof rejection, RPC timeout/retry tests, unauthorized signing, lifecycle gates, concurrent reservations, restart recovery, GPT wire contracts and social factual/repetition controls all remain covered.

See `artifacts/milestone-3a/full-tests.txt`, `default-tests.txt`, `clippy.txt` and `validation.json`. All Cargo integration providers are mocks; only the explicitly recorded Devnet check processes accessed the actual chain.

## Changes and outstanding approval

Changed implementation files: `domain/src/wallet.rs`, `providers/src/solana.rs`, `persistence/src/wallet.rs`, `runtime/tests/wallet.rs`, new `runtime/examples/devnet_verify.rs`, `.gitignore`, new `scripts/git-local` and `scripts/scan-secrets.py`. Synthetic test-literal formatting changed in `providers/src/authentication.rs` and `runtime/examples/live_eval_support/mod.rs` without changing their behavior. Documentation and sanitized evidence were added. All six crates and four existing migrations are preserved.

The actual receipt, actual-payment duplicate test and actual-payment history restart check are **pending**, not successful. No faucet or transfer request was made. The proposed next workflow, requiring separate approval, is a human-controlled external Devnet transfer of **0.01 test USDC (10000 atoms)** using the canonical mint to the public wallet above. The external sender must pay any Devnet fee/account-creation cost. Supa does not need to sign. Return the transaction signature afterward; only then run the read-only `--verify` command and save its result. A separate SOL airdrop is optional and also requires approval; it was not requested.

The local encrypted vault retains the Milestone 3 limitations: same-user/root process access, no hardware isolation, no seed export, and backup recovery requiring the original encrypted envelope plus master and database mapping. No production custody claim is made. No Mainnet transfer, swap, Pump.fun launch, X publication, deployment or paid inference occurred.

Public push also requires explicit approval after review of the local commit and scan. The pre-push guard remains installed. Nothing has been pushed to GitHub.
