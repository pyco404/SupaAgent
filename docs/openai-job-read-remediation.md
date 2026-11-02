# GPT evaluation job-read remediation

Validated locally on 2 November 2026. No external provider requests, paid generation, primary PostgreSQL startup, deployment or push occurred during this remediation.

## Cause and correction

The evaluation runner performs authenticated GETs to `/health`, `/agents/{id}/live-work`, and `/jobs/{id}` before creating its request journal or submitting consent. The job handler first checks customer/agent access through `Store::economic_job`, then reads the optional recipient through `Store::wallet`.

`wallet()` unnecessarily opened a transaction and called the same `identity()` helper used by financial mutations. That helper executes `SELECT ... FOR UPDATE` on `wallet_identities`. PostgreSQL checks the required UPDATE privilege even when there is no matching wallet. Consequently the pinned evaluation role, which correctly has no wallet UPDATE privileges, received a database error mapped to HTTP 503. No provider call or reservation was needed to encounter it. Earlier HTTP readiness coverage had omitted the exact job endpoint.

`Store::wallet` now performs a single non-locking SELECT and shares the existing row decoder with the locking helper. Its API, returned fields, missing-wallet behavior and authorization callers are unchanged. A single identity lookup requires no cross-statement financial consistency lock. The former transaction ended before its callers' later operations anyway.

All in-transaction callers of `identity()` retain their wallet row lock: audit writes, balance observation, payment recording/failure handling, policy changes, reservation allocation/release and related reconciliation. Treasury's multi-query financial summary also retains its existing lock; this patch does not claim every treasury endpoint supports SELECT-only roles. Financial writes still acquire their existing agent/wallet locks and enforce ownership, lifecycle, policy, available balance and replay checks. Wallet lookup callers outside these transactions do not gain mutation authority.

No migration or grant changes were made to the evaluation role. Its existing wallet UPDATE/table/column privileges remain false.

## Files

- `crates/persistence/src/wallet.rs`: separate non-locking identity read from transactional financial locks.
- `crates/api/tests/wallet_read_permissions.rs`: fresh disposable database, SELECT-only/read-only role, populated and absent wallets, owner isolation, lock contention, and mock work execution with grants matching the reviewed execution role.
- `scripts/test-openai-scoped-api.py`: actual job GET, wallet privilege assertion, and complete real-runner preparation through an intercepted POST boundary.
- `scripts/test-live-work-evaluation.py`: HTTP 503 job-read regression proving no POST or request journal follows the failed preflight.
- This report.

## Results

| Check | Actual result |
| --- | --- |
| Rust workspace, all targets, locked/offline | 95 passed, 0 failed; 99 opt-in tests ignored |
| Selected real PostgreSQL integration suites | 32 passed, 0 failed, none ignored |
| Runner / launcher / scope regressions | 10 / 8 / 4 passed |
| Actual locked API HTTP/SQL rehearsal | All 8 groups passed, before and after clean database/API restart |
| Frontend desktop/tablet/mobile contract and security tests | 36 passed |
| Frontend lint, type check and production build | Passed |
| Rust formatting and workspace Clippy, warnings denied | Passed |
| Secret scanner self-tests | 15 passed |

The 32 PostgreSQL tests cover authorization (8), controlled inference (6), economic jobs (1), live work (7), the new permission regression (1), economic payments (1) and wallets (8). They include concurrent reservations, duplicate receipts, uncertain-charge/restart fences, paused/unpaid job protection and mocked provider failures. No actual blockchain or inference requests occur in these tests.

The new regression proves both absent and existing wallets can be read using SELECT-only privileges and `default_transaction_read_only=on`. Another owner's credential and the public credential cannot read the job. A read completes while another transaction holds the wallet lock; a financial policy write waits until that lock is released. Unauthorized policy changes and the SELECT-only role's writes fail.

A second role in that fresh throwaway database mirrors the existing evaluation execution grants, without wallet UPDATE. A fully mocked work session reaches `awaiting_approval`, persists usage/accounting and rejects duplicate execution without another mock generation. This does not consume the reviewed real evaluation budget or nonce.

The actual reviewed database rehearsal forwards the real runner's three GETs to the locked Rust API. It intercepts the final POST in Python before any network dispatch, validates the fixed job/input/nonce/request ceiling, and uses a temporary synthetic journal outside the real request-journal location. It never submits paid consent to the real API. Public-table fingerprints match before/after rehearsal and database/API restart.

Initial Node subprocess tests and the frontend build encountered sandbox child-process restrictions; rerunning with local process execution permitted passed. One test-interceptor method accessor and a Rust test URL conversion were corrected during validation. Final checks above reflect the corrected code.

## Preserved evaluation and protected state

- Scope: `0453528bc228c83e6137eabd53ebeb4a7b98a198e7e6937e397ece240a51a6f0`.
- Project/model: `proj_npsyg93Ofk7ivKheGa8IDw5q` / `gpt-6-luna`.
- Database: `supa_m10a_a13db38ce0a5_test`, approved Unix socket on port 55681.
- Agent/job/input/nonce unchanged. Agent paused, job created/unpaid, revision 0; no schedules, revenue or inference sessions.
- Independent budget `6ec8370e-fcd8-4675-84dd-1b754f6664c2`: $0.005 ceiling, $0 committed; one generation maximum, $0.004 estimated request ceiling.
- Job expires 3 November 2026 at 10:52:29.570357 WAT; not renewed or extended.
- Renewed disposable credentials expire 2 November 2026 at 15:02:40 WAT. The API remains running with paid dispatch disabled and exits at its credential deadline. Credentials remain private and ignored.
- Primary PostgreSQL remains stopped; protected primary/backup/accounting hashes and historical plans/journals match the existing baseline.
- Historical OpenAI $0.004048 hold and Claude $0.003536 unknown reservation remain unchanged. No settlement/refund/release is performed.
- Additional regression cluster is stopped after testing; no unrelated files or evidence were deleted.

Sanitized local evidence is under ignored `artifacts/openai-job-read-fix/` and `artifacts/openai-final-readiness/`. It is not committed. Full main history and reachable-history secrets audits are required by the commit hooks and final validation.

## Remaining approval boundary

The local job-read blocker is resolved, and the complete preparatory execution path and restricted-role mocked execution passed. This is not proof that a future provider request will succeed or that shared provider capacity is currently available. The operator has separately confirmed the intended project's spending permission; that is operator evidence, not independent account inspection.

No paid retry is authorized by this remediation. Obtain fresh approval for the unchanged scope and limits, and recheck temporary credential and job validity immediately before dispatch. If either has expired, stop for fresh approval; never extend the job automatically.

Protected command, not executed during remediation:

```bash
python3 scripts/openai-work-api.py execute \
  --authorize-paid \
  --scope-sha256 0453528bc228c83e6137eabd53ebeb4a7b98a198e7e6937e397ece240a51a6f0
```
