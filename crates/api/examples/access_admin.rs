//! Explicit local ownership provisioning. Does not activate agents or grant execution.
use supa_domain::{Error, Result};
use supa_persistence::Store;
use uuid::Uuid;
#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!(
            "{}",
            serde_json::json!({"error":error.to_string(),"ownership_changed":false})
        );
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 || args[0] != "--authorize-local-ownership" {
        return Err(Error::Denied);
    }
    let agent: Uuid = args[1].parse().map_err(|_| Error::Invalid)?;
    let owner: Uuid = args[2].parse().map_err(|_| Error::Invalid)?;
    let url = std::env::var("DATABASE_URL").map_err(|_| Error::Invalid)?;
    let options: sqlx::postgres::PgConnectOptions = url.parse().map_err(|_| Error::Invalid)?;
    if !matches!(options.get_host(), "127.0.0.1" | "localhost" | "::1") {
        return Err(Error::Denied);
    }
    let store = Store::connect(&url).await?;
    match store.wallet(agent).await {
        Ok(wallet) if wallet.owner_id != owner => return Err(Error::Denied),
        Err(Error::NotFound) | Ok(_) => {}
        Err(error) => return Err(error),
    }
    store.establish_owner(agent, owner).await?;
    println!(
        "{}",
        serde_json::json!({"ownership_changed":true,"agent_id":agent,"owner_id":owner,"execution_started":false})
    );
    Ok(())
}
