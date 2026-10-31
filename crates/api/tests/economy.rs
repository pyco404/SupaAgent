//! Disposable database proof: no real epoch, paid provider, network transfer or existing agent.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::sync::Arc;
use supa_api::{AppState, OperatorCredential, router};
use supa_domain::*;
use supa_persistence::{Role, Store};
use supa_runtime::Runtime;
use tower::ServiceExt;
use uuid::Uuid;
struct ForbiddenRecommendation {
    paid: bool,
}
#[async_trait::async_trait]
impl supa_providers::Reasoner for ForbiddenRecommendation {
    fn policy(&self) -> InferencePolicy {
        if self.paid {
            InferencePolicy {
                live: true,
                ..Default::default()
            }
        } else {
            InferencePolicy::default()
        }
    }
    async fn plan(&self, _: &supa_providers::Context) -> Result<Plan> {
        assert!(!self.paid, "paid provider must never be called");
        Ok(Plan {
            actions: vec![ProposedAction {
                tool: "transfer".into(),
                event_ids: vec![],
                lesson: None,
                evaluation: None,
            }],
        })
    }
    async fn draft(&self, _: &supa_providers::Context, _: &[Event]) -> Result<TweetDraft> {
        Err(Error::Denied)
    }
}
const OWNER: &str = "economic-owner-fixture-credential-0001";
const CUSTOMER: &str = "economic-customer-fixture-credential-02";
async fn req(app: &Router, path: &str, token: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut r = Request::builder()
        .uri(path)
        .header("authorization", format!("Bearer {token}"));
    let b = if let Some(v) = body {
        r = r.method("POST").header("content-type", "application/json");
        Body::from(v.to_string())
    } else {
        Body::empty()
    };
    let response = app.clone().oneshot(r.body(b).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1000000).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
fn command(j: &EconomicJob, operation: &str) -> JobCommand {
    JobCommand {
        request_id: Uuid::new_v4(),
        expected_revision: j.revision,
        operation: operation.into(),
        authorize: true,
    }
}
#[tokio::test]
#[ignore = "requires disposable local TEST_DATABASE_URL"]
async fn contracts_are_durable_scoped_fenced_and_conserve_liabilities() {
    let base = std::env::var("TEST_DATABASE_URL").unwrap();
    assert!(base.contains("localhost") && !base.contains("options="));
    let admin = Store::connect(&base).await.unwrap();
    let db = format!("supa_test_economy_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {db} TEMPLATE template0"))
        .execute(admin.pool())
        .await
        .unwrap();
    let url = sqlx::ConnectOptions::to_url_lossy(
        &base
            .parse::<sqlx::postgres::PgConnectOptions>()
            .unwrap()
            .database(&db),
    )
    .to_string();
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let owner = Uuid::new_v4();
    let customer = Uuid::new_v4();
    let stranger = Uuid::new_v4();
    let mut a = Agent::new(
        "Economic integration fixture".into(),
        Personality::default(),
        CycleConfig::default(),
        [Capability::ReadMemory, Capability::EvaluateGoal].into(),
    )
    .unwrap();
    a.lifecycle = Lifecycle::Active;
    store.create_owned(&a, owner).await.unwrap();
    let spec = ServiceSpec {
        name: "Supplied text summary".into(),
        kind: ServiceKind::Summary,
        price_atoms: 5000,
        estimated_inference_atoms: 1000,
        estimated_infrastructure_atoms: 100,
        max_input_bytes: 8000,
        max_attempts: 2,
    };
    let service = Uuid::new_v4();
    assert!(
        store
            .offer_service(a.id, stranger, service, spec.clone())
            .await
            .is_err()
    );
    store
        .offer_service(a.id, owner, service, spec.clone())
        .await
        .unwrap();
    let runtime = Runtime::new(store.clone(), Arc::new(supa_providers::Offline));
    let state = AppState::new(
        store.clone(),
        runtime.clone(),
        "economic-public-service-fixture-token-00".into(),
    )
    .unwrap()
    .with_credentials(vec![
        OperatorCredential {
            id: owner,
            role: Role::Owner,
            token: OWNER.into(),
        },
        OperatorCredential {
            id: customer,
            role: Role::Operator,
            token: CUSTOMER.into(),
        },
    ])
    .unwrap();
    let app = router(state);
    let recipient_path = format!("/agents/{}/economy/recipient", a.id);
    let recipient_body =
        json!({"address":"11111111111111111111111111111112","confirm_watch_only":true});
    assert_eq!(
        req(
            &app,
            &recipient_path,
            CUSTOMER,
            Some(recipient_body.clone())
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        req(
            &app,
            &recipient_path,
            OWNER,
            Some(json!({"address":"11111111111111111111111111111112","confirm_watch_only":false}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (code, public_wallet) =
        req(&app, &recipient_path, OWNER, Some(recipient_body.clone())).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(public_wallet["watch_only"], true);
    assert_eq!(public_wallet["external_control_verified"], false);
    assert_eq!(public_wallet["signing_enabled"], false);
    let identity = store.wallet(a.id).await.unwrap().id;
    assert_eq!(
        req(&app, &recipient_path, OWNER, Some(recipient_body))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(store.wallet(a.id).await.unwrap().id, identity);
    assert_eq!(
        req(
            &app,
            &recipient_path,
            OWNER,
            Some(json!({"address":"11111111111111111111111111111113","confirm_watch_only":true}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(req(&app,&recipient_path,OWNER,Some(json!({"address":"11111111111111111111111111111112","confirm_watch_only":true,"private_key":"forbidden"}))).await.0,StatusCode::UNPROCESSABLE_ENTITY);

    let spoof = Request::builder()
        .method("POST")
        .uri(format!("/agents/{}/economy/capital", a.id))
        .header("authorization", format!("Bearer {CUSTOMER}"))
        .header("x-supa-principal", owner.to_string())
        .header("x-supa-role", "owner")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"request_id":Uuid::new_v4(),"amount_atoms":10,"authorize_simulated":true})
                .to_string(),
        ))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(spoof).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        req(
            &app,
            "/jobs",
            "economic-public-service-fixture-token-00",
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let request = CreateJob {
        request_id: Uuid::new_v4(),
        service_id: service,
        network: EconomicNetwork::Simulated,
        input: "Verified source sentence. Another supplied fact.".into(),
        valid_for_seconds: 3600,
    };
    assert_eq!(req(&app,"/jobs",CUSTOMER,Some(json!({"request_id":request.request_id,"service_id":service,"network":"mainnet","input":"analysis","valid_for_seconds":3600}))).await.0,StatusCode::FORBIDDEN);
    let (status, v) = req(&app, "/jobs", CUSTOMER, Some(json!(request))).await;
    assert_eq!(status, StatusCode::OK, "{v}");
    let mut j: EconomicJob = serde_json::from_value(v).unwrap();
    assert_eq!(
        store
            .create_economic_job(customer, request.clone())
            .await
            .unwrap()
            .id,
        j.id
    );
    assert!(store.create_economic_job(stranger, request).await.is_err());
    assert!(store.economic_job(j.id, stranger).await.is_err());
    assert!(store.economic_accounts(a.id, customer).await.is_err());
    let c = command(&j, "request_payment");
    j = store
        .economic_command(j.id, customer, c.clone())
        .await
        .unwrap();
    assert_eq!(
        store
            .economic_command(j.id, customer, c)
            .await
            .unwrap()
            .revision,
        j.revision
    );
    let key = Uuid::new_v4();
    let pay = || SimulatedReceipt {
        request_id: key,
        amount_atoms: 2000,
        authorize_simulated: true,
    };
    let (p1, p2) = tokio::join!(
        store.simulated_job_payment(j.id, customer, pay()),
        store.simulated_job_payment(j.id, customer, pay())
    );
    assert!(p1.is_ok() && p2.is_ok());
    j = store.economic_job(j.id, customer).await.unwrap();
    assert_eq!(j.received_atoms, 2000);
    assert_eq!(j.state, JobState::AwaitingPayment);
    j = store
        .simulated_job_payment(
            j.id,
            customer,
            SimulatedReceipt {
                request_id: Uuid::new_v4(),
                amount_atoms: 3500,
                authorize_simulated: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(j.state, JobState::Funded);
    let accounts = store.economic_accounts(a.id, owner).await.unwrap();
    assert_eq!(accounts["networks"][0]["liability_atoms"], 5500);
    assert_eq!(accounts["networks"][0]["earned_atoms"], 0);
    assert!(
        store
            .economic_command(j.id, customer, command(&j, "accept"))
            .await
            .is_err()
    );
    assert!(
        store
            .economic_command(j.id, owner, command(&j, "accept"))
            .await
            .is_err()
    );
    store
        .economic_capital(a.id, owner, Uuid::new_v4(), 10000)
        .await
        .unwrap();
    store
        .economic_policy(
            a.id,
            owner,
            5000,
            1000,
            vec!["inference".into(), "infrastructure".into()],
        )
        .await
        .unwrap();
    j = store
        .economic_command(j.id, owner, command(&j, "accept"))
        .await
        .unwrap();
    let c = command(&j, "execute");
    let (r1, r2) = tokio::join!(
        runtime.execute_economic_job(j.id, owner, c.clone()),
        runtime.execute_economic_job(j.id, owner, c)
    );
    assert_ne!(r1.is_ok(), r2.is_ok());
    j = store.economic_job(j.id, customer).await.unwrap();
    assert_eq!(j.state, JobState::Completed);
    assert_eq!(j.earned_atoms, 5000);
    assert_eq!(j.refund_eligible_atoms(), 500);
    assert!(j.output_sha256.as_ref().is_some_and(|s| s.len() == 64));
    assert_eq!(
        j.output.as_ref().unwrap()["analysis"]["sentences"][0],
        "Verified source sentence"
    );
    let accounts = store.economic_accounts(a.id, owner).await.unwrap();
    assert_eq!(accounts["networks"][0]["liability_atoms"], 500);
    assert_eq!(accounts["networks"][0]["net_result_atoms"], 3900);
    assert_eq!(accounts["networks"][1]["cash_atoms"], 0);
    let spend = Uuid::new_v4();
    store
        .economic_spend(a.id, owner, spend, 100, "infrastructure")
        .await
        .unwrap();
    store
        .economic_spend(a.id, owner, spend, 100, "infrastructure")
        .await
        .unwrap();
    assert!(
        store
            .economic_spend(a.id, owner, spend, 101, "infrastructure")
            .await
            .is_err()
    );
    assert!(
        store
            .economic_spend(a.id, owner, Uuid::new_v4(), 5001, "inference")
            .await
            .is_err()
    );
    assert!(
        store
            .economic_spend(a.id, owner, Uuid::new_v4(), 1, "arbitrary_destination")
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE economic_entries SET amount_atoms=0")
            .execute(store.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM economic_receipts")
            .execute(store.pool())
            .await
            .is_err()
    );
    let request = CreateJob {
        request_id: Uuid::new_v4(),
        service_id: service,
        network: EconomicNetwork::Simulated,
        input: "Interrupted work fixture".into(),
        valid_for_seconds: 3600,
    };
    let mut interrupted = store.create_economic_job(customer, request).await.unwrap();
    interrupted = store
        .economic_command(
            interrupted.id,
            customer,
            command(&interrupted, "request_payment"),
        )
        .await
        .unwrap();
    interrupted = store
        .simulated_job_payment(
            interrupted.id,
            customer,
            SimulatedReceipt {
                request_id: Uuid::new_v4(),
                amount_atoms: 5000,
                authorize_simulated: true,
            },
        )
        .await
        .unwrap();
    interrupted = store
        .economic_command(interrupted.id, owner, command(&interrupted, "accept"))
        .await
        .unwrap();
    let old = command(&interrupted, "execute");
    interrupted = store
        .claim_economic_job(interrupted.id, owner, old.clone(), json!({"fixture":true}))
        .await
        .unwrap();
    assert!(
        store
            .economic_command(interrupted.id, owner, command(&interrupted, "recover"))
            .await
            .is_err()
    );
    sqlx::query("UPDATE economic_jobs SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(interrupted.id)
        .execute(store.pool())
        .await
        .unwrap();
    let restarted = Store::connect(&url).await.unwrap();
    interrupted = restarted
        .economic_command(interrupted.id, owner, command(&interrupted, "recover"))
        .await
        .unwrap();
    assert_eq!(interrupted.state, JobState::Failed);
    assert!(
        restarted
            .finish_economic_job(interrupted.id, owner, old.request_id, Some(json!({})), None)
            .await
            .is_err()
    );
    assert_eq!(interrupted.refund_eligible_atoms(), 5000);
    interrupted = restarted
        .economic_command(interrupted.id, owner, command(&interrupted, "retry"))
        .await
        .unwrap();
    assert_eq!(interrupted.state, JobState::Funded);
    interrupted = restarted
        .economic_command(interrupted.id, owner, command(&interrupted, "accept"))
        .await
        .unwrap();
    let paid = Runtime::new(
        restarted.clone(),
        Arc::new(ForbiddenRecommendation { paid: true }),
    );
    assert!(
        paid.execute_economic_job(interrupted.id, owner, command(&interrupted, "execute"))
            .await
            .is_err()
    );
    assert_eq!(
        restarted
            .economic_job(interrupted.id, owner)
            .await
            .unwrap()
            .state,
        JobState::Accepted
    );
    let malicious = Runtime::new(
        restarted.clone(),
        Arc::new(ForbiddenRecommendation { paid: false }),
    );
    interrupted = malicious
        .execute_economic_job(interrupted.id, owner, command(&interrupted, "execute"))
        .await
        .unwrap();
    assert_eq!(interrupted.state, JobState::Failed);
    assert_eq!(interrupted.earned_atoms, 0);
    assert!(
        restarted
            .economic_command(interrupted.id, owner, command(&interrupted, "retry"))
            .await
            .is_err()
    );
    j = store
        .economic_command(j.id, customer, command(&j, "dispute"))
        .await
        .unwrap();
    assert_eq!(j.state, JobState::Disputed);
    assert_eq!(j.earned_atoms, 5000); // no silent refund/revenue rewrite
    assert_eq!(
        req(&app, &format!("/jobs/{}", j.id), CUSTOMER, None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        req(&app, &format!("/agents/{}/economy", a.id), CUSTOMER, None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    restarted.pool().close().await;
    store.pool().close().await;
    sqlx::query(&format!("DROP DATABASE {db} WITH (FORCE)"))
        .execute(admin.pool())
        .await
        .unwrap();
}
