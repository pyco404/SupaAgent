# Local agent launch experience

The approved terminal website now supports creating a named disposable test agent, saving its personality and goal, assigning a simulated seed, and using the existing Rust finite scheduler. No Rust runtime, database migration or provider implementation was replaced or rebuilt for this work.

## Try it

The demonstrated local services are running at **http://127.0.0.1:3200**. This uses the existing approved terminal host (`reference-web/server.mjs`) and existing Rust API on loopback port 8080. The Next.js frontend also passed its build/type/lint checks; the demonstrated launch journey uses the terminal host, not a new Next.js dashboard.

1. Open `/operator` and sign in as the disposable `launch-owner`. Obtain its password locally from `accounts[0].password` in owner-only, Git-ignored `artifacts/local-launch/runtime.json`. Do not share that file or paste its contents into chat. The distinct API/service/database credentials remain server-side.
2. Open **Agents → Create a test agent**. Enter name, personality and goal. Budget is in simulated microUSD: `20000` means **0.020 simulated USD**, not real money. Confirm creation, then select **Create authorized test agent**. Creation leaves it inactive.
3. In **Intelligence**, open **Agent lifecycle and bounded scheduling**. Select **Start scheduling**, approve the saved goal and set finite limits. The demonstrated limits were 6 cycles, a 10-second interval, 6,600 simulated microUSD expense ceiling and 3,600-second expiry. Add a reason and fresh consent, then apply.
4. Watch saved outcomes in Intelligence. Idle polling runs every 15 seconds; **Reload saved schedule** retrieves records immediately. Polling does not overwrite focused or unfinished form fields or dispatch work. Agent selection survives a full browser reload. Worker health is an observation, not proof of intelligence or activity.
5. Open **Memory** for actual observations and hypotheses with event/cycle references. Intelligence shows goals, assessments, cycles, tool actions and activity. Economy shows the simulated ledger. Offline progress estimates do not prove task completion.
6. To pause or resume, reload the saved schedule, select the operation, enter a reason and provide fresh consent. Resume preserves the existing authorization, expiry, budget and cycle consumption. It cannot reset limits.
7. Pause before restarting the application, then run `node scripts/local-launch-demo.mjs restart`. Reload the website and inspect the same identity/history. Credentials and the database remain the same; no grant or scope is renewed automatically.
8. **Stop permanently** ends that test identity and prevents resume. Its records remain readable. Create a new test identity to try another run; the demonstrated Supa Observatory identity is already terminated.

Service commands:

```sh
node scripts/local-launch-demo.mjs status
node scripts/local-launch-demo.mjs restart
node scripts/local-launch-demo.mjs stop
```

`status` distinguishes ready, unavailable and stopped services and reports finite scope validity without printing credentials. `restart` checks the scope, cluster identity and binary before stopping healthy services; expired or nearly expired scopes fail closed without renewal.

`stop` stops only this demo's API and website; it preserves the private disposable PostgreSQL cluster/data. `init` creates a fresh isolated cluster only when no demo runtime file exists; it refuses to replace an existing environment. It does not import dotenv or historical database data. Do not delete or replace runtime/data files to bypass expiry. A fresh scope needs explicit local preparation. The present test credentials/scope expire **2026-11-03 02:03:46 UTC**; no automatic extension is implemented.

## What was demonstrated through REA and the actual backend

Agent: `78063318-1db3-4998-ac12-2c3aa1f739c8`, **Supa Observatory**.

Goal: “Inspect saved execution events, retain a verified observation and assess goal progress without claiming completion.”

Personality: “Curious, witty and cautious. Report only verified observations; admit uncertainty.”

Actual retained disposable outcomes:

| Measure | Result |
| --- | --- |
| Completed offline cycles | 6 / maximum 6 |
| Persistent memory records | 12 |
| Persisted tool actions | 17 |
| Agent activity events | 88 |
| Goals | 1, still open |
| Finite schedule authorizations | 1; resume did not mint another |
| Simulated seed | 20,000 microUSD |
| Simulated expenses | 6,600 microUSD |
| Simulated balance | 13,400 microUSD |
| Earned revenue / real payments | 0 / 0 |
| Paid inference requests | 0 |
| Final lifecycle | terminated; scheduling disabled |

REA performed the actual form creation, start, pause, resume, memory navigation and permanent-stop actions. The API/frontend were restarted while paused after cycle 2. Before/after records matched for the schedule, cycles, memories, goals, actions, activity and financial amounts. The wall-clock-derived runway estimate is intentionally excluded from equality; its elapsed-time calculation can change without any accounting mutation.

The scheduler subsequently reached its six-cycle ceiling and reported `schedule_limit_reached`. Permanent stop was then recorded through the website. Subsequent API checks rejected resume and observed no additional cycles or expenses. Model-generated goal assessments were not treated as verified completion or revenue.

## Implementation and safeguards

