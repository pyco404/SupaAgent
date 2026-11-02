//! CI-only schema bootstrap. Never targets an application epoch or arbitrary database.
use supa_domain::{Error, Result};
use supa_persistence::Store;

async fn bootstrap() -> Result<()> {
    if std::env::var("SUPA_DISPOSABLE_SCHEMA").as_deref() != Ok("true") {
        return Err(Error::Denied);
    }
    let store = Store::connect(&std::env::var("DATABASE_URL").map_err(|_| Error::Invalid)?).await?;
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(store.pool())
        .await
        .map_err(|_| Error::Database)?;
    if !matches!(database.as_str(), "supa_ci_test" | "supa_pg18_compat_test")
        || store.application_epoch().await?.is_some()
    {
        return Err(Error::Denied);
    }
    store.migrate().await
}

#[tokio::main]
async fn main() {
    if let Err(error) = bootstrap().await {
        eprintln!("disposable_schema_bootstrap_failed: {error}");
        std::process::exit(1);
    }
    println!("disposable_schema_bootstrap_complete");
}
