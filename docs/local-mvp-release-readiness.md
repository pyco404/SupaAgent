# Local MVP release readiness

Validated 2026-11-02. The existing local launch experience is usable; private Railway deployment is **not yet cleared**. No deployment, paid provider request, primary startup/migration or historical review delegation was performed.

## Practical stabilization

The approved terminal design, ten clean routes, Rust editor animation and displayed pixel-robot assets are preserved. No Rust runtime, provider contract or database schema was changed.

Fixed reproduced defects:

- Selected agent identity was lost on full reload. The browser now remembers an optional agent UUID; server authorization still decides which records are accessible.
- A background lifecycle refresh could discard a blurred, unfinished launch form. Dirty forms now block background replacement, including after asynchronous retrieval. Explicit navigation/reload still discards a draft.
- Definite permission/expiry/conflict failures previously appeared as ambiguous outcomes. Launch/lifecycle controls now give status-specific recovery instructions while preserving no-retry behavior. Unknown outcomes still require inspecting saved records.
- Simulated seed values now include a decimal USD equivalent and explain that the finite session expense ceiling is separate. Lifecycle descriptions explain created, paused, active, suspended and permanently stopped states without inferring execution from authorization.
- Service status now distinguishes ready/unavailable/stopped and scope validity. Restart verifies finite scope, socket-only cluster identity and reviewed binary **before** stopping healthy services. Nearly expired scopes and unsafe runtime paths fail closed, without renewal.
- Container packaging omitted the server-only launch module; the isolated build context also omitted the not-found page. Dockerfile, allowlist and CI context inventory now agree, with a regression check.

The operator confirmed deletion of four unused `web/public/supa-agent-logo-{dark-background,transparent}.{png,svg}` variants. Each deletion has its own commit. The actual terminal logo assets are retained.

## Actual website and PostgreSQL demonstration

The original desktop journey remains preserved: **Supa Observatory**, `78063318-1db3-4998-ac12-2c3aa1f739c8`, six completed cycles, twelve memories, seventeen actions and permanent stop. Its verification was rerun successfully; no new execution allowance was granted.

A second complete journey used REA with the real disposable API/database on mobile:

| Step | Observed outcome |
| --- | --- |
| Create through Agents | **Supa MVP Mobile**, `1e257201-c430-40ec-b40d-a2eb44c21d7a`; saved personality and evidence-only goal; inactive identity |
| Simulated seed | 10,000 microUSD; no real funds |
| Explicit session consent | Six-cycle maximum, ten-second interval, 6,600 simulated microUSD ceiling, 600-second authorization |
| Pause | One completed cycle, two memories; scheduling disabled |
| Stop/restart API and frontend | Eleven record groups, including identity, schedule and ledger, matched exactly before/after; derived runway excluded |
| Reload/resume | Correct agent remained selected; original authorization and limits reused |
| Inspect Memory/Intelligence | Genuine saved observations, cycles, tools and goal history |
| Permanent stop | Five completed cycles, ten memories, fourteen actions, seventy-four activity events, one open goal |
| Final accounting | 5,500 simulated microUSD expenses; balance 4,500; zero revenue and paid requests |
| Stop fencing | Subsequent readback found no new cycles, memories, actions or accounting changes |

We intentionally stopped before the six-cycle ceiling; five is the measured result, not a completion claim. Offline observations/goal assessments do not establish arbitrary customer-task completion.

Original desktop REA identifiers and screenshots: [launch guide](local-agent-launch.md). New REA evidence:

| Action | Evidence ID |
| --- | --- |
| Mobile creation | `ev_09412dff8331feebb74f896d7dbb0c5779294ed76c6878692c78b3a1afc47250` |
| Finite start | `ev_f6e11af6a93f37277492ad131bce15454023553660cdaef0fc33acd8760ea5a3` |
| Pause | `ev_500c3675080de3cd1482afc0defbd042d1b33b152ebbf154a8aa7d55c68772d5` |
| Resume after restart | `ev_ad2db415522d933a90939892111d6d5a405b896dacf92ac4ba024ddfd89ba3dd` |
| Read memories | `ev_56ab18739348680d2752c5851ef100449c3d905c7ddf2699a5cb3286632c34ec` |
| Permanent stop | `ev_fee29361324629b8da798206e5f5916e9bdf9e1fd0e41a1c40ca23614c360b30` |
| Verified 390px viewport | `ev_a5871abb2118911cb2fcc4d255487d26c9277e19e362b7dc6f6751c77fc4d691` |
| Verified 1440px desktop | `ev_1b59336687ea3057bda6999ba2c97c62a721a29c85ea802421a33f1485f829f2` |

