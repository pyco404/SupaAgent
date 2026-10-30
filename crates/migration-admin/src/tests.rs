use super::*;
use ring::{
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair},
};
use serde_json::json;
use std::{process::Command, sync::Arc};
use tokio::sync::Notify;

fn migration_test_url() -> String {
    std::env::var("TEST_MIGRATION_DATABASE_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .unwrap()
}
fn key() -> Ed25519KeyPair {
    Ed25519KeyPair::from_pkcs8(
        Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
            .unwrap()
            .as_ref(),
    )
    .unwrap()
}
fn plan(t: &Target, a: &Authority) -> Plan {
    Plan {
        version: 1,
        approval_id: Uuid::new_v4(),
        scope: "migrate_development_epoch_only".into(),
        principal_id: t.principal,
        issued_at: Utc::now().timestamp(),
        expires_at: Utc::now().timestamp() + 1800,
        database: t.database.clone(),
        epoch_id: t.epoch,
        accounting_namespace: t.namespace,
        system_identifier: a.system_identifier.clone(),
        socket_directory: t.socket.to_string_lossy().into_owned(),
        port: t.port,
        data_directory: t.data.to_string_lossy().into_owned(),
        expected_version: 8,
        target_version: 10,
        backup_sha256: t.backup_sha.clone(),
        source_snapshot_sha256: sha(b"snapshot"),
        source_catalog_sha256: t.catalog_sha.clone(),
        reviewed_state_sha256: t.reviewed_state_sha.clone(),
        executable_sha256: sha(b"test executable only"),
        migration_sha384: inventory(),
    }
}
fn authority(t: &Target, k: &Ed25519KeyPair) -> Authority {
    Authority {
        version: 1,
        principal_id: t.principal,
        public_key_hex: hex(k.public_key().as_ref()),
        system_identifier: "123456789".into(),
        migrator_login: "fixture_admin".into(),
        identity_and_independent_control_verified: true,
    }
}
fn sign(p: &Plan, k: &Ed25519KeyPair) -> (Vec<u8>, Vec<u8>) {
    let raw = serde_json::to_vec(p).unwrap();
    let sig = k.sign(&raw).as_ref().to_vec();
    (raw, sig)
}
#[test]
fn signed_approval_rejects_forgery_tampering_targets_scope_and_expiry() {
    let t = Target::primary();
    let k = key();
    let a = authority(&t, &k);
    let p = plan(&t, &a);
    let now = Utc::now().timestamp();
    let (raw, sig) = sign(&p, &k);
    assert!(verify(&raw, &sig, &a, &t, &p.executable_sha256, now).is_ok());
    assert!(
        verify(
            &[raw.clone(), vec![b' ']].concat(),
            &sig,
            &a,
            &t,
            &p.executable_sha256,
            now
        )
        .is_err()
    );
    assert!(
        verify(
            &raw,
            key().sign(&raw).as_ref(),
            &a,
            &t,
            &p.executable_sha256,
            now
        )
        .is_err()
    );
    for change in [
        "database",
        "epoch_id",
        "principal_id",
        "scope",
        "expected_version",
        "target_version",
        "executable_sha256",
        "source_catalog_sha256",
        "migration_sha384",
    ] {
        let mut value = serde_json::to_value(&p).unwrap();
        value[change] = match change {
            "epoch_id" | "principal_id" => json!(Uuid::new_v4()),
            "expected_version" => json!(9),
            "target_version" => json!(11),
            "migration_sha384" => json!({}),
            _ => json!("forged"),
        };
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(
            verify(
                &bytes,
                k.sign(&bytes).as_ref(),
                &a,
                &t,
                &p.executable_sha256,
                now
            )
            .is_err()
        );
    }
    assert!(verify(&raw, &sig, &a, &t, &p.executable_sha256, p.expires_at).is_err());
    assert!(verify(&raw, &sig, &a, &t, &p.executable_sha256, p.issued_at - 1).is_err());
    let mut unknown = serde_json::to_value(&p).unwrap();
    unknown["activate"] = json!(true);
    let b = serde_json::to_vec(&unknown).unwrap();
    assert!(verify(&b, k.sign(&b).as_ref(), &a, &t, &p.executable_sha256, now).is_err());
}
#[test]
fn private_inputs_reject_links_shared_modes_and_oversized_files() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("approval");
    std::fs::write(&file, b"fixture").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    let uid = unsafe { libc::geteuid() };
    assert!(private_read(&file, uid, 100, true).is_ok());
    assert!(private_read(&file, uid, 3, true).is_err());
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&file, &link).unwrap();
    assert!(private_read(&link, uid, 100, true).is_err());
    std::fs::hard_link(&file, dir.path().join("hard")).unwrap();
    assert!(private_read(&file, uid, 100, true).is_err());
    std::fs::remove_file(dir.path().join("hard")).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(private_read(&file, uid, 100, true).is_err());
}
#[tokio::test]
async fn production_interface_has_no_arbitrary_database_or_disposable_override() {
    assert!(
        command(vec!["--database".into(), DATABASE.into()])
            .await
            .is_err()
    );
    assert!(
        command(vec!["--exercise-disposable".into(), "migrate".into()])
            .await
            .is_err()
    );
    assert!(command(vec![]).await.is_err());
}

