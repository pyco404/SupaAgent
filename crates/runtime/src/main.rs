//! Dedicated scheduler process. Starting the process never authorizes agent activation.
use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use supa_domain::{Error, Result};
use supa_persistence::Store;
use supa_providers::{Offline, Reasoner};
use supa_runtime::Runtime;

fn enabled(value: Option<&str>) -> Result<bool> {
    match value {
        None | Some("false") => Ok(false),
        Some("true") => Ok(true),
        _ => Err(Error::Invalid),
    }
}
fn bind_address(value: &str, private_staging: bool) -> Result<SocketAddr> {
    let address: SocketAddr = value.parse().map_err(|_| Error::Invalid)?;
    if !private_staging && !address.ip().is_loopback() {
        return Err(Error::Denied);
    }
    Ok(address)
}
fn provider(active: bool, name: &str) -> Result<Option<Arc<dyn Reasoner>>> {
    // Even configured credentials are not read or validated while globally paused.
    if !active {
        return Ok(None);
    }
    match name {
        "offline" => Ok(Some(Arc::new(Offline))),
        // Paid providers are unavailable in the staging worker in this milestone.
        "openai" => Err(Error::Denied),
        _ => Err(Error::Invalid),
    }
}
async fn ready(State(store): State<Store>) -> (StatusCode, Json<serde_json::Value>) {
    if store.ping().await.is_ok() {
        (StatusCode::OK, Json(serde_json::json!({"status":"ready"})))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status":"unavailable"})),
        )
    }
}
async fn shutdown() {
    #[cfg(unix)]
    {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = signal.recv() => {} }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter("supa_runtime=info,supa_persistence=info")
        .init();
    if let Err(error) = run().await {
        tracing::error!(error_code=%error, "worker_failed");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let active = enabled(std::env::var("SUPA_WORKER_ENABLED").ok().as_deref())?;
    let private = enabled(std::env::var("SUPA_PRIVATE_STAGING").ok().as_deref())?;
    let port = std::env::var("PORT").unwrap_or_else(|_| "8081".into());
    let bind = bind_address(
        &std::env::var("SUPA_WORKER_BIND").unwrap_or_else(|_| format!("127.0.0.1:{port}")),
        private,
    )?;
    let seconds: u64 = std::env::var("SUPA_WORKER_POLL_SECONDS")
        .unwrap_or_else(|_| "5".into())
        .parse()
        .map_err(|_| Error::Invalid)?;
    if !(1..=3600).contains(&seconds) {
        return Err(Error::Invalid);
    }
    let reasoner = provider(
        active,
        &std::env::var("SUPA_PROVIDER").unwrap_or_else(|_| "offline".into()),
    )?;
    let store = Store::connect(&std::env::var("DATABASE_URL").map_err(|_| Error::Invalid)?).await?;
    if store.application_epoch().await?.is_some() {
        return Err(Error::Denied); // Prepared epochs cannot start workers, even paused heartbeats.
    }
    store.migrate().await?;
    store.ping().await?;
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|_| Error::Invalid)?;
    let monitor =
        supa_runtime::health::WorkerMonitor::start(store.clone(), "worker", active, seconds as i64)
            .await?;
    let worker_id = monitor.id;
    let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(false);
    let task = reasoner.map(|reasoner| {
        let runtime = Runtime::new(store.clone(), reasoner).with_worker(worker_id);
        tokio::spawn(async move {
            let mut ticks = tokio::time::interval(Duration::from_secs(seconds));
            ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = stop_rx.changed() => break,
                    _ = ticks.tick() => if let Err(error) = runtime.tick().await {
                        tracing::warn!(error_code=%error, "worker_tick_failed");
                    }
                }
            }
        })
    });
    tracing::info!(
        paused = !active,
        economic_mode = "simulation",
        "worker_ready"
    );
    let result = axum::serve(
        listener,
        Router::new().route("/ready", get(ready)).with_state(store),
    )
    .with_graceful_shutdown(async move {
        shutdown().await;
        let _ = stop_tx.send(true);
    })
    .await
    .map_err(|_| Error::Invalid);
    // A canceled in-flight cycle remains fenced by its persisted lease and recovers after expiry.
    if let Some(task) = task {
        task.abort();
        let _ = task.await;
    }
    monitor.shutdown().await?;
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paused_by_default_never_constructs_a_provider() {
        assert!(!enabled(None).unwrap());
        assert!(!enabled(Some("false")).unwrap());
        assert!(enabled(Some("true")).unwrap());
        assert!(enabled(Some("yes")).is_err());
        assert!(
            provider(false, "invalid-provider-with-no-credentials")
                .unwrap()
                .is_none()
        );
        assert!(provider(true, "offline").unwrap().is_some());
        assert!(provider(true, "invalid").is_err());
        assert!(provider(true, "openai").is_err());
    }
    #[test]
    fn listener_requires_explicit_private_network_opt_in() {
        assert!(bind_address("127.0.0.1:8081", false).is_ok());
        assert!(bind_address("0.0.0.0:8081", false).is_err());
        assert!(bind_address("0.0.0.0:8081", true).is_ok());
        assert!(bind_address("[::]:8081", true).is_ok());
        assert!(bind_address("invalid", true).is_err());
    }
}