Screenshots, viewport measurements, paused/restart/final records and logs remain Git-ignored under `artifacts/local-mvp/`. Use `mobile-verified.png` and `desktop-verified.png` for viewport proof. Earlier attached-CDP screenshots can retain the previous CSS viewport despite a larger capture area; those are not desktop layout evidence. Final measurements confirmed viewport/document widths of 390/390 and 1440/1440, reduced motion enabled, animation state `still`, and keyboard focus moving name→personality. Successful REA scenarios reported no application page errors; this is not an exhaustive accessibility certification.

## Local operating checklist

| Area | Status / procedure |
| --- | --- |
| Open website | `http://127.0.0.1:3200/operator`; use the existing disposable owner account locally, then `/agents` |
| Authentication | Owner-only `artifacts/local-launch/runtime.json`, directory 0700/file 0600. Never print/share/commit its contents. Browser Basic credentials map to distinct server-only API tokens |
| Startup | `node scripts/local-launch-demo.mjs restart`; validates current cluster/binary and finite scope; does not create a replacement environment |
| Health | `node scripts/local-launch-demo.mjs status`; API `/ready`, frontend `/healthz` |
| Shutdown | Pause/stop any authorized agent first; `node scripts/local-launch-demo.mjs stop` stops API/frontend only and retains disposable PostgreSQL data |
| Lifecycle | Creating an agent is inactive; fresh owner consent authorizes finite offline limits; pause/resume preserve limits; stop is permanent |
| Persistence | Actual application restart verified; selected identity and saved database records persist |
| Permissions | Owner controls new authorizations; scoped operator/viewer access remains enforced; unrelated owner and spoofed viewer denied |
| Financial isolation | Seed/expenses are simulated; no earned revenue, wallet action or paid provider dispatch; historical holds untouched |
| Logs/errors | Private service log in `artifacts/local-launch/services.log`; client messages contain safe status/request identifiers. Inspect partial outcomes before new requests; never replay blindly |
| Recovery | Current test PostgreSQL data lives under `/tmp/supa-launch-7e902e18357d`; survives application restart, **not** OS cleanup/disk loss. Do not treat it as durable production storage or independent backup |
| Expiration | Existing demo scope/credentials expire 2026-11-03 02:03:46 UTC (03:03:46 WAT). No automatic extension; fresh preparation requires explicit authorization |

Local launch configuration is produced by the existing initializer, not copied from dotenv:

| Service | Required private/configuration inputs |
| --- | --- |
| Rust API | `DATABASE_URL`, `PGPASSWORD`, `SUPA_BIND=127.0.0.1:8080`, distinct `SUPA_API_TOKEN`, synthetic `SUPA_OPERATOR_CREDENTIALS` and finite `SUPA_OPERATOR_SESSION_DEADLINES` |
| Terminal host | `PORT=3200`, `SUPA_BACKEND_URL`, server-only service token, `SUPA_WEB_OPERATORS`, private `SUPA_LOCAL_LAUNCH_SCOPE`, empty `SUPA_SITE_ENV_FILE` |
| Offline demo only | `SUPA_PROVIDER=offline`; API scheduler enabled only for this authorized disposable demonstration and finite per-agent consent |
| Disabled | OpenAI, Anthropic, controlled work/inference, standalone worker and wallet controls; no provider keys inherited |

`init` refuses an existing runtime file. Do not delete it, change expiry or replace the cluster to bypass finite authorization. No real owner/custodian provisioning is implied by the synthetic `launch-owner` test identity.

## Validation results

