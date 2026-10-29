//! Mock provider + real isolated PostgreSQL. These tests never contact OpenAI.
use async_trait::async_trait;
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
use supa_providers::{Invocation, Offline, controlled::ControlledProvider};
use supa_runtime::{Runtime, controlled::ControlledInference};
use tower::ServiceExt;
use uuid::Uuid;
const TOKEN: &str = "fixture-controlled-owner-000000000000";
struct Mock {
    policy: InferencePolicy,
    calls: AtomicUsize,
    mode: u8,
}
#[async_trait]
impl ControlledProvider for Mock {
    fn policy(&self) -> InferencePolicy {
        self.policy.clone()
    }
    fn project(&self) -> &str {
        "proj_fixture"
    }
    async fn verify(&self) -> Result<()> {
        if self.mode == 3 {
            Err(Error::ProviderHttp(401))
        } else {
            Ok(())
        }
    }
    async fn reason(&self, _: &str, _: &[Memory]) -> Invocation<LiveProposal> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        if self.mode == 6 {
            std::future::pending::<()>().await;
        }
        if self.mode == 5 {
            return Invocation {
                usage: None,
                result: Err(Error::ProviderHttp(429)),
            };
        }
        if self.mode == 1 {
            return Invocation {
                usage: None,
                result: Err(Error::ProviderTransport),
            };
        }
        let usage = TokenUsage {
            input_tokens: if self.mode == 4 { 30000 } else { 100 },
            output_tokens: 50,
            cached_input_tokens: 0,
            cache_write_tokens: 0,
        };
        Invocation {
            usage: Some(usage),
            result: Ok(LiveProposal {
                summary: "Review the supplied memories before acting.".into(),
                memory_ids: vec![],
                actions: vec![ProposedAction {
                    tool: if self.mode == 2 {
                        "transfer"
                    } else {
                        "read_memory"
                    }
                    .into(),
                    event_ids: vec![],
                    lesson: None,
                    evaluation: None,
                }],
            }),
        }
    }
}
struct Fixture {
    store: Store,
    admin: Store,
    schema: String,
    url: String,
    agent: Uuid,
    owner: Uuid,
    provider: Arc<Mock>,
    service: ControlledInference,
    app: Router,
}
impl Fixture {
    async fn new(mode: u8) -> Self {
        let base = std::env::var("TEST_DATABASE_URL").expect("dedicated test database");
        assert!(base.contains("_test"));
        let admin = Store::connect(&base).await.unwrap();
        let schema = format!("controlled_{}", Uuid::new_v4().simple());
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
            "Controlled fixture".into(),
            Personality::default(),
            CycleConfig::default(),
            [Capability::ReadMemory].into_iter().collect(),
        )
        .unwrap();
        store.create_owned(&agent, owner).await.unwrap();
        let policy = InferencePolicy {
            model: "gpt-6-luna".into(),
            live: true,
            budget_id: Some(Uuid::new_v4()),
            budget_limit_microusd: 100000,
            input_rate: 100000,
            cached_input_rate: 10000,
            cache_write_rate: 125000,
            output_rate: 500000,
            input_token_bound: 24192,
            max_output_tokens: 1024,
        };
        // Fixture-only provisioning. Runtime code cannot create or refill this budget.
        sqlx::query(
            "INSERT INTO live_budgets(id,limit_microusd,committed_microusd) VALUES($1,100000,7948)",
        )
        .bind(policy.budget_id)
        .execute(store.pool())
        .await
        .unwrap();
        let provider = Arc::new(Mock {
            policy,
            calls: AtomicUsize::new(0),
            mode,
        });
        let service = ControlledInference {
            store: store.clone(),
            provider: provider.clone(),
        };
        let state = AppState::new(
            store.clone(),
            Runtime::new(store.clone(), Arc::new(Offline)),
            TOKEN.into(),
        )
        .unwrap()
        .with_operator(owner)
        .unwrap()
        .with_controlled_inference(service.clone());
        let app = router(state);
        Self {
            store,
            admin,
            schema,
            url,
            agent: agent.id,
            owner,
            provider,
            service,
            app,
        }
    }
    fn consent(&self) -> LiveConsent {
        serde_json::from_value(json!({"request_id":Uuid::new_v4(),"goal":"Inspect permitted memories","model":"gpt-6-luna","authorize_paid":true,"consented_at":now(),"expires_at":future(),"max_requests":1,"max_input_tokens":24192,"max_output_tokens":1024,"max_cost_microusd":10000})).unwrap()
    }
    async fn spent(&self) -> i64 {
        sqlx::query_scalar("SELECT committed_microusd FROM live_budgets WHERE id=$1")
            .bind(self.provider.policy.budget_id)
            .fetch_one(self.store.pool())
            .await
            .unwrap()
    }
    async fn close(self) {
        self.store.pool().close().await;
        sqlx::query(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .execute(self.admin.pool())
            .await
            .unwrap();
    }
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
fn future() -> String {
    (chrono::Utc::now() + chrono::Duration::seconds(300)).to_rfc3339()
}
async fn http(
    app: &Router,
    token: Option<&str>,
    method: &str,
    path: &str,
    body: Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let r = app
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    (
        r.status(),
        serde_json::from_slice(&to_bytes(r.into_body(), 128000).await.unwrap()).unwrap(),
    )
}
#[tokio::test]
#[ignore = "requires isolated local PostgreSQL; mocked inference only"]
async fn bounded_session_separates_inference_approval_accounting_and_restart() {
    let f = Fixture::new(0).await;
    let consent = f.consent();
    let id = consent.request_id;
    let before = f.store.financials(f.agent).await.unwrap().balance_microusd;
    let s = f
        .service
        .run(f.agent, f.owner, consent.clone())
        .await
        .unwrap();
    assert_eq!(s.status, "awaiting_approval");
    assert_eq!(s.generation_requests, 1);
    assert!(s.project_verified);
    assert!(s.tool_result.is_none());
    assert_eq!(s.usage.unwrap().total(), 150);
    assert_eq!(f.spent().await, 7983);
    assert!(f.service.run(f.agent, f.owner, consent).await.is_err());
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), 1);
    let reopened = Store::connect(&f.url).await.unwrap();
    assert_eq!(reopened.live_sessions(f.agent).await.unwrap()[0].id, id);
    reopened.pool().close().await;
    let (a, b) = tokio::join!(
        f.service.approve(f.agent, id, f.owner, true),
        f.service.approve(f.agent, id, f.owner, true)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert_eq!(
        f.store.financials(f.agent).await.unwrap().balance_microusd,
        before
    );
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM live_session_audit WHERE operation='approve_tools'",
    )
    .fetch_one(f.store.pool())
    .await
    .unwrap();
    assert_eq!(audits, 1);
    f.close().await;
}
#[tokio::test]
#[ignore = "requires isolated local PostgreSQL; mocked inference only"]
async fn concurrent_sessions_cannot_duplicate_or_exceed_cumulative_budget() {
    let f = Fixture::new(0).await;
    let consent = f.consent();
    let (a, b) = tokio::join!(
        f.service.run(f.agent, f.owner, consent.clone()),
        f.service.run(f.agent, f.owner, consent)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), 1);
    sqlx::query("UPDATE live_budgets SET committed_microusd=99999")
        .execute(f.store.pool())
        .await
        .unwrap();
    assert!(matches!(
        f.service.run(f.agent, f.owner, f.consent()).await,
        Err(Error::LiveBudget)
    ));
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), 1);
    f.close().await;
}
#[tokio::test]
#[ignore = "requires isolated local PostgreSQL; mocked inference only"]
async fn uncertain_charges_and_crash_recovery_never_retry_or_release_reservations() {
    let f = Fixture::new(1).await;
    let s = f.service.run(f.agent, f.owner, f.consent()).await.unwrap();
    assert_eq!(s.status, "unknown");
    assert_eq!(f.spent().await, 7948 + s.reserved_microusd);
    assert!(matches!(
        f.service.run(f.agent, f.owner, f.consent()).await,
        Err(Error::Busy)
    ));
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), 1);
    f.close().await;
    let f = Fixture::new(0).await;
    let (s, _) = f
        .store
        .reserve_live_session(f.agent, f.owner, &f.consent(), &f.provider.policy)
        .await
        .unwrap();
    sqlx::query("UPDATE live_sessions SET lease_until=now()-interval '1 second'")
        .execute(f.store.pool())
        .await
        .unwrap();
    f.store.recover_live_sessions().await.unwrap();
    assert_eq!(
        f.store.live_sessions(f.agent).await.unwrap()[0].status,
        "unknown"
    );
    assert!(f.service.run(f.agent, f.owner, f.consent()).await.is_err());
    assert_eq!(f.spent().await, 7948 + s.reserved_microusd);
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), 0);
    f.close().await;
}
#[tokio::test]
#[ignore = "requires isolated local PostgreSQL; mocked inference only"]
async fn malicious_proposals_authentication_failures_and_overruns_fail_closed() {
    for mode in [2, 3, 4] {
        let f = Fixture::new(mode).await;
        let s = f.service.run(f.agent, f.owner, f.consent()).await.unwrap();
        assert_eq!(s.status, if mode == 4 { "unknown" } else { "failed" });
        assert!(s.proposal.is_none());
        assert!(
            f.service
                .approve(f.agent, s.id, f.owner, true)
                .await
                .is_err()
        );
        if mode == 3 {
            assert_eq!(s.generation_requests, 0);
            assert_eq!(f.spent().await, 7948);
        }
        f.close().await;
    }
}
#[tokio::test]
#[ignore = "requires isolated local PostgreSQL; mocked inference only"]
async fn authentication_ownership_explicit_consent_and_model_caps_are_enforced() {
    let f = Fixture::new(0).await;
    let path = format!("/agents/{}/live", f.agent);
    let c = f.consent();
    assert_eq!(
        http(
            &f.app,
            None,
            "POST",
            &path,
            serde_json::to_value(&c).unwrap()
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert!(matches!(
        f.service.run(f.agent, Uuid::new_v4(), c.clone()).await,
        Err(Error::Denied)
    ));
    for field in [
        "authorize_paid",
        "model",
        "max_requests",
        "max_cost_microusd",
        "max_input_tokens",
    ] {
        let mut v = serde_json::to_value(&c).unwrap();
        v[field] = match field {
            "authorize_paid" => json!(false),
            "model" => json!("unapproved-model"),
            "max_requests" => json!(2),
            "max_cost_microusd" => json!(1),
            _ => json!(9000),
        };
        let (code, _) = http(&f.app, Some(TOKEN), "POST", &path, v).await;
        assert!(!code.is_success());
    }
    assert_eq!(f.provider.calls.load(Ordering::SeqCst), 0);
    assert_eq!(f.spent().await, 7948);
    f.close().await;
}

#[tokio::test]
#[ignore = "requires isolated local PostgreSQL; one 26-second mock timeout, no OpenAI"]
async fn rate_limits_and_timeouts_never_retry_and_keep_uncertain_charges() {
    for mode in [5, 6] {
        let f = Fixture::new(mode).await;
        let s = f.service.run(f.agent, f.owner, f.consent()).await.unwrap();
        assert_eq!(s.status, "unknown");
        assert_eq!(s.generation_requests, 1);
        assert_eq!(f.provider.calls.load(Ordering::SeqCst), 1);
        assert_eq!(f.spent().await, 7948 + s.reserved_microusd);
        f.close().await;
    }
}
