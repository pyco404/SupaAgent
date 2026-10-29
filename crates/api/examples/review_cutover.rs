//! Disposable-database review exercise only; never targets the prepared development candidate.
use supa_domain::{Error, Result};
use supa_persistence::Store;
use uuid::Uuid;
#[tokio::main]
async fn main() {
    if run().await.is_err() {
        eprintln!("cutover_review_failed_closed");
        std::process::exit(2);
    }
}
async fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2
        || args[0] != "--exercise-disposable"
        || !matches!(
            args[1].as_str(),
            "activate_read_only" | "rollback" | "migrate"
        )
    {
        return Err(Error::Denied);
    }
    let store = Store::connect(&std::env::var("DATABASE_URL").map_err(|_| Error::Invalid)?).await?;
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(store.pool())
        .await
        .map_err(|_| Error::Database)?;
    if !database.starts_with("supa_cutover_test_") {
        return Err(Error::Denied);
    }
    if args[1] == "migrate" {
        store.migrate().await?;
        println!("disposable_review_schema_prepared");
        return Ok(());
    }
    let id = std::env::var("SUPA_REVIEW_EPOCH_ID")
        .map_err(|_| Error::Invalid)?
        .parse()
        .map_err(|_| Error::Invalid)?;
    let actor: Uuid = std::env::var("SUPA_REVIEW_ACTOR_ID")
        .map_err(|_| Error::Invalid)?
        .parse()
        .map_err(|_| Error::Invalid)?;
    let approval = std::env::var("SUPA_REVIEW_APPROVAL_ID")
        .map_err(|_| Error::Invalid)?
        .parse()
        .map_err(|_| Error::Invalid)?;
    let revision = std::env::var("SUPA_REVIEW_REVISION")
        .map_err(|_| Error::Invalid)?
        .parse()
        .map_err(|_| Error::Invalid)?;
    let digest = std::env::var("SUPA_REVIEW_PLAN_SHA256").map_err(|_| Error::Invalid)?;
    let epoch = store
        .exercise_epoch_transition(id, actor, approval, revision, &args[1], &digest)
        .await?;
    println!(
        "{}",
        serde_json::to_string(&epoch).map_err(|_| Error::Invalid)?
    );
    Ok(())
}
