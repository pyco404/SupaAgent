# Milestone 7A — Real Devnet receipt readiness

Validation dates: 2026-10-30–31 (Africa/Lagos). No primary database activation, funding, signing, paid inference, deployment or push is authorized by this milestone.

## Result and evidence boundary

Nonfinancial preparation is implemented. The real recipient remains **unassigned**, as explicitly requested. No genuine job payment has been observed or credited. Positive receipt/accounting assertions below use mocked RPC responses; they are not blockchain revenue.

Actual read-only Solana Devnet RPC observations verified genesis `EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG` and canonical USDC mint `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU`: initialized, six decimals, non-executable, owned by the canonical Token Program. Mint observation slot: 516913517. A deliberately nonexistent signature submitted through the isolated UI returned an unconfirmed/conflict response and created no receipt or credit. No faucet, airdrop, transfer, key generation, signing or paid request occurred.

The isolated unfunded browser fixture uses public address `11111111111111111111111111111112`. **Never fund this fixture.** It is not an externally controlled recipient and the runner refuses to use fixture state as genuine receipt evidence. Its persisted job is `81bbce7c-ec87-4b97-80c5-5f99c407d46e`, awaiting payment, received/earned atoms zero, receipt/entry counts zero, including after PostgreSQL restart.

## Implementation

- Owner-only `POST /agents/{id}/economy/recipient` accepts only a public address and explicit `confirm_watch_only: true`. It checks authenticated owner role and agent ownership server-side. Sequential identical requests reuse the wallet; changing an existing wallet is rejected. Unknown/private-key fields are rejected. It creates no keypair or vault material, and reports `external_control_verified: false` and `signing_enabled: false`. Address registration does not prove control. The epoch write gate remains in force.
- The existing verifier checks configured network genesis, finalized status, transaction success, canonical mint/program/decimals, recipient, amount and job memo before durable reconciliation. Missing/nonfinalized statuses or missing transactions now return unconfirmed rather than malformed-payment errors. No incomplete RPC result is credited.
- Existing receipt uniqueness, transaction/row locking, immutable ledger and job revision controls prevent duplicate credit and cross-job reuse. Underpayments accumulate as liabilities and do not make an unfunded job executable. Receipt is not earned revenue: delivery/acceptance accounting remains separate. Expired jobs do not become execution-eligible; refunds/transfers remain disabled.
- Frontend pending-receipt errors are distinguished from stale command errors. Background polling cannot replace economic action feedback while an action is pending or unresolved. The operator must explicitly reload records before another attempt; no automatic mutation retry was added. Appearance, logo, navigation, motion and ten sections are preserved.
- The migration-only administrator now explicitly limits both its inventory and execution to versions 9–10, even when the newly compiled application embeds migration 11. Migration 11 needs separate approval. A newly built administrator requires a newly verified executable digest in any future approval.

## Guarded rehearsal tooling

`scripts/devnet-receipt.mjs` supports prepare, inspect and verify. With no recipient:

```sh
node scripts/devnet-receipt.mjs prepare
```

This returns `recipient: unassigned` without database, wallet, file or network operations. Private recipient intent is also unset.

The runner accepts only local passwordless `pikoo` socket connections on port 55450, database `supa_devnet_receipt_test`, under this repository's `postgres-data/devnet-receipt-readiness/<isolated-run>/socket`. It independently checks server data directory, database identity, socket-only listening, and absence of any application epoch row before starting its API. Primary/epoch/remote/password/arbitrary-port targets are refused. State is owner-private and outside Git. Ephemeral test tokens never enter browser JavaScript or persistent state. Providers are offline and scheduler, worker, wallet signing and live inference are disabled.

After a real recipient is selected, independently verified, and isolated setup is approved, the exact workflow is:

