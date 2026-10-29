//! Only disposable schema fixtures; no provider transport or historical writes.
use chrono::{Duration, Utc};
use serde_json::json;
use supa_domain::*;
use supa_persistence::Store;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires dedicated TEST_DATABASE_URL; never contacts providers"]
async fn archived_budget_cannot_be_recreated_reserved_or_refunded() {
    let base = std::env::var("TEST_DATABASE_URL").expect("dedicated test database");
    assert!(base.contains("_test"));
    let admin = Store::connect(&base).await.unwrap();
    let schema = format!("quarantine_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(admin.pool())
        .await
        .unwrap();
    let url = format!(
        "{base}{}options=-csearch_path%3D{schema}",
        if base.contains('?') { '&' } else { '?' }
    );
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let owner = Uuid::new_v4();
    let agent = Agent::new(
        "Quarantine fixture".into(),
        Personality::default(),
        CycleConfig::default(),
        [Capability::ReadMemory].into(),
    )
    .unwrap();
    store.create_owned(&agent, owner).await.unwrap();
    let archived = uuid::uuid!("ad91e475-f1da-495a-8247-139643d2d8b1");
    let policy = InferencePolicy {
        model: "mock-only".into(),
        live: true,
        budget_id: Some(archived),
        budget_limit_microusd: 100000,
        input_rate: 100000,
        cached_input_rate: 10000,
        cache_write_rate: 125000,
        output_rate: 500000,
        input_token_bound: 24192,
        max_output_tokens: 1024,
    };
    let consent = LiveConsent {
        request_id: Uuid::new_v4(),
        goal: "Inspect test memories".into(),
        model: policy.model.clone(),
        authorize_paid: true,
        consented_at: Utc::now(),
        expires_at: Utc::now() + Duration::minutes(2),
        max_requests: 1,
        max_input_tokens: 24192,
        max_output_tokens: 1024,
        max_cost_microusd: 5000,
    };
    // Both paths must reject before touching a budget, even if absent after data loss.
    assert!(matches!(
        store
            .reserve_inference(agent.id, Uuid::new_v4(), "plan", 0, &policy, 1000)
            .await,
        Err(Error::LiveBudget)
    ));
    assert!(matches!(
        store
            .reserve_live_session(agent.id, owner, &consent, &policy)
            .await,
        Err(Error::LiveBudget)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM live_budgets")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
    // Explicit fixture representing incomplete restored evidence, not adopted history.
    sqlx::query(
        "INSERT INTO live_budgets(id,limit_microusd,committed_microusd) VALUES($1,100000,10299)",
    )
    .bind(archived)
    .execute(store.pool())
    .await
    .unwrap();
    store
        .credit(agent.id, LedgerKind::Seed, 10000, Uuid::new_v4())
        .await
        .unwrap();
    store
        .transition(
            agent.id,
            Lifecycle::Active,
            "isolated offline fixture",
            true,
        )
        .await
        .unwrap();
    let cycle = store.begin_cycle(agent.id, false).await.unwrap();
    let call = Uuid::new_v4();
    sqlx::query("INSERT INTO inference_calls(id,agent_id,cycle_id,purpose,attempt,policy,budget_id,reserved_microusd,accounted_microusd,status) VALUES($1,$2,$3,'plan',0,$4,$5,4048,4048,'reserved')")
        .bind(call).bind(agent.id).bind(cycle).bind(json!(policy)).bind(archived).execute(store.pool()).await.unwrap();
    let session = LiveSession {
        id: consent.request_id,
        agent_id: agent.id,
        approved_by: owner,
        consent: consent.clone(),
        policy: policy.clone(),
        status: "reserved".into(),
        memory_ids: vec![],
        proposal: None,
        usage: None,
        reserved_microusd: 4048,
        accounted_microusd: 4048,
        failure_code: None,
        metadata_requests: 0,
        generation_requests: 0,
        project_verified: false,
        tool_result: None,
        created_at: Utc::now(),
        finished_at: None,
    };
    sqlx::query("INSERT INTO live_sessions(id,agent_id,approved_by,budget_id,status,lease_until,data) VALUES($1,$2,$3,$4,'reserved',now()+interval '1 minute',$5)")
        .bind(session.id).bind(agent.id).bind(owner).bind(archived).bind(json!(session)).execute(store.pool()).await.unwrap();
    let (a, b) = tokio::join!(
        store.reserve_inference(agent.id, cycle, "plan", 0, &policy, 1000),
        store.reserve_live_session(agent.id, owner, &consent, &policy)
    );
    assert!(matches!(a, Err(Error::LiveBudget)));
    assert!(matches!(b, Err(Error::LiveBudget)));
    assert!(matches!(
        store
            .finish_inference(agent.id, call, None, Some(&Error::ProviderHttp(429)))
            .await,
        Err(Error::LiveBudget)
    ));
    assert!(matches!(
        store
            .settle_live_session(agent.id, session.id, None, None, None, false)
            .await,
        Err(Error::LiveBudget)
    ));
    let reopened = Store::connect(&url).await.unwrap();
    let committed: i64 =
        sqlx::query_scalar("SELECT committed_microusd FROM live_budgets WHERE id=$1")
            .bind(archived)
            .fetch_one(reopened.pool())
            .await
            .unwrap();
    assert_eq!(committed, 10299);
    let held: i64 = sqlx::query_scalar(
        "SELECT accounted_microusd FROM inference_calls WHERE id=$1 AND status='reserved'",
    )
    .bind(call)
    .fetch_one(reopened.pool())
    .await
    .unwrap();
    assert_eq!(held, 4048);
    let unchanged: serde_json::Value =
        sqlx::query_scalar("SELECT data FROM live_sessions WHERE id=$1 AND status='reserved'")
            .bind(session.id)
            .fetch_one(reopened.pool())
            .await
            .unwrap();
    assert_eq!(unchanged, json!(session));
    assert_eq!(
        reopened
            .financials(agent.id)
            .await
            .unwrap()
            .balance_microusd,
        10000
    );
    reopened.pool().close().await;
    store.pool().close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(admin.pool())
        .await
        .unwrap();
}
