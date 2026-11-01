//! Exact M10A disposable target only; passive HTTP validation, never a paid runner.
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

async fn reads_only(request: Request, next: Next) -> Response {
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(request).await
}
#[tokio::main]
async fn main() {
    if run().await.is_err() {
        eprintln!("readonly_evaluation_preflight_failed_closed");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    if std::path::Path::new("postgres-data/infrastructure-v1/data/postmaster.pid").exists() {
        return Err(Error::Denied);
    }
    // No configurable DB target, migrations, recovery, monitor, provider or dotenv loader.
    let url = "postgresql://localhost/supa_m10a_0b780645f120_test?host=/tmp/supa-m10a-pg-0b780645f120/s&port=55681&options=-c%20default_transaction_read_only%3Don";
    let store = Store::connect(url).await?;
    let identity: (String, String, String, String) = sqlx::query_as("SELECT current_database(),current_setting('data_directory'),system_identifier::text,current_setting('default_transaction_read_only') FROM pg_control_system()")
        .fetch_one(store.pool()).await.map_err(|_| Error::Invalid)?;
    if identity
        != (
            "supa_m10a_0b780645f120_test".into(),
            "/tmp/supa-m10a-pg-0b780645f120/data".into(),
            "7703020438665878846".into(),
            "on".into(),
        )
    {
        return Err(Error::Denied);
    }
    let actor = "28092c72-3475-48fa-b703-a68728ea4fa4"
        .parse()
        .map_err(|_| Error::Invalid)?;
    let token = std::env::var("SUPA_PREFLIGHT_TEST_TOKEN").map_err(|_| Error::Invalid)?;
    let public = std::env::var("SUPA_PREFLIGHT_SERVICE_TOKEN").map_err(|_| Error::Invalid)?;
    if token == public {
        return Err(Error::Denied);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Invalid)?
        .as_secs();
    let state = AppState::new(
        store.clone(),
        Runtime::new(store, Arc::new(Offline)),
        public,
    )?
    .with_credentials(vec![OperatorCredential {
        id: actor,
        role: Role::Owner,
        token,
    }])?
    .with_credential_deadlines(BTreeMap::from([(actor, now + 120)]))?;
    // Full existing router, but all mutations denied before dispatch; live work remains disabled.
    let app = router(state).layer(middleware::from_fn(reads_only));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:55682")
        .await
        .map_err(|_| Error::Invalid)?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            if let Ok(mut signal) =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            {
                signal.recv().await;
            }
        })
        .await
        .map_err(|_| Error::Invalid)
}
