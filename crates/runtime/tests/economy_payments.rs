//! Mocked RPC receipts are never represented as actual Devnet revenue.
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI64, Ordering},
};
use supa_domain::*;
use supa_persistence::Store;
use supa_providers::solana::{RpcMethod, SolanaRpc};
use supa_runtime::Runtime;
use uuid::Uuid;
const ADDRESS: &str = "11111111111111111111111111111111";
struct Rpc {
    amount: AtomicI64,
    unavailable: AtomicBool,
    bad_network: AtomicBool,
    interrupt_tx: AtomicBool,
    missing_tx: AtomicBool,
    wrong_recipient: AtomicBool,
    failed_tx: AtomicBool,
    owner: String,
    memo: String,
    pending: AtomicBool,
    bad_mint: AtomicBool,
}
#[async_trait]
impl SolanaRpc for Rpc {
    async fn read(&self, m: RpcMethod, p: Value) -> Result<Value> {
        if self.unavailable.load(Ordering::SeqCst) {
            return Err(Error::Rpc);
        }
        let amount = self.amount.load(Ordering::SeqCst).to_string();
        let recipient = if self.wrong_recipient.load(Ordering::SeqCst) {
            ADDRESS
        } else {
            &self.owner
        };
        let mint = if self.bad_mint.load(Ordering::SeqCst) {
            SolanaNetwork::Mainnet.usdc_mint()
        } else {
            SolanaNetwork::Devnet.usdc_mint()
        };
        let row = |amount: &str| json!({"accountIndex":0,"mint":mint,"owner":recipient,"programId":TOKEN_PROGRAM,"uiTokenAmount":{"amount":amount,"decimals":6}});
        Ok(match m {
            RpcMethod::Genesis => json!(if self.bad_network.load(Ordering::SeqCst) {
                SolanaNetwork::Mainnet.genesis()
            } else {
                SolanaNetwork::Devnet.genesis()
            }),
            RpcMethod::MintAccount => {
                json!({"context":{"slot":42},"value":{"owner":TOKEN_PROGRAM,"executable":false,"data":{"parsed":{"type":"mint","info":{"decimals":6,"isInitialized":true}}}}})
            }
            RpcMethod::SignatureStatus => {
                json!({"value":[{"slot":42,"err":null,"confirmationStatus":if self.pending.load(Ordering::SeqCst){"confirmed"}else{"finalized"}}]})
            }
            RpcMethod::Transaction => {
                if self.interrupt_tx.load(Ordering::SeqCst) {
                    return Err(Error::Rpc);
                }
                if self.missing_tx.load(Ordering::SeqCst) {
                    return Ok(Value::Null);
                }
                json!({"slot":42,"transaction":{"signatures":[p[0]],"message":{"accountKeys":[ADDRESS,mint],"instructions":[{"programId":TOKEN_PROGRAM,"parsed":{"type":"transferChecked","info":{"source":mint,"destination":ADDRESS,"mint":mint,"tokenAmount":{"amount":amount,"decimals":6}}}},{"programId":"MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr","parsed":self.memo}]}},"meta":{"err":if self.failed_tx.load(Ordering::SeqCst){json!({"InstructionError":[0,"Custom"]})}else{Value::Null},"preTokenBalances":[row("0")],"postTokenBalances":[row(&amount)],"innerInstructions":[]}})
            }
            _ => return Err(Error::Denied),
        })
    }
}
#[tokio::test]
#[ignore = "requires disposable local TEST_DATABASE_URL"]
async fn mint_finality_memo_replay_and_restart_reconciliation() {
    let url = std::env::var("TEST_DATABASE_URL").unwrap();
    assert!(url.contains("localhost") && url.contains("_test"));
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let owner = Uuid::new_v4();
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    let address = bs58::encode(bytes).into_string();
    let customer = Uuid::new_v4();
    let mut agent = Agent::new(
        "Payment reconciliation test".into(),
        Personality::default(),
        CycleConfig::default(),
        [Capability::EvaluateGoal].into(),
    )
    .unwrap();
    agent.lifecycle = Lifecycle::Active;
    store.create_owned(&agent, owner).await.unwrap();
    store
        .register_wallet(
            &WalletIdentity {
                id: Uuid::new_v4(),
                agent_id: agent.id,
                network: SolanaNetwork::Devnet,
                address: address.clone(),
                owner_id: owner,
                origin: IdentityOrigin::HumanOperator,
            },
            owner,
        )
        .await
        .unwrap();
    let service = Uuid::new_v4();
    store
        .offer_service(
            agent.id,
            owner,
            service,
            ServiceSpec {
                name: "Receipt fixture service".into(),
                kind: ServiceKind::Summary,
                price_atoms: 5000,
                estimated_inference_atoms: 1000,
                estimated_infrastructure_atoms: 100,
                max_input_bytes: 8000,
                max_attempts: 2,
            },
        )
        .await
        .unwrap();
    let mut j = store
        .create_economic_job(
            customer,
            CreateJob {
                request_id: Uuid::new_v4(),
                service_id: service,
                network: EconomicNetwork::Devnet,
                input: "Harmless supplied text".into(),
                valid_for_seconds: 60,
            },
        )
        .await
        .unwrap();
    j = store
        .economic_command(
            j.id,
            customer,
            JobCommand {
                request_id: Uuid::new_v4(),
                expected_revision: 0,
                operation: "request_payment".into(),
                authorize: true,
            },
        )
        .await
        .unwrap();
    let runtime = Runtime::new(store.clone(), Arc::new(supa_providers::Offline));
    let signature = bs58::encode(Uuid::new_v4().as_bytes().repeat(4)).into_string();
    let rpc = Rpc {
        amount: AtomicI64::new(5000),
        unavailable: AtomicBool::new(false),
        bad_network: AtomicBool::new(false),
        interrupt_tx: AtomicBool::new(false),
        missing_tx: AtomicBool::new(false),
        wrong_recipient: AtomicBool::new(false),
        failed_tx: AtomicBool::new(false),
        owner: address.clone(),
        memo: j.id.to_string(),
        pending: AtomicBool::new(true),
        bad_mint: AtomicBool::new(false),
    };
    assert!(
        runtime
            .verify_economic_payment(j.id, customer, &signature, &rpc)
            .await
            .is_err()
    );
    rpc.pending.store(false, Ordering::SeqCst);
    rpc.bad_mint.store(true, Ordering::SeqCst);
    assert!(
        runtime
            .verify_economic_payment(j.id, customer, &signature, &rpc)
            .await
            .is_err()
    );
    rpc.bad_mint.store(false, Ordering::SeqCst);
    for flag in [
        &rpc.unavailable,
        &rpc.bad_network,
        &rpc.interrupt_tx,
        &rpc.missing_tx,
        &rpc.wrong_recipient,
        &rpc.failed_tx,
    ] {
        flag.store(true, Ordering::SeqCst);
        assert!(
            runtime
                .verify_economic_payment(j.id, customer, &signature, &rpc)
                .await
                .is_err()
        );
        flag.store(false, Ordering::SeqCst);
        assert_eq!(
            store
                .economic_job(j.id, customer)
                .await
                .unwrap()
                .received_atoms,
            0
        );
    }
    assert!(
        runtime
            .verify_economic_payment(j.id, Uuid::new_v4(), &signature, &rpc)
            .await
            .is_err()
    );
    assert!(
        store
            .economic_command(
                j.id,
                owner,
                JobCommand {
                    request_id: Uuid::new_v4(),
                    expected_revision: j.revision,
                    operation: "accept".into(),
                    authorize: true
                }
            )
            .await
            .is_err()
    );

    let wrong = Rpc {
        amount: AtomicI64::new(5000),
        unavailable: AtomicBool::new(false),
        bad_network: AtomicBool::new(false),
        interrupt_tx: AtomicBool::new(false),
        missing_tx: AtomicBool::new(false),
        wrong_recipient: AtomicBool::new(false),
        failed_tx: AtomicBool::new(false),
        owner: address.clone(),
        memo: Uuid::new_v4().to_string(),
        pending: AtomicBool::new(false),
        bad_mint: AtomicBool::new(false),
    };
    assert!(
        runtime
            .verify_economic_payment(j.id, customer, &signature, &wrong)
            .await
            .is_err()
    );
    rpc.amount.store(2000, Ordering::SeqCst);
    let partial = runtime
        .verify_economic_payment(
            j.id,
            customer,
            &bs58::encode(Uuid::new_v4().as_bytes().repeat(4)).into_string(),
            &rpc,
        )
        .await
        .unwrap();
    assert_eq!(partial.state, JobState::AwaitingPayment);
    assert_eq!(partial.received_atoms, 2000);
    assert_eq!(partial.earned_atoms, 0);
    rpc.amount.store(3000, Ordering::SeqCst);
    let (r1, r2) = tokio::join!(
        runtime.verify_economic_payment(j.id, customer, &signature, &rpc),
        runtime.verify_economic_payment(j.id, customer, &signature, &rpc)
    );
    assert!(r1.is_ok() && r2.is_ok(), "{r1:?} {r2:?}");
    let restarted = Store::connect(&url).await.unwrap();
    assert_eq!(
        restarted
            .economic_job(j.id, customer)
            .await
            .unwrap()
            .received_atoms,
        5000
    );
    let accounts = restarted.economic_accounts(agent.id, owner).await.unwrap();
    assert_eq!(accounts["networks"][1]["liability_atoms"], 5000);
    assert_eq!(accounts["networks"][1]["earned_atoms"], 0);
    let other = store
        .create_economic_job(
            customer,
            CreateJob {
                request_id: Uuid::new_v4(),
                service_id: service,
                network: EconomicNetwork::Devnet,
                input: "Second job".into(),
                valid_for_seconds: 60,
            },
        )
        .await
        .unwrap();
    assert!(
        runtime
            .verify_economic_payment(other.id, customer, &signature, &rpc)
            .await
            .is_err()
    );
    // A late receipt remains a refundable liability; no silent expiration loses a deposit.
    sqlx::query("UPDATE economic_jobs SET expires_at=now()-interval '1 second' WHERE id=$1")
        .bind(other.id)
        .execute(store.pool())
        .await
        .unwrap();
    let late = Rpc {
        amount: AtomicI64::new(5000),
        unavailable: AtomicBool::new(false),
        bad_network: AtomicBool::new(false),
        interrupt_tx: AtomicBool::new(false),
        missing_tx: AtomicBool::new(false),
        wrong_recipient: AtomicBool::new(false),
        failed_tx: AtomicBool::new(false),
        owner: address.clone(),
        memo: other.id.to_string(),
        pending: AtomicBool::new(false),
        bad_mint: AtomicBool::new(false),
    };
    let paid = runtime
        .verify_economic_payment(
            other.id,
            customer,
            &bs58::encode(Uuid::new_v4().as_bytes().repeat(4)).into_string(),
            &late,
        )
        .await
        .unwrap();
    assert_eq!(paid.state, JobState::Expired);
    assert_eq!(paid.refund_eligible_atoms(), 5000);
}
