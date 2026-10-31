//! Independent migration-only 8 -> 12. No credential or operational authorization issuance.
#[tokio::main]
async fn main() {
    match supa_migration_admin::command_twelve(std::env::args().skip(1).collect()).await {
        Ok(()) => println!("migration_only_complete_schema_12_epoch_prepared"),
        Err(code) => {
            eprintln!("migration_twelve_failed_closed:{code}");
            std::process::exit(2);
        }
    }
}
