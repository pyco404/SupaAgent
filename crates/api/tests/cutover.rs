//! Independent approval authority and restricted executor in disposable PostgreSQL only.
use serde_json::{Value, json};
use sqlx::{ConnectOptions, PgPool, postgres::PgConnectOptions};
use supa_persistence::Store;
use uuid::Uuid;

struct Fixture {
    admin: Store,
    authority: Store,
    executor: Store,
    reader: Store,
    db: String,
    epoch: Uuid,
    namespace: Uuid,
    actor: Uuid,
    authority_login: String,
    executor_login: String,
}
impl Fixture {
    async fn new() -> Self {
        let base = std::env::var("TEST_DATABASE_URL").unwrap();
        assert!(base.contains("_test"));
        let root = Store::connect(&base).await.unwrap();
        let db = format!("supa_cutover_test_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {db} TEMPLATE template0"))
            .execute(root.pool())
            .await
            .unwrap();
        let options = base.parse::<PgConnectOptions>().unwrap().database(&db);
        let admin = Store::connect(options.to_url_lossy().as_str())
            .await
            .unwrap();
        admin.migrate().await.unwrap();
        let (epoch, namespace, actor) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        sqlx::query("INSERT INTO application_epoch(id,accounting_namespace,label,kind,state,historical_database) VALUES($1,$2,'Synthetic trusted approval fixture','development','prepared','unrecovered')")
            .bind(epoch).bind(namespace).execute(admin.pool()).await.unwrap();
        let mut stores = Vec::new();
        let mut logins = Vec::new();
        for kind in ["authority", "executor", "reader"] {
            let login = format!("guard_{kind}_{}", Uuid::new_v4().simple());
            let password = Uuid::new_v4().simple().to_string();
            sqlx::query(&format!("CREATE ROLE {login} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT PASSWORD '{password}'"))
                .execute(root.pool()).await.unwrap();
            sqlx::raw_sql(&format!("GRANT USAGE ON SCHEMA public,epoch_guard TO {login}; GRANT SELECT ON ALL TABLES IN SCHEMA public TO {login}"))
                .execute(admin.pool()).await.unwrap();
            let opts = options.clone().username(&login).password(&password);
            stores.push(Store::connect(opts.to_url_lossy().as_str()).await.unwrap());
            logins.push(login);
        }
        let authority_login = logins[0].clone();
        let executor_login = logins[1].clone();
        sqlx::query("INSERT INTO epoch_guard.authorities(login,principal_id) VALUES($1,$2)")
            .bind(&authority_login)
            .bind(Uuid::new_v4())
            .execute(admin.pool())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO epoch_guard.executors(login,principal_id,role) VALUES($1,$2,'owner')",
        )
        .bind(&executor_login)
        .bind(actor)
        .execute(admin.pool())
        .await
        .unwrap();
        sqlx::raw_sql(&format!("GRANT EXECUTE ON FUNCTION epoch_guard.approve(uuid,text,timestamptz),epoch_guard.revoke_approval(uuid) TO {authority_login}; GRANT EXECUTE ON FUNCTION epoch_guard.execute_transition(uuid,uuid,text,uuid,bigint,text,text) TO {executor_login}"))
            .execute(admin.pool()).await.unwrap();
        sqlx::query(&format!(
            "ALTER DATABASE {db} SET default_transaction_read_only=on"
        ))
        .execute(root.pool())
        .await
        .unwrap();
        // Reconnect: each role must inherit the database's frozen default.
        for store in &stores {
            store.pool().close().await;
        }
        let mut reconnected = Vec::new();
        for login in &logins {
            // The pool's options retain the synthetic login/password; no secret is logged.
            let opts = stores[reconnected.len()]
                .pool()
                .connect_options()
                .as_ref()
                .clone();
            assert_eq!(opts.get_username(), login);
            reconnected.push(Store::connect(opts.to_url_lossy().as_str()).await.unwrap());
        }
        let reader = reconnected.pop().unwrap();
        let executor = reconnected.pop().unwrap();
        let authority = reconnected.pop().unwrap();
        Self {
            admin,
            authority,
            executor,
            reader,
            db,
            epoch,
            namespace,
            actor,
            authority_login,
            executor_login,
        }
    }
    async fn plan(&self, operation: &str, revision: i64) -> String {
        let migrations: Value = sqlx::query_scalar("SELECT jsonb_object_agg(version::text,encode(checksum,'hex')) FROM _sqlx_migrations WHERE success")
            .fetch_one(self.admin.pool()).await.unwrap();
        json!({"version":1,"database":self.db,"epoch_id":self.epoch,"accounting_namespace":self.namespace,
            "principal_id":self.actor,"role":"owner","operation":operation,"expected_revision":revision,
            "scope":"local_development_read_only","config_sha256":{"api":"a".repeat(64),"frontend":"b".repeat(64),"postgres":"c".repeat(64)},
            "migrations_sha384":migrations}).to_string()
    }
    async fn approve(&self, id: Uuid, plan: &str, seconds: i32) -> String {
        let mut tx = self.authority.pool().begin().await.unwrap();
        sqlx::query("SET TRANSACTION READ WRITE")
            .execute(&mut *tx)
            .await
            .unwrap();
        let digest: String=sqlx::query_scalar("SELECT epoch_guard.approve($1,$2,clock_timestamp()+($3::integer * interval '1 second'))")
            .bind(id).bind(plan).bind(seconds).fetch_one(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        digest
    }
    async fn execute(
        &self,
        id: Uuid,
        plan: &str,
        revision: i64,
        operation: &str,
    ) -> supa_domain::Result<supa_persistence::ApplicationEpoch> {
        self.executor
            .exercise_epoch_transition(self.epoch, self.actor, id, revision, operation, plan)
            .await
    }
}
async fn write_rejected(pool: &PgPool, sql: &str) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET TRANSACTION READ WRITE")
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(sqlx::query(sql).execute(&mut *tx).await.is_err());
}