- `reference-web/launch.mjs` is server-only orchestration of existing Rust create, add-goal and simulated-seed endpoints. It requires private, finite disposable scope metadata and an authenticated owner; validates the exact backend database/absent epoch; enforces same-origin JSON consent, fixed nonfinancial capabilities and a bounded simulated seed. It never starts the identity or calls an LLM.
- Private request journals reserve each creation request before dispatch. Duplicate IDs are rejected. Partial/uncertain creation returns the known identity/stage for inspection without automatic replay. Funding uses the same stable idempotency key. No refund/revenue path is added.
- `reference-web/backend.mjs` and `server.mjs` expose only the exact local launch route. The server-only module and journal are not publicly served. Staging launch is denied. Prepared/read-only epoch views skip the launch probe entirely.
- Agents gets a terminal-style creation form and discoverable operator sign-in. Intelligence uses the existing schedule controls, shows actual personality and clearly labels the disposable offline mode. Existing layouts, logo, carousel, intro, animations and all ten routes remain.
- Form editing/pending actions are protected from automatic record refresh. The small CSS addition uses the existing terminal palette, typography and 44px form controls.
- `scripts/local-launch-demo.mjs` creates a new socket-only PostgreSQL cluster/database with synthetic accounts, no historical data, no dotenv/provider keys and paid/wallet controls disabled. It reuses the prebuilt Rust API, verifies its hash and the cluster identity, and enables its offline scheduler only in this explicitly authorized demo. Scheduling still requires each agent's finite consent.
- `scripts/verify-local-launch.mjs` checks genuine stored outcomes and scoped owner/operator/viewer behavior. Only synthetic operator/viewer ACLs on this disposable agent were granted for these checks. An unrelated owner cannot read private memory; a viewer cannot spoof owner headers or mutate; an operator cannot authorize a new schedule. No real owner/custodian credentials or grants were created.

## Tests and browser evidence

Passed:

- 41 existing domain unit tests and 4 existing API unit tests, using prebuilt test executables; no Rust rebuild.
- New launch checks: origin/local access, authentication/roles, epoch/database fence, strict request/budget validation, inactive creation, duplicate prevention, partial outcomes, and absence of schedule/provider dispatch from creation.
- Four existing adapter suites: autonomy, backend security, economy and epoch behavior. The initial epoch regression exposed unnecessary probes; the fix preserves the original prepared/read-only no-controls boundary, and the rerun passed.
- 34 existing desktop/mobile Playwright platform and scheduling regressions.
- Actual disposable-system checks: six cycles, persistence, authorization ceilings, roles/spoofing, open goal, zero revenue, paid controls disabled and terminal stop fencing.
- Frontend `npm run typecheck`, `npm run lint`, `npm run build`; JavaScript syntax checks; secrets scanner/self-tests and complete one-file history audit.

Actual REA evidence identifiers:

| Journey step | Evidence |
| --- | --- |
| Create through Agents | `ev_0b8f7fb90f0c04f74e71bd4db81248f047ffbd59bf5ec0843a76e0b4a9d9b5c8` |
| Authorize/start finite session | `ev_eb3816ae30a728d4065b6139feff0d9592d59b2b1e47d21b71fafcb5f7f90630` |
| Pause | `ev_5389307cd7965b5bacfa8a8db315d82f66d99d287bb5bc20f90ed55eb77ef669` |
| Resume after restart verification | `ev_67515d62e3af7e3bc7475d0abfc89e831dc539dd973ca7fbc06f00a2ff4e7d64` |
| Read stored memories | `ev_36f8d47e755a1f8732fb45f3eb55ce47794f1aba11890a7ebd010f091dd2b9fa` |
| Permanent stop | `ev_d97e7dd717604a08fd1b208f162720d32c58f51be19ddaee744d5acc91806996` |
| Reload terminated state | `ev_912f87e19280e4317fe28a7ffe42910c22fe5811e598ef12e621742a0f9fb51b` |
| Verified mobile/reduced motion | `ev_a9343823a85cabbd7e60f0f15b02fceb047f5bbe58756055cf7158e41df324b5` |

Local screenshots and evidence are ignored/unpushed under `artifacts/local-launch/`: `01-created.png`, `02-running.png`, `03-paused.png`, `04-resumed.png`, `05-memory.png`, `06-stopped.png`, `08-final-history.png`, `10-mobile-reduced-motion.png`, system/restart validation JSON and viewport metrics. Final mobile metrics: viewport and document width both 390px, keyboard focus name→personality correct, reduced motion true and animation state `still`. An earlier CDP viewport capture cropped the desktop layout; it is not used as responsive proof. Explicit viewport/media emulation plus passive REA capture produced the final verified mobile evidence. A memory locator initially matched multiple valid records; its corrected scenario completed. No application page errors appeared in the successful recorded scenarios.

## Limits and protected state

This is a finite **offline development** experience. The deterministic provider reads memories, stores evidence-linked observations and suggests goal progress; it does not perform independent research or establish arbitrary customer-goal completion. There are no fabricated payments or achievements. Simulated inference expenses are not provider charges. Paid GPT/Claude, wallet actions, trading, token launches and publishing are disabled. Real onboarding, primary activation and Railway deployment are outside this demonstration.

The private database is disposable and stored under `/tmp/supa-launch-7e902e18357d`; it survives application restart but is not durable production infrastructure or a disk-loss backup. The services use a dedicated local PostgreSQL role in that isolated cluster, with no route to the protected primary through their configured connection. HTTP test credentials expire; the script does not silently renew them. Disk headroom remains limited; no cleanup or compilation-cache deletion occurred.

The protected primary stayed stopped. All 29 protected/original evidence file hashes matched, including original evaluation output/journal, dormant real-owner configuration, primary control/configuration files and historical accounting evidence. Historical review remains pending with its original ownership; $0.004048 and $0.003536 holds were not changed. No primary or historical database was migrated, no paid request or deployment ran, and nothing was pushed. The four unused logo variants were subsequently confirmed for deletion by the operator; see [local MVP release readiness](local-mvp-release-readiness.md) for the stabilization rerun and Git status. Every changed file is committed separately on main.
