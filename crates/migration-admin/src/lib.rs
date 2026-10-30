//! Pinned, independently signed schema 8 -> 10 administration. Never activates an epoch.
use chrono::Utc;
use ring::{
    digest::{SHA256, digest},
    signature::{ED25519, UnparsedPublicKey},
};
use serde::{Deserialize, Serialize};
use sqlx::{
    Connection, PgConnection,
    migrate::{Migrate, Migrator},
    postgres::{PgConnectOptions, PgSslMode},
};
use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use uuid::Uuid;
type Result<T> = std::result::Result<T, &'static str>;
const EPOCH: &str = "fd2f52e0-33fc-4c0d-893a-7b08b26ab748";
const NAMESPACE: &str = "641457b2-5fd8-4b31-b85f-c58839626831";
const CUSTODIAN: &str = "e8c6ade3-917f-4fe4-a092-f8a58eccb487";
const DATABASE: &str = "supa_epoch_dev_fd2f52e033fc4c0d893a7b08b26ab748";
const ROOT: &str = "/home/pikoo/Desktop/supa-agent";
const TRUST: &str = "/etc/supa-agent/migration-authority.json";
const BACKUP: &str = "postgres-data/preflight/1d0e9efe-e66e-4e4f-afc4-478eee4f0833/database.dump";
const BACKUP_SHA: &str = "600d2bb19b52b4f17d4278459ba722ea5f1135df703b8efc1b0b3db0c8360671";
static MIGRATIONS: Migrator = sqlx::migrate!("../../migrations");
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn sha(bytes: &[u8]) -> String {
    hex(digest(&SHA256, bytes).as_ref())
}
fn unhex(raw: &str, n: usize) -> Result<Vec<u8>> {
    if raw.len() != n * 2
        || !raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("invalid_hex");
    }
    (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).map_err(|_| "invalid_hex"))
        .collect()
}
fn check(ok: bool, code: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(code) }
}
fn private_read(path: &Path, uid: u32, max: usize, private: bool) -> Result<Vec<u8>> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "approval_or_configuration_file_unavailable")?;
    let m = file.metadata().map_err(|_| "file_metadata")?;
    check(
        m.is_file()
            && m.uid() == uid
            && m.nlink() == 1
            && m.mode() & if private { 0o077 } else { 0o022 } == 0
            && m.len() <= max as u64,
        "unsafe_file_permissions",
    )?;
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "file_read")?;
    check(bytes.len() <= max, "oversized_file")?;
    Ok(bytes)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authority {
    version: u32,
    principal_id: Uuid,
    public_key_hex: String,
    system_identifier: String,
    migrator_login: String,
    identity_and_independent_control_verified: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Credentials {
    username: String,
    password: Option<String>,
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct Plan {
    version: u32,
    approval_id: Uuid,
    scope: String,
    principal_id: Uuid,
    issued_at: i64,
    expires_at: i64,
    database: String,
    epoch_id: Uuid,
    accounting_namespace: Uuid,
    system_identifier: String,
    socket_directory: String,
    port: u16,
    data_directory: String,
    expected_version: i64,
    target_version: i64,
    backup_sha256: String,
    source_snapshot_sha256: String,
    source_catalog_sha256: String,
    reviewed_state_sha256: String,
    executable_sha256: String,
    migration_sha384: BTreeMap<i64, String>,
}
struct Target {
    database: String,
    socket: PathBuf,
    data: PathBuf,
    port: u16,
    epoch: Uuid,
    namespace: Uuid,
    principal: Uuid,
    backup_sha: String,
    catalog_sha: String,
    reviewed_state_sha: String,
}
impl Target {
    fn primary() -> Self {
        Self {
            database: DATABASE.into(),
            socket: Path::new(ROOT).join("postgres-data/infrastructure-v1/socket"),
            data: Path::new(ROOT).join("postgres-data/infrastructure-v1/data"),
            port: 55440,
            epoch: Uuid::parse_str(EPOCH).unwrap(),
            namespace: Uuid::parse_str(NAMESPACE).unwrap(),
            principal: Uuid::parse_str(CUSTODIAN).unwrap(),
            backup_sha: BACKUP_SHA.into(),
            catalog_sha: "7142ac86166fe58c439819eb312b7530bdb426e2eec39d0a6f8fcda9c4ce53c0".into(),
            reviewed_state_sha: "1d3fe0b1122312ff0926669ad4f62ec54079f408005c634257df0fad9049116b"
                .into(),
        }
    }
}
fn inventory() -> BTreeMap<i64, String> {
    MIGRATIONS
        .iter()
        .map(|m| (m.version, hex(&m.checksum)))
        .collect()
}
fn verify(
    raw: &[u8],
    signature: &[u8],
    authority: &Authority,
    target: &Target,
    executable: &str,
    now: i64,
) -> Result<Plan> {
    check(
        authority.version == 1
            && authority.identity_and_independent_control_verified
            && authority.principal_id == target.principal
            && !authority.migrator_login.is_empty()
            && authority.migrator_login.len() <= 63
            && authority
                .migrator_login
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        "unverified_authority",
    )?;
    UnparsedPublicKey::new(&ED25519, unhex(&authority.public_key_hex, 32)?)
        .verify(raw, signature)
        .map_err(|_| "untrusted_signature")?;
    let p: Plan = serde_json::from_slice(raw).map_err(|_| "invalid_plan_schema")?;
    check(
        p.version == 1
            && !p.approval_id.is_nil()
            && p.scope == "migrate_development_epoch_only"
            && p.principal_id == authority.principal_id,
        "approval_scope",
    )?;
    check(
        p.issued_at <= now
            && p.issued_at >= now - 3600
            && p.expires_at > now
            && p.expires_at - p.issued_at <= 3600,
        "approval_expired_or_future",
    )?;
    check(
        p.database == target.database
            && p.epoch_id == target.epoch
            && p.accounting_namespace == target.namespace
            && p.socket_directory == target.socket.to_string_lossy()
            && p.data_directory == target.data.to_string_lossy()
            && p.port == target.port
            && p.system_identifier == authority.system_identifier,
        "pinned_target_mismatch",
    )?;
    check(
        p.expected_version == 8
            && p.target_version == 10
            && p.backup_sha256 == target.backup_sha
            && p.source_catalog_sha256 == target.catalog_sha
            && p.reviewed_state_sha256 == target.reviewed_state_sha
            && p.executable_sha256 == executable
            && unhex(&p.source_snapshot_sha256, 32).is_ok(),
        "plan_limits_or_artifacts",
    )?;
    check(
        inventory().keys().copied().eq(1..=10)
            && p.migration_sha384 == inventory()
            && MIGRATIONS.iter().all(|m| !m.no_tx),
        "migration_inventory",
    )?;
    Ok(p)
}
fn trusted_authority() -> Result<Authority> {
    for p in ["/", "/etc", "/etc/supa-agent"] {
        let m = std::fs::symlink_metadata(p).map_err(|_| "trust_anchor_not_installed")?;
        check(
            m.is_dir() && m.uid() == 0 && m.mode() & 0o022 == 0,
            "unsafe_trust_directory",
        )?;
    }
    serde_json::from_slice(&private_read(Path::new(TRUST), 0, 4096, false)?)
        .map_err(|_| "invalid_trust_anchor")
}
fn clean_environment() -> Result<()> {
    check(
        !std::env::vars_os().any(|(name, _)| {
            let n = name.to_string_lossy();
            n.starts_with("PG")
                || n.starts_with("SUPA_")
                || n == "DATABASE_URL"
                || n == "OPENAI_API_KEY"
        }),
        "inherited_database_or_application_environment",
    )
}
/// The production interface has no database/socket/epoch override and no disposable switch.
pub async fn command(args: Vec<String>) -> Result<()> {
    clean_environment()?;
    check(
        args.len() == 6
            && args[0] == "--approval-file"
            && args[2] == "--signature-file"
            && args[4] == "--connection-file",
        "exact_arguments_required",
    )?;
    let authority = trusted_authority()?;
    let uid = unsafe { libc::geteuid() };
    let raw = private_read(Path::new(&args[1]), uid, 32768, true)?;
    let sigtext = private_read(Path::new(&args[3]), uid, 128, true)?;
    let signature = unhex(
        std::str::from_utf8(&sigtext).map_err(|_| "signature_encoding")?,
        64,
    )?;
    let executable = std::fs::read(std::env::current_exe().map_err(|_| "executable")?)
        .map_err(|_| "executable")?;
    let target = Target::primary();
    let plan = verify(
        &raw,
        &signature,
        &authority,
        &target,
        &sha(&executable),
        Utc::now().timestamp(),
    )?;
    let creds: Credentials =
        serde_json::from_slice(&private_read(Path::new(&args[5]), uid, 4096, true)?)
            .map_err(|_| "invalid_connection_file")?;
    check(
        creds.username == authority.migrator_login,
        "migrator_login_mismatch",
    )?;
    let backup = private_read(&Path::new(ROOT).join(BACKUP), uid, 64 * 1024 * 1024, true)?;
    check(sha(&backup) == target.backup_sha, "verified_backup_changed")?;
    let opts = PgConnectOptions::new_without_pgpass()
        .socket(&target.socket)
        .port(target.port)
        .database(&target.database)
        .username(&creds.username)
        .password(creds.password.as_deref().unwrap_or(""))
        .ssl_mode(PgSslMode::Disable)
        .application_name("supa-migration-only")
        .options([
            ("search_path", "public"),
            ("timezone", "UTC"),
            ("statement_timeout", "10000"),
            ("lock_timeout", "3000"),
        ]);
    execute(
        &opts,
        &target,
        &authority,
        &plan,
        &raw,
        &signature,
        Fault::Never,
    )
    .await
}
async fn sql<T>(result: std::result::Result<T, sqlx::Error>) -> Result<T> {
    result.map_err(|_| "database_validation_or_transaction_failed")
}
async fn tables(conn: &mut PgConnection) -> Result<Vec<String>> {
    sql(sqlx::query_scalar(
        "SELECT tablename FROM pg_tables WHERE schemaname='public' ORDER BY tablename",
    )
    .fetch_all(conn)
    .await)
    .await
}
fn qualified(name: &str) -> String {
    format!("public.\"{}\"", name.replace('"', "\"\""))
}
async fn snapshot(conn: &mut PgConnection, names: &[String]) -> Result<String> {
    let mut bytes = Vec::new();
    for name in names {
        let query = if name == "application_epoch" {
            "SELECT coalesce(jsonb_agg(to_jsonb(t)-'revision'-'approval_id' ORDER BY to_jsonb(t)::text),'[]')::text FROM public.application_epoch t".into()
        } else if name == "_sqlx_migrations" {
            "SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY t.version),'[]')::text FROM public._sqlx_migrations t WHERE version<=8".into()
        } else {
            format!(
                "SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]')::text FROM {} t",
                qualified(name)
            )
        };
        let value: String = sql(sqlx::query_scalar(&query).fetch_one(&mut *conn).await).await?;
        bytes.extend(name.as_bytes());
        bytes.push(0);
        bytes.extend(value.as_bytes());
        bytes.push(0);
    }
    Ok(sha(&bytes))
}
async fn catalog(conn: &mut PgConnection) -> Result<String> {
    let queries = [
        "SELECT jsonb_agg(to_jsonb(x) ORDER BY name)::text FROM (SELECT n.nspname||'.'||c.relname AS name,c.relkind,pg_get_userbyid(c.relowner) AS owner,c.relacl FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public') x",
        "SELECT jsonb_agg(to_jsonb(x) ORDER BY name)::text FROM (SELECT n.nspname AS name,pg_get_userbyid(n.nspowner) AS owner,n.nspacl FROM pg_namespace n WHERE n.nspname NOT LIKE 'pg_%' AND n.nspname<>'information_schema') x",
        "SELECT coalesce(jsonb_agg(to_jsonb(x) ORDER BY signature),'[]')::text FROM (SELECT p.oid::regprocedure::text AS signature,pg_get_functiondef(p.oid) AS definition,p.proacl,pg_get_userbyid(p.proowner) AS owner FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public' AND p.prokind IN ('f','p')) x",
        "SELECT coalesce(jsonb_agg(pg_get_triggerdef(t.oid) ORDER BY t.tgname),'[]')::text FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND NOT t.tgisinternal",
    ];
    let mut parts = Vec::new();
    for q in queries {
        parts.push(
            sql(sqlx::query_scalar::<_, String>(q)
                .fetch_one(&mut *conn)
                .await)
            .await?,
        );
    }
    Ok(sha(parts.join("\n").as_bytes()))
}
// Same complete logical-state algorithm used by the verified Stage 1 backup manifest.
async fn reviewed_state(conn: &mut PgConnection, names: &[String]) -> Result<String> {
    let table_json = format!(
        "[{}]",
        names
            .iter()
            .map(|name| format!("[\"public\", {}]", serde_json::to_string(name).unwrap()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut parts = Vec::new();
    for name in names {
        parts.push(sql(sqlx::query_scalar::<_,String>(&format!("SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]')::text FROM {} t",qualified(name))).fetch_one(&mut *conn).await).await?);
    }
    for q in [
        "SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY table_schema,table_name,ordinal_position),'[]')::text FROM (SELECT table_schema,table_name,column_name,ordinal_position,data_type,is_nullable,column_default FROM information_schema.columns WHERE table_schema NOT LIKE 'pg_%' AND table_schema <> 'information_schema') c",
        "SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY schema,name,definition),'[]')::text FROM (SELECT n.nspname AS schema,conname AS name,pg_get_constraintdef(c.oid) AS definition FROM pg_constraint c JOIN pg_namespace n ON n.oid=c.connamespace WHERE n.nspname NOT LIKE 'pg_%' AND n.nspname <> 'information_schema') c",
        "SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY schemaname,indexname),'[]')::text FROM (SELECT schemaname,indexname,indexdef FROM pg_indexes WHERE schemaname NOT LIKE 'pg_%' AND schemaname <> 'information_schema') c",
        "SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY schemaname,sequencename),'[]')::text FROM (SELECT schemaname,sequencename,start_value,min_value,max_value,increment_by,cycle,cache_size,last_value FROM pg_sequences WHERE schemaname NOT LIKE 'pg_%' AND schemaname <> 'information_schema') c",
    ] {
        parts.push(
            sql(sqlx::query_scalar::<_, String>(q)
                .fetch_one(&mut *conn)
                .await)
            .await?,
        );
    }
    Ok(sha((table_json + &parts.join("\n")).as_bytes()))
}
enum Fault {
    Never,
    #[cfg(test)]
    FailAfterNine,
    #[cfg(test)]
    PauseAfterNine(
        std::sync::Arc<tokio::sync::Notify>,
        std::sync::Arc<tokio::sync::Notify>,
    ),
}
async fn execute(
    opts: &PgConnectOptions,
    target: &Target,
    authority: &Authority,
    p: &Plan,
    raw: &[u8],
    signature: &[u8],
    fault: Fault,
) -> Result<()> {
    for path in [&target.socket, &target.data] {
        let m = std::fs::symlink_metadata(path).map_err(|_| "pinned_storage_missing")?;
        check(
            m.is_dir() && m.uid() == unsafe { libc::geteuid() } && m.mode() & 0o077 == 0,
            "unsafe_pinned_storage",
        )?;
    }
    let mut conn = sql(PgConnection::connect_with(opts).await).await?;
    let actual:(String,String,String,String,String,String,bool)=sql(sqlx::query_as("SELECT current_database(),current_setting('data_directory'),current_setting('unix_socket_directories'),current_setting('port'),(SELECT system_identifier::text FROM pg_control_system()),session_user::text,inet_server_addr() IS NULL").fetch_one(&mut conn).await).await?;
    check(
        actual
            == (
                target.database.clone(),
                target.data.to_string_lossy().into_owned(),
                target.socket.to_string_lossy().into_owned(),
                target.port.to_string(),
                authority.system_identifier.clone(),
                authority.migrator_login.clone(),
                true,
            ),
        "server_identity_mismatch",
    )?;
    let settings:(String,String,bool)=sql(sqlx::query_as("SELECT current_setting('listen_addresses'),current_setting('default_transaction_read_only'),current_setting('fsync')='on' AND current_setting('synchronous_commit')='on' AND current_setting('full_page_writes')='on' AND current_setting('data_checksums')='on'").fetch_one(&mut conn).await).await?;
    check(
        settings == (String::new(), "on".into(), true),
        "unsafe_server_settings",
    )?;
    let roles: Vec<String> = sql(sqlx::query_scalar(
        "SELECT rolname FROM pg_roles WHERE rolname NOT LIKE 'pg_%' ORDER BY rolname",
    )
    .fetch_all(&mut conn)
    .await)
    .await?;
    check(
        roles == vec![authority.migrator_login.clone()],
        "unexpected_database_roles",
    )?;
    let grants_ok:bool=sql(sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_default_acl) AND NOT EXISTS(SELECT 1 FROM pg_hba_file_rules WHERE error IS NOT NULL OR (type='local' AND auth_method<>'peer') OR (type LIKE 'host%' AND auth_method<>'reject')) AND EXISTS(SELECT 1 FROM pg_database WHERE datname=current_database() AND datacl IS NULL AND pg_get_userbyid(datdba)=session_user)").fetch_one(&mut conn).await).await?;
    check(grants_ok, "unexpected_global_grants_or_authentication")?;
    tokio::time::timeout(std::time::Duration::from_secs(5), Migrate::lock(&mut conn))
        .await
        .map_err(|_| "migration_lock_timeout")?
        .map_err(|_| "migration_lock_failed")?;
    let mut tx = sql(conn.begin().await).await?;
    sql(sqlx::raw_sql("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE READ WRITE; SET LOCAL search_path=public; SET LOCAL timezone='UTC'; SET LOCAL statement_timeout='10s'; SET LOCAL lock_timeout='3s'; LOCK TABLE public._sqlx_migrations IN ACCESS EXCLUSIVE MODE").execute(&mut *tx).await).await?;
    let applied: Vec<(i64, Vec<u8>, bool)> = sql(sqlx::query_as(
        "SELECT version,checksum,success FROM public._sqlx_migrations ORDER BY version",
    )
    .fetch_all(&mut *tx)
    .await)
    .await?;
    check(
        applied.len() == 8
            && applied.iter().enumerate().all(|(i, (v, c, s))| {
                *v == i as i64 + 1 && *s && inventory().get(v) == Some(&hex(c))
            }),
        "starting_schema_must_be_exactly_eight",
    )?;
    let epoch:(Uuid,Uuid,String,String,String)=sql(sqlx::query_as("SELECT id,accounting_namespace,state,kind,historical_database FROM public.application_epoch FOR UPDATE").fetch_one(&mut *tx).await).await?;
    check(
        epoch
            == (
                target.epoch,
                target.namespace,
                "prepared".into(),
                "development".into(),
                "unrecovered".into(),
            ),
        "epoch_not_prepared",
    )?;
    let names = tables(&mut tx).await?;
    for name in &names {
        sql(
            sqlx::query(&format!("LOCK TABLE {} IN SHARE MODE", qualified(name)))
                .execute(&mut *tx)
                .await,
        )
        .await?;
        if name != "application_epoch" && name != "_sqlx_migrations" {
            let count: i64 = sql(sqlx::query_scalar(&format!(
                "SELECT count(*) FROM {}",
                qualified(name)
            ))
            .fetch_one(&mut *tx)
            .await)
            .await?;
            check(count == 0, "unexpected_application_or_accounting_records")?;
        }
    }
    check(
        snapshot(&mut tx, &names).await? == p.source_snapshot_sha256,
        "source_snapshot_mismatch",
    )?;
    check(
        catalog(&mut tx).await? == p.source_catalog_sha256,
        "source_security_catalog_mismatch",
    )?;
    check(
        reviewed_state(&mut tx, &names).await? == p.reviewed_state_sha256,
        "reviewed_schema_or_records_changed",
    )?;
    for migration in MIGRATIONS.iter().filter(|m| m.version >= 9) {
        sql(sqlx::raw_sql(&migration.sql).execute(&mut *tx).await).await?;
        sql(sqlx::query("INSERT INTO public._sqlx_migrations(version,description,success,checksum,execution_time) VALUES($1,$2,true,$3,-1)").bind(migration.version).bind(migration.description.as_ref()).bind(migration.checksum.as_ref()).execute(&mut *tx).await).await?;
        if migration.version == 9 {
            match &fault {
                Fault::Never => {}
                #[cfg(test)]
                Fault::FailAfterNine => {
                    sql(sqlx::query("SELECT 1/0").execute(&mut *tx).await).await?;
                }
                #[cfg(test)]
                Fault::PauseAfterNine(ready, resume) => {
                    ready.notify_one();
                    resume.notified().await;
                }
            }
        }
    }
    sql(sqlx::raw_sql("CREATE TABLE epoch_guard.migration_receipts(approval_id uuid PRIMARY KEY,principal_id uuid NOT NULL,plan_sha256 text NOT NULL,executable_sha256 text NOT NULL,source_snapshot_sha256 text NOT NULL,plan_text text NOT NULL,signature_hex text NOT NULL,authority_public_key_hex text NOT NULL,system_identifier text NOT NULL,backup_sha256 text NOT NULL,from_version bigint NOT NULL CHECK(from_version=8),to_version bigint NOT NULL CHECK(to_version=10),completed_at timestamptz NOT NULL DEFAULT clock_timestamp()); REVOKE ALL ON epoch_guard.migration_receipts FROM PUBLIC; CREATE TRIGGER migration_receipt_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON epoch_guard.migration_receipts FOR EACH STATEMENT EXECUTE FUNCTION epoch_guard.immutable()").execute(&mut *tx).await).await?;
    sql(sqlx::query("INSERT INTO epoch_guard.migration_receipts(approval_id,principal_id,plan_sha256,executable_sha256,source_snapshot_sha256,plan_text,signature_hex,authority_public_key_hex,system_identifier,backup_sha256,from_version,to_version) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,8,10)").bind(p.approval_id).bind(p.principal_id).bind(sha(raw)).bind(&p.executable_sha256).bind(&p.source_snapshot_sha256).bind(std::str::from_utf8(raw).map_err(|_|"plan_encoding")?).bind(hex(signature)).bind(&authority.public_key_hex).bind(&p.system_identifier).bind(&p.backup_sha256).execute(&mut *tx).await).await?;
    check(
        snapshot(&mut tx, &names).await? == p.source_snapshot_sha256,
        "application_records_changed",
    )?;
    let untouched:bool=sql(sqlx::query_scalar("SELECT state='prepared' AND revision=0 AND approval_id IS NULL AND (SELECT count(*) FROM epoch_transitions)=0 AND (SELECT count(*) FROM epoch_guard.approvals)=0 AND (SELECT count(*) FROM epoch_guard.authorities)=0 AND (SELECT count(*) FROM epoch_guard.executors)=0 AND (SELECT count(*) FROM epoch_guard.consumptions)=0 FROM public.application_epoch").fetch_one(&mut *tx).await).await?;
    check(untouched, "migration_created_runtime_authority")?;
    check(
        Utc::now().timestamp() < p.expires_at,
        "approval_expired_before_commit",
    )?;
    // A lost COMMIT response is ambiguous: never retry automatically; inspect schema and receipt independently.
    sql(tx.commit().await).await?;
    sql(conn.close().await).await?;
    Ok(())
}

#[cfg(test)]
mod tests;
