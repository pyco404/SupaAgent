# Milestone 7: bounded economic services and contractual accounting

Implemented locally on main. The approved ten-section frontend, logo, intro, hands,
portal, navigation and branch animations remain in place. No primary database,
credential installation, owner grant, activation, deployment or push was performed.

## Services and execution

Owners can explicitly register immutable offers at `POST /agents/{id}/services`.
The public registry is `GET /services`; it projects offer metadata, never customer
input, memory, ownership credentials or job history. Each offer declares price,
input size/type, output contract, permission requirements, estimated simulated
resource costs, maximum attempts, provider timeout and execution lease limits.

Five bounded implementations are available:

| Kind | Delivered work | Deliberate limit |
| --- | --- | --- |
| `summary` | First three nonempty supplied sentences, capped excerpts and word count | Extractive; no invented facts |
| `research_report` | Supplied-material inventory and source excerpt | No external research or verification claim |
| `code_analysis` | Static line count and TODO locations | Never executes or compiles customer code |
| `api_report` | Structural report for a supplied JSON object | No arbitrary URL fetching or freshness claim |
| `delegation_simulation` | Declared simulated subtasks | Creates zero child agents |

The configured `Reasoner` receives the supplied goal and, only with ReadMemory
permission, relevant agent memories. It proposes at most one allowlisted
`read_memory` action. Current permissions are checked again before honoring a
proposal and at successful delivery. The application-defined analysis tool, not a
model assertion, produces the structured delivery. Output content and a SHA-256
of its PostgreSQL JSONB representation are durable. No hidden model reasoning is
requested or stored. Provider recommendations cannot mutate accounting.

Only non-live, zero-input/output-rate provider policies are accepted. A single
provider invocation has a ten-second timeout and no automatic retry. A job has a
30-second execution lease and at most two explicitly authorized attempts. This is
an operator-controlled offline foundation, not unrestricted continuous economic
intelligence or evidence that GPT can complete arbitrary customer tasks.

## Durable jobs and access

`POST /jobs` uses the authenticated principal as customer; caller-supplied
principal/role/customer overrides are rejected. The request UUID is the durable
job identity and idempotency key. Private `GET /jobs`, `GET /jobs/{id}` and provider
Economy reads are scoped in Rust, independently of the frontend.

The ordinary path is:

`created -> awaiting_payment -> funded -> accepted -> executing -> completed/failed`

Commands at `POST /jobs/{id}/commands` require explicit consent, a request UUID
and expected revision. Request-body changes and stale revisions fail closed.
Customers can request payment, cancel eligible work, dispute and expire jobs.
Acceptance, execution, recovery and retry require agent operator access. Global
operator credentials without a grant on the service agent can act as customers,
but cannot accept work, inspect provider accounts or grant themselves access.
Service registration, simulated capital and spending-policy changes require both
an owner credential role and the existing per-agent owner grant.

Agent-row transaction locks serialize money and job transitions; execution tokens
fence stale finishers. Expired leases become failed only through explicit recovery.
Recovery never reruns work automatically. Retry requires a fresh authorization,
remaining attempts, unexpired contract and a new acceptance/cost reservation.
Expired/cancelled deposits remain liabilities. A completed dispute is recorded
without silently rewriting earned revenue; resolution/credit notes require future
reviewed functionality. Refund eligibility reports unearned deposits and excess
receipts. No refund transfer implementation is enabled.

Any identified prepared or active-read-only application epoch rejects economic
mutations. This work neither adopts historical agents nor bypasses activation.

## Payment reconciliation

Jobs support **simulation and Devnet only**; Rust and SQL reject Mainnet jobs.
Payment instructions expose the configured public wallet, network-specific mint,
six decimals, integer atoms, exact contract price, expiry and required job UUID memo.
A Devnet job requires an existing registered watch-only Devnet wallet identity.

`POST /jobs/{id}/simulated-payment` requires customer consent and records explicitly
simulated money. `POST /jobs/{id}/verify-payment` accepts only a transaction
signature and explicit read-only verification consent. The RPC URL comes solely
from server-side `SUPA_ECONOMY_DEVNET_RPC_URL`; it is unset by default. No client or
model can supply an RPC endpoint or signing material.

Read-only verification checks Devnet genesis, the canonical initialized USDC mint
and six decimals, finalized successful transaction/status with matching slot and
signature, canonical token-program transfers and the recipient's token balance
delta. Exactly one canonical Memo instruction must contain the job UUID. The
same fetched transaction is used for memo and transfer checks. Network/signature
uniqueness prevents allocation to multiple jobs; exact receipt replay is a no-op.
Mock and configured-RPC evidence cannot be mixed within one agent/network ledger.
Underpayments remain awaiting payment; overpayments remain liabilities beyond the
contract price. Late detection records a refundable liability rather than losing
the deposit. Restart recovery uses persisted receipts and immutable postings.

