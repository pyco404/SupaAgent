//! Migration-only process. No API, worker, inference, or application runtime dependencies.
#[tokio::main]
async fn main() {
    match supa_migration_admin::command(std::env::args().skip(1).collect()).await {
        Ok(()) => println!("migration_only_complete_schema_10_epoch_prepared"),
        Err(code) => {
            eprintln!("migration_only_failed_closed:{code}");
            std::process::exit(2);
        }
    }
}