#[tokio::test]
#[ignore = "requires disposable TEST_DATABASE_URL and SCRAM test logins"]
async fn trusted_approval_rejects_forgery_replay_tampering_and_direct_writes() {
    let f = Fixture::new().await;
    let plan = f.plan("activate_read_only", 0).await;
    let approval = Uuid::new_v4();
    assert!(
        f.execute(approval, &plan, 0, "activate_read_only")
            .await
            .is_err()
    );
    let digest = f.approve(approval, &plan, 600).await;
    assert_eq!(digest.len(), 64);
    let stale_approval = Uuid::new_v4();
    f.approve(stale_approval, &plan, 600).await;
    for attack in [
        "0".repeat(64),
        format!("{plan} "),
        plan.replace("local_development_read_only", "unrestricted"),
        plan.replace(&"a".repeat(64), &"d".repeat(64)),
    ] {
        assert!(
            f.execute(approval, &attack, 0, "activate_read_only")
                .await
                .is_err()
        );
    }
    assert!(
        f.execute(approval, &plan, 1, "activate_read_only")
            .await
            .is_err()
    );
    assert!(
        f.executor
            .exercise_epoch_transition(
                Uuid::new_v4(),
                f.actor,
                approval,
                0,
                "activate_read_only",
                &plan
            )
            .await
            .is_err()
    );
    assert!(
        f.executor
            .exercise_epoch_transition(
                f.epoch,
                Uuid::new_v4(),
                approval,
                0,
                "activate_read_only",
                &plan
            )
            .await
            .is_err()
    );
    assert!(
        f.reader
            .exercise_epoch_transition(f.epoch, f.actor, approval, 0, "activate_read_only", &plan)
            .await
            .is_err()
    );
    let mut tx = f.executor.pool().begin().await.unwrap();
    sqlx::query("SET TRANSACTION READ WRITE")
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(
        sqlx::query(
            "SELECT epoch_guard.execute_transition($1,$2,'viewer',$3,0,'activate_read_only',$4)"
        )
        .bind(approval)
        .bind(f.actor)
        .bind(f.epoch)
        .bind(&plan)
        .execute(&mut *tx)
        .await
        .is_err()
    );
    drop(tx);
    for store in [&f.reader, &f.executor] {
        write_rejected(
            store.pool(),
            "UPDATE epoch_guard.approvals SET plan_sha256=repeat('0',64)",
        )
        .await;
        write_rejected(store.pool(), "DELETE FROM epoch_guard.approvals").await;
        write_rejected(store.pool(), "TRUNCATE epoch_guard.approvals").await;
        write_rejected(store.pool(), &format!("SET ROLE {}", f.authority_login)).await;
        write_rejected(store.pool(),"SELECT epoch_guard.approve('11111111-1111-4111-8111-111111111111','{}',clock_timestamp()+interval '1 minute')").await;
    }
    // Defense in depth even if ordinary DML is accidentally granted to the executor.
    sqlx::raw_sql(&format!("GRANT UPDATE,DELETE,TRUNCATE ON application_epoch TO {}; GRANT INSERT ON epoch_transitions TO {}",f.executor_login,f.executor_login)).execute(f.admin.pool()).await.unwrap();
    write_rejected(
        f.executor.pool(),
        "UPDATE application_epoch SET state='active_read_only',revision=1",
    )
    .await;
    write_rejected(
        f.executor.pool(),
        "UPDATE application_epoch SET id='11111111-1111-4111-8111-111111111111'",
    )
    .await;
    write_rejected(f.executor.pool(), "DELETE FROM application_epoch").await;
    write_rejected(f.executor.pool(), "TRUNCATE application_epoch").await;
    write_rejected(f.executor.pool(),&format!("INSERT INTO epoch_transitions VALUES('{approval}','{}',1,'activate_read_only','{}','{}','prepared','active_read_only',now())",f.epoch,f.actor,digest)).await;
    let (a, b) = tokio::join!(
        f.execute(approval, &plan, 0, "activate_read_only"),
        f.execute(approval, &plan, 0, "activate_read_only")
    );
    assert_eq!(a.is_ok() as u8 + b.is_ok() as u8, 1);
    // A separate, genuine but now stale approval must fail too.
    assert!(
        f.execute(stale_approval, &plan, 0, "activate_read_only")
            .await
            .is_err()
    );
    assert!(
        f.execute(approval, &plan, 0, "activate_read_only")
            .await
            .is_err()
    );
    assert!(
        f.reader
            .require_read_only_epoch(f.epoch, "active_read_only", Some(approval))
            .await
            .is_ok()
    );
    let rollback = f.plan("rollback", 1).await;
    let undo = Uuid::new_v4();
    f.approve(undo, &rollback, 600).await;
    assert_eq!(
        f.execute(undo, &rollback, 1, "rollback")
            .await
            .unwrap()
            .revision,
        2
    );
    assert!(
        f.reader
            .require_read_only_epoch(f.epoch, "active_read_only", Some(approval))
            .await
            .is_err()
    );
    let counts:(i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM epoch_transitions),(SELECT count(*) FROM epoch_guard.consumptions),(SELECT count(*) FROM agents),(SELECT count(*) FROM agent_access)").fetch_one(f.admin.pool()).await.unwrap();
    assert_eq!(counts, (2, 2, 0, 0));
}

