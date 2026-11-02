//! Only the fresh PostgreSQL 18 suite database is accepted; never an application epoch.
use sqlx::{ConnectOptions, postgres::PgConnectOptions};
use supa_domain::Error;
use supa_persistence::Store;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires isolated PostgreSQL 18 validation cluster"]
async fn staging_runtime_verifies_schema_without_migration_authority() {
    let url = std::env::var("TEST_DATABASE_URL").expect("disposable suite URL");
    let options: PgConnectOptions = url.parse().unwrap();
    assert_eq!(options.get_database(), Some("supa_pg18_suite_test"));
    let admin = Store::connect(&url).await.unwrap();
    let database = format!("supa_schema_{}_test", Uuid::new_v4().simple());
    let role = format!("supa_runtime_{}", Uuid::new_v4().simple());
    let password = Uuid::new_v4().simple().to_string();
    sqlx::query(&format!("CREATE DATABASE {database}"))
        .execute(admin.pool())
        .await
        .unwrap();
    let schema_url = url.replace("/supa_pg18_suite_test?", &format!("/{database}?"));
    assert_ne!(schema_url, url);
    let schema = Store::connect(&schema_url).await.unwrap();
    schema.migrate().await.unwrap();
    assert!(matches!(
        schema.verify_runtime_schema().await,
        Err(Error::Denied)
    ));
    sqlx::raw_sql(&format!(
        "CREATE ROLE {role} LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS PASSWORD '{password}';
         REVOKE CREATE ON SCHEMA public FROM PUBLIC;
         GRANT USAGE ON SCHEMA public TO {role};
         GRANT SELECT ON ALL TABLES IN SCHEMA public TO {role};"
    )).execute(schema.pool()).await.unwrap();
    let runtime_url = options
        .clone()
        .database(&database)
        .username(&role)
        .password(&password)
        .to_url_lossy()
        .to_string();
    let runtime = Store::connect(&runtime_url).await.unwrap();
    runtime.verify_runtime_schema().await.unwrap();
    for denied in [
        "CREATE TABLE forbidden_schema_test(id int)",
        "UPDATE _sqlx_migrations SET success=false",
        "UPDATE agents SET simulated_balance_microusd=1",
        "DELETE FROM epoch_guard.approvals",
    ] {
        assert!(sqlx::query(denied).execute(runtime.pool()).await.is_err());
    }
    sqlx::query("CREATE TABLE schema_revision_backup AS SELECT * FROM _sqlx_migrations")
        .execute(schema.pool())
        .await
        .unwrap();
    for tamper in [
        "UPDATE _sqlx_migrations SET checksum=decode('00','hex') WHERE version=13",
        "UPDATE _sqlx_migrations SET success=false WHERE version=13",
        "DELETE FROM _sqlx_migrations WHERE version=13",
        "INSERT INTO _sqlx_migrations(version,description,success,checksum,execution_time) VALUES(9999,'unreviewed',true,decode('00','hex'),0)",
    ] {
        sqlx::query(tamper).execute(schema.pool()).await.unwrap();
        assert!(matches!(
            runtime.verify_runtime_schema().await,
            Err(Error::Database)
        ));
        sqlx::raw_sql("DELETE FROM _sqlx_migrations; INSERT INTO _sqlx_migrations SELECT * FROM schema_revision_backup")
            .execute(schema.pool()).await.unwrap();
        runtime.verify_runtime_schema().await.unwrap();
    }
    sqlx::query(&format!("GRANT CREATE ON SCHEMA public TO {role}"))
        .execute(schema.pool())
        .await
        .unwrap();
    assert!(matches!(
        runtime.verify_runtime_schema().await,
        Err(Error::Denied)
    ));
    runtime.pool().close().await;
    schema.pool().close().await;
    sqlx::query(&format!("DROP DATABASE {database}"))
        .execute(admin.pool())
        .await
        .unwrap();
    sqlx::query(&format!("DROP ROLE {role}"))
        .execute(admin.pool())
        .await
        .unwrap();
    admin.pool().close().await;
}