Actual Devnet read-only validation succeeded on 2026-10-30: genesis matched,
canonical mint was initialized with six decimals at slot 516904235, and a finalized
balance query succeeded at slot 516904245. The balance probe used the public
system-program address, **not an agent-owned wallet**. No funded agent wallet or
actual customer payment was established. Receipt, finality, wrong-mint, wrong-memo,
concurrent replay, cross-job reuse and late-payment tests used mocked RPC data.
No real receipt, real revenue, paid inference or transfer is claimed.

## Accounting and simulated infrastructure

Migration `0011_economic_jobs.sql` adds isolated services, jobs, commands, receipts,
spending policies, cost holds, audit history and double-entry economic postings.
It imports no historical treasury, live-inference budget or simulated autonomy
ledger records. Existing migrations were not edited.

All monetary values use integer six-decimal units. Each posting has explicit
debit/credit accounts, a positive bounded amount and an immutable idempotency key.
Deposits debit cash and credit customer liabilities. Verified delivery debits the
contract liability and credits revenue atomically with output persistence. Only
surplus operating capital can cover reserved simulated costs: outstanding customer
liabilities and owner-configured reserves are excluded. Simulated inference and
infrastructure expense postings debit expense and credit cash. Mainnet has no
posting path. Devnet and simulated networks have separate balances and keys.

Owner-only simulated capital/policy/spending endpoints are:

- `POST /agents/{id}/economy/capital`: `request_id`, `amount_atoms`,
  `authorize_simulated: true`.
- `POST /agents/{id}/economy/policy`: cumulative `ceiling_atoms`, `reserve_atoms`,
  `destinations` drawn from `inference`/`infrastructure`, `authorize_simulated: true`.
- `POST /agents/{id}/economy/spend`: `request_id`, `amount_atoms`, allowlisted
  `destination`, `authorize_simulated: true`.

The declared illustrative simulated rates are 1,000 inference and 100 infrastructure
atoms per successful job. They are not provider invoices. Real infrastructure
costs are not established; zero Devnet expense postings do not prove zero real
cost. Runway remains unavailable with an explanation rather than an invented
burn rate or sustainability metric. Policies cannot invalidate existing cost holds
or lower ceilings below consumed/reserved expenses. Policy changes have immutable
actor-attributed audit entries. Financial history rejects UPDATE, DELETE and TRUNCATE.
A database superuser can still override database protections; deployment roles,
external backup retention and operational custody remain necessary.

## Frontend and browser demonstration

Agents lists actual registered services. Economy adds a collapsible terminal form
for authenticated test-network jobs, explicit per-action consent, payment
instructions, read-only Devnet receipt verification, state/revision/attempt tracking,
outputs/checksums, refund eligibility, reserves, liabilities, revenue and expenses.
The existing ledger and wallet observations remain separately labeled. No demo
customers or successful jobs are seeded in the primary application.

The server adapter restricts exact routes, authenticates credentials server-side,
checks same-origin JSON mutations, rejects unknown input fields, bounds response
sizes and validates contracts. It does not forward caller identity fields or raw
upstream errors. Unknown responses disable the submitted control and instruct the
operator to inspect saved records; they are never automatically retried.

`node scripts/economic-smoke.mjs` exercises a real disposable PostgreSQL -> Rust ->
Next.js -> Chrome workflow. It creates only explicitly named test fixtures in a
local `*_test` database, with workers and live inference disabled. It demonstrated
creation, simulated funding, acceptance, execution, completed report and accounting,
then API restart persistence. Simulated revenue was 5,000 atoms, expenses 1,100,
and net result 3,900. Devnet earned revenue, actual earned USDC, transfers and paid
provider requests were all zero. Desktop 1440x1000, tablet 768x1024 and mobile
390x844 fitted without page overflow; all ten sections loaded. No page errors were
observed by the owned browser harness.

REA evidence IDs: desktop final `ev_d5c1cd323a0e1ff8967644ee46b476f4ffc1f5915dc835bf96a20c0dc188df31`; mobile `ev_3c6908694da4254f1dfd3a62debcac818572dd4b7d18ddc2127cb3819dedfcc6`; interactive scenario `ev_5df961d8d9109b6f1bfab35cb55742fd2821595c8a078bbbe34e89979e2fc292`.

