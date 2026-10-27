//! Operator-driven, observation-only Devnet verification. Never signs, funds or invokes an LLM.
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};
use supa_domain::*;
use supa_persistence::Store;
use supa_providers::{
    solana::{self, HttpSolanaRpc, SolanaRpc},
    vault::KeyVault,
};
use supa_runtime::wallet::WalletService;
use uuid::Uuid;
const KEYS: [&str; 6] = [
    "SUPA_DEVNET_DATABASE_URL",
    "SUPA_SOLANA_RPC_URL",
    "SUPA_WALLET_MASTER_FILE",
    "SUPA_WALLET_VAULT_DIR",
    "SUPA_WALLET_OPERATOR_ID",
    "SUPA_DEVNET_AGENT_ID",
];
#[derive(Debug, PartialEq)]
enum Mode {
    Setup,
    Check,
    Verify(String),
}
fn mode(args: &[String]) -> Result<Mode> {
    match args {
        [a] if a == "--setup" => Ok(Mode::Setup),
        [a] if a == "--check" => Ok(Mode::Check),
        [a, sig] if a == "--verify" => {
            validate_solana_signature(sig)?;
            Ok(Mode::Verify(sig.clone()))
        }
        _ => Err(Error::Invalid),
    }
}
struct Config {
    url: String,
    rpc: String,
    master: PathBuf,
    vault: PathBuf,
    operator: Uuid,
    agent: Uuid,
}
impl Config {
    fn load() -> Result<Self> {
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env.devnet");
        let meta = std::fs::symlink_metadata(&file).map_err(|_| Error::Invalid)?;
        if !meta.is_file() || meta.len() > 8192 {
            return Err(Error::Invalid);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if meta.permissions().mode() & 0o077 != 0 {
                return Err(Error::Denied);
            }
        }
        let mut values = BTreeMap::new();
        for entry in dotenvy::from_path_iter(file).map_err(|_| Error::Invalid)? {
            let (key, value) = entry.map_err(|_| Error::Invalid)?;
            if !KEYS.contains(&key.as_str())
                || value.is_empty()
                || values.insert(key.clone(), value.clone()).is_some()
            {
                return Err(Error::Invalid);
            }
            if std::env::var(&key).is_ok_and(|shell| shell != value) {
                return Err(Error::Denied);
            }
        }
        let take = |k: &str| values.get(k).cloned().ok_or(Error::Invalid);
        let url = take("SUPA_DEVNET_DATABASE_URL")?;
        check_database(&url)?;
        Ok(Self {
            url,
            rpc: take("SUPA_SOLANA_RPC_URL")?,
            master: take("SUPA_WALLET_MASTER_FILE")?.into(),
            vault: take("SUPA_WALLET_VAULT_DIR")?.into(),
            operator: take("SUPA_WALLET_OPERATOR_ID")?
                .parse()
                .map_err(|_| Error::Invalid)?,
            agent: take("SUPA_DEVNET_AGENT_ID")?
                .parse()
                .map_err(|_| Error::Invalid)?,
        })
    }
    fn service(&self, store: Store, rpc: Arc<dyn SolanaRpc>) -> Result<WalletService> {
        Ok(WalletService::new(
            store,
            KeyVault::from_files(&self.vault, &self.master)?,
            rpc,
            self.operator,
        ))
    }
}
fn check_database(url: &str) -> Result<()> {
    let o = sqlx::postgres::PgConnectOptions::from_str(url).map_err(|_| Error::Invalid)?;
    if !["127.0.0.1", "localhost", "::1"].contains(&o.get_host())
        || o.get_database() != Some("supa_devnet")
    {
        return Err(Error::Denied);
    }
    Ok(())
}
async fn stage<T>(name: &str, result: Result<T>) -> std::result::Result<T, (&str, Error)> {
    result.map_err(|e| (name, e))
}
#[tokio::main]
async fn main() {
    let result = run().await;
    match result {
        Ok(report) => println!(
            "{}",
            json!({"success":true,"report":report,"signing_enabled":false,"funding_requested":false,"paid_inference_requests":0})
        ),
        Err((stage, error)) => {
            println!(
                "{}",
                json!({"success":false,"stage":stage,"error_code":error.to_string(),"signing_enabled":false,"paid_inference_requests":0})
            );
            std::process::exit(1)
        }
    }
}
async fn run() -> std::result::Result<Value, (&'static str, Error)> {
    let mode = stage(
        "arguments",
        mode(&std::env::args().skip(1).collect::<Vec<_>>()),
    )
    .await?;
    let c = stage("configuration", Config::load()).await?;
    let store = stage("database_connect", Store::connect(&c.url).await).await?;
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(store.pool())
        .await
        .map_err(|_| ("database_identity", Error::Database))?;
    if database != "supa_devnet" {
        return Err(("database_identity", Error::Denied));
    }
    if mode == Mode::Setup {
        stage("migrations", store.migrate().await).await?;
        match store.get(c.agent).await {
            Ok(_) => {}
            Err(Error::NotFound) => {
                let mut agent = stage(
                    "agent_configuration",
                    Agent::new(
                        "Supa Devnet Verification".into(),
                        Personality::default(),
                        CycleConfig {
                            scheduler_enabled: false,
                            ..CycleConfig::default()
                        },
                        Default::default(),
                    ),
                )
                .await?;
                agent.id = c.agent;
                stage("agent_create", store.create(&agent).await).await?;
            }
            Err(e) => return Err(("agent_lookup", e)),
        }
    }
    let rpc = Arc::new(
        stage(
            "rpc_configuration",
            HttpSolanaRpc::new(&c.rpc, SolanaNetwork::Devnet),
        )
        .await?,
    );
    let service = stage("vault_configuration", c.service(store.clone(), rpc.clone())).await?;
    if mode == Mode::Setup {
        let wallet = stage("wallet_create", service.create(c.agent, c.operator).await).await?;
        return Ok(
            json!({"mode":"setup","wallet":wallet,"network_calls":0,"lifecycle":store.get(c.agent).await.map_err(|e|("agent_lookup",e))?.lifecycle}),
        );
    }
    let w = stage("wallet_lookup", store.wallet(c.agent).await).await?;
    if w.owner_id != c.operator {
        return Err(("wallet_owner", Error::Denied));
    }
    stage("vault_recovery", service.recover(c.agent, c.operator).await).await?;
    let mint = stage(
        "mint_verification",
        solana::verify_usdc_mint(rpc.as_ref(), SolanaNetwork::Devnet).await,
    )
    .await?;
    let balance = stage(
        "balance_read",
        solana::read_balances(rpc.as_ref(), &w).await,
    )
    .await?;
    stage(
        "balance_persistence",
        store.observe_balance(c.agent, &balance).await,
    )
    .await?;
    if let Mode::Verify(signature) = mode {
        let before = stage("treasury_before", store.treasury(c.agent).await).await?;
        let inserted = stage(
            "payment_verification",
            service
                .verify_payment(c.agent, c.operator, &signature)
                .await,
        )
        .await?;
        let recorded = stage("treasury_recorded", store.treasury(c.agent).await).await?;
        // New pool, service and key-vault objects contain no state from the original service.
        let reopened = stage("restart_database_connect", Store::connect(&c.url).await).await?;
        let restarted = stage("restart_vault", c.service(reopened.clone(), rpc.clone())).await?;
        let recovered = stage(
            "restart_wallet_recovery",
            restarted.recover(c.agent, c.operator).await,
        )
        .await?;
        if recovered != w.address {
            return Err(("restart_identity", Error::KeyVault));
        }
        let duplicate = stage(
            "duplicate_observation",
            restarted
                .verify_payment(c.agent, c.operator, &signature)
                .await,
        )
        .await?;
        let after = stage("treasury_after_restart", reopened.treasury(c.agent).await).await?;
        if duplicate || after.verified_incoming_usdc_atoms != recorded.verified_incoming_usdc_atoms
        {
            return Err(("duplicate_invariant", Error::InvalidPayment));
        }
        return Ok(
            json!({"mode":"verify","wallet":w,"mint":mint,"signature":signature,"confirmation_status":"finalized","inserted":inserted,"before":before,"recorded":recorded,"after_restart":after,"duplicate_inserted":duplicate,"restart_recovery_passed":true,"evidence_source":"configured_rpc"}),
        );
    }
    Ok(
        json!({"mode":"check","wallet":w,"mint":mint,"balance":balance,"treasury":stage("treasury",store.treasury(c.agent).await).await?,"receipt_verification":"not_attempted_requires_approved_external_test_transaction","evidence_source":"configured_rpc"}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_has_no_funding_signing_or_paid_generation_mode() {
        for arg in ["--fund", "--airdrop", "--sign", "--run", "--authorize-paid"] {
            assert!(mode(&[arg.into()]).is_err());
        }
        assert_eq!(mode(&["--check".into()]).unwrap(), Mode::Check);
        assert!(mode(&["--verify".into(), "invalid".into()]).is_err());
    }
    #[test]
    fn only_dedicated_local_devnet_database_is_accepted() {
        assert!(check_database("postgresql://supa_devnet@127.0.0.1:55432/supa_devnet").is_ok());
        for url in [
            "postgresql://pikoo@127.0.0.1/supa_eval",
            "postgresql://pikoo@127.0.0.1/supa_agent_test",
            "postgresql://pikoo@example.com/supa_devnet",
        ] {
            assert!(check_database(url).is_err());
        }
    }
}
