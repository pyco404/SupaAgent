//! Mocked Devnet only. All PostgreSQL writes require an explicitly selected disposable test database.
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use supa_domain::*;
use supa_persistence::Store;
use supa_providers::{
    solana::{RpcMethod, SolanaRpc},
    vault::KeyVault,
};
use supa_runtime::wallet::WalletService;
use uuid::Uuid;
const TOKEN: &str = "11111111111111111111111111111111";
fn signature(n: u8) -> String {
    bs58::encode([n; 64]).into_string()
}
struct MockRpc {
    owner: Mutex<String>,
    fail: AtomicBool,
    pending: AtomicBool,
    bad_network: AtomicBool,
    calls: AtomicUsize,
    pages: bool,
    fail_page: AtomicBool,
}
impl MockRpc {
    fn new() -> Self {
        Self {
            owner: Mutex::new(String::new()),
            fail: AtomicBool::new(false),
            pending: AtomicBool::new(false),
            bad_network: AtomicBool::new(false),
            calls: AtomicUsize::new(0),
            pages: false,
            fail_page: AtomicBool::new(false),
        }
    }
}
#[async_trait]
impl SolanaRpc for MockRpc {
    async fn read(&self, m: RpcMethod, p: Value) -> Result<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail.swap(false, Ordering::SeqCst) {
            return Err(Error::Rpc);
        }
        let owner = self.owner.lock().unwrap().clone();
        let mint = SolanaNetwork::Devnet.usdc_mint();
        let amount = 1_500_000;
        Ok(match m {
            RpcMethod::Genesis => json!(if self.bad_network.load(Ordering::SeqCst) {
                SolanaNetwork::Mainnet.genesis()
            } else {
                SolanaNetwork::Devnet.genesis()
            }),
            RpcMethod::MintAccount => return Err(Error::Denied),
            RpcMethod::Balance => json!({"context":{"slot":42},"value":2_000_000_000i64}),
            RpcMethod::TokenAccounts => {
                json!({"context":{"slot":42},"value":[{"pubkey":TOKEN,"account":{"owner":TOKEN_PROGRAM,"data":{"parsed":{"type":"account","info":{"owner":owner,"mint":mint,"tokenAmount":{"amount":amount.to_string(),"decimals":6}}}}}}]})
            }
            RpcMethod::Signatures => {
                if self.pages && p[0] == owner && p[1]["until"].is_null() {
                    if p[1]["before"].is_null() {
                        json!(
                            (1..=50)
                                .map(|n| json!({"signature":signature(n),"err":null}))
                                .collect::<Vec<_>>()
                        )
                    } else {
                        if self.fail_page.swap(false, Ordering::SeqCst) {
                            return Err(Error::Rpc);
                        }
                        json!([{ "signature":signature(51),"err":null}])
                    }
                } else if self.pages || !p[1]["until"].is_null() {
                    json!([])
                } else {
                    json!([{ "signature":signature(1),"err":null}])
                }
            }
            RpcMethod::SignatureStatus => {
                json!({"value":[{"slot":42,"err":null,"confirmationStatus":if self.pending.load(Ordering::SeqCst){"confirmed"}else{"finalized"}}]})
            }
            RpcMethod::Transaction => {
                let sig = p[0].as_str().unwrap();
                let credit = if sig == signature(1) { amount } else { 0 };
                let row = |v: i64| json!({"accountIndex":0,"mint":mint,"owner":owner,"programId":TOKEN_PROGRAM,"uiTokenAmount":{"amount":v.to_string(),"decimals":6}});
                json!({"slot":42,"transaction":{"signatures":[sig],"message":{"accountKeys":[TOKEN,mint],"instructions":[{"programId":TOKEN_PROGRAM,"parsed":{"type":"transferChecked","info":{"source":mint,"destination":TOKEN,"mint":mint,"tokenAmount":{"amount":credit.to_string(),"decimals":6}}}}]}},"meta":{"err":null,"preTokenBalances":[row(0)],"postTokenBalances":[row(credit)],"innerInstructions":[]}})
            }
            RpcMethod::Simulate => return Err(Error::Denied),
        })
    }
}
async fn database() -> Store {
    let url = std::env::var("TEST_DATABASE_URL").expect("disposable TEST_DATABASE_URL required");
    let s = Store::connect(&url).await.unwrap();
    s.migrate().await.unwrap();
    s
}
struct Fixture {
    store: Store,
    agent: Agent,
    operator: Uuid,
    rpc: Arc<MockRpc>,
    service: WalletService,
    dir: tempfile::TempDir,
}
async fn setup(pages: bool) -> Fixture {
    let store = database().await;
    let agent = Agent::new(
        "Devnet observer".into(),
        Personality::default(),
        CycleConfig::default(),
        Default::default(),
    )
    .unwrap();
    store.create(&agent).await.unwrap();
    store
        .transition(agent.id, Lifecycle::Active, "wallet test", true)
        .await
        .unwrap();
    let operator = Uuid::new_v4();
    let dir = tempfile::tempdir().unwrap();
    let vault = KeyVault::open(&dir.path().join("vault"), [5; 32]).unwrap();
    let mut rpc = MockRpc::new();
    rpc.pages = pages;
    let rpc = Arc::new(rpc);
    let service = WalletService::new(store.clone(), vault, rpc.clone(), operator);
    let w = service.create(agent.id, operator).await.unwrap();
    *rpc.owner.lock().unwrap() = w.address;
    Fixture {
        store,
        agent,
        operator,
        rpc,
        service,
        dir,
    }
}
async fn enable(f: &Fixture, per: i64, daily: i64) {
    f.store
        .set_spending_policy(
            f.agent.id,
            f.operator,
            &SpendingPolicy {
                locked: false,
                reservations_enabled: true,
                per_transaction_atoms: per,
                daily_atoms: daily,
                destinations: [TOKEN.into()].into(),
            },
        )
        .await
        .unwrap();
}
#[tokio::test]
#[ignore = "requires disposable TEST_DATABASE_URL"]
async fn mocked_devnet_demo_identity_receipts_duplicate_restart_and_signing_denial() {
    let f = setup(false).await;
    let id = f.agent.id;
    let w = f.store.wallet(id).await.unwrap();
    validate_solana_address(&w.address).unwrap();
    let result = f.service.sync(id, f.operator).await.unwrap();
    assert_eq!(result["new_payments"], 1);
    assert_eq!(
        result["treasury"]["observed_sol_lamports"],
        2_000_000_000i64
    );
    assert_eq!(
        result["treasury"]["verified_incoming_usdc_atoms"],
        1_500_000
    );
    assert!(result["treasury"]["reconciled"].as_bool().unwrap());
    assert!(
        result["treasury"]["environment"]
            .as_str()
            .unwrap()
            .starts_with("mocked_")
    );
    assert!(
        !f.service
            .verify_payment(id, f.operator, &signature(1))
            .await
            .unwrap()
    );
    assert_eq!(
        f.store.wallet_history(id, "payments").await.unwrap().len(),
        1
    );
    assert_eq!(f.store.financials(id).await.unwrap().balance_microusd, 0);
    let reopened = database().await;
    reopened.recover().await.unwrap();
    let recovered = WalletService::new(
        reopened.clone(),
        KeyVault::open(&f.dir.path().join("vault"), [5; 32]).unwrap(),
        f.rpc.clone(),
        f.operator,
    );
    assert_eq!(recovered.recover(id, f.operator).await.unwrap(), w.address);
    assert_eq!(reopened.wallet(id).await.unwrap().id, w.id);
    assert!(
        !recovered
            .verify_payment(id, f.operator, &signature(1))
            .await
            .unwrap()
    );
    assert!(matches!(
        recovered.attempt_signing(id, Uuid::new_v4()).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        recovered.attempt_signing(id, f.operator).await,
        Err(Error::Denied)
    ));
    let audit = reopened.wallet_history(id, "audit").await.unwrap();
    assert!(
        audit
            .iter()
            .any(|a| a["outcome"] == "milestone_signing_disabled")
    );
    assert!(audit.iter().any(|a| a["outcome"] == "denied"));
    println!(
        "mocked Devnet evidence: 2 SOL test balance, 1.5 USDC test receipt, exactly one persisted payment, recovery successful, both signing attempts denied; no external RPC or transactions"
    );
}
#[tokio::test]
#[ignore = "requires disposable TEST_DATABASE_URL"]
async fn unconfirmed_rpc_failure_and_network_mismatch_never_credit() {
    let f = setup(false).await;
    let id = f.agent.id;
    f.rpc.fail.store(true, Ordering::SeqCst);
    assert!(matches!(
        f.service.sync(id, f.operator).await,
        Err(Error::Rpc)
    ));
    f.rpc.pending.store(true, Ordering::SeqCst);
    assert!(matches!(
        f.service
            .verify_payment(id, f.operator, &signature(1))
            .await,
        Err(Error::Unconfirmed)
    ));
    assert!(
        f.store
            .wallet_history(id, "payments")
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        f.store.wallet_history(id, "checks").await.unwrap()[0]["status"],
        "payment_unconfirmed"
    );
    f.rpc.pending.store(false, Ordering::SeqCst);
    f.rpc.bad_network.store(true, Ordering::SeqCst);
    assert!(f.service.sync(id, f.operator).await.is_err());
    assert_eq!(
        f.store
            .treasury(id)
            .await
            .unwrap()
            .available_operating_usdc_atoms,
        0
    );
    f.rpc.bad_network.store(false, Ordering::SeqCst);
    assert_eq!(
        f.service.sync(id, f.operator).await.unwrap()["new_payments"],
        1
    );
}
#[tokio::test]
#[ignore = "requires disposable TEST_DATABASE_URL"]
async fn concurrent_reservations_caps_replay_owner_and_restart() {
    let f = setup(false).await;
    let id = f.agent.id;
    f.service.sync(id, f.operator).await.unwrap();
    enable(&f, 1_000_000, 1_000_000).await;
    let keys = [Uuid::new_v4(), Uuid::new_v4()];
    let (a, b) = tokio::join!(
        f.store.reserve_operating(
            id,
            f.operator,
            keys[0],
            TOKEN,
            700_000,
            OperatingPurpose::Inference
        ),
        f.store.reserve_operating(
            id,
            f.operator,
            keys[1],
            TOKEN,
            700_000,
            OperatingPurpose::Inference
        )
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let winner = if a.is_ok() { keys[0] } else { keys[1] };
    f.store
        .reserve_operating(
            id,
            f.operator,
            winner,
            TOKEN,
            700_000,
            OperatingPurpose::Inference,
        )
        .await
        .unwrap();
    assert!(
        f.store
            .reserve_operating(
                id,
                f.operator,
                winner,
                TOKEN,
                600_000,
                OperatingPurpose::Inference
            )
            .await
            .is_err()
    );
    assert!(
        f.store
            .reserve_operating(
                id,
                Uuid::new_v4(),
                Uuid::new_v4(),
                TOKEN,
                1,
                OperatingPurpose::Inference
            )
            .await
            .is_err()
    );
    assert!(
        f.store
            .set_spending_policy(id, Uuid::new_v4(), &SpendingPolicy::default())
            .await
            .is_err()
    );
    let reopened = database().await;
    reopened.recover().await.unwrap();
    assert_eq!(
        reopened.treasury(id).await.unwrap().reserved_usdc_atoms,
        700_000
    );
    assert_eq!(
        reopened
            .wallet_history(id, "reservations")
            .await
            .unwrap()
            .len(),
        1
    );
    reopened
        .release_operating(id, f.operator, winner)
        .await
        .unwrap();
    assert_eq!(reopened.treasury(id).await.unwrap().reserved_usdc_atoms, 0);
    assert!(
        reopened
            .reserve_operating(
                id,
                f.operator,
                Uuid::new_v4(),
                TOKEN,
                400_001,
                OperatingPurpose::Infrastructure
            )
            .await
            .is_err()
    );

    assert!(
        reopened
            .reserve_operating(
                id,
                f.operator,
                winner,
                TOKEN,
                700_000,
                OperatingPurpose::Inference
            )
            .await
            .is_err()
    );
    assert!(
        reopened
            .wallet_history(id, "audit")
            .await
            .unwrap()
            .iter()
            .filter(|v| v["outcome"] == "denied")
            .count()
            >= 3
    );
}
#[tokio::test]
#[ignore = "requires disposable TEST_DATABASE_URL"]
async fn lifecycle_emergency_lock_stale_and_unreconciled_capital_fail_closed() {
    let f = setup(false).await;
    let id = f.agent.id;
    // Observing a balance without verifying a payment is not revenue.
    let w = f.store.wallet(id).await.unwrap();
    let b = supa_providers::solana::read_balances(f.rpc.as_ref(), &w)
        .await
        .unwrap();
    f.store.observe_balance(id, &b).await.unwrap();
    assert_eq!(
        f.store
            .treasury(id)
            .await
            .unwrap()
            .available_operating_usdc_atoms,
        0
    );
    f.service
        .verify_payment(id, f.operator, &signature(1))
        .await
        .unwrap();
    enable(&f, 1_000_000, 1_000_000).await;
    for state in [Lifecycle::Paused, Lifecycle::Suspended] {
        f.store
            .transition(id, state, "control test", true)
            .await
            .unwrap();
        assert!(
            f.store
                .reserve_operating(
                    id,
                    f.operator,
                    Uuid::new_v4(),
                    TOKEN,
                    1,
                    OperatingPurpose::Infrastructure
                )
                .await
                .is_err()
        );
        f.store
            .transition(id, Lifecycle::Active, "explicit operator resume", true)
            .await
            .unwrap();
    }
    f.store.emergency_stop(id, "lock wallet").await.unwrap();
    assert!(f.store.spending_policy(id).await.unwrap().locked);
    f.store
        .transition(id, Lifecycle::Active, "resume does not unlock wallet", true)
        .await
        .unwrap();
    assert!(
        f.store
            .reserve_operating(
                id,
                f.operator,
                Uuid::new_v4(),
                TOKEN,
                1,
                OperatingPurpose::Inference
            )
            .await
            .is_err()
    );
    enable(&f, 1_000_000, 1_000_000).await;
    sqlx::query(
        "UPDATE wallet_balances SET observed_at=now()-interval '121 seconds' WHERE wallet_id=$1",
    )
    .bind(w.id)
    .execute(f.store.pool())
    .await
    .unwrap();
    assert_eq!(
        f.store
            .treasury(id)
            .await
            .unwrap()
            .available_operating_usdc_atoms,
        0
    );
    assert!(
        f.store
            .reserve_operating(
                id,
                f.operator,
                Uuid::new_v4(),
                TOKEN,
                1,
                OperatingPurpose::Inference
            )
            .await
            .is_err()
    );
    f.store
        .transition(id, Lifecycle::Terminated, "terminal", true)
        .await
        .unwrap();
    assert!(f.service.create(id, f.operator).await.is_ok());
    assert!(
        f.store
            .reserve_operating(
                id,
                f.operator,
                Uuid::new_v4(),
                TOKEN,
                1,
                OperatingPurpose::Inference
            )
            .await
            .is_err()
    );
}
#[tokio::test]
#[ignore = "requires disposable TEST_DATABASE_URL"]
async fn paginated_monitor_resumes_after_failure_without_losing_receipts() {
    let f = setup(true).await;
    let id = f.agent.id;
    let address = f.store.wallet(id).await.unwrap().address;
    f.rpc.fail_page.store(true, Ordering::SeqCst);
    assert!(f.service.sync(id, f.operator).await.is_err());
    let cursor = f.store.scan_cursor(id, &address).await.unwrap();
    assert_eq!(cursor["before"], signature(50));
    assert!(cursor["watermark"].is_null());
    let reopened = database().await;
    let service = WalletService::new(
        reopened.clone(),
        KeyVault::open(&f.dir.path().join("vault"), [5; 32]).unwrap(),
        f.rpc.clone(),
        f.operator,
    );
    service.sync(id, f.operator).await.unwrap();
    let cursor = reopened.scan_cursor(id, &address).await.unwrap();
    assert_eq!(cursor["watermark"], signature(1));
    assert!(cursor["before"].is_null());
    assert_eq!(
        reopened.wallet_history(id, "payments").await.unwrap().len(),
        1
    );
    assert_eq!(service.sync(id, f.operator).await.unwrap()["scanned"], 0);
}
#[tokio::test]
#[ignore = "requires disposable TEST_DATABASE_URL"]
async fn mock_and_configured_evidence_and_other_agents_are_separated() {
    let f = setup(false).await;
    let id = f.agent.id;
    f.service.sync(id, f.operator).await.unwrap();
    let mut b = f.store.wallet_balance(id).await.unwrap().unwrap();
    b.evidence_source = ChainEvidence::ConfiguredRpc;
    assert!(f.store.observe_balance(id, &b).await.is_err());
    assert!(f.service.recover(id, Uuid::new_v4()).await.is_err());
    assert!(
        f.service
            .verify_payment(id, Uuid::new_v4(), &signature(1))
            .await
            .is_err()
    );
    let w = f.store.wallet(id).await.unwrap();
    let mut child = w.clone();
    child.id = Uuid::new_v4();
    child.origin = IdentityOrigin::Agent {
        parent_agent_id: id,
    };
    assert!(f.store.register_wallet(&child, f.operator).await.is_err());
    let mut tx = f
        .rpc
        .read(RpcMethod::Transaction, json!([signature(1)]))
        .await
        .unwrap();
    tx["meta"]["postTokenBalances"][0]["uiTokenAmount"]["amount"] = json!("1600000");
    tx["transaction"]["message"]["instructions"][0]["parsed"]["info"]["tokenAmount"]["amount"] =
        json!("1600000");
    let proof = VerifiedPayment::verify(
        ChainEvidence::MockRpc,
        SolanaNetwork::Devnet,
        &w.address,
        &signature(1),
        &json!({"slot":42,"err":null,"confirmationStatus":"finalized"}),
        &tx,
    )
    .unwrap();
    assert!(matches!(
        f.store.record_payment(id, &proof).await,
        Err(Error::InvalidPayment)
    ));
    assert_eq!(
        f.store
            .treasury(id)
            .await
            .unwrap()
            .verified_incoming_usdc_atoms,
        1_500_000
    );
    assert_eq!(
        f.store.wallet_history(id, "payments").await.unwrap().len(),
        1
    );
    assert!(f.store.financials(id).await.unwrap().simulated);
}

#[tokio::test]
#[ignore = "requires disposable TEST_DATABASE_URL"]
async fn wallet_context_is_observation_only_and_cannot_fund_reasoning() {
    use supa_providers::{Context, Reasoner};
    use supa_runtime::Runtime;
    struct Observer(Arc<Mutex<Option<TreasurySummary>>>);
    #[async_trait]
    impl Reasoner for Observer {
        async fn plan(&self, c: &Context) -> Result<Plan> {
            *self.0.lock().unwrap() = c.wallet_treasury.clone();
            Ok(Plan { actions: vec![] })
        }
        async fn draft(&self, _: &Context, _: &[Event]) -> Result<TweetDraft> {
            Err(Error::Denied)
        }
    }
    let f = setup(false).await;
    let id = f.agent.id;
    f.service.sync(id, f.operator).await.unwrap();
    let seen = Arc::new(Mutex::new(None));
    let runtime = Runtime::new(f.store.clone(), Arc::new(Observer(seen.clone())));
    assert!(matches!(
        runtime.cycle(id, false).await,
        Err(Error::InsufficientBalance)
    ));
    assert_eq!(
        f.store.get(id).await.unwrap().lifecycle,
        Lifecycle::Suspended
    );
    assert!(seen.lock().unwrap().is_none());
    f.store
        .credit(id, LedgerKind::Seed, 10_000, Uuid::new_v4())
        .await
        .unwrap();
    f.store
        .transition(
            id,
            Lifecycle::Active,
            "explicit simulated funding evaluation",
            true,
        )
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET next_cycle_at=now() WHERE id=$1")
        .bind(id)
        .execute(f.store.pool())
        .await
        .unwrap();
    runtime.cycle(id, false).await.unwrap();
    let treasury = seen.lock().unwrap().clone().unwrap();
    assert_eq!(treasury.verified_incoming_usdc_atoms, 1_500_000);
    assert!(treasury.environment.contains("no_monetary_value"));
    assert_eq!(treasury.api_inference_estimate_microusd, 0);
}

#[tokio::test]
#[ignore = "requires disposable TEST_DATABASE_URL"]
async fn concurrent_observations_credit_once_and_later_weak_confirmation_cannot_recredit() {
    let f = setup(false).await;
    let id = f.agent.id;
    let second = WalletService::new(
        database().await,
        KeyVault::open(&f.dir.path().join("vault"), [5; 32]).unwrap(),
        f.rpc.clone(),
        f.operator,
    );
    let wallet = f.store.wallet(id).await.unwrap();
    let balance = supa_providers::solana::read_balances(f.rpc.as_ref(), &wallet)
        .await
        .unwrap();
    f.store.observe_balance(id, &balance).await.unwrap();
    let sig = signature(1);
    let (a, b) = tokio::join!(
        f.service.verify_payment(id, f.operator, &sig),
        second.verify_payment(id, f.operator, &sig)
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    let treasury = f.store.treasury(id).await.unwrap();
    assert_eq!(treasury.verified_incoming_usdc_atoms, 1_500_000);
    assert_eq!(
        f.store.wallet_history(id, "payments").await.unwrap().len(),
        1
    );
    enable(&f, 1_000_000, 1_000_000).await;
    assert_eq!(
        f.store
            .treasury(id)
            .await
            .unwrap()
            .available_operating_usdc_atoms,
        1_500_000
    );
    // A transient/weak status never becomes fresh finalized evidence and cannot add another credit.
    f.rpc.pending.store(true, Ordering::SeqCst);
    assert!(matches!(
        second.verify_payment(id, f.operator, &sig).await,
        Err(Error::Unconfirmed)
    ));
    assert_eq!(
        f.store.wallet_history(id, "checks").await.unwrap()[0]["status"],
        "verified"
    );
    assert_eq!(
        f.store
            .treasury(id)
            .await
            .unwrap()
            .verified_incoming_usdc_atoms,
        1_500_000
    );
    assert!(f.store.spending_policy(id).await.unwrap().locked);
    assert_eq!(
        f.store
            .treasury(id)
            .await
            .unwrap()
            .available_operating_usdc_atoms,
        0
    );
    assert!(
        f.store
            .wallet_history(id, "audit")
            .await
            .unwrap()
            .iter()
            .any(|v| v["outcome"] == "confirmation_regressed")
    );
    f.rpc.pending.store(false, Ordering::SeqCst);
    assert!(!second.verify_payment(id, f.operator, &sig).await.unwrap());
    assert_eq!(f.store.financials(id).await.unwrap().balance_microusd, 0);
}