```sh
# TEST_DATABASE_URL must identify the separately provisioned disposable cluster.
node scripts/devnet-receipt.mjs prepare \
  --state postgres-data/devnet-receipt-readiness/<isolated-run>/receipt-state.json \
  --recipient <verified-public-address> --consent-isolated-setup
node scripts/devnet-receipt.mjs inspect \
  --state postgres-data/devnet-receipt-readiness/<isolated-run>/receipt-state.json
# Only after separately approved external funding/signing and a finalized payment:
node scripts/devnet-receipt.mjs verify \
  --state postgres-data/devnet-receipt-readiness/<isolated-run>/receipt-state.json \
  --signature <finalized-public-signature> --consent-reconciliation
```

Preparation creates synthetic identities, one bounded summary offer/job and a watch-only wallet **only in the isolated test database**. It never adopts or recreates historical agents. Preparation records an exclusive state file before mutations; if interrupted in `preparing`, inspect database records manually rather than rerunning it. Inspection does not resume mutations. Verification intentionally repeats the same read-only receipt reconciliation concurrently and across an API restart to assert durable idempotency; it does not create an outgoing transaction. Unexpected existing receipts or accounting require operator review rather than silently continuing.

The proposed request is 5000 atoms = 0.005 **Devnet test USDC**, with six decimals, the exact canonical mint above, an exact job-UUID memo, and a 24-hour expiry. No actual payable request for a verified external wallet exists yet. Use the newly generated request's amount, memo, recipient and expiry; do not reuse the fixture UUID.

## External funding boundary

Before any genuine test: select the public system-wallet recipient, independently establish external control, approve isolated setup, then explicitly approve external test funding/signing. Private keys remain with the external signer and never enter the LLM, API, repository or runner.