struct Fixture {
    admin: PgConnection,
    opts: PgConnectOptions,
    target: Target,
    authority: Authority,
    plan: Plan,
    raw: Vec<u8>,
    signing_key: Ed25519KeyPair,
}
impl Fixture {
    async fn new() -> Self {
        let url = migration_test_url();
        assert!(url.contains("_test"));
        let base: PgConnectOptions = url.parse().unwrap();
        assert_eq!(base.get_port(), 55444);
        let socket = base.get_socket().unwrap().to_owned();
        assert!(
            socket
                .to_string_lossy()
                .contains("/postgres-data/stage-two/")
        );
        let mut root = PgConnection::connect_with(&base).await.unwrap();
        let data: String = sqlx::query_scalar("SHOW data_directory")
            .fetch_one(&mut root)
            .await
            .unwrap();
        assert!(data.contains("/postgres-data/stage-two/"));
        assert_ne!(data, Target::primary().data.to_string_lossy());
        let db = format!("supa_stage2_test_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {db} TEMPLATE template0"))
            .execute(&mut root)
            .await
            .unwrap();
        let dump = Path::new(ROOT).join(BACKUP);
        assert_eq!(sha(&std::fs::read(&dump).unwrap()), BACKUP_SHA);
        let r = Command::new("/usr/lib/postgresql/16/bin/pg_restore")
            .args(["--exit-on-error", "--single-transaction", "--dbname", &db])
            .arg(dump)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("PGHOST", &socket)
            .env("PGPORT", "55444")
            .env("PGUSER", base.get_username())
            .output()
            .unwrap();
        assert!(r.status.success(), "disposable restore failed");
        let opts = base
            .database(&db)
            .application_name("stage2-migration-test")
            .options([
                ("timezone", "UTC"),
                ("search_path", "public"),
                ("statement_timeout", "10000"),
                ("lock_timeout", "3000"),
            ]);
        let mut admin = PgConnection::connect_with(&opts).await.unwrap();
        let names = tables(&mut admin).await.unwrap();
        let mut target = Target::primary();
        target.database = db.clone();
        target.data = PathBuf::from(data);
        target.socket = socket;
        target.port = 55444;
        target.principal = Uuid::new_v4();
        assert_eq!(catalog(&mut admin).await.unwrap(), target.catalog_sha);
        let k = key();
        let mut authority = authority(&target, &k);
        authority.system_identifier =
            sqlx::query_scalar("SELECT system_identifier::text FROM pg_control_system()")
                .fetch_one(&mut admin)
                .await
                .unwrap();
        authority.migrator_login = opts.get_username().into();
        let mut p = plan(&target, &authority);
        p.source_snapshot_sha256 = snapshot(&mut admin, &names).await.unwrap();
        let (raw, sig) = sign(&p, &k);
        verify(
            &raw,
            &sig,
            &authority,
            &target,
            &p.executable_sha256,
            Utc::now().timestamp(),
        )
        .unwrap();
        sqlx::query(&format!(
            "ALTER DATABASE {db} SET default_transaction_read_only=on"
        ))
        .execute(&mut root)
        .await
        .unwrap();
        root.close().await.unwrap();
        Self {
            admin,
            opts,
            target,
            authority,
            plan: p,
            raw,
            signing_key: k,
        }
    }
    async fn run(&self, fault: Fault) -> Result<()> {
        let signature = self.signing_key.sign(&self.raw);
        verify(
            &self.raw,
            signature.as_ref(),
            &self.authority,
            &self.target,
            &self.plan.executable_sha256,
            Utc::now().timestamp(),
        )?;
        execute(
            &self.opts,
            &self.target,
            &self.authority,
            &self.plan,
            &self.raw,
            signature.as_ref(),
            fault,
        )
        .await
    }
    async fn version(&mut self) -> i64 {
        sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
            .fetch_one(&mut self.admin)
            .await
            .unwrap()
    }
    async fn prepared(&mut self) {
        let row: (String, i64, Option<Uuid>) =
            sqlx::query_as("SELECT state,revision,approval_id FROM application_epoch")
                .fetch_one(&mut self.admin)
                .await
                .unwrap();
        assert_eq!(row, ("prepared".into(), 0, None));
    }
}
#[tokio::test]
#[ignore = "requires Stage 1 backup and isolated stage-two cluster on port 55444; never primary"]
async fn cloned_schema_permissions_atomic_failures_concurrency_and_recovery() {
    let mut good = Fixture::new().await;
    good.run(Fault::Never).await.unwrap();
    assert_eq!(good.version().await, 10);
    good.prepared().await;
    assert!(good.run(Fault::Never).await.is_err());
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM epoch_guard.migration_receipts")
        .fetch_one(&mut good.admin)
        .await
        .unwrap();
    assert_eq!(receipts, 1);
    let (text,sig,key,digest):(String,String,String,String)=sqlx::query_as("SELECT plan_text,signature_hex,authority_public_key_hex,plan_sha256 FROM epoch_guard.migration_receipts").fetch_one(&mut good.admin).await.unwrap();
    assert_eq!(sha(text.as_bytes()), digest);
    assert_eq!(key, good.authority.public_key_hex);
    UnparsedPublicKey::new(&ED25519, unhex(&key, 32).unwrap())
        .verify(text.as_bytes(), &unhex(&sig, 64).unwrap())
        .unwrap();

    let dangerous:i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM agent_access)+(SELECT count(*) FROM agents)+(SELECT count(*) FROM live_budgets)+(SELECT count(*) FROM schedule_authorizations)+(SELECT count(*) FROM epoch_transitions)+(SELECT count(*) FROM epoch_guard.authorities)+(SELECT count(*) FROM epoch_guard.executors)+(SELECT count(*) FROM epoch_guard.approvals)").fetch_one(&mut good.admin).await.unwrap();
    assert_eq!(dangerous, 0);
    let leaks:i64=sqlx::query_scalar("SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace CROSS JOIN LATERAL aclexplode(p.proacl) a WHERE n.nspname='epoch_guard' AND a.grantee=0 AND p.proname<>'verify_active'").fetch_one(&mut good.admin).await.unwrap();
    assert_eq!(leaks, 0);
    for statement in [
        "UPDATE application_epoch SET state='active_read_only',revision=1",
        "DELETE FROM epoch_transitions",
        "DELETE FROM epoch_guard.migration_receipts",
        "SELECT epoch_guard.approve(gen_random_uuid(),'{}',clock_timestamp()+interval '1 minute')",
    ] {
        assert!(
            sqlx::query(statement)
                .execute(&mut good.admin)
                .await
                .is_err()
        );
    }
    // An error after migration 9 rolls back DDL and SQLx bookkeeping, not just the receipt.
    let mut failed = Fixture::new().await;
    assert!(failed.run(Fault::FailAfterNine).await.is_err());
    assert_eq!(failed.version().await, 8);
    let guard:bool=sqlx::query_scalar("SELECT to_regclass('epoch_transitions') IS NULL AND to_regnamespace('epoch_guard') IS NULL").fetch_one(&mut failed.admin).await.unwrap();
    assert!(guard);
    // Manual recovery with a new approval identifier; no automatic retry loop exists.
    failed.plan.approval_id = Uuid::new_v4();
    failed.raw = serde_json::to_vec(&failed.plan).unwrap();
    failed.run(Fault::Never).await.unwrap();
    failed.prepared().await;
    let mut concurrent = Fixture::new().await;
    let (a, b) = tokio::join!(concurrent.run(Fault::Never), concurrent.run(Fault::Never));
    assert_eq!(a.is_ok() as u8 + b.is_ok() as u8, 1);
    assert_eq!(concurrent.version().await, 10);
    // Terminate a real backend while its transaction contains migration 9.
    let mut killed = Fixture::new().await;
    let ready = Arc::new(Notify::new());
    let resume = Arc::new(Notify::new());
    let future = killed.run(Fault::PauseAfterNine(ready.clone(), resume.clone()));
    let control = async {
        ready.notified().await;
        let base = migration_test_url();
        let mut root = PgConnection::connect(&base).await.unwrap();
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM (SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1 AND application_name='stage2-migration-test' AND pid<>pg_backend_pid()) t").bind(&killed.target.database).fetch_one(&mut root).await.unwrap();
        assert!(count >= 1);
        resume.notify_one();
    };
    let (result, _) = tokio::join!(future, control);
    assert!(result.is_err());
    // Admin connection was terminated too; reconnect only for read-only diagnosis.
    killed.admin = PgConnection::connect_with(&killed.opts).await.unwrap();
    assert_eq!(killed.version().await, 8);
    let mut corrupt = Fixture::new().await;
    sqlx::query(
        "UPDATE _sqlx_migrations SET checksum=decode(repeat('00',48),'hex') WHERE version=8",
    )
    .execute(&mut corrupt.admin)
    .await
    .unwrap();
    assert!(corrupt.run(Fault::Never).await.is_err());
    assert_eq!(corrupt.version().await, 8);
    let mut altered = Fixture::new().await;
    sqlx::query("GRANT SELECT ON agents TO PUBLIC")
        .execute(&mut altered.admin)
        .await
        .unwrap();
    assert!(altered.run(Fault::Never).await.is_err());
    assert_eq!(altered.version().await, 8);
    let mut structure = Fixture::new().await;
    sqlx::query("ALTER TABLE agents ADD COLUMN unexpected integer")
        .execute(&mut structure.admin)
        .await
        .unwrap();
    assert!(structure.run(Fault::Never).await.is_err());
    assert_eq!(structure.version().await, 8);
    let mut wrong = Fixture::new().await;
    sqlx::query("UPDATE application_epoch SET id=$1")
        .bind(Uuid::new_v4())
        .execute(&mut wrong.admin)
        .await
        .unwrap();
    assert!(wrong.run(Fault::Never).await.is_err());
    assert_eq!(wrong.version().await, 8);
    let mut partial = Fixture::new().await;
    Migrate::apply(
        &mut partial.admin,
        MIGRATIONS.iter().find(|m| m.version == 9).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(partial.version().await, 9);
    assert!(partial.run(Fault::Never).await.is_err());
    assert_eq!(partial.version().await, 9);
}
