//! Isolated PostgreSQL tests; no existing identity or live provider is modified.
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
const TOKEN: &str = "scheduling-test-token-with-32-characters";
struct Trap(Arc<AtomicUsize>);
#[async_trait::async_trait]
impl Reasoner for Trap {
    async fn plan(&self, _: &Context) -> Result<Plan> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(Error::Provider)
    }
    async fn draft(&self, _: &Context, _: &[Event]) -> Result<TweetDraft> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(Error::Provider)
    }
}
async fn http(app: &Router, id: Uuid, body: Option<Value>, auth: bool) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .uri(format!("/agents/{id}/schedule"))
        .header("content-type", "application/json");
    if auth {
        request = request.header("authorization", format!("Bearer {TOKEN}"));
    }
    let response = app
        .clone()
        .oneshot(if let Some(body) = body {
            request
                .method("POST")
                .body(Body::from(body.to_string()))
                .unwrap()
        } else {
            request.body(Body::empty()).unwrap()
        })
        .await
        .unwrap();
    (
        response.status(),
        serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap(),
    )
}
fn control(view: &ScheduleStatus, operation: ScheduleOperation) -> ScheduleCommand {
    ScheduleCommand {
        request_id: Uuid::new_v4(),
        expected_revision: view.revision,
        operation,
        authorize_offline: true,
        reason: "Explicit isolated scheduling test".into(),
        policy: None,
    }
}
async fn fixture(store: &Store) -> (Agent, ScheduleCommand) {
    let agent = Agent::new(
        "Isolated scheduled identity".into(),
        Personality::default(),
        CycleConfig::default(),
        [
            Capability::ReadMemory,
            Capability::Remember,
            Capability::EvaluateGoal,
        ]
        .into(),
    )
    .unwrap();
    store.create(&agent).await.unwrap();
    let goal = store
        .add_goal(
            agent.id,
            "Review verified execution and retain lessons".into(),
        )
        .await
        .unwrap();
    store
        .credit(agent.id, LedgerKind::Seed, 100000, Uuid::new_v4())
        .await
        .unwrap();
    let mut start = control(
        &store.schedule_status(agent.id).await.unwrap(),
        ScheduleOperation::Start,
    );
    start.policy = Some(SchedulePolicy {
        goal_ids: vec![goal.id],
        interval_seconds: 5,
        max_cycles: 3,
        simulated_budget_microusd: 3300,
        valid_for_seconds: 3600,
    });
    (agent, start)
}
async fn due(store: &Store, id: Uuid) {
    sqlx::query("UPDATE agents SET next_cycle_at=now()-interval '1 second' WHERE id=$1")
        .bind(id)
        .execute(store.pool())
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires disposable local TEST_DATABASE_URL; uses a fresh schema"]
async fn bounded_scheduling_lifecycle_concurrency_recovery_and_permissions() {
    let base = std::env::var("TEST_DATABASE_URL").expect("local test database");
    assert!(base.contains("127.0.0.1") || base.contains("localhost"));
    assert!(!base.contains("options="));
    let admin = Store::connect(&base).await.unwrap();
    let schema = format!("scheduling_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(admin.pool())
        .await
        .unwrap();
    let sep = if base.contains('?') { '&' } else { '?' };
    let url = format!("{base}{sep}options=-csearch_path%3D{schema}");
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = Runtime::new(store.clone(), Arc::new(Trap(calls.clone())));
    let app = router(AppState::new(store.clone(), runtime.clone(), TOKEN.into()).unwrap());
    let (agent, start) = fixture(&store).await;
    let id = agent.id;
    assert_eq!(
        http(&app, id, None, false).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        http(&app, id, Some(serde_json::to_value(&start).unwrap()), false)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    for bad in [
        json!({"authorize_offline":false}),
        {
            let mut v = serde_json::to_value(&start).unwrap();
            v["provider"] = json!("openai");
            v
        },
        {
            let mut v = serde_json::to_value(&start).unwrap();
            v["policy"]["max_cycles"] = json!(101);
            v
        },
    ] {
        assert!(!http(&app, id, Some(bad), true).await.0.is_success());
    }
    assert_eq!(store.get(id).await.unwrap().lifecycle, Lifecycle::Created);
    assert!(!store.get(id).await.unwrap().config.scheduler_enabled);
    assert_eq!(
        http(&app, id, Some(serde_json::to_value(&start).unwrap()), true)
            .await
            .0,
        StatusCode::OK
    );
    let (left, right) = tokio::join!(runtime.cycle(id, true), runtime.cycle(id, true));
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    assert_eq!(store.schedule_status(id).await.unwrap().cycles_started, 1);
    assert!(matches!(runtime.cycle(id, true).await, Err(Error::Busy)));
    let view = store.schedule_status(id).await.unwrap();
    let pause = control(&view, ScheduleOperation::Pause);
    store.schedule_control(id, &pause).await.unwrap();
    assert_eq!(
        store.schedule_control(id, &start).await.unwrap().lifecycle,
        Lifecycle::Paused
    ); // delayed start cannot reactivate
    let resumed = control(
        &store.schedule_status(id).await.unwrap(),
        ScheduleOperation::Resume,
    );
    let after = store.schedule_control(id, &resumed).await.unwrap();
    assert_eq!(after.cycles_started, 1);
    assert_eq!(after.reserved_microusd, 1100);
    assert!(matches!(
        store
            .schedule_control(id, &control(&view, ScheduleOperation::Stop))
            .await,
        Err(Error::Stale)
    ));
    due(&store, id).await;
    runtime.tick().await.unwrap();
    due(&store, id).await;
    runtime.tick().await.unwrap();
    let final_view = store.schedule_status(id).await.unwrap();
    assert_eq!(final_view.lifecycle, Lifecycle::Paused);
    assert!(!final_view.enabled);
    assert_eq!(final_view.cycles_started, 3);
    assert_eq!(final_view.reserved_microusd, 3300);
    assert_eq!(store.financials(id).await.unwrap().balance_microusd, 96700);
    assert!(store.get(id).await.unwrap().memory.len() >= 6);
    assert!(
        store
            .history(id, "actions")
            .await
            .unwrap()
            .iter()
            .all(|a| a["authorization_outcome"] == "allowed" && a["status"] == "succeeded")
    );
    assert!(matches!(
        store
            .schedule_control(id, &control(&final_view, ScheduleOperation::Resume))
            .await,
        Err(Error::ResourceBudget)
    ));
    let reconnect = Store::connect(&url).await.unwrap();
    assert_eq!(
        reconnect.schedule_status(id).await.unwrap().cycles_started,
        3
    );
    assert_eq!(
        reconnect.get(id).await.unwrap().lifecycle,
        Lifecycle::Paused
    );
    let mut altered = start.clone();
    altered.reason = "Changed request content".into();
    assert!(matches!(
        store.schedule_control(id, &altered).await,
        Err(Error::Invalid)
    ));
    // A worker cannot adopt a legacy active/scheduler-enabled identity without authorization.
    let (legacy, _) = fixture(&store).await;
    store
        .transition(legacy.id, Lifecycle::Active, "Legacy test", true)
        .await
        .unwrap();
    runtime.tick().await.unwrap();
    assert_eq!(store.completed_cycle_count(legacy.id).await.unwrap(), 0);
    assert!(matches!(
        runtime.cycle(legacy.id, true).await,
        Err(Error::Denied)
    ));
    // Expired leases retain allowance consumption; unknown work is fenced, never replayed.
    let (crashed, approval) = fixture(&store).await;
    store.schedule_control(crashed.id, &approval).await.unwrap();
    let cycle = store.begin_cycle(crashed.id, true).await.unwrap();
    sqlx::query("UPDATE cycles SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(cycle)
        .execute(store.pool())
        .await
        .unwrap();
    runtime.tick().await.unwrap();
    let recovered = store.schedule_status(crashed.id).await.unwrap();
    assert_eq!(recovered.lifecycle, Lifecycle::Suspended);
    assert!(!recovered.enabled);
    assert_eq!(recovered.reserved_microusd, 1100);
    assert_eq!(
        store.history(crashed.id, "cycles").await.unwrap()[0]["status"],
        "interrupted"
    );
    assert_eq!(store.history(crashed.id, "cycles").await.unwrap().len(), 1);
    assert!(matches!(
        store
            .charge(
                crashed.id,
                cycle,
                LedgerKind::Operating,
                100,
                Uuid::new_v4()
            )
            .await,
        Err(Error::Inactive)
    ));
    let resumed = store
        .schedule_control(crashed.id, &control(&recovered, ScheduleOperation::Resume))
        .await
        .unwrap();
    let stop = control(&resumed, ScheduleOperation::Stop);
    store.schedule_control(crashed.id, &stop).await.unwrap();
    assert_eq!(
        store
            .schedule_control(crashed.id, &stop)
            .await
            .unwrap()
            .lifecycle,
        Lifecycle::Terminated
    );
    assert!(
        store
            .schedule_control(
                crashed.id,
                &control(
                    &store.schedule_status(crashed.id).await.unwrap(),
                    ScheduleOperation::Resume
                )
            )
            .await
            .is_err()
    );
    // Independent expiry, treasury and cost limits stop without creating a cycle or changing funding.
    for boundary in ["expiry", "treasury", "budget"] {
        let (a, mut cmd) = fixture(&store).await;
        if boundary == "budget" {
            cmd.policy.as_mut().unwrap().simulated_budget_microusd = 1100;
        }
        store.schedule_control(a.id, &cmd).await.unwrap();
        if boundary == "expiry" {
            sqlx::query(
                "UPDATE agent_schedules SET expires_at=now()-interval '1 second' WHERE agent_id=$1",
            )
            .bind(a.id)
            .execute(store.pool())
            .await
            .unwrap();
        }
        if boundary == "treasury" {
            sqlx::query("UPDATE agents SET simulated_balance_microusd=0 WHERE id=$1")
                .bind(a.id)
                .execute(store.pool())
                .await
                .unwrap();
        }
        runtime.tick().await.unwrap();
        let status = store.schedule_status(a.id).await.unwrap();
        assert!(!status.enabled);
        assert_eq!(
            status.cycles_started,
            if boundary == "budget" { 1 } else { 0 }
        );
    }
    // Duplicate action effects, excessive charges and live reservations are rejected in storage.
    let (local, authorization) = fixture(&store).await;
    store
        .schedule_control(local.id, &authorization)
        .await
        .unwrap();
    let cycle = store.begin_cycle(local.id, true).await.unwrap();
    let action = store
        .propose(
            local.id,
            cycle,
            &[json!({"tool":"remember","event_ids":[]})],
        )
        .await
        .unwrap()[0];
    store
        .authorize_action(local.id, cycle, action, true)
        .await
        .unwrap();
    store
        .execute_action(
            local.id,
            cycle,
            action,
            Some("An isolated permission test lesson"),
            None,
        )
        .await
        .unwrap();
    assert!(matches!(
        store
            .execute_action(
                local.id,
                cycle,
                action,
                Some("An isolated permission test lesson"),
                None
            )
            .await,
        Err(Error::Stale)
    ));
    assert_eq!(store.get(local.id).await.unwrap().memory.len(), 1);
    let live = InferencePolicy {
        live: true,
        budget_id: Some(Uuid::new_v4()),
        budget_limit_microusd: 100000,
        input_rate: 1,
        output_rate: 1,
        input_token_bound: 1000,
        max_output_tokens: 256,
        ..InferencePolicy::default()
    };
    assert!(matches!(
        store
            .reserve_inference(local.id, cycle, "plan", 0, &live, 1000)
            .await,
        Err(Error::Denied)
    ));
    store
        .charge(local.id, cycle, LedgerKind::Operating, 100, Uuid::new_v4())
        .await
        .unwrap();
    store
        .charge(local.id, cycle, LedgerKind::Inference, 1000, Uuid::new_v4())
        .await
        .unwrap();
    assert!(matches!(
        store
            .charge(local.id, cycle, LedgerKind::Operating, 1, Uuid::new_v4())
            .await,
        Err(Error::ResourceBudget)
    ));
    assert!(matches!(
        store
            .transition(local.id, Lifecycle::Active, "Legacy bypass", true)
            .await,
        Err(Error::Denied)
    ));
    store
        .emergency_stop(local.id, "Explicit emergency stop")
        .await
        .unwrap();
    assert!(!store.schedule_status(local.id).await.unwrap().enabled);
    let (race, authorization) = fixture(&store).await;
    let active = store
        .schedule_control(race.id, &authorization)
        .await
        .unwrap();
    let pause = control(&active, ScheduleOperation::Pause);
    let stop = control(&active, ScheduleOperation::Stop);
    let (a, b) = tokio::join!(
        store.schedule_control(race.id, &pause),
        store.schedule_control(race.id, &stop)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    // Wider stored permissions remain intact, but scheduled plans cannot use social tools.
    let wider = Agent::new(
        "Existing broader permissions".into(),
        Personality::default(),
        CycleConfig::default(),
        [
            Capability::ReadMemory,
            Capability::Remember,
            Capability::DraftTweet,
            Capability::Conserve,
        ]
        .into(),
    )
    .unwrap();
    store.create(&wider).await.unwrap();
    let goal = store
        .add_goal(wider.id, "Retain verified outcomes".into())
        .await
        .unwrap();
    store
        .credit(wider.id, LedgerKind::Seed, 10000, Uuid::new_v4())
        .await
        .unwrap();
    let mut command = authorization.clone();
    command.request_id = Uuid::new_v4();
    command.policy.as_mut().unwrap().goal_ids = vec![goal.id];
    store.schedule_control(wider.id, &command).await.unwrap();
    runtime.cycle(wider.id, true).await.unwrap();
    assert_eq!(
        store.get(wider.id).await.unwrap().capabilities,
        wider.capabilities
    );
    assert!(
        store
            .history(wider.id, "actions")
            .await
            .unwrap()
            .iter()
            .all(|action| matches!(
                action["proposal"]["tool"].as_str(),
                Some("read_memory" | "remember" | "evaluate_goal")
            ))
    );
    assert!(store.social_history(wider.id).await.unwrap().is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 0); // even a configured provider is never invoked
    reconnect.pool().close().await;
    store.pool().close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(admin.pool())
        .await
        .unwrap();
}
