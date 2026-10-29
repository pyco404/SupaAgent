//! Prepare one empty development database. No server, credentials, ownership or activation.
use supa_domain::{Error, Result};
use supa_persistence::Store;
use uuid::Uuid;
#[tokio::main]
async fn main() {
    if run().await.is_err() {
        eprintln!("epoch_preparation_failed_no_activation");
        std::process::exit(2);
    }
}
async fn run() -> Result<()> {
    if std::env::args().skip(1).collect::<Vec<_>>() != ["--prepare-development-epoch"] {
        return Err(Error::Denied);
    }
    let id: Uuid = std::env::var("SUPA_PREPARE_EPOCH_ID")
        .map_err(|_| Error::Invalid)?
        .parse()
        .map_err(|_| Error::Invalid)?;
    let namespace: Uuid = std::env::var("SUPA_ACCOUNTING_NAMESPACE")
        .map_err(|_| Error::Invalid)?
        .parse()
        .map_err(|_| Error::Invalid)?;
    if id.is_nil() || namespace.is_nil() || id == namespace {
        return Err(Error::Invalid);
    }
    let store = Store::connect(&std::env::var("DATABASE_URL").map_err(|_| Error::Invalid)?).await?;
    let name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(store.pool())
        .await
        .map_err(|_| Error::Database)?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_tables WHERE schemaname NOT LIKE 'pg_%' AND schemaname<>'information_schema'").fetch_one(store.pool()).await.map_err(|_| Error::Database)?;
    if name != format!("supa_epoch_dev_{}", id.simple()) || count != 0 {
        return Err(Error::Denied);
    }
    store.migrate().await?;
    sqlx::query("INSERT INTO application_epoch(id,accounting_namespace,label,kind,state,historical_database) VALUES($1,$2,'Fresh development epoch','development','prepared','unrecovered')")
        .bind(id).bind(namespace).execute(store.pool()).await.map_err(|_| Error::Database)?;
    println!("prepared_development_epoch_not_activated");
    Ok(())
}