The [official Circle faucet](https://faucet.circle.com/) supports Solana Devnet test USDC; verify the [official mint list](https://developers.circle.com/stablecoins/usdc-contract-addresses) at execution time. Obtain only test SOL for externally signed fees/account setup through [Solana's Devnet faucet guidance](https://solana.com/developers/cookbook/development/airdrops-and-faucets). Fund the sender externally, then send the job-bound payment with canonical Token Program `TransferChecked`, exact mint/decimals/recipient/amount, and the exact memo in the same transaction. A faucet transfer without the job memo is not automatically a valid job payment. See [Solana token transfer documentation](https://solana.com/docs/tokens/basics/transfer-tokens).

The runner cannot sign, request funds, broadcast or enable Mainnet. An address/balance read does not prove recipient control. No financial action is authorized by this report.

## Database and migration readiness

The primary cluster stayed stopped and untouched. Its last live verified state remains schema 8, prepared epoch `fd2f52e0-33fc-4c0d-893a-7b08b26ab748`, accounting namespace `641457b2-5fd8-4b31-b85f-c58839626831`, no owner grants. This report uses offline hash protection and restored-clone inspection, not a fresh primary SQL query.

Verified Stage 1 dump SHA-256: `600d2bb19b52b4f17d4278459ba722ea5f1135df703b8efc1b0b3db0c8360671`. A separate PostgreSQL 18.3 restored clone migrated 8→11 through the existing disposable-only review entry point. `scripts/validate-devnet-receipt.py` checked all SQLx SHA-384 migration checksums, preserved original columns/records across 28 source tables, and confirmed prepared revision 0, zero grants, approvals, live budgets, schedules and economic entries. No historical records were imported into the operational test fixture database.

A fresh private custom-format migrated dump restored into another isolated database with identical schema/row/constraint/index/sequence fingerprint:

`be104660f94a5acf9a9ff3544a2d2088bca4ec02ce4c4c2ddd68f62278b27417`

Migrated dump SHA-256: `817df424bce5b6c95588b11655e5f1f26cbab8500102d93c10cbefdc40fbee9a`.

The previously skipped specialized migration-admin integration test was rerun against a separate PostgreSQL 16 disposable cluster. It verifies approval boundaries, checksums, atomic failure rollback, interruption/restart, duplicate/concurrent execution, and that a fresh build embedding migration 11 still installs only 9–10. No real-target approval was issued.

Recovery policy: retain committed migrations and immutable audits; do not edit applied checksums or down-migrate financial history. The administrator's approved 9–10 transaction rolls back together on failure. General application migrations are committed per migration, so a failure after earlier commits requires inspection and separately reviewed forward repair or restoration to a new database, never an automatic overwrite of primary state.

Historical database remains **unrecovered**. Quarantined budget `ad91e475-f1da-495a-8247-139643d2d8b1`, ceiling $0.10 and unresolved $0.004048 hold remain protected; no reuse, refund, balance reset or claim of restored history. Independent backups remain unset; same-disk restoration proves restorability, not protection from disk loss.

## Validation and local evidence

- Rust workspace/all-target tests with ignored tests explicitly included: **139 passed; zero failed or ignored**. The specialized migration test ran rather than being skipped.
- Expanded receipt tests cover unauthorized verification, unfunded execution denial, underpayment, wrong genesis/mint/recipient/memo, failed transaction, interrupted/null RPC, expiration, cross-job signature binding, concurrent replay and restart persistence. The final expanded receipt target also passed separately.
- `cargo fmt --all -- --check` and workspace/all-target Clippy with warnings denied passed.
- Next.js production build, frontend lint and TypeScript checks passed.
- Frontend unit tests: **28 passed**. Guarded runner unit tests: **3 passed**, including unset recipient and target rejection.
- New browser consent/pending-feedback regression: **3 passed** across desktop/tablet/mobile. It uses mocked upstream responses and proves no write before consent, one request only, no automatic replay, feedback preservation across polling, and no horizontal overflow.
- Additional existing browser regressions: **15 passed** across desktop/tablet/mobile, covering read-only polling, ten sections, public access, simulated-vs-Devnet labels and contrast. Desktop/tablet completed 10 checks before the combined run was interrupted; mobile was rerun separately and completed all 5. Interrupted attempts and stale fixture-listener startup errors are not counted as successful runs.
- REA exercised the actual isolated Rust/API/frontend fixture and read-only negative RPC path. Scenario evidence `ev_053b3ddefa7db32aa3f643c0f42b4782898d0b43ad7a9b40fcb42cd5cbf1fc11` records completed steps and visible unresolved-receipt feedback. Attached event capture is incomplete; it cannot prove absence of pre-attach events or complete network coverage.
- Final REA screenshots: mobile 390×844, evidence `ev_6421809d126f3278d8fb0f90276ff21152b11a361d1bd9d5f8c2d8f6ed8276d8`; desktop 1440×1000, evidence `ev_1a4348d8df752ae96d0ed7f68b8799cfab1fd086e6c55a3acb5cb1cd2ad53b4a`. Both fit viewport width. Device testing is browser emulation, not physical devices.

Sanitized results, screenshots, test logs and private restored dumps stay ignored under `artifacts/milestone-7a/`; private test state stays under `postgres-data/devnet-receipt-readiness/`. They are not GitHub deliverables. No production credentials or wallet secrets were used.

## Remaining approvals and limits

1. Select/verify an externally controlled recipient public address; no recipient is assigned automatically.
2. Separately authorize test-token funding and external signing before a genuine receipt test. Finalized real receipt, recipient balance changes and actual on-chain idempotency remain unverified until then.
3. Real migration 9–10 requires independently trusted signed migration approval for the current executable/plan, exact database and reviewed state. Migration 11 requires separately reviewed authorization. None is granted here.
4. Real owner/custodian grants, epoch activation and frontend cutover remain separate approval boundaries. The primary frontend was not repointed.
5. Autonomous workers/spending, paid inference, Mainnet transfers, trading, token launches, X publishing, Railway deployment and pushing remain disabled or unauthorized.

The next milestone should be a single externally signed Devnet receipt rehearsal after recipient selection and explicit funding/signing approval, not production economic operation.