- Six new desktop/mobile browser regressions passed. All three tests first reproduced the defects before correction: reload selection, blurred draft refresh and definite denial handling.
- Thirty-four existing desktop/mobile platform/scheduling regressions passed: ten sections, data states, terminal interactions, picker/menu keyboard behavior, consent, limits, viewer health and permanent termination.
- Three service-scope/health tests and the network-free launch security/orchestration checks passed.
- Four existing backend/autonomy/economy/epoch adapter suites passed.
- Forty-one domain and four API Rust unit tests passed using existing test executables; Rust source was unchanged. No full rebuild/Clippy pass was attempted with limited disk capacity.
- Actual disposable PostgreSQL/API role, spoofing, execution, accounting and restart checks passed. Primary migrations, historical delegation and real credential provisioning were not run.
- Frontend typecheck, lint and production Next.js build passed. An initial restricted build failed while parsing child-process TypeScript output; one subsequent run was interrupted during trace collection. The final normal-access build completed successfully. An interrupted browser regression left fixture processes; only those verified fixtures were stopped before the clean thirty-four-test rerun.
- Seven staging security/context tests and eight Railway graph/entrypoint tests passed. These validate source/context/configuration, **not** container execution or vulnerability-free images.
- Secrets gate/self-tests passed; each commit runs the full gate again. Complete one-file main history audit is required after the final documentation commits. No third-party scanner is installed; targeted scanning cannot prove absence of every possible disclosure.

Protected integrity checks: twenty-nine protected/original file hashes unchanged; primary remained stopped. A fresh read-only before/after fingerprint of all thirty-eight retained evaluation tables matched. The original output/request journal remains unchanged. Historical review is still pending under its original ownership; the $0.004048 and $0.003536 holds were neither released nor settled. GPT-6 Luna remains the configured preferred provider and Claude optional; this demo runs offline and does not change either provider implementation.

## Minimum private Railway work remaining

The existing provisioning report identifies private **Supa Agent** project `99648d2a-69a1-4d8c-98d0-7f2a26543c9c`, staging `ff0b2092-e42b-4a89-ae25-441ab95d92d8`, in workspace `e319f68a-9897-49ed-b119-743f1f846aa0`. The Amsterdam backup bucket ID is `76923b39-b3a7-4390-8a0b-783d8a8113e9`. This milestone inspected local configuration and that report; it did not retrieve bucket credentials or claim a fresh live inventory check.

The graph contains PostgreSQL 18 with persistent storage, API:8080, disabled worker:8081 and frontend:3000. Backend traffic is private; domains/proxies are absent. The frontend has a whole-site staging gate plus separate operator mapping. Runtime/migration privileges, exact HTTPS origin and newly generated staging secrets need separate review. Never import local test identities, runtime files, provider keys, wallet material, historical simulated balances or private evidence.

Important deployment gaps:

1. The graph's authoring label is `Supa Agent Staging`; reconcile/link it explicitly to the **existing** approved project/environment and bucket before any apply. Do not blindly create a second project or delete existing resources.
2. The local creation endpoint deliberately rejects staging, loopback/database fences are local, and Next.js does not expose its creation POST. Deploying current containers alone will not reproduce the launch wizard. Add a separately reviewed staging-scoped authenticated creation path with durable journals and fresh isolated database state, preserving local/primary fences.
3. API scheduler and standalone worker remain disabled in staging. A separately approved finite offline demonstration needs an explicit execution topology/authorization; do not silently enable worker scheduling during deployment.
4. Actual CI container builds, startup/health/shutdown, non-root execution and vulnerability scans/digests remain required. Docker is unavailable locally; none was installed. The existing manual isolated CI workflow can supply evidence after separately approved synchronization/run.
5. Backup bucket provisioning is not a recoverable database backup. Independently verified encryption/signing fingerprints and recovery copies remain pending; export/upload/restore each need approval. Current free disk is only approximately 0.6 GiB, so approved reproducible-artifact cleanup/capacity recovery is needed before builds or backup restoration. No further cleanup was performed.

Git fetch found the official remote an ancestor of local main, with no divergence. No push was attempted. The four confirmed logo removals and each changed source/test/report file are committed separately; generated evidence and private files stay untracked/ignored.

## Prioritized next steps

1. Recover safe disk headroom through separately approved reproducible-artifact cleanup, preserving executables, databases and evidence.
2. Approve GitHub synchronization and isolated container CI; inspect real build/run/scan results before claiming staging readiness.
3. Implement/review staging-specific owner-scoped launch provisioning and durable recovery, without lifting the existing local/primary fences.
4. Complete independent backup key verification and an approved encrypted restore drill.
5. Approve the exact existing-project staging change set, fresh database/credentials and one finite offline session separately; deploy only after those approvals.
