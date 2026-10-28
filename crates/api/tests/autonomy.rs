//! Real database workflow in a fresh schema. No provider networking or existing-row writes.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use supa_api::{AppState, router};
use supa_domain::*;
use supa_persistence::Store;
use supa_providers::{Context, Reasoner};
use supa_runtime::Runtime;
use tower::ServiceExt;
use uuid::Uuid;
const TOKEN: &str = "autonomy-test-token-with-32-characters";
struct ForbiddenProvider(Arc<AtomicUsize>);
#[async_trait::async_trait]
impl Reasoner for ForbiddenProvider {
    async fn plan(&self, _: &Context) -> Result<Plan> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(Error::Provider)
    }
    async fn draft(&self, _: &Context, _: &[Event]) -> Result<TweetDraft> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(Error::Provider)
    }
}
async fn request(app: &Router, body: Option<Value>, authorized: bool) -> (StatusCode, Value) {
    let mut builder = Request::builder().uri("/autonomy");
    if authorized {
        builder = builder.header("authorization", format!("Bearer {TOKEN}"));
    }
    let req = if let Some(body) = body {
        builder
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let value =
        serde_json::from_slice(&to_bytes(response.into_body(), 32768).await.unwrap()).unwrap();
    (status, value)
}
#[tokio::test]
#[ignore = "requires disposable local TEST_DATABASE_URL; creates a fresh isolated schema"]
async fn bounded_offline_operator_workflow_persists_and_rejects_replay() {
    let base = std::env::var("TEST_DATABASE_URL").expect("disposable local database required");
    // Match the existing integration-test contract; never target a remote database.
    assert!(base.contains("localhost") || base.contains("127.0.0.1"));
    assert!(
        !base.contains("options="),
        "base test URL must not override schema"
    );
    let admin = Store::connect(&base).await.unwrap();
    let schema = format!("autonomy_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(admin.pool())
        .await
        .unwrap();
    let separator = if base.contains('?') { '&' } else { '?' };
    let url = format!("{base}{separator}options=-csearch_path%3D{schema}");
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let state = AppState::new(
        store.clone(),
        Runtime::new(store.clone(), Arc::new(ForbiddenProvider(calls.clone()))),
        TOKEN.into(),
    )
    .unwrap();
    let app = router(state.clone());
    assert_eq!(request(&app, None, false).await.0, StatusCode::UNAUTHORIZED);
    let (status, limits) = request(&app, None, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(limits["max_cycles"], 3);
    assert_eq!(limits["paid_requests"], 0);
    let id = Uuid::new_v4();
    let body = json!({"request_id":id,"goal":"Review execution outcomes and record lessons","authorize_offline":true});
    assert_eq!(
        request(&app, Some(body.clone()), false).await.0,
        StatusCode::UNAUTHORIZED
    );
    for bad in [
        json!({"request_id":id,"goal":"","authorize_offline":true}),
        json!({"request_id":id,"goal":"Learn","authorize_offline":false}),
        json!({"request_id":id,"goal":"Learn","authorize_offline":true,"capabilities":["draft_tweet"]}),
    ] {
        assert!(!request(&app, Some(bad), true).await.0.is_success());
    }
    assert!(store.list().await.unwrap().is_empty());
    let permit = state.gate.acquire().await.unwrap();
    assert_eq!(
        request(&app, Some(body.clone()), true).await.0,
        StatusCode::CONFLICT
    );
    drop(permit);
    let (status, result) = request(&app, Some(body.clone()), true).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["cycles"].as_array().unwrap().len(), 3);
    assert_eq!(result["paid_requests"], 0);
    let agent = store.get(id).await.unwrap();
    assert_eq!(agent.lifecycle, Lifecycle::Paused);
    assert!(!agent.config.scheduler_enabled);
    assert_eq!(
        agent.capabilities,
        [
            Capability::ReadMemory,
            Capability::Remember,
            Capability::EvaluateGoal
        ]
        .into()
    );
    assert_eq!(agent.goals.len(), 1);
    assert!(agent.memory.len() >= 3);
    assert!(!agent.goals[0].completed);
    assert!(agent.goals[0].suggested_progress_bps > 0);
    let cycles = store.history(id, "cycles").await.unwrap();
    assert_eq!(cycles.len(), 3);
    assert!(cycles.iter().all(|cycle| cycle["status"] == "completed"));
    let actions = store.history(id, "actions").await.unwrap();
    assert!(actions.len() >= 8);
    assert!(
        actions.iter().all(|action| action["status"] == "succeeded"
            && action["authorization_outcome"] == "allowed")
    );
    assert_eq!(store.history(id, "evaluations").await.unwrap().len(), 2);
    let financials = store.financials(id).await.unwrap();
    assert!(financials.simulated);
    assert_eq!(financials.balance_microusd, 6700);
    assert_eq!(financials.inference_microusd, 3000);
    assert_eq!(financials.operating_microusd, 300);
    let ledger = store.history(id, "ledger").await.unwrap();
    assert_eq!(ledger.len(), 7);
    assert_eq!(
        request(&app, Some(body), true).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(store.history(id, "ledger").await.unwrap(), ledger);
    assert_eq!(store.history(id, "cycles").await.unwrap(), cycles);
    let unrelated = Agent::new(
        "Untouched identity".into(),
        Personality::default(),
        CycleConfig::default(),
        Default::default(),
    )
    .unwrap();
    store.create(&unrelated).await.unwrap();
    assert_eq!(
        request(
            &app,
            Some(
                json!({"request_id":unrelated.id,"goal":"Never run this","authorize_offline":true})
            ),
            true
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        store.get(unrelated.id).await.unwrap().lifecycle,
        Lifecycle::Created
    );
    assert!(
        store
            .history(unrelated.id, "ledger")
            .await
            .unwrap()
            .is_empty()
    );
    // Fail a newly created rehearsal at cycle start; unrelated identities remain untouched.
    let failed_id = Uuid::new_v4();
    sqlx::query("CREATE FUNCTION fail_rehearsal_cycle() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture failure'; END $$")
        .execute(store.pool()).await.unwrap();
    sqlx::query(&format!("CREATE TRIGGER fail_rehearsal BEFORE INSERT ON cycles FOR EACH ROW WHEN (NEW.agent_id = '{failed_id}'::uuid) EXECUTE FUNCTION fail_rehearsal_cycle()"))
        .execute(store.pool()).await.unwrap();
    let (status,failure)=request(&app,Some(json!({"request_id":failed_id,"goal":"Private failure fixture goal","authorize_offline":true})),true).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(failure["stage"], "cycles");
    assert_eq!(failure["agent_id"], failed_id.to_string());
    assert_eq!(failure["emergency_stop_confirmed"], true);
    assert!(!failure.to_string().contains("Private failure fixture"));
    assert_eq!(
        store.get(failed_id).await.unwrap().lifecycle,
        Lifecycle::Suspended
    );
    assert_eq!(
        store.financials(failed_id).await.unwrap().balance_microusd,
        10000
    );
    assert!(
        store
            .history(failed_id, "transitions")
            .await
            .unwrap()
            .iter()
            .any(|row| row["reason"] == "offline_rehearsal_failed")
    );
    // The detached workflow completes its final pause even after its caller goes away.
    let disconnected_id = Uuid::new_v4();
    let disconnected_app = app.clone();
    let caller = tokio::spawn(async move {
        request(&disconnected_app,Some(json!({"request_id":disconnected_id,"goal":"Persist after HTTP disconnect","authorize_offline":true})),true).await
    });
    for _ in 0..100 {
        if store.get(disconnected_id).await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    caller.abort();
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    assert_eq!(
        store.get(disconnected_id).await.unwrap().lifecycle,
        Lifecycle::Paused
    );
    assert_eq!(
        store
            .history(disconnected_id, "cycles")
            .await
            .unwrap()
            .len(),
        3
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let reconnected = Store::connect(&url).await.unwrap();
    assert_eq!(
        reconnected.get(id).await.unwrap().lifecycle,
        Lifecycle::Paused
    );
    assert_eq!(reconnected.history(id, "cycles").await.unwrap(), cycles);
    assert!(reconnected.due_agents().await.unwrap().is_empty());
    reconnected.pool().close().await;
    store.pool().close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(admin.pool())
        .await
        .unwrap();
}
