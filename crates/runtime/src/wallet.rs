//! Operator-only observation and reservation service. No signing or broadcasting implementation.
use serde_json::{Value, json};
use std::sync::Arc;
use supa_domain::*;
use supa_persistence::Store;
use supa_providers::{
    solana::{self, RpcMethod, SolanaRpc},
    vault::KeyVault,
};
use tokio::sync::Mutex;
use uuid::Uuid;
#[derive(Clone)]
pub struct WalletService {
    store: Store,
    vault: Arc<KeyVault>,
    rpc: Arc<dyn SolanaRpc>,
    operator: Uuid,
    gate: Arc<Mutex<()>>,
}
impl WalletService {
    pub fn new(store: Store, vault: KeyVault, rpc: Arc<dyn SolanaRpc>, operator: Uuid) -> Self {
        Self {
            store,
            vault: Arc::new(vault),
            rpc,
            operator,
            gate: Arc::new(Mutex::new(())),
        }
    }
    pub fn operator(&self) -> Uuid {
        self.operator
    }
    async fn owned(&self, agent: Uuid, actor: Uuid) -> Result<WalletIdentity> {
        let w = self.store.wallet(agent).await?;
        if actor != self.operator || actor != w.owner_id {
            return Err(Error::Denied);
        }
        Ok(w)
    }
    pub async fn create(&self, agent: Uuid, actor: Uuid) -> Result<WalletIdentity> {
        if actor != self.operator {
            return Err(Error::Denied);
        }
        let _guard = self.gate.try_lock().map_err(|_| Error::Busy)?;
        match self.store.wallet(agent).await {
            Ok(w) => {
                return if w.owner_id == actor {
                    Ok(w)
                } else {
                    Err(Error::Denied)
                };
            }
            Err(Error::NotFound) => {}
            Err(e) => return Err(e),
        }
        if self.store.get(agent).await?.lifecycle == Lifecycle::Terminated {
            return Err(Error::Inactive);
        }
        let id = Uuid::new_v4();
        let address = self.vault.create_public(id, SolanaNetwork::Devnet)?;
        let w = WalletIdentity {
            id,
            agent_id: agent,
            network: SolanaNetwork::Devnet,
            address,
            owner_id: actor,
            origin: IdentityOrigin::HumanOperator,
        };
        self.store.register_wallet(&w, actor).await?;
        Ok(w)
    }
    pub async fn recover(&self, agent: Uuid, actor: Uuid) -> Result<String> {
        let w = self.owned(agent, actor).await?;
        let result = self
            .vault
            .recover_public(w.id, w.network)
            .and_then(|address| {
                if address == w.address {
                    Ok(address)
                } else {
                    Err(Error::KeyVault)
                }
            });
        self.store
            .wallet_audit(
                agent,
                actor,
                "recover_wallet",
                if result.is_ok() {
                    "recovered"
                } else {
                    "key_vault_failed"
                },
            )
            .await?;
        result
    }
    pub async fn verify_payment(&self, agent: Uuid, actor: Uuid, signature: &str) -> Result<bool> {
        let _guard = self.gate.try_lock().map_err(|_| Error::Busy)?;
        let w = self.owned(agent, actor).await?;
        self.verify_inner(&w, signature).await
    }
    async fn verify_inner(&self, w: &WalletIdentity, signature: &str) -> Result<bool> {
        validate_solana_signature(signature)?;
        match solana::verify_incoming(self.rpc.as_ref(), w, signature).await {
            Ok(p) => self.store.record_payment(w.agent_id, &p).await,
            Err(e) => {
                self.store
                    .payment_failure(w.agent_id, signature, &e)
                    .await?;
                Err(e)
            }
        }
    }
    pub async fn sync(&self, agent: Uuid, actor: Uuid) -> Result<Value> {
        let _guard = self.gate.try_lock().map_err(|_| Error::Busy)?;
        let w = self.owned(agent, actor).await?;
        let result = tokio::time::timeout(std::time::Duration::from_secs(60), self.sync_inner(&w))
            .await
            .unwrap_or(Err(Error::Rpc));
        self.store
            .wallet_audit(
                agent,
                actor,
                "sync",
                if result.is_ok() {
                    "completed"
                } else {
                    "rpc_failed"
                },
            )
            .await?;
        result
    }
    async fn sync_inner(&self, w: &WalletIdentity) -> Result<Value> {
        // Discover token accounts before scanning; previously known (including closed) accounts are retained.
        let balance = solana::read_balances(self.rpc.as_ref(), w).await?;
        self.store.observe_balance(w.agent_id, &balance).await?;
        let addresses = self.store.monitored_addresses(w.agent_id).await?;
        if addresses.len() > 101 {
            return Err(Error::ResourceBudget);
        }
        let mut scanned = 0;
        let mut inserted = 0;
        let mut incomplete = false;
        for address in addresses {
            let cursor = self.store.scan_cursor(w.agent_id, &address).await?;
            let watermark = cursor["watermark"].as_str();
            let mut before = cursor["before"].as_str().map(str::to_owned);
            let mut head = cursor["head"].as_str().map(str::to_owned);
            for page in 0..2 {
                let mut config = json!({"limit":50,"commitment":"finalized"});
                if let Some(s) = watermark {
                    config["until"] = json!(s)
                }
                if let Some(s) = &before {
                    config["before"] = json!(s)
                }
                let response = self
                    .rpc
                    .read(RpcMethod::Signatures, json!([address, config]))
                    .await?;
                let rows = response
                    .as_array()
                    .filter(|v| v.len() <= 50)
                    .ok_or(Error::Rpc)?;
                if head.is_none() {
                    head = rows
                        .first()
                        .and_then(|v| v["signature"].as_str())
                        .map(str::to_owned)
                }
                let mut seen = std::collections::BTreeSet::new();
                for row in rows {
                    let sig = row["signature"].as_str().ok_or(Error::Rpc)?;
                    validate_solana_signature(sig)?;
                    if !seen.insert(sig) || before.as_deref() == Some(sig) {
                        return Err(Error::Rpc);
                    }
                    scanned += 1;
                    match self.verify_inner(w, sig).await {
                        Ok(true) => inserted += 1,
                        Ok(false) | Err(Error::InvalidPayment) => {}
                        Err(e) => return Err(e),
                    }
                }
                if rows.len() < 50 {
                    self.store
                        .save_cursor(
                            w.agent_id,
                            &address,
                            head.as_deref().or(watermark),
                            None,
                            None,
                        )
                        .await?;
                    break;
                }
                before = rows
                    .last()
                    .and_then(|v| v["signature"].as_str())
                    .map(str::to_owned);
                self.store
                    .save_cursor(
                        w.agent_id,
                        &address,
                        watermark,
                        before.as_deref(),
                        head.as_deref(),
                    )
                    .await?;
                if page == 1 {
                    incomplete = true
                }
            }
        }
        // Re-read after verified receipts so reconciliation and freshness are checked before reservation.
        let balance = solana::read_balances(self.rpc.as_ref(), w).await?;
        self.store.observe_balance(w.agent_id, &balance).await?;
        Ok(
            json!({"scanned":scanned,"new_payments":inserted,"scan_incomplete":incomplete,"treasury":self.store.treasury(w.agent_id).await?}),
        )
    }
    pub async fn attempt_signing(&self, agent: Uuid, actor: Uuid) -> Result<()> {
        let w = self.store.wallet(agent).await?;
        self.store
            .wallet_audit(
                agent,
                actor,
                "sign",
                if actor == self.operator && actor == w.owner_id {
                    "milestone_signing_disabled"
                } else {
                    "denied"
                },
            )
            .await?;
        Err(Error::Denied)
    }
}
