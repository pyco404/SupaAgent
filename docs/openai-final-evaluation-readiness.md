# Final GPT-6 Luna evaluation preflight

Validated 2 November 2026. **No external provider request or paid inference was performed.** Primary PostgreSQL remains stopped and unchanged. No persistent agent, worker, publication, transfer, trading, launch, deployment or push occurred.

## Account limits: evidence and remaining blocker

The intended project remains `proj_npsyg93Ofk7ivKheGa8IDw5q`. The owner-only ignored configuration contains the application key and exact project/model preferences. Prior authorized model-metadata HTTP 200 evidence was not unnecessarily repeated.

The operator supplied these values from an earlier OpenAI Platform screenshot:

| GPT-6 Luna limit | Reported value | Evaluation requirement |
| --- | --- | --- |
| Requests/minute | 10 | One generation; runtime also verifies model metadata |
| Tokens/minute | 60,000 | At most 24,192 input + 1,024 output = 25,216 tokens |
| Requests/day | 50 | One generation |
| Account credit | $5.99, automatic recharge disabled | Maximum request ceiling $0.004 |

The proposal fits the reported static limits. The screenshot was described by the operator, not independently retrieved here; current shared capacity and organization-level limits are not established by those values. **Project-specific spending restrictions remain unverified, as the operator explicitly confirmed. Do not report them cleared.**

