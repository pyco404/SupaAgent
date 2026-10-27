# Local development

Rust 1.93 or later is required. `Cargo.lock` is committed as a project artifact for reproducible dependency resolution when Git is available. Run commands from the repository root. The API binary is `supa-api`.

## PostgreSQL on Ubuntu

The default PostgreSQL service was unavailable in the initial workspace. PostgreSQL 16 server binaries were already installed; Docker was absent. For an Ubuntu machine that lacks PostgreSQL:

```bash
sudo apt update
sudo apt install postgresql postgresql-contrib libpq-dev
sudo systemctl enable --now postgresql
pg_isready
sudo -u postgres createuser --pwprompt supa
sudo -u postgres createdb --owner=supa supa_agent
sudo -u postgres createdb --owner=supa supa_agent_test
```

Choose a local development password. Use PostgreSQL's local authentication configuration rather than weakening global authentication. Export `DATABASE_URL` with the development database and `TEST_DATABASE_URL` with the separate test database. The app role needs migration DDL rights in its local database. PostgreSQL server major versions can vary by Ubuntu release; version 16 is the validated baseline.

## Isolated database without system changes

With PostgreSQL 16 binaries already installed, a non-root user can initialize a new disposable cluster. Use a fresh directory; never reinitialize an existing cluster:

```bash
/usr/lib/postgresql/16/bin/initdb -D /tmp/supa-agent-pg --encoding=UTF8 --no-locale -A trust
/usr/lib/postgresql/16/bin/pg_ctl -D /tmp/supa-agent-pg \
  -l /tmp/supa-agent-pg.log -o '-k /tmp -p 55432 -h 127.0.0.1' start
createdb -h /tmp -p 55432 -E UTF8 -T template0 supa_agent_test
export TEST_DATABASE_URL="postgresql://$(id -un)@127.0.0.1:55432/supa_agent_test"
cargo test --workspace --locked -- --ignored
/usr/lib/postgresql/16/bin/pg_ctl -D /tmp/supa-agent-pg stop
```

Trust authentication is suitable only for this disposable, same-machine test instance; local users can access it. It never listens on a public interface. The sandbox may require permission to create or connect to local sockets. The test environment used `/tmp/supa-agent-pg`, localhost port 55432, and a UTF8 test database. No system service was installed or reconfigured.

## OpenAI opt-in

The default provider is `offline`. To deliberately permit real OpenAI API usage in local development:

```bash
export SUPA_PROVIDER=openai
export SUPA_ALLOW_OPENAI=true
read -rs -p 'OpenAI API key: ' OPENAI_API_KEY
export OPENAI_API_KEY
export OPENAI_MODEL='your-account-supported-model'
# Explicitly approve a small local evaluation budget; retain its UUID across restarts.
export SUPA_LIVE_BUDGET_ID='operator-selected-valid-uuid'
export SUPA_LIVE_BUDGET_MICROUSD=100000
# Fill numeric rates after verifying the selected model's current pricing.
export SUPA_INPUT_RATE_MICROUSD_PER_MILLION='approved-numeric-rate'
export SUPA_CACHED_RATE_MICROUSD_PER_MILLION='approved-numeric-rate'
export SUPA_OUTPUT_RATE_MICROUSD_PER_MILLION='approved-numeric-rate'
export SUPA_MAX_OUTPUT_TOKENS=1024
cargo run -p supa-api
```

