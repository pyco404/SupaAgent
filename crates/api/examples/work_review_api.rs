//! Retained evaluation review only. No migrations, recovery, credential generation or LLM.
use axum::{
    extract::Request,
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use supa_api::{AppState, OperatorCredential, router};
use supa_domain::{Error, Result};
use supa_persistence::{Role, Store};
use supa_providers::Offline;
use supa_runtime::Runtime;
use uuid::{Uuid, uuid};
const AGENT: Uuid = uuid!("82c254ff-4659-4404-94c4-c497565567e6");
const SESSION: Uuid = uuid!("41316d3e-40b8-4b7d-8605-83db7aab36b8");
const OWNER: Uuid = uuid!("acede749-746b-4541-b178-0085434dcd39");
const HASH: &str = "01fcfc0576fb7d8cebfc09666c060eb1bff5c85a08b2682d2b17842cfe5abc42";
const DATABASE: &str = "supa_m10a_8c6356653dc2_test";
fn allowed(method: &Method, path: &str) -> bool {
    let read = method == Method::GET || method == Method::HEAD;
    (path == "/health" && read)
        || (path == format!("/agents/{AGENT}/live-work/{SESSION}/review")
            && (read || method == Method::POST))
}
async fn scope(request: Request, next: Next) -> Response {
    if !allowed(request.method(), request.uri().path()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(request).await
}
#[tokio::main]
async fn main() {
    if run().await.is_err() {
        eprintln!("work_review_startup_failed_closed");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    if std::env::args().len() != 1
        || std::path::Path::new("postgres-data/infrastructure-v1/data/postmaster.pid").exists()
    {
        return Err(Error::Denied);
    }
    // These must be supplied by a separately approved human-authentication setup.
    // Never read generation runtime.json or mint a substitute owner credential.
    let credentials: Vec<OperatorCredential> = serde_json::from_str(
        &std::env::var("SUPA_OPERATOR_CREDENTIALS").map_err(|_| Error::Denied)?,
    )
    .map_err(|_| Error::Denied)?;
    let deadlines: BTreeMap<Uuid, u64> = serde_json::from_str(
        &std::env::var("SUPA_OPERATOR_SESSION_DEADLINES").map_err(|_| Error::Denied)?,
    )
    .map_err(|_| Error::Denied)?;
    let public = std::env::var("SUPA_API_TOKEN").map_err(|_| Error::Denied)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Denied)?
        .as_secs();
    let deadline = *deadlines.get(&OWNER).ok_or(Error::Denied)?;
    if credentials.len() != 1
        || credentials[0].id != OWNER
        || credentials[0].role != Role::Owner
        || credentials[0].token == public
        || deadlines.len() != 1
        || deadline <= now
        || deadline > now + 3600
    {
        return Err(Error::Denied);
    }
    let store=Store::connect("postgresql://supa_work_review@localhost/supa_m10a_8c6356653dc2_test?host=/tmp/supa-m10a-pg-8c6356653dc2/s&port=55681").await?;
    let identity:(String,String,String,String)=sqlx::query_as("SELECT current_database(),current_setting('data_directory'),system_identifier::text,current_user FROM pg_control_system()")
        .fetch_one(store.pool()).await.map_err(|_|Error::Database)?;
    if identity
        != (
            DATABASE.into(),
            "/tmp/supa-m10a-pg-8c6356653dc2/data".into(),
            "7703596362529484555".into(),
            "supa_work_review".into(),
        )
        || store.application_epoch().await?.is_some()
    {
        return Err(Error::Denied);
    }
    let migration: bool = sqlx::query_scalar(
        "SELECT success AND checksum=sha384($1) FROM _sqlx_migrations WHERE version=13",
    )
    .bind(include_bytes!("../../../migrations/0013_work_evaluation_reviews.sql").as_slice())
    .fetch_one(store.pool())
    .await
    .map_err(|_| Error::Denied)?;
    let finance:bool=sqlx::query_scalar("SELECT has_table_privilege(current_user,'economic_entries','INSERT') OR has_table_privilege(current_user,'economic_jobs','UPDATE') OR has_table_privilege(current_user,'live_sessions','INSERT') OR has_column_privilege(current_user,'live_budgets','committed_microusd','UPDATE') OR has_column_privilege(current_user,'live_sessions','data','UPDATE')")
        .fetch_one(store.pool()).await.map_err(|_|Error::Denied)?;
    if !migration || finance {
        return Err(Error::Denied);
    }
    let context = store.work_review_context(AGENT, SESSION, OWNER).await?;
    if context["report_sha256"] != HASH {
        return Err(Error::Stale);
    }
    let state = AppState::new(
        store.clone(),
        Runtime::new(store, Arc::new(Offline)),
        public,
    )?
    .with_credentials(credentials)?
    .with_credential_deadlines(deadlines)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:55683")
        .await
        .map_err(|_| Error::Invalid)?;
    axum::serve(listener,router(state).layer(middleware::from_fn(scope))).with_graceful_shutdown(async move {
        tokio::select! {
            _=tokio::time::sleep(std::time::Duration::from_secs(deadline-now))=>{},
            _=async {if let Ok(mut s)=tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()){s.recv().await;}}=>{},
        }
    }).await.map_err(|_|Error::Invalid)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_scope_never_exposes_generation_or_other_mutation_routes() {
        let review = format!("/agents/{AGENT}/live-work/{SESSION}/review");
        assert!(allowed(&Method::GET, &review));
        assert!(allowed(&Method::POST, &review));
        for path in [
            format!("/agents/{AGENT}/live-work"),
            format!("/agents/{AGENT}/live"),
            format!("/agents/{AGENT}/live/{SESSION}/approval"),
            "/jobs".into(),
            "/agents".into(),
        ] {
            assert!(!allowed(&Method::POST, &path));
            assert!(!allowed(&Method::GET, &path));
        }
        assert!(!allowed(&Method::DELETE, &review));
        assert!(!allowed(&Method::POST, "/health"));
    }
}