No Admin API key, account-management connector or authorized limits export is available locally. The [project rate-limits API](https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/projects/subresources/rate_limits/methods/list_rate_limits) requires administrative authorization. An application key/model-metadata response cannot establish that account setting. [OpenAI rate-limit guidance](https://developers.openai.com/api/docs/guides/rate-limits)

Before paid authorization, confirm in OpenAI Platform that this exact project's model access and applicable project/organization spending restrictions allow the request, and that shared request/token/day capacity remains available. Share only non-secret confirmation. No administration credentials need to be supplied to Codex.

## Scoped disposable API

A dedicated example, `crates/api/examples/openai_work_api.rs`, uses the existing Rust router, controlled OpenAI adapter, ownership/authentication, persistence and accounting architecture. It does not run migrations, recovery writes, schedulers or provider calls at startup.

It is pinned to:

- API `127.0.0.1:55682`.
- Database `supa_m10a_a13db38ce0a5_test`, socket `/tmp/supa-m10a-pg-a13db38ce0a5/s`, port 55681.
- Data directory `/tmp/supa-m10a-pg-a13db38ce0a5/data` and system identifier `7703509812952711377`.
- Existing paused test agent `d4849ce9-5cef-4b76-9390-64ef6d83a99d`.
- Existing unpaid job `5cdfe440-dc36-4f94-8fc3-93b766ea31c5`.
- Existing synthetic owner `f84eeab5-ee3c-46c5-b04c-71303bff9044`.
- Existing independent budget `6ec8370e-fcd8-4675-84dd-1b754f6664c2`.

`GET live-work` confirms a real OpenAI adapter is configured with the exact model/project/budget and continuous execution disabled. **Configured does not mean authorized: an outer dispatch lock rejects every mutation by default, including paid-consent bodies.** Only the exact reviewed work endpoint can be admitted after separate paid CLI approval. It binds the fixed nonce, agent, job, input hash, goal, tools, model, memory policy, revision, token and cost ceilings. All other mutation routes remain unavailable. An atomic gate admits at most one dispatch per process; existing database and exclusive-journal fences protect across process restarts. No automatic retry is added.

The launcher `scripts/openai-work-api.py` uses a clean child environment containing only the necessary scoped provider/test credentials. It never imports the old database URLs, budgets, wallet secrets, Claude credentials or administrative keys. The connection URL contains no password; SQLx receives only the scoped password through the private child environment. The original private OpenAI execution gates are not changed. API tokens and database password remain in an ignored 0600 runtime file under a 0700 evidence directory; nothing secret is printed or committed. This is disposable service/test authentication, **not real owner onboarding or a primary ownership grant**.

### Primary isolation and database permissions

The API runs under Bubblewrap with separate user/PID namespaces and no capabilities. Only system libraries, the exact executable, DNS/certificate files and the approved disposable Unix-socket directory are mounted. The socket mount is read-only. The repository, private dotenv files, primary data/backups, other PostgreSQL sockets and host home directory are absent. No source/configuration directory is mounted into the process.

The API has a separate `supa_openai_eval` database login, with no superuser, create-database, create-role, replication or bypass-RLS privileges. Test-cluster HBA allows that login only to the approved database and rejects other databases. The login expires with the scoped credentials. It can read synthetic test records and write inference-session/audit and committed-budget fields. ID-column update grants on agent/job tables are needed for PostgreSQL row locking; the API exposes no identity-changing operations. It cannot alter agent data, job input or budget ceilings, delete budgets, create persistent tables in public, or read arbitrary server files. No primary database role or configuration is touched.

Networking remains shared for loopback HTTP and any future separately approved OpenAI request; this is not a claim of network egress filtering. Fixed connection construction, isolated sockets/filesystem, scoped login and route/tool restrictions enforce application database isolation. No arbitrary target or shell tool is exposed.

## Frozen scope and accounting

The exact evaluation scope remains:

`0453528bc228c83e6137eabd53ebeb4a7b98a198e7e6937e397ece240a51a6f0`

The plan `config/openai-work-evaluation-current.json` is byte-identical. Task, evidence, agent/job IDs, nonce, database, project, independent budget and expiration were not renewed or replaced. Supplied-document input hash remains:

`613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e`

- Model: `gpt-6-luna`.
- Independent budget: **$0.005**, committed **$0**.
- Maximum generation requests: **one**, no automatic retries.
- Per-request ceiling: **$0.004**.
- Token ceilings: **24,192 input / 1,024 output**.
- Conservative reservation if separately authorized: **$0.003536**, currently not reserved or charged.
- Job expiration: **3 November 2026, 09:52:29.570357 UTC / 10:52:29.570357 WAT**.

Over 23 hours remained at final validation. Check again immediately before execution. API/test-login credentials expire **2 November 2026, 12:28:59 WAT**; the API exits automatically at its deadline. A later approved launcher run may create new short-lived test credentials while the same job remains valid; it never extends the job or changes its scope.

The historical Claude **$0.003536** reservation remains unknown and untouched. The historical OpenAI **$0.004048** hold remains quarantined. The new proposed reservation is an independent calculation, not reuse or reconciliation of either hold. All 26 protected primary/backup/accounting hashes and historical plans/journals remain unchanged. No previous evaluation database was started or modified.

## Actual non-paid validation

The original passive rehearsal passed against the existing job before configuration. The scoped live-provider API then passed six grouped checks over actual local HTTP/SQL:

1. Exact real-adapter policy/project/job visible without any provider request.
2. Unauthenticated access, public-service identity spoofing and other-agent routes rejected.
3. Dispatch locked and all other mutation routes rejected. Live-key HTTP tests used malformed/non-consenting bodies; exact paid-consent locking was tested separately with an in-process stub and no provider.
4. Restricted database role cannot change inputs or budget ceilings, delete budgets, read files or connect to another database; required row locks work with rollback only.
5. Sandbox hides primary files, private configuration, database data directories and other cluster sockets.
6. Same frozen scope, valid expiration, zero-spent budget, all public-table row fingerprints and protected hashes preserved.

The clean API shutdown/restart rehearsal passed. During initial testing, stopping the Bubblewrap supervisor left its inner API process alive. Only the positively identified locked test child was stopped; there were no provider requests. The launcher now runs the API directly as the namespace process, records Bubblewrap's host child PID and its start-time identity, and signals only that verified child. PID-reuse and child-versus-supervisor shutdown regression tests were added. No PID guessing or unrelated-process termination is used by the final launcher.

| Final check | Result |
| --- | --- |
| Rust workspace, all targets, locked/offline | 95 passed, 0 failed; 98 opt-in ignored |
| Dedicated dispatch/scope guard tests | 3 passed, in-process stub only |
| Mock-provider suite | 44 passed |
| Disposable controlled/live-work PostgreSQL tests | 13 passed |
| Disposable accounting-quarantine test | 1 passed |
| Separate disposable crash/restart | 9 session/budget fingerprints preserved |
| Launcher unit tests | 8 passed |
| JavaScript preflight tests | 4 passed |
| Existing evaluation-runner tests | 9 passed |
| Claude credential regressions | 6 passed |
| Formatting / Clippy all targets, warnings denied | Passed |
| Secret scanner self-tests | 15 passed |

Ignored infrastructure tests are not claimed as passed. All inference responses in automated tests were mocked. No frontend changes or browser evidence are claimed. Sanitized evidence remains local under ignored `artifacts/openai-final-readiness/`; private runtime material and database files are untracked.

## Remaining authorization and exact command

The scoped API is running **with paid dispatch locked**. Primary PostgreSQL is stopped; the prepared disposable PostgreSQL cluster is running. The agent remains paused, its job unpaid, and no budget reservation or inference session exists.

Remaining requirements:

1. Confirm the exact project's spending restrictions permit the request and shared rate-limit capacity is available.
2. Give fresh explicit approval for the unchanged scope, GPT-6 Luna, $0.005 independent budget, one generation and $0.004 request ceiling.
3. Recheck scope, expiry and zero-spent state immediately before dispatch. A consumed journal/session or an expired job stops execution; no automatic retries or renewal.

Protected execution command, **not run**:

```bash
python3 scripts/openai-work-api.py execute \
  --authorize-paid \
  --scope-sha256 0453528bc228c83e6137eabd53ebeb4a7b98a198e7e6937e397ece240a51a6f0
```

After fresh approval, this wrapper checks for an existing attempt, restarts only the scoped API with the one-shot gate enabled, loads the scoped test-owner token privately and invokes the already reviewed command:

```bash
python3 scripts/live-work-evaluation.py \
  --plan config/openai-work-evaluation-current.json \
  --run --authorize-paid \
  --scope-sha256 0453528bc228c83e6137eabd53ebeb4a7b98a198e7e6937e397ece240a51a6f0 \
  --model gpt-6-luna --max-cost-microusd 4000
```

It shuts the API down afterward, including on a failed or ambiguous outcome. It does not automatically repeat a request. These commands grant no authority until the operator separately approves their paid execution. Stopping the locked API without spending is available through `python3 scripts/openai-work-api.py stop`; this does not stop or alter the primary database.