Supply a model supporting structured JSON-schema outputs on the Responses API; no model name or pricing is hardcoded. Keep test runs offline. The implementation uses POST `/v1/responses`, `store: false`, a bounded `max_output_tokens`, and strict structured outputs, following the [official OpenAI Responses API reference](https://developers.openai.com/api/reference/typescript/resources/responses/methods/create). Refused, incomplete, malformed and oversized responses fail safely. No production tools or built-in network tools are passed to OpenAI. No live paid call was made during milestone validation.

This opt-in can incur real provider charges, even though treasury metrics are hypothetical. Simulated balances do not enforce provider billing limits. A separate durable estimated budget reserves worst-case token costs using your configured rates; it is not an authoritative invoice. Configure account-side usage restrictions before enabling and activate agents deliberately. Only explicit rate-limit rejections may retry within configured budgets; uncertain calls never retry.

## Commands and troubleshooting

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --locked -- --ignored
cargo run -p supa-api
```

`invalid_input` at startup: check a numeric loopback bind address and an API token of at least 32 characters; ensure provider selection is valid. `database_failed`: verify availability and database permissions, then inspect PostgreSQL's local administrative logs; the service intentionally does not print connection strings or SQL errors. `unauthorized`: supply `Authorization: Bearer …`. `agent_inactive`: explicitly activate an eligible funded agent. `cycle_busy`: a cycle or unexpired lease exists, or scheduled interval has not elapsed. `insufficient_simulated_balance`: credit simulated funding and explicitly reactivate the suspended agent. `provider_failed`: inspect account/model configuration without logging credentials; outputs must conform to the strict schema. Lease recovery can take up to 240 seconds after an abrupt stop.

Send Ctrl-C for shutdown. An interrupted in-flight cycle is conservatively recovered after lease expiry on the next startup/tick. Never delete or overwrite Git metadata to fix this workspace. Its original `.git` placeholder is read-only and not a valid repository. Once the owner provides functional metadata, use `main` and Conventional Commits such as `feat(domain): add agent lifecycle and simulated treasury rules` and `feat(runtime): persist bounded reasoning cycles and recovery`. Do not force-push.

## Controlled demonstration artifacts

`cargo build -p supa-runtime --example demo --locked` builds a strictly offline demonstration harness. It uses `TEST_DATABASE_URL`, never OpenAI credentials. Stages are `init` (five timed cycles), `continue` (five more in a new process), `crash` (reserve unfinished work and wait), `recover-test` (explicit test-only lease expiry), `drafts` (five cycles using a run-specific recorded output artifact), and `verify-paused`. `scripts/demo-crash.py` starts/kills only the child demo worker after its durable marker exists.

The saved run is under `artifacts/demo`; preserve it. `init` refuses to overwrite an existing identity file. To create a separate run, invoke the absolute built binary from a fresh scratch working directory; its relative artifact paths stay inside that directory. New event IDs require newly generated event-grounded draft outputs at `artifacts/demo/generated-drafts.json`. The harness contains no prewritten tweet text. Without live credentials, the current run's prose was composed by the coding assistant after inspecting the snapshots, then replayed. Deterministic tests do not establish live creative ability.

`python3 scripts/verify-demo.py` validates the supplied run without network or database access. `scripts/smoke.py` uses `TEST_DATABASE_URL` and the built `supa-api` to test the actual local server scheduler and abrupt restart; it always forces offline mode and stops its child process afterward. Both use disposable data only. The controlled demo leaves Supa paused.

## Milestone 2A updates

The current supported-model defaults are Luna for routine work and drafts, with explicitly selected Sol for complex planning. Pricing now defaults to a dated verified model table; optional overrides must be at least that baseline, including cache writes. The older setup example above is illustrative; use [Milestone 2A's current exact setup and authorization procedure](milestone-2a.md) for a new live evaluation. The evaluator uses `SUPA_EVAL_DATABASE_URL`, not the server's `DATABASE_URL`, and requires explicit CLI paid authorization.

## Configured dedicated evaluation database

The existing isolated PostgreSQL 16.15 cluster is available at `127.0.0.1:55432`, with data in `/tmp/supa-agent-pg`. The dedicated database is now `supa_eval`, owned by login role `supa_eval`. This role has no superuser, role-creation, database-creation, replication or RLS-bypass privileges. Read-only privilege checks confirmed that it cannot create schemas or write relations in `postgres`, `supa_agent_test` or `supa_agent_demo`. Existing databases, roles and credentials were preserved. A create/insert probe in the new evaluation database was rolled back completely.

The existing cluster uses loopback-only development trust authentication, so this connection needs no database password. Authentication was not weakened or otherwise changed. Trust authentication assumes local users are trusted; this setup is for local development, not production or untrusted shared machines.

The evaluator automatically loads the project's private `.env.eval` before starting Tokio worker threads. That file is mode 0600 and matched by the repository's existing ignore rules. It stores the dedicated database URL and keeps `SUPA_ALLOW_OPENAI=false`. No OpenAI key was available during setup or written to it. Add your key privately to that file as `OPENAI_API_KEY=...`, or export it in your own terminal; never paste it into chat or commit it. Existing exported values take precedence over file values.

The same command works in later terminal sessions without sourcing the file:

```bash
cargo run -p supa-runtime --example live_eval --locked -- --check
```

`--check` reads configuration only and never makes paid requests or changes database data. Connectivity was verified separately using the restricted role. Missing key, opt-in, budget ID/ceiling and explicit paid authorization continue to block paid execution. Evaluation migrations run automatically before the first authorized evaluation, using this restricted role in its own database.

The loader accepts only evaluation-related keys, rejects symbolic links and files readable by other users, and sanitizes parsing errors. `SUPA_EVAL_ENV_FILE` may select another private file; exporting an empty value disables file loading, which is used to isolate safety tests from real operator secrets. The regular API server's environment behavior is unchanged.

## Generation-free authentication diagnostics

Use `cargo run -p supa-runtime --example auth_probe --locked -- --check` to identify the selected key source and shell/file override status without displaying credentials or sending network requests. A separate explicitly authorized metadata GET is documented in [OpenAI authentication diagnosis](openai-authentication.md). It never launches inference or modifies evaluation budgets.
