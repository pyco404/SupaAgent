//! Real restored development epochs with synthetic, independently bound SQL identities only.
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use sqlx::postgres::PgConnectOptions;
use std::{process::Command, sync::Arc};
use supa_domain::*;
use supa_persistence::Store;
use supa_providers::Offline;
use supa_runtime::Runtime;
use uuid::Uuid;

struct Fixture {
    root: Store,
    service: Store,
    authority: Store,
    epoch: Uuid,
    namespace: Uuid,
    owner: Uuid,
    credential: Uuid,
    agent: Uuid,
    database: String,
}
impl Fixture {
    async fn new() -> Self {
        let base = std::env::var("TEST_DATABASE_URL").expect("disposable database required");
        let opts: PgConnectOptions = base.parse().unwrap();
        assert!(opts.get_database().unwrap().ends_with("_test"));
        assert!(
            opts.get_socket()
                .unwrap()
                .to_string_lossy()
                .contains("/postgres-data/")
        );
        let admin = Store::connect(&base).await.unwrap();
        let suffix = Uuid::new_v4().simple().to_string();
        let database = format!("supa_writable_{suffix}_test");
        sqlx::query(&format!("CREATE DATABASE {database} TEMPLATE template0"))
            .execute(admin.pool())
            .await
            .unwrap();
        let backup = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../postgres-data/preflight/1d0e9efe-e66e-4e4f-afc4-478eee4f0833/database.dump",
        );
        assert!(
            Command::new("/usr/lib/postgresql/16/bin/pg_restore")
                .args([
                    "--exit-on-error",
                    "--single-transaction",
                    "--dbname",
                    &database
                ])
                .arg(backup)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("PGHOST", opts.get_socket().unwrap())
                .env("PGPORT", opts.get_port().to_string())
                .env("PGUSER", opts.get_username())
                .output()
                .unwrap()
                .status
                .success()
        );
        let rootopts = opts
            .clone()
            .database(&database)
            .options([("default_transaction_read_only", "off")]);
        let root = Store::connect(sqlx::ConnectOptions::to_url_lossy(&rootopts).as_str())
            .await
            .unwrap();
        root.migrate().await.unwrap();
        let epoch = root.application_epoch().await.unwrap().unwrap();
        assert_eq!(epoch.state, "prepared");
        assert_eq!(epoch.revision, 0);
        let owner = Uuid::new_v4();
        let custodian = Uuid::new_v4();
        let credential = Uuid::new_v4();
        let login = format!("op_{}", &suffix[..20]);
        let issuer = format!("cust_{}", &suffix[..20]);
        let password = Uuid::new_v4().simple().to_string();
        let issuer_password = Uuid::new_v4().simple().to_string();
        assert!(password != issuer_password);
        for role in [&login, &issuer] {
            let password = if role == &login {
                &password
            } else {
                &issuer_password
            };
            sqlx::query(&format!(
                "CREATE ROLE {role} LOGIN NOINHERIT PASSWORD '{password}'"
            ))
            .execute(root.pool())
            .await
            .unwrap();
            sqlx::raw_sql(&format!("GRANT CONNECT ON DATABASE {database} TO {role}; GRANT USAGE ON SCHEMA public,epoch_guard TO {role}; GRANT SELECT ON ALL TABLES IN SCHEMA public TO {role}")).execute(root.pool()).await.unwrap();
        }
        sqlx::raw_sql(&format!("GRANT INSERT,UPDATE,DELETE,TRUNCATE ON ALL TABLES IN SCHEMA public TO {login}; GRANT USAGE,SELECT ON ALL SEQUENCES IN SCHEMA public TO {login}; GRANT EXECUTE ON FUNCTION epoch_guard.approve_operation(uuid,jsonb),epoch_guard.revoke_operation(uuid),epoch_guard.revoke_credential(uuid) TO {issuer}")).execute(root.pool()).await.unwrap();
        sqlx::query("INSERT INTO epoch_guard.authorities VALUES($1,$2)")
            .bind(&issuer)
            .bind(custodian)
            .execute(root.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO epoch_guard.executors VALUES($1,$2,'owner')")
            .bind(&login)
            .bind(owner)
            .execute(root.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO epoch_guard.operational_principals VALUES($1,$2,$3,$4,clock_timestamp(),clock_timestamp()+interval '1 hour')").bind(credential).bind(owner).bind(&login).bind(custodian).execute(root.pool()).await.unwrap();
        sqlx::query(&format!(
            "ALTER DATABASE {database} SET default_transaction_read_only=on"
        ))
        .execute(root.pool())
        .await
        .unwrap();
        let connect = |role: &str| {
            sqlx::ConnectOptions::to_url_lossy(
                &opts
                    .clone()
                    .database(&database)
                    .username(role)
                    .password(if role == login {
                        &password
                    } else {
                        &issuer_password
                    }),
            )
            .to_string()
        };
        Self {
            root,
            service: Store::connect(&connect(&login)).await.unwrap(),
            authority: Store::connect(&connect(&issuer)).await.unwrap(),
            epoch: epoch.id,
            namespace: epoch.accounting_namespace,
            owner,
            credential,
            agent: Uuid::new_v4(),
            database,
        }
    }
    fn plan(&self, scope: &str) -> Value {
        json!({"version":1,"epoch_id":self.epoch,"accounting_namespace":self.namespace,"database":self.database,"principal_id":self.owner,"credential_id":self.credential,"agent_id":self.agent,"scope":scope,"expected_revision":0,"expires_at":(Utc::now()+Duration::minutes(5)).to_rfc3339(),"max_transactions":300,"max_cycles":2,"budget_microusd":2200,"seed_microusd":2200})
    }
    async fn issue(&self, p: Value) -> Uuid {
        let id = Uuid::new_v4();
        let mut tx = self.authority.pool().begin().await.unwrap();
        sqlx::query("SET TRANSACTION READ WRITE")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("SELECT epoch_guard.approve_operation($1,$2)")
            .bind(id)
            .bind(p)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        id
    }
    async fn admit(&self, scope: &str) -> (Uuid, Store) {
        let id = self.issue(self.plan(scope)).await;
        let s = self
            .service
            .admit_operation(id, self.epoch, self.namespace)
            .await
            .unwrap();
        (id, s)
    }
    async fn revoke(&self, id: Uuid) {
        let mut tx = self.authority.pool().begin().await.unwrap();
        sqlx::query("SET TRANSACTION READ WRITE")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("SELECT epoch_guard.revoke_operation($1)")
            .bind(id)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    async fn setup(&self) -> Goal {
        let (_, store) = self.admit("agent_creation").await;
        let mut agent = Agent::new(
            "Epoch fixture".into(),
            Personality::default(),
            CycleConfig::default(),
            [
                Capability::ReadMemory,
                Capability::Remember,
                Capability::EvaluateGoal,
            ]
            .into(),
        )
        .unwrap();
        agent.id = self.agent;
        store.create_owned(&agent, self.owner).await.unwrap();
        assert!(
            store
                .credit(self.agent, LedgerKind::Seed, 2200, Uuid::new_v4())
                .await
                .is_err()
        );
        store
            .add_goal(self.agent, "Assess only verified offline evidence".into())
            .await
            .unwrap()
    }
    async fn activate(&self, goal: Uuid) -> Store {
        let (_, store) = self.admit("offline_control").await;
        store
            .credit(self.agent, LedgerKind::Seed, 2200, Uuid::new_v4())
            .await
            .unwrap();
        store
            .schedule_control_as(
                self.agent,
                &ScheduleCommand {
                    request_id: Uuid::new_v4(),
                    expected_revision: 0,
                    operation: ScheduleOperation::Start,
                    authorize_offline: true,
                    consented_at: Some(Utc::now()),
                    reason: "Explicit disposable offline authorization".into(),
                    policy: Some(SchedulePolicy {
                        goal_ids: vec![goal],
                        interval_seconds: 5,
                        max_cycles: 2,
                        simulated_budget_microusd: 2200,
                        valid_for_seconds: 60,
                    }),
                },
                Some(self.owner),
            )
            .await
            .unwrap();
        store
    }
}

#[tokio::test]
#[ignore = "requires isolated restored PostgreSQL; never primary"]
async fn epoch_scopes_offline_execution_and_revocation_fence_existing_leases() {
    let f = Fixture::new().await;
    let goal = f.setup().await;
    let control = f.activate(goal.id).await;
    assert!(control.begin_cycle(f.agent, false).await.is_err());
    let (id, execution) = f.admit("offline_execution").await;
    let monitor = supa_runtime::health::WorkerMonitor::start(execution.clone(), "worker", true, 1)
        .await
        .unwrap();
    Runtime::new(execution.clone(), Arc::new(Offline))
        .with_worker(monitor.id)
        .cycle(f.agent, false)
        .await
        .unwrap();
    assert_eq!(execution.history(f.agent, "cycles").await.unwrap().len(), 1);
    assert!(!execution.get(f.agent).await.unwrap().memory.is_empty());
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let cycle = execution
        .begin_cycle_for(f.agent, false, Some(monitor.id))
        .await
        .unwrap();
    let bounded:bool=sqlx::query_scalar("SELECT c.lease_until<=(a.plan->>'expires_at')::timestamptz FROM cycles c JOIN epoch_guard.operational_approvals a ON a.id=c.epoch_permit_id WHERE c.id=$1").bind(cycle).fetch_one(f.root.pool()).await.unwrap();
    assert!(bounded);
    f.revoke(id).await;
    assert!(
        execution
            .propose(f.agent, cycle, &[json!({"tool":"remember"})])
            .await
            .is_err()
    );
    assert!(execution.finish(f.agent, cycle, None).await.is_err());
    assert!(
        !control.execution_health(f.agent).await.unwrap()["execution_running"]
            .as_bool()
            .unwrap()
    );
    assert!(execution.heartbeat(monitor.id).await.is_err());
    monitor.shutdown().await.unwrap_err();
    assert!(
        f.service
            .admit_operation(id, f.epoch, f.namespace)
            .await
            .is_err()
    );
    let (_, replacement) = f.admit("offline_execution").await;
    assert!(replacement.finish(f.agent, cycle, None).await.is_err());
    control
        .transition(
            f.agent,
            Lifecycle::Suspended,
            "Explicit disposable recovery fence",
            true,
        )
        .await
        .unwrap();
    assert_eq!(
        f.root
            .history(f.agent, "cycles")
            .await
            .unwrap()
            .iter()
            .filter(|row| row["status"] == "interrupted")
            .count(),
        1
    );

    assert_eq!(
        f.root.application_epoch().await.unwrap().unwrap().state,
        "prepared"
    );
    assert_eq!(f.root.history(f.agent, "ledger").await.unwrap().len(), 3);
    let dangerous:i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM live_budgets)+(SELECT count(*) FROM live_sessions)+(SELECT count(*) FROM economic_receipts)+(SELECT count(*) FROM wallet_identities)").fetch_one(f.root.pool()).await.unwrap();
    assert_eq!(dangerous, 0);
}

#[tokio::test]
#[ignore = "requires isolated restored PostgreSQL; never primary"]
async fn epoch_admission_rejects_forgery_scope_identity_replay_and_concurrency() {
    let f = Fixture::new().await;
    assert!(
        f.service
            .admit_operation(Uuid::new_v4(), f.epoch, f.namespace)
            .await
            .is_err()
    );
    for key in [
        "database",
        "epoch_id",
        "accounting_namespace",
        "principal_id",
        "credential_id",
        "scope",
        "expires_at",
    ] {
        let mut p = f.plan("agent_creation");
        p[key] = match key {
            "database" => json!("wrong_test"),
            "scope" => json!("paid_inference"),
            "expires_at" => json!((Utc::now() - Duration::seconds(1)).to_rfc3339()),
            _ => json!(Uuid::new_v4()),
        };
        let mut tx = f.authority.pool().begin().await.unwrap();
        sqlx::query("SET TRANSACTION READ WRITE")
            .execute(&mut *tx)
            .await
            .unwrap();
        assert!(
            sqlx::query("SELECT epoch_guard.approve_operation($1,$2)")
                .bind(Uuid::new_v4())
                .bind(p)
                .execute(&mut *tx)
                .await
                .is_err()
        );
    }
    let id = f.issue(f.plan("agent_creation")).await;
    assert!(
        f.service
            .admit_operation(id, Uuid::new_v4(), f.namespace)
            .await
            .is_err()
    );
    assert!(
        f.service
            .admit_operation(id, f.epoch, Uuid::new_v4())
            .await
            .is_err()
    );
    let (a, b) = tokio::join!(
        f.service.admit_operation(id, f.epoch, f.namespace),
        f.service.admit_operation(id, f.epoch, f.namespace)
    );
    assert_eq!(u8::from(a.is_ok()) + u8::from(b.is_ok()), 1);
    assert!(
        f.service
            .admit_operation(id, f.epoch, f.namespace)
            .await
            .is_err()
    );
    let mut tx = f.service.pool().begin().await.unwrap();
    sqlx::query("SET TRANSACTION READ WRITE")
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(
        sqlx::query(
            "UPDATE application_epoch SET state='active_read_only',revision=1,approval_id=$1"
        )
        .bind(Uuid::new_v4())
        .execute(&mut *tx)
        .await
        .is_err()
    );
    assert!(
        sqlx::query("INSERT INTO epoch_guard.operational_approvals VALUES($1,$2,$3,now())")
            .bind(Uuid::new_v4())
            .bind(f.plan("agent_creation"))
            .bind(f.owner)
            .execute(f.service.pool())
            .await
            .is_err()
    );
    let mut tx = f.service.pool().begin().await.unwrap();
    sqlx::query("SET TRANSACTION READ WRITE")
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(
        sqlx::query("TRUNCATE agents CASCADE")
            .execute(&mut *tx)
            .await
            .is_err()
    );
    let revoked = f.issue(f.plan("agent_creation")).await;
    f.revoke(revoked).await;
    assert!(
        f.service
            .admit_operation(revoked, f.epoch, f.namespace)
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires isolated restored PostgreSQL; never primary"]
async fn epoch_expiry_at_commit_credential_revocation_and_readonly_fallback() {
    let f = Fixture::new().await;
    let mut p = f.plan("agent_creation");
    p["expires_at"] = json!((Utc::now() + Duration::seconds(2)).to_rfc3339());
    let id = f.issue(p).await;
    let store = f
        .service
        .admit_operation(id, f.epoch, f.namespace)
        .await
        .unwrap();
    let mut tx = store.pool().begin().await.unwrap();
    let a = Agent::new(
        "Expiry fixture".into(),
        Personality::default(),
        CycleConfig::default(),
        Default::default(),
    )
    .unwrap();
    let mut data = serde_json::to_value(a).unwrap();
    data["id"] = json!(f.agent);
    sqlx::query("INSERT INTO agents(id,data) VALUES($1,$2)")
        .bind(f.agent)
        .bind(data)
        .execute(&mut *tx)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    assert!(tx.commit().await.is_err());
    assert!(f.root.get(f.agent).await.is_err());
    let (_, store) = f.admit("agent_creation").await;
    let unused = f.issue(f.plan("agent_creation")).await;
    let mut tx = f.authority.pool().begin().await.unwrap();
    sqlx::query("SET TRANSACTION READ WRITE")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT epoch_guard.revoke_credential($1)")
        .bind(f.credential)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(
        store
            .require_operation(f.owner, "agent_creation", None)
            .await
            .is_err()
    );
    assert!(
        f.service
            .admit_operation(unused, f.epoch, f.namespace)
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires isolated restored PostgreSQL; never primary"]
async fn epoch_api_process_crash_requires_new_admission_and_preserves_readonly_default() {
    use std::io::{Read, Write};
    let f = Fixture::new().await;
    let id = f.issue(f.plan("agent_creation")).await;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let service_token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let options = f.service.pool().connect_options();
    let url = sqlx::ConnectOptions::to_url_lossy(options.as_ref()).to_string();
    let launch = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_supa-api"))
            .env_clear()
            .env("DATABASE_URL", &url)
            .env("SUPA_API_TOKEN", &service_token)
            .env("SUPA_BIND", address.to_string())
            .env("SUPA_PROVIDER", "offline")
            .env("SUPA_EPOCH_MODE", "controlled_offline")
            .env("SUPA_EXPECTED_EPOCH_ID", f.epoch.to_string())
            .env(
                "SUPA_EXPECTED_ACCOUNTING_NAMESPACE",
                f.namespace.to_string(),
            )
            .env("SUPA_OPERATION_APPROVAL_ID", id.to_string())
            .env(
                "SUPA_OPERATOR_CREDENTIALS",
                json!([{"id":f.owner,"role":"owner","token":token}]).to_string(),
            )
            .env(
                "SUPA_OPERATOR_SESSION_DEADLINES",
                json!({f.owner.to_string():(Utc::now()+Duration::minutes(5)).timestamp()})
                    .to_string(),
            )
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut child = launch();
    for _ in 0..100 {
        if std::net::TcpStream::connect(address).is_ok() {
            break;
        }
        assert!(child.try_wait().unwrap().is_none());
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let mut stream = std::net::TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(3)))
        .unwrap();
    write!(stream,"GET /agents HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    child.kill().unwrap();
    child.wait().unwrap(); // Real process crash; no admission token persisted to disk.
    let mut restart = launch();
    for _ in 0..150 {
        if let Some(status) = restart.try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(restart.try_wait().unwrap().is_some());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM epoch_guard.operational_claims")
            .fetch_one(f.root.pool())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SHOW default_transaction_read_only")
            .fetch_one(f.service.pool())
            .await
            .unwrap(),
        "on"
    );
    assert_eq!(
        f.root.application_epoch().await.unwrap().unwrap().state,
        "prepared"
    );
}

#[tokio::test]
#[ignore = "requires isolated restored PostgreSQL; never primary"]
async fn epoch_transaction_limit_serializes_concurrent_writes_and_binds_agent() {
    let f = Fixture::new().await;
    let mut p = f.plan("agent_creation");
    p["max_transactions"] = json!(1);
    let id = f.issue(p).await;
    let store = f
        .service
        .admit_operation(id, f.epoch, f.namespace)
        .await
        .unwrap();
    let mut a = Agent::new(
        "Concurrent fixture".into(),
        Personality::default(),
        CycleConfig::default(),
        Default::default(),
    )
    .unwrap();
    a.id = f.agent;
    let (first, second) = tokio::join!(
        store.create_owned(&a, f.owner),
        store.create_owned(&a, f.owner)
    );
    assert_eq!(u8::from(first.is_ok()) + u8::from(second.is_ok()), 1);
    assert!(
        store
            .add_goal(f.agent, "Another transaction".into())
            .await
            .is_err()
    );
    let wrong = Agent::new(
        "Wrong resource".into(),
        Personality::default(),
        CycleConfig::default(),
        Default::default(),
    )
    .unwrap();
    assert!(store.create_owned(&wrong, f.owner).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM epoch_guard.operational_operations")
            .fetch_one(f.root.pool())
            .await
            .unwrap(),
        1
    );
    let (_, accounting) = f.admit("agent_creation").await;
    let mut tx = accounting.pool().begin().await.unwrap();
    sqlx::query("UPDATE agents SET simulated_balance_microusd=1 WHERE id=$1")
        .bind(f.agent)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(tx.commit().await.is_err());
    assert_eq!(
        f.root.financials(f.agent).await.unwrap().balance_microusd,
        0
    );
    // The immutable approval and consumption cannot be edited by a runtime login.
    assert!(
        sqlx::query("UPDATE epoch_guard.operational_approvals SET plan='{}'")
            .execute(store.pool())
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires isolated restored PostgreSQL and disposable worker binary; never primary"]
async fn epoch_worker_process_is_finite_and_restart_cannot_replay_admission() {
    let f = Fixture::new().await;
    let goal = f.setup().await;
    let _control = f.activate(goal.id).await;
    let id = f.issue(f.plan("offline_execution")).await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let options = f.service.pool().connect_options();
    let url = sqlx::ConnectOptions::to_url_lossy(options.as_ref()).to_string();
    let binary =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/supa-runtime");
    let launch = || {
        Command::new(&binary)
            .env_clear()
            .env("DATABASE_URL", &url)
            .env("SUPA_PROVIDER", "offline")
            .env("SUPA_WORKER_ENABLED", "true")
            .env("SUPA_WORKER_POLL_SECONDS", "1")
            .env("SUPA_WORKER_BIND", address.to_string())
            .env("SUPA_EPOCH_MODE", "controlled_offline")
            .env("SUPA_EXPECTED_EPOCH_ID", f.epoch.to_string())
            .env(
                "SUPA_EXPECTED_ACCOUNTING_NAMESPACE",
                f.namespace.to_string(),
            )
            .env("SUPA_OPERATION_APPROVAL_ID", id.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut child = launch();
    for _ in 0..200 {
        assert!(child.try_wait().unwrap().is_none());
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM cycles WHERE status='completed'")
            .fetch_one(f.root.pool())
            .await
            .unwrap();
        if count == 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM cycles WHERE status='completed'")
        .fetch_one(f.root.pool())
        .await
        .unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(count, 2);
    let mut restart = launch();
    for _ in 0..100 {
        if let Some(status) = restart.try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(restart.try_wait().unwrap().is_some());
    assert_eq!(
        f.root.financials(f.agent).await.unwrap().balance_microusd,
        0
    );
    assert_eq!(f.root.history(f.agent, "cycles").await.unwrap().len(), 2);
    assert!(!f.root.get(f.agent).await.unwrap().memory.is_empty());
}
