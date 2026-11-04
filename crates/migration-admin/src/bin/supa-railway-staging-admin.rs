//! Owner-executed, fresh Railway staging bootstrap. Never invoked by API/worker.
use chrono::Utc;
use ring::digest::{SHA256, digest};
use serde::{Deserialize, Serialize};
use sqlx::{
    Connection, PgConnection, Row,
    migrate::Migrator,
    postgres::{PgConnectOptions, PgSslMode},
};
use std::{
    collections::BTreeMap,
    env,
    fs::OpenOptions,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
type Result<T> = std::result::Result<T, &'static str>;
const PROJECT: &str = "99648d2a-69a1-4d8c-98d0-7f2a26543c9c";
const ENVIRONMENT: &str = "ff0b2092-e42b-4a89-ae25-441ab95d92d8";
const SERVICE: &str = "77352ca9-435e-4870-b092-58365310c0f3";
const APP: &str = "bfc29f500af08e0e2cc4676b8950b5b17f26007a";
const DATABASE: &str = "supa_staging";
const HOST: &str = "supa-postgres.railway.internal";
const ADMIN: &str = "supa_staging_admin";
const MIGRATOR: &str = "supa_staging_migration";
const RUNTIME: &str = "supa_staging_runtime";
const CERT: &str = "/var/lib/postgresql/data/supa-tls/ca.crt";
static MIGRATIONS: Migrator = sqlx::migrate!("../../migrations");
const CHECKSUMS: &[(i64, &str)] = &[
    (
        1,
        "a5e942c425b73c9a306af8031ccf03bc854bf8d53a0c5e49ba610b6afb2d8f9cfaaeae0558f5b5c87acfa1b411e44ab0",
    ),
    (
        2,
        "67120ef19c4d2e95e776c7b13b1daee4c7ab705eec3db2b133352927a0bc0b644ffa04efc62c66577f85c6f636c58724",
    ),
    (
        3,
        "b6f2e1960a868d3b9d68810ed76b4d98e0fe18d548e11922444fa720cbbdfa542b297d6657d8b35756e0c96512460023",
    ),
    (
        4,
        "e1511eeb20f04b3d493d55f8626cdbb5a2306ef1ecc7cc6f882da79c635b80b226e26d62278d87337db8300527366fd5",
    ),
    (
        5,
        "cb69ecb87c22c13db079a27b4b019c229839989e29e7dee88e0d00bda0c87e9ca252661feb53ef7658349f39cedf140a",
    ),
    (
        6,
        "092333ed058ea9059eeef58a640528c2b6fb5efe813d910c54c324d4f9e70874c8514d992515b8fc316c38b17f223319",
    ),
    (
        7,
        "375701d347ef15b7468dbf2c919090993577172c6a376e8f3155745c1604638fee9d55cf427a9bfa664ad13427357ed0",
    ),
    (
        8,
        "d66772a21ad6a5cc373535498e68336efcf4f4dc214e20522ae4da3b0b0ac696fa2bb90b096bf6b950126232e0999872",
    ),
    (
        9,
        "81c5ef7a9e5a41f1bbc9831ab3b0e6d43649f19ba21676ddbc6fa715d9952b043d2a653e664f565073ecdb3a4804f7ac",
    ),
    (
        10,
        "d63d9683d3defa051985cf9f2aa020554defd86364589711063fe849cf7c7690c6c7d97ebcf3cad74b9ad83a90ed8516",
    ),
    (
        11,
        "67b078e30d8701cec610d7f902960fc2745ee74873fa81437ee0a9b5981f055798ced66b9ff59bb13168460bcc62b183",
    ),
    (
        12,
        "7c46324057e8c04afd0cf74f7be1968f2fa65b35f50d138dac5f636d775bd0a0ba2d7bbeeb36031bf65dcaa31a853874",
    ),
    (
        13,
        "6add4696c45a4961b66cdba39567068e7a5117db1b2682d54adc3d3c2c5b9b4d0326f20f681cdfd3365d3c6215cd37eb",
    ),
];
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    version: u32,
    project_id: String,
    environment_id: String,
    service_id: String,
    database: String,
    application_revision: String,
    system_identifier: String,
    operation: String,
    issued_at: i64,
    expires_at: i64,
    migration_sha384: BTreeMap<i64, String>,
}
fn check(ok: bool, code: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(code) }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn inventory() -> BTreeMap<i64, String> {
    CHECKSUMS.iter().map(|(v, h)| (*v, h.to_string())).collect()
}
fn validate(s: &Scope, now: i64) -> Result<()> {
    check(
        s.version == 1
            && s.project_id == PROJECT
            && s.environment_id == ENVIRONMENT
            && s.service_id == SERVICE
            && s.database == DATABASE
            && s.application_revision == APP
            && s.operation == "fresh_staging_schema_13",
        "scope_target_mismatch",
    )?;
    check(
        s.system_identifier.len() >= 16 && s.system_identifier.bytes().all(|c| c.is_ascii_digit()),
        "cluster_identity_required",
    )?;
    check(
        s.issued_at <= now
            && now < s.expires_at
            && s.expires_at - s.issued_at <= 1800
            && s.expires_at > s.issued_at,
        "scope_expired_or_unbounded",
    )?;
    check(
        s.migration_sha384 == inventory(),
        "approved_checksums_mismatch",
    )?;
    let actual: BTreeMap<_, _> = MIGRATIONS
        .iter()
        .map(|m| (m.version, hex(&m.checksum)))
        .collect();
    check(
        actual == inventory(),
        "compiled_migrations_differ_from_frozen_source",
    )
}
fn load(path: &Path, hash: &str) -> Result<Scope> {
    let mut f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| "scope_unavailable")?;
    let m = f.metadata().map_err(|_| "scope_metadata")?;
    check(
        m.is_file()
            && m.nlink() == 1
            && m.mode() & 0o077 == 0
            && m.uid() == unsafe { libc::geteuid() }
            && m.len() <= 8192,
        "unsafe_scope_file",
    )?;
    let mut raw = Vec::new();
    f.read_to_end(&mut raw).map_err(|_| "scope_read")?;
    check(
        hex(digest(&SHA256, &raw).as_ref()) == hash,
        "scope_hash_mismatch",
    )?;
    let scope = serde_json::from_slice(&raw).map_err(|_| "scope_json")?;
    validate(&scope, Utc::now().timestamp())?;
    Ok(scope)
}
fn connection_failure(error: sqlx::Error) -> &'static str {
    match error {
        sqlx::Error::Tls(ref tls) if tls.to_string().contains("CaUsedAsEndEntity") => {
            "tls_ca_used_as_server_certificate"
        }
        sqlx::Error::Tls(ref tls) if tls.to_string().contains("NotValidForName") => {
            "tls_server_hostname_mismatch"
        }
        sqlx::Error::Tls(_) => "tls_certificate_or_handshake_rejected",
        sqlx::Error::Io(ref io) if io.to_string().contains("CaUsedAsEndEntity") => {
            "tls_ca_used_as_server_certificate"
        }
        sqlx::Error::Io(ref io) if io.to_string().contains("NotValidForName") => {
            "tls_server_hostname_mismatch"
        }
        sqlx::Error::Io(ref io) if io.to_string().contains("UnknownIssuer") => "tls_unknown_issuer",
        sqlx::Error::Io(ref io) if io.to_string().contains("certificate") => {
            "tls_certificate_rejected_during_io"
        }
        sqlx::Error::Io(ref io) if io.kind() == std::io::ErrorKind::ConnectionRefused => {
            "database_connection_refused"
        }
        sqlx::Error::Io(ref io) if io.kind() == std::io::ErrorKind::TimedOut => {
            "database_connection_timeout"
        }
        sqlx::Error::Io(ref io) if io.to_string().contains("Name or service not known") => {
            "database_hostname_resolution_failed"
        }
        sqlx::Error::Io(_) => "database_network_connection_failed",
        sqlx::Error::Database(ref database) if database.code().as_deref() == Some("28P01") => {
            "database_credential_rejected"
        }
        _ => "tls_database_connection_failed",
    }
}
async fn connect(user: &str, variable: &str) -> Result<PgConnection> {
    let password = env::var(variable).map_err(|_| "existing_credential_missing")?;
    check(
        (32..=256).contains(&password.len())
            && password
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'),
        "credential_format",
    )?;
    let options = PgConnectOptions::new()
        .host(HOST)
        .port(5432)
        .database(DATABASE)
        .username(user)
        .password(&password)
        .ssl_mode(PgSslMode::VerifyFull)
        .ssl_root_cert(CERT);
    let mut c = PgConnection::connect_with(&options)
        .await
        .map_err(connection_failure)?;
    let row=sqlx::query("SELECT current_database() AS db,current_user AS login,(SELECT ssl FROM pg_stat_ssl WHERE pid=pg_backend_pid()) AS ssl").fetch_one(&mut c).await.map_err(|_|"connection_identity")?;
    check(
        row.get::<String, _>("db") == DATABASE
            && row.get::<String, _>("login") == user
            && row.get::<bool, _>("ssl"),
        "connection_identity_mismatch",
    )?;
    Ok(c)
}
async fn sql(c: &mut PgConnection, query: &str) -> Result<()> {
    sqlx::raw_sql(query)
        .execute(c)
        .await
        .map_err(|_| "database_operation_failed")?;
    Ok(())
}
async fn history(c: &mut PgConnection) -> Result<()> {
    let rows = sqlx::query(
        "SELECT version,checksum,success FROM public._sqlx_migrations ORDER BY version",
    )
    .fetch_all(c)
    .await
    .map_err(|_| "migration_history_read")?;
    check(rows.len() == 13, "migration_count")?;
    for (row, (version, expected)) in rows.iter().zip(CHECKSUMS) {
        check(
            row.get::<i64, _>("version") == *version
                && hex(&row.get::<Vec<u8>, _>("checksum")) == *expected
                && row.get::<bool, _>("success"),
            "migration_history_mismatch",
        )?;
    }
    Ok(())
}
async fn execute(scope: &Scope) -> Result<()> {
    validate(scope, Utc::now().timestamp())?;
    let mut admin = connect(ADMIN, "POSTGRES_PASSWORD").await?;
    let row=sqlx::query("SELECT system_identifier::text AS id,current_setting('data_directory') AS data,current_setting('server_version_num') AS version,current_setting('log_statement') AS logging,current_setting('password_encryption') AS encryption FROM pg_control_system()").fetch_one(&mut admin).await.map_err(|_|"cluster_identity")?;
    check(
        row.get::<String, _>("id") == scope.system_identifier
            && row.get::<String, _>("data") == "/var/lib/postgresql/data/pgdata"
            && row.get::<String, _>("version") == "180006"
            && row.get::<String, _>("logging") == "none"
            && row.get::<String, _>("encryption") == "scram-sha-256",
        "cluster_identity_or_security_mismatch",
    )?;
    let tables:i64=sqlx::query_scalar("SELECT count(*) FROM pg_tables WHERE schemaname NOT IN ('pg_catalog','information_schema')").fetch_one(&mut admin).await.map_err(|_|"fresh_state_read")?;
    let extra: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pg_roles WHERE rolcanlogin AND rolname<>$1")
            .bind(ADMIN)
            .fetch_one(&mut admin)
            .await
            .map_err(|_| "role_state_read")?;
    check(
        tables == 0 && extra == 0,
        "fresh_database_or_role_state_mismatch",
    )?;
    let mut passwords = Vec::new();
    for name in [
        "SUPA_STAGING_MIGRATION_PASSWORD",
        "SUPA_STAGING_RUNTIME_PASSWORD",
    ] {
        let p = env::var(name).map_err(|_| "existing_credential_missing")?;
        check(
            (32..=256).contains(&p.len())
                && p.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'),
            "credential_format",
        )?;
        passwords.push(p);
    }
    check(
        passwords[0] != passwords[1]
            && passwords
                .iter()
                .all(|p| env::var("POSTGRES_PASSWORD").as_ref() != Ok(p)),
        "credentials_not_separate",
    )?;
    // This file is owner-only and database-specific; a second invocation must stop.
    let marker = Path::new("/tmp/supa-staging-bootstrap-consumed");
    let _marker = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(marker)
        .map_err(|_| "bootstrap_already_attempted")?;
    sql(
        &mut admin,
        "SET log_min_error_statement=panic; SET log_min_duration_statement=-1; BEGIN; SELECT pg_advisory_xact_lock(935017013);",
    )
    .await?;
    for (role, password) in [MIGRATOR, RUNTIME].into_iter().zip(&passwords) {
        sql(&mut admin,&format!("CREATE ROLE {role} LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{password}';")).await?;
    }
    sql(&mut admin,"REVOKE CREATE ON SCHEMA public FROM PUBLIC; REVOKE TEMPORARY ON DATABASE supa_staging FROM PUBLIC; GRANT CONNECT,CREATE ON DATABASE supa_staging TO supa_staging_migration; GRANT USAGE,CREATE ON SCHEMA public TO supa_staging_migration; GRANT CONNECT ON DATABASE supa_staging TO supa_staging_runtime; COMMIT;").await?;
    let mut migration = connect(MIGRATOR, "SUPA_STAGING_MIGRATION_PASSWORD").await?;
    validate(scope, Utc::now().timestamp())?;
    MIGRATIONS
        .run(&mut migration)
        .await
        .map_err(|_| "sqlx_migration_failed_no_retry")?;
    history(&mut migration).await?;
    sql(&mut admin,"GRANT USAGE ON SCHEMA public TO supa_staging_runtime; GRANT SELECT ON ALL TABLES IN SCHEMA public TO supa_staging_runtime; GRANT INSERT,UPDATE ON worker_health TO supa_staging_runtime; GRANT UPDATE ON live_sessions TO supa_staging_runtime;").await?;
    let mut runtime = connect(RUNTIME, "SUPA_STAGING_RUNTIME_PASSWORD").await?;
    history(&mut runtime).await?;
    let forbidden:bool=sqlx::query_scalar("SELECT rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls OR rolinherit OR has_database_privilege(current_user,current_database(),'CREATE') OR has_database_privilege(current_user,current_database(),'TEMP') OR has_schema_privilege(current_user,'public','CREATE') OR has_table_privilege(current_user,'public._sqlx_migrations','INSERT,UPDATE,DELETE,TRUNCATE') FROM pg_roles WHERE rolname=current_user").fetch_one(&mut runtime).await.map_err(|_|"runtime_permissions_read")?;
    check(!forbidden, "runtime_permissions_excessive")?;
    for command in [
        "CREATE TABLE public.forbidden_staging(id int)",
        "CREATE SCHEMA forbidden_staging",
        "UPDATE public._sqlx_migrations SET success=false WHERE false",
        "UPDATE agents SET simulated_balance_microusd=1 WHERE false",
        "INSERT INTO simulated_ledger(id) VALUES(NULL)",
        "DELETE FROM epoch_guard.approvals WHERE false",
        "DELETE FROM economic_holds WHERE false",
    ] {
        sql(&mut runtime, "BEGIN").await?;
        let denied = sqlx::raw_sql(command).execute(&mut runtime).await;
        let code = denied
            .as_ref()
            .err()
            .and_then(|e| e.as_database_error())
            .and_then(|e| e.code());
        check(
            code.as_deref() == Some("42501"),
            "expected_permission_denial_missing",
        )?;
        sql(&mut runtime, "ROLLBACK").await?;
    }
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM application_epoch")
        .fetch_one(&mut admin)
        .await
        .map_err(|_| "epoch_state_read")?;
    check(rows == 0, "unexpected_epoch_created")?;
    println!(
        "{{\"status\":\"schema_13_bootstrap_passed\",\"migrations\":13,\"runtime_permission_denials\":7,\"epoch_rows\":0,\"seeded\":false}}"
    );
    Ok(())
}
#[tokio::main]
async fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let result = async {
        check(
            args.len() == 3
                && matches!(
                    args[0].as_str(),
                    "--authorize-fresh-staging-bootstrap" | "--verify-staging-tls"
                ),
            "explicit_scope_required",
        )?;
        let scope = load(Path::new(&args[1]), &args[2])?;
        if args[0] == "--verify-staging-tls" {
            let mut connection = connect(ADMIN, "POSTGRES_PASSWORD").await?;
            sql(&mut connection, "SET default_transaction_read_only=on").await?;
            println!("staging_readonly_tls_verified");
            Ok(())
        } else {
            execute(&scope).await
        }
    }
    .await;
    if let Err(code) = result {
        eprintln!("staging_bootstrap_failed_closed:{code}");
        std::process::exit(2);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn scope() -> Scope {
        Scope {
            version: 1,
            project_id: PROJECT.into(),
            environment_id: ENVIRONMENT.into(),
            service_id: SERVICE.into(),
            database: DATABASE.into(),
            application_revision: APP.into(),
            system_identifier: "7704432314739101731".into(),
            operation: "fresh_staging_schema_13".into(),
            issued_at: 100,
            expires_at: 200,
            migration_sha384: inventory(),
        }
    }
    #[test]
    fn exact_fresh_scope_and_frozen_checksums_pass() {
        assert!(validate(&scope(), 150).is_ok());
    }
    #[test]
    fn cross_database_project_service_and_revision_fail() {
        for key in 0..5 {
            let mut s = scope();
            match key {
                0 => s.database = "supa_ci_test".into(),
                1 => s.service_id = "other".into(),
                2 => s.project_id = "other".into(),
                3 => s.environment_id = "other".into(),
                _ => s.application_revision = "other".into(),
            };
            assert!(validate(&s, 150).is_err());
        }
    }
    #[test]
    fn expired_future_or_unbounded_scope_fails() {
        let s = scope();
        assert!(validate(&s, 99).is_err());
        assert!(validate(&s, 200).is_err());
        let mut s = scope();
        s.expires_at = 2000;
        assert!(validate(&s, 150).is_err());
    }
    #[test]
    fn changed_or_missing_migrations_fail() {
        let mut s = scope();
        s.migration_sha384.remove(&13);
        assert!(validate(&s, 150).is_err());
        let mut s = scope();
        s.migration_sha384.insert(1, "wrong".into());
        assert!(validate(&s, 150).is_err());
    }
    #[test]
    fn scope_json_rejects_unknown_fields() {
        let mut value = serde_json::to_value(scope()).unwrap();
        value["unapproved"] = serde_json::json!(true);
        assert!(serde_json::from_value::<Scope>(value).is_err());
    }
    #[test]
    fn tls_errors_are_specific_without_exposing_source_text() {
        let error = sqlx::Error::Io(std::io::Error::other(
            "synthetic-private-value: CaUsedAsEndEntity",
        ));
        assert_eq!(
            connection_failure(error),
            "tls_ca_used_as_server_certificate"
        );
        let error = sqlx::Error::Io(std::io::Error::other("synthetic-private-value"));
        assert_eq!(
            connection_failure(error),
            "database_network_connection_failed"
        );
    }
    #[test]
    fn scope_files_require_private_permissions_hash_and_no_symlinks() {
        use std::io::Write;
        use std::os::unix::fs::{PermissionsExt, symlink};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("scope.json");
        let mut current = scope();
        current.issued_at = Utc::now().timestamp() - 1;
        current.expires_at = current.issued_at + 100;
        let raw = serde_json::to_vec(&current).unwrap();
        let hash = hex(digest(&SHA256, &raw).as_ref());
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        file.write_all(&raw).unwrap();
        drop(file);
        assert!(load(&path, &hash).is_ok());
        assert!(load(&path, "wrong").is_err());
        let link = directory.path().join("link");
        symlink(&path, &link).unwrap();
        assert!(load(&link, &hash).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load(&path, &hash).is_err());
    }
}