#[tokio::test]
#[ignore = "requires disposable TEST_DATABASE_URL and SCRAM test logins"]
async fn approval_expiry_revocation_and_scope_fail_closed() {
    let f = Fixture::new().await;
    let raw = f.plan("activate_read_only", 0).await;
    let mut changed: Value = serde_json::from_str(&raw).unwrap();
    changed["role"] = json!("viewer");
    write_rejected(
        f.authority.pool(),
        &format!(
            "SELECT epoch_guard.approve('{}','{}',clock_timestamp()+interval '1 minute')",
            Uuid::new_v4(),
            changed
        ),
    )
    .await;
    changed = serde_json::from_str(&raw).unwrap();
    changed["migrations_sha384"] = json!({});
    write_rejected(
        f.authority.pool(),
        &format!(
            "SELECT epoch_guard.approve('{}','{}',clock_timestamp()+interval '1 minute')",
            Uuid::new_v4(),
            changed
        ),
    )
    .await;
    let expired = Uuid::new_v4();
    f.approve(expired, &raw, 1).await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    assert!(
        f.execute(expired, &raw, 0, "activate_read_only")
            .await
            .is_err()
    );
    let revoked = Uuid::new_v4();
    f.approve(revoked, &raw, 600).await;
    let mut tx = f.authority.pool().begin().await.unwrap();
    sqlx::query("SET TRANSACTION READ WRITE")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT epoch_guard.revoke_approval($1)")
        .bind(revoked)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(
        f.execute(revoked, &raw, 0, "activate_read_only")
            .await
            .is_err()
    );
    assert_eq!(
        f.reader
            .require_epoch_preview(f.epoch)
            .await
            .unwrap()
            .revision,
        0
    );
    // Superuser/migrator cannot issue or execute consent without separately provisioned mappings.
    write_rejected(
        f.admin.pool(),
        &format!(
            "SELECT epoch_guard.approve('{}','{}',clock_timestamp()+interval '1 minute')",
            Uuid::new_v4(),
            raw
        ),
    )
    .await;
}

#[test]
fn sqlx_socket_urls_parse_reserved_credentials_database_names_and_paths() {
    // Synthetic reserved characters, constructed separately from the endpoint.
    let encoded = "p@ss:word/%#"
        .replace('%', "%25")
        .replace('@', "%40")
        .replace(':', "%3A")
        .replace('/', "%2F")
        .replace('#', "%23");
    let address = [
        "postgresql://reader",
        ":",
        &encoded,
        "@localhost/supa%20db?host=%2Ftmp%2Fprivate%20socket&port=55443",
    ]
    .concat();
    let opts: PgConnectOptions = address.parse().unwrap();
    assert_eq!(opts.get_username(), "reader");
    assert_eq!(opts.get_database(), Some("supa db"));
    assert_eq!(
        opts.get_socket().unwrap().to_str(),
        Some("/tmp/private socket")
    );
    assert_eq!(opts.get_port(), 55443);
    assert!(
        "postgresql://reader:your-password@/db?host=/tmp/socket"
            .parse::<PgConnectOptions>()
            .is_err()
    );
}