REA independently exercised the economic disclosure and completed-history controls,
collected an accessibility tree and captured mobile/desktop screenshots. Its
attached event capture was incomplete; absence of browser errors is established
by the full-session Playwright listener, not inferred from absent REA events.
These are emulated Chromium viewports, not physical-device certification.
Impeccable guided a focused review; only the new controls received scoped terminal
palette/focus styling. No approved branding, layout or animation was redesigned.

Local evidence remains ignored under `artifacts/milestone-7/`:
`rust-suite.log`, `rust-suite-final.log`, `frontend-build.log`, `actual-devnet-read.json`,
`migration-checksums.json`, `backup-restore.json`, `protected-state.json`, and
`browser/economic-system-check.json`, screenshots and `rea-evidence.json`.
No dumps, screenshots, credentials or generated artifacts belong in Git.

## Validation and operational boundaries

- Complete Rust/PostgreSQL suite: 125 passed on disposable PostgreSQL 18.3.
- Migration-admin unit tests: 3 passed; 1 existing Stage-1-backup-specific test
  remains ignored and was not represented as executed here.
- Final targeted economic contract and mocked receipt tests passed again, including
  role spoofing, paid-provider rejection and malicious tool proposals.
- Rust formatting and warnings-as-errors Clippy passed.
- Next.js production build, ESLint and TypeScript checks passed.
- Frontend adapter unit tests: 27 passed, including new economic security cases.
- Full desktop/tablet/mobile browser regression suite: 156 passed (10.5 minutes).
- All migrations 1–11 have matching SQLx SHA-384 checksums in the disposable DB.
- Private disposable backup restored successfully: all 43 non-system tables matched.
  This is a same-disk recovery test, not an independent/offsite backup.
- Protected-state hashes for 13 historical/configuration files were unchanged;
  the primary PostgreSQL postmaster remained absent throughout validation.

The first broad Rust run exposed an incomplete disposable HBA profile for existing
restricted-role tests. Correcting that test-only profile yielded the complete pass.
A repeated mocked-receipt fixture reused its old signature and was correctly denied;
test signatures and wallets now use unique per-run identifiers. No duplicate
protection was weakened. REA's attached viewport required an explicit caller-side
reset before the final desktop capture; no product layout change was needed.

The historical database remains unrecovered. Prepared epoch
`fd2f52e0-33fc-4c0d-893a-7b08b26ab748` and pending owner
`6fd863e1-745d-4a06-b68e-eea69e854a69` were not activated or granted access.
Quarantined budget `ad91e475-f1da-495a-8247-139643d2d8b1`, its $0.10 ceiling and
unresolved $0.004048 hold were not reused, refunded or modified. Independent
backup storage remains unset. Primary credentials and frontend routing are unchanged.

## Remaining steps toward a first real USDC earning

This milestone establishes tested simulated contracts and mocked Devnet receipt
reconciliation; it does **not** establish independent real-money operation.
Before any live earning, separately review and authorize:

1. Exact target/epoch activation and deployment roles. The earlier migration-only
   administrator remains pinned to migrations 9–10; migration 11 needs its own
   reviewed execution/backup plan. Do not point an auto-migrating API at the primary.
2. Real owner/customer identities, watch-only wallet ownership, offered service
   terms, acceptance criteria, dispute resolution and manually approved refunds.
3. A funded Devnet test receipt with the exact mint, amount and memo. No airdrop,
   test transfer or wallet creation with live state occurred during this milestone.
4. A separate, finite paid-provider authorization and independent accounting design
   if GPT is required. Economic execution currently rejects live provider policies.
5. Explicit read-only Mainnet receiving support and real price/cost verification.
   Mainnet jobs are currently rejected even though receiving could avoid signing.
6. Customer quotas/rate limits, pagination beyond the bounded 100-record views,
   settlement/reconciliation operations and independent backups before a marketplace.

Workers, continuous paid scheduling, signing, real spending, trading, launches and
publishing remain disabled. No CI/container/deployment readiness beyond existing
validated evidence is implied by this source change. A future push needs fresh
explicit approval; no push or deployment is part of this milestone.

## Git verification

Every changed source, test and documentation file receives its own unique
Conventional Commit through `scripts/commit-one`; the helper verifies an empty
starting index, main branch and exactly one explicitly staged path. Hooks run the
reachable-history/staged-content secrets gate. Generated artifacts and the private
backup remain ignored. Final complete-history and secrets results are retained
locally as `artifacts/milestone-7/git-audit.json` and
`artifacts/milestone-7/secrets-final.json`. The final response gives the resulting
commit count and HEAD. No history rewrite, push, deployment or primary migration
is authorized or performed.
