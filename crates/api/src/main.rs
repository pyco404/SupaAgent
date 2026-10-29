use std::{sync::Arc, time::Duration};
use supa_api::{AppState, local_address, router};
use supa_domain::{Error, Result};
use supa_persistence::Store;
use supa_providers::{Offline, Reasoner, RoutedOpenAi};
use supa_runtime::Runtime;

#[tokio::main]
async fn main() {
    // Restrict targets: HTTP clients and SQL libraries cannot emit request/connection details.
    tracing_subscriber::fmt()
        .json()
        .with_env_filter("supa_api=info,supa_runtime=info,supa_persistence=info")
        .init();
    if let Err(error) = run().await {
        tracing::error!(error_code=%error,"startup_or_server_failed");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let private_staging = flag("SUPA_PRIVATE_STAGING", false)?;
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let bind = server_address(
        &std::env::var("SUPA_BIND").unwrap_or_else(|_| format!("127.0.0.1:{port}")),
        private_staging,
    )?;
    let scheduler_enabled = flag("SUPA_API_SCHEDULER_ENABLED", false)?;
    let token = std::env::var("SUPA_API_TOKEN").map_err(|_| Error::Invalid)?;
    let provider = std::env::var("SUPA_PROVIDER").unwrap_or_else(|_| "offline".into());
    let reasoner = configured_provider(&provider, private_staging)?;
    let url = std::env::var("DATABASE_URL").map_err(|_| Error::Invalid)?;
    let store = Store::connect(&url).await?;
    store.migrate().await?;
    store.recover().await?;
    let runtime = Runtime::new(store.clone(), reasoner);
    let mut state = AppState::new(store.clone(), runtime, token)?;
    if let Ok(id) = std::env::var("SUPA_OPERATOR_ID") {
        state = state.with_operator(id.parse().map_err(|_| Error::Invalid)?)?;
    }
    if let Ok(value) = std::env::var("SUPA_OPERATOR_CREDENTIALS") {
        state =
            state.with_credentials(serde_json::from_str(&value).map_err(|_| Error::Invalid)?)?;
    }
    if flag("SUPA_CONTROLLED_INFERENCE", false)? {
        if scheduler_enabled || provider != "offline" {
            return Err(Error::Denied);
        }
        state = state.with_controlled_inference(supa_runtime::controlled::ControlledInference {
            store: store.clone(),
            provider: Arc::new(supa_providers::OpenAi::controlled_from_env()?),
        });
    }
    store.recover_live_sessions().await?;
    state.wallet_read_operator = std::env::var("SUPA_WALLET_READ_OPERATOR_ID")
        .ok()
        .map(|value| value.parse().map_err(|_| Error::Invalid))
        .transpose()?;
    if std::env::var("SUPA_WALLET_ENABLED").as_deref() == Ok("true") {
        let operator = std::env::var("SUPA_WALLET_OPERATOR_ID")
            .map_err(|_| Error::Invalid)?
            .parse()
            .map_err(|_| Error::Invalid)?;
        let root = std::env::var("SUPA_WALLET_VAULT_DIR").map_err(|_| Error::Invalid)?;
        let master = std::env::var("SUPA_WALLET_MASTER_FILE").map_err(|_| Error::Invalid)?;
        let endpoint = std::env::var("SUPA_SOLANA_RPC_URL").map_err(|_| Error::Invalid)?;
        let vault = supa_providers::vault::KeyVault::from_files(
            std::path::Path::new(&root),
            std::path::Path::new(&master),
        )?;
        let rpc = supa_providers::solana::HttpSolanaRpc::new(
            &endpoint,
            supa_domain::SolanaNetwork::Devnet,
        )?;
        state.wallets = Some(supa_runtime::wallet::WalletService::new(
            store,
            vault,
            Arc::new(rpc),
            operator,
        ));
    }
    let wallet_poll = std::env::var("SUPA_WALLET_POLL_SECONDS")
        .ok()
        .map(|v| v.parse::<u64>().map_err(|_| Error::Invalid))
        .transpose()?;
    if wallet_poll.is_some_and(|v| v < 30) || wallet_poll.is_some() && state.wallets.is_none() {
        return Err(Error::Invalid);
    }
    let monitor = supa_runtime::health::WorkerMonitor::start(
        state.store.clone(),
        "api",
        scheduler_enabled,
        1,
    )
    .await?;
    let mut scheduler_state = state.clone();
    scheduler_state.runtime = scheduler_state.runtime.with_worker(monitor.id);
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
    let mut wallet_shutdown = shutdown_rx.clone();
    let wallet_state = state.clone();
    let wallet_scheduler=wallet_poll.map(|seconds|tokio::spawn(async move {
        let mut interval=tokio::time::interval(Duration::from_secs(seconds));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {tokio::select! {
            _=wallet_shutdown.changed()=>break,
            _=interval.tick()=>{
                if let Some(service)=&wallet_state.wallets {
                    match wallet_state.store.list().await {
                        Ok(agents)=>for agent in agents {
                            if let Ok(wallet)=wallet_state.store.wallet(agent.id).await
                                && wallet.owner_id==service.operator()
                                && let Err(error)=service.sync(agent.id,service.operator()).await {
                                tracing::warn!(agent_id=%agent.id,error_code=%error,"wallet_monitor_failed");
                            }
                        },
                        Err(error)=>tracing::warn!(error_code=%error,"wallet_monitor_failed"),
                    }
                }
            }
        }}
    }));
    let scheduler = scheduler_enabled.then(|| {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _=shutdown_rx.changed()=>break,
                    _=interval.tick()=>{
                        if let Ok(_permit)=scheduler_state.gate.try_acquire()
                            && let Err(e)=scheduler_state.runtime.tick().await {
                            tracing::warn!(error_code=%e,"scheduler_failed");
                        }
                    }
                }
            }
        })
    });
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|_| Error::Invalid)?;
    tracing::info!(address=%bind,economic_mode="simulation",provider=provider.as_str(),"local_api_ready");
    let result = axum::serve(listener, router(state))
        .with_graceful_shutdown(async move {
            shutdown().await;
            let _ = shutdown_tx.send(true);
        })
        .await
        .map_err(|_| Error::Invalid);
    if let Some(task) = wallet_scheduler {
        task.abort();
        let _ = task.await;
    }
    if let Some(task) = scheduler {
        task.abort();
        let _ = task.await;
    }
    monitor.shutdown().await?;
    result
}

