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
    let bind =
        local_address(&std::env::var("SUPA_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into()))?;
    let token = std::env::var("SUPA_API_TOKEN").map_err(|_| Error::Invalid)?;
    let provider = std::env::var("SUPA_PROVIDER").unwrap_or_else(|_| "offline".into());
    let reasoner: Arc<dyn Reasoner> = match provider.as_str() {
        "offline" => Arc::new(Offline),
        "openai" => Arc::new(RoutedOpenAi::from_env()?),
        _ => return Err(Error::Invalid),
    };
    let url = std::env::var("DATABASE_URL").map_err(|_| Error::Invalid)?;
    let store = Store::connect(&url).await?;
    store.migrate().await?;
    store.recover().await?;
    let runtime = Runtime::new(store.clone(), reasoner);
    let mut state = AppState::new(store.clone(), runtime, token)?;
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
    let scheduler_state = state.clone();
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
    let scheduler = tokio::spawn(async move {
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
    });
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|_| Error::Invalid)?;
    tracing::info!(address=%bind,economic_mode="simulation",provider=provider.as_str(),"local_api_ready");
    let result = axum::serve(listener, router(state))
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            let _ = shutdown_tx.send(true);
        })
        .await
        .map_err(|_| Error::Invalid);
    if let Some(task) = wallet_scheduler {
        task.abort();
        let _ = task.await;
    }
    scheduler.abort();
    let _ = scheduler.await;
    result
}
