//! Independent migration-only 8 -> 11 profile. No runtime or activation entry point.
#[tokio::main]
async fn main() {
    match supa_migration_admin::command_eleven(std::env::args().skip(1).collect()).await {
        Ok(()) => println!("migration_only_complete_schema_11_epoch_prepared"),
        Err(code) => {
            eprintln!("migration_eleven_failed_closed:{code}");
            std::process::exit(2);
        }
    }
}