fn configured_provider(name: &str, private_staging: bool) -> Result<Arc<dyn Reasoner>> {
    if private_staging && name != "offline" {
        return Err(Error::Denied);
    }
    match name {
        "offline" => Ok(Arc::new(Offline)),
        "openai" => Ok(Arc::new(RoutedOpenAi::from_env()?)),
        _ => Err(Error::Invalid),
    }
}
fn flag(name: &str, fallback: bool) -> Result<bool> {
    parse_flag(std::env::var(name).ok().as_deref(), fallback)
}
fn parse_flag(value: Option<&str>, fallback: bool) -> Result<bool> {
    match value {
        None => Ok(fallback),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        _ => Err(Error::Invalid),
    }
}
fn server_address(value: &str, private_staging: bool) -> Result<std::net::SocketAddr> {
    if private_staging {
        value.parse().map_err(|_| Error::Invalid)
    } else {
        local_address(value)
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn container_bind_requires_explicit_private_staging() {
        assert!(server_address("0.0.0.0:8080", false).is_err());
        assert!(server_address("0.0.0.0:8080", true).is_ok());
        assert!(server_address("[::]:8080", true).is_ok());
        assert!(server_address("127.0.0.1:8080", false).is_ok());
    }
    #[test]
    fn staging_api_cannot_select_a_paid_provider() {
        assert!(configured_provider("offline", true).is_ok());
        assert!(matches!(
            configured_provider("openai", true),
            Err(Error::Denied)
        ));
        assert!(configured_provider("unknown", false).is_err());
    }
    #[test]
    fn scheduler_defaults_can_be_disabled_without_ambiguous_flags() {
        assert!(!parse_flag(None, false).unwrap());
        assert!(parse_flag(None, true).unwrap());
        assert!(!parse_flag(Some("false"), true).unwrap());
        assert!(parse_flag(Some("yes"), false).is_err());
    }
}
