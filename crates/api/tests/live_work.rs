//! Mocked inference + genuine disposable PostgreSQL. Never contacts OpenAI or the primary cluster.
#[path = "support/work_provider.rs"]
mod support;
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
use supa_api::{AppState, OperatorCredential, router};
use supa_domain::*;
use supa_persistence::{Role, Store};
use supa_providers::Offline;
use supa_runtime::{Runtime, controlled::ControlledInference};
use tower::ServiceExt;
use uuid::Uuid;
const OWNER: &str = "fixture-work-owner-credential-0000000000";
const OTHER: &str = "fixture-work-other-credential-0000000000";
async fn request(
    app: &Router,
    path: &str,
    token: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .uri(path)
        .header("authorization", format!("Bearer {token}"));
    let body = if let Some(body) = body {
        req = req
            .method("POST")
            .header("content-type", "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    let response = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let data = to_bytes(response.into_body(), 2000000).await.unwrap();
    (status, serde_json::from_slice(&data).unwrap_or(Value::Null))
}
async fn setup() -> (Store, Uuid, Uuid, InferencePolicy) {
    let base = std::env::var("TEST_DATABASE_URL").unwrap();
    let options: sqlx::postgres::PgConnectOptions = base.parse().unwrap();
    assert!(
        matches!(options.get_host(), "localhost" | "127.0.0.1")
            && options.get_port() == 55680
            && options.get_database() == Some("supa_m10_test")
    );
    let admin = Store::connect(&base).await.unwrap();
    let db = format!("supa_m10_{}_test", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {db} TEMPLATE template0"))
        .execute(admin.pool())
        .await
        .unwrap();
    let url = sqlx::ConnectOptions::to_url_lossy(&options.database(&db)).to_string();
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let actor = Uuid::new_v4();
    let mut a = Agent::new(
        "Disposable mock GPT work".into(),
        Personality::default(),
        CycleConfig::default(),
        [Capability::ReadMemory, Capability::EvaluateGoal].into(),
    )
    .unwrap();
    a.lifecycle = Lifecycle::Active;
    store.create_owned(&a, actor).await.unwrap();
    let cycle = store.begin_cycle(a.id, false).await.unwrap();
    store.finish(a.id, cycle, None).await.unwrap();
    store
        .transition(
            a.id,
            Lifecycle::Paused,
            "Disposable evaluation agent remains paused",
            true,
        )
        .await
        .unwrap();
    let policy = support::policy();
    sqlx::query("INSERT INTO live_budgets(id,limit_microusd) VALUES($1,$2)")
        .bind(policy.budget_id)
        .bind(policy.budget_limit_microusd)
        .execute(store.pool())
        .await
        .unwrap();
    (store, a.id, actor, policy)
}
async fn job(store: &Store, agent: Uuid, actor: Uuid, case: &Value) -> EconomicJob {
    let service = Uuid::new_v4();
    let kind: ServiceKind = serde_json::from_value(case["kind"].clone()).unwrap();
    store
        .offer_service(
            agent,
            actor,
            service,
            ServiceSpec {
                name: case["id"].as_str().unwrap().into(),
                kind,
                price_atoms: 5000,
                estimated_inference_atoms: 1000,
                estimated_infrastructure_atoms: 100,
                max_input_bytes: 8000,
                max_attempts: 2,
            },
        )
        .await
        .unwrap();
    store.create_economic_job(actor,CreateJob{request_id:Uuid::new_v4(),service_id:service,network:EconomicNetwork::Simulated,input:json!({"objective":case["objective"],"material":case["material"],"requirements":case["requirements"]}).to_string(),valid_for_seconds:3600}).await.unwrap()
}
async fn consent(
    store: &Store,
    j: &EconomicJob,
    actor: Uuid,
    policy: &InferencePolicy,
    memory: bool,
) -> LiveWorkConsent {
    let r = WorkRequest::parse(&j.input, j.spec.kind).unwrap();
    let mut tools = r
        .tools(j.spec.kind, false)
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if memory {
        tools.push("read_memory".into());
    }
    LiveWorkConsent {
        job_id: j.id,
        expected_revision: j.revision,
        input_sha256: store.work_input_sha256(j.id, actor).await.unwrap(),
        approved_tools: tools,
        use_memory: memory,
        consent: LiveConsent {
            request_id: Uuid::new_v4(),
            goal: r.objective,
            model: policy.model.clone(),
            authorize_paid: true,
            consented_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(2),
            max_requests: 1,
            max_input_tokens: policy.input_token_bound,
            max_output_tokens: policy.max_output_tokens,
            max_cost_microusd: 4000,
        },
    }
}
fn make_app(
    store: &Store,
    agent: Uuid,
    actor: Uuid,
    policy: &InferencePolicy,
    mode: u8,
) -> (Router, Arc<support::MockWorkProvider>) {
    let provider = Arc::new(support::MockWorkProvider {
        policy: policy.clone(),
        calls: AtomicUsize::new(0),
        mode,
    });
    let state = AppState::new(
        store.clone(),
        Runtime::new(store.clone(), Arc::new(Offline)),
        "fixture-public-work-service-00000000000".into(),
    )
    .unwrap()
    .with_credentials(vec![
        OperatorCredential {
            id: actor,
            role: Role::Owner,
            token: OWNER.into(),
        },
        OperatorCredential {
            id: Uuid::new_v4(),
            role: Role::Owner,
            token: OTHER.into(),
        },
    ])
    .unwrap()
    .with_controlled_inference(ControlledInference {
        store: store.clone(),
        provider: provider.clone(),
    })
    .with_live_work_evaluation();
    let _ = agent;
    (router(state), provider)
}
#[tokio::test]
#[ignore = "requires the isolated PostgreSQL supa_m10_test on loopback 55680"]
async fn customer_style_matrix_preserves_evidence_scope_and_accounting() {
    let (store, agent, actor, policy) = setup().await;
    let cases: Value = serde_json::from_str(include_str!(
        "../../../config/work-evaluation-fixtures.json"
    ))
    .unwrap();
    let path = format!("/agents/{agent}/live-work");
    let mut references = vec![];
    for case in cases["cases"].as_array().unwrap() {
        let j = job(&store, agent, actor, case).await;
        let input = consent(
            &store,
            &j,
            actor,
            &policy,
            case["use_memory"].as_bool().unwrap(),
        )
        .await;
        let (app, provider) = make_app(
            &store,
            agent,
            actor,
            &policy,
            case["mode"].as_u64().unwrap() as u8,
        );
        assert_eq!(
            request(&app, &path, OTHER, Some(json!(input))).await.0,
            StatusCode::FORBIDDEN
        );
        let (status, result) = request(&app, &path, OWNER, Some(json!(input))).await;
        assert_eq!(status, StatusCode::OK, "{result}");
        assert_eq!(result["status"], case["expected"], "{}", case["id"]);
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
        assert_eq!(result["accounted_microusd"], 35);
        assert_eq!(result["work"]["provider_evidence"], "mock");
        if case["id"] == "memory-free" {
            assert_eq!(result["memory_ids"].as_array().unwrap().len(), 0);
        }
        if case["id"] == "memory-assisted" {
            assert_eq!(result["memory_ids"].as_array().unwrap().len(), 1);
            assert_eq!(
                result["work"]["report"]["memory_ids"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
        }
        if case["id"] == "unsupported-claim" {
            assert!(result["work"]["report"].is_null());
        }
        assert_ne!(
            request(&app, &path, OWNER, Some(json!(input))).await.0,
            StatusCode::OK
        );
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            store.economic_job(j.id, actor).await.unwrap().state,
            JobState::Created
        );
        let legacy = request(&app, &format!("/agents/{agent}/live"), OWNER, None)
            .await
            .1;
        assert!(legacy["sessions"].as_array().unwrap().is_empty());
        references.push(result["id"].clone());
    }
    let accounted: i64 =
        sqlx::query_scalar("SELECT committed_microusd FROM live_budgets WHERE id=$1")
            .bind(policy.budget_id)
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(accounted, 315);
    let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM economic_entries")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(entries, 0);
    assert_eq!(store.get(agent).await.unwrap().lifecycle, Lifecycle::Paused);
    assert_eq!(store.live_sessions(agent).await.unwrap().len(), 9);
    println!(
        "M10: 9 frozen mocked work cases; grounded results, one unsupported observation rejected; memory-free 0 vs assisted 1 verified reference; independent mocked provider cost 315 microUSD; commercial revenue, payments, simulated expenses and real API calls all zero"
    );
}
#[tokio::test]
#[ignore = "requires the isolated PostgreSQL supa_m10_test on loopback 55680"]
async fn ambiguous_charges_concurrency_and_scope_fail_closed() {
    let (store, agent, actor, policy) = setup().await;
    let cases: Value = serde_json::from_str(include_str!(
        "../../../config/work-evaluation-fixtures.json"
    ))
    .unwrap();
    let j = job(&store, agent, actor, &cases["cases"][0]).await;
    let good = consent(&store, &j, actor, &policy, false).await;
    let path = format!("/agents/{agent}/live-work");
    let (app, provider) = make_app(&store, agent, actor, &policy, 0);
    for mutate in 0..5 {
        let mut bad = good.clone();
        match mutate {
            0 => bad.input_sha256 = "a".repeat(64),
            1 => bad.expected_revision += 1,
            2 => bad.approved_tools[0] = "transfer".into(),
            3 => bad.consent.expires_at = chrono::Utc::now() - chrono::Duration::seconds(1),
            _ => bad.consent.max_cost_microusd = 1,
        };
        assert_ne!(
            request(&app, &path, OWNER, Some(json!(bad))).await.0,
            StatusCode::OK
        );
    }
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    let (a, b) = tokio::join!(
        request(&app, &path, OWNER, Some(json!(good))),
        request(&app, &path, OWNER, Some(json!(good)))
    );
    assert_ne!(a.0 == StatusCode::OK, b.0 == StatusCode::OK);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let (app, provider) = make_app(&store, agent, actor, &policy, 3);
    let auth = consent(&store, &j, actor, &policy, false).await;
    let (_, unknown) = request(&app, &path, OWNER, Some(json!(auth))).await;
    assert_eq!(unknown["status"], "unknown");
    assert_eq!(unknown["accounted_microusd"], policy.reservation().unwrap());
    let next = consent(&store, &j, actor, &policy, false).await;
    assert_ne!(
        request(&app, &path, OWNER, Some(json!(next))).await.0,
        StatusCode::OK
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let options = store.pool().connect_options();
    let reopened = Store::connect(sqlx::ConnectOptions::to_url_lossy(options.as_ref()).as_ref())
        .await
        .unwrap();
    assert!(
        reopened
            .live_sessions(agent)
            .await
            .unwrap()
            .iter()
            .any(|s| s.status == "unknown")
    );
    assert!(
        store
            .live_sessions(agent)
            .await
            .unwrap()
            .iter()
            .any(|s| s.status == "unknown")
    );
}

#[tokio::test]
#[ignore = "requires the isolated PostgreSQL supa_m10_test on loopback 55680"]
async fn revocation_expired_leases_and_restart_retain_charge_evidence() {
    use supa_providers::controlled::ControlledProvider;
    let (store, agent, actor, policy) = setup().await;
    let cases: Value = serde_json::from_str(include_str!(
        "../../../config/work-evaluation-fixtures.json"
    ))
    .unwrap();
    let j = job(&store, agent, actor, &cases["cases"][0]).await;
    let auth = consent(&store, &j, actor, &policy, false).await;
    let (session, _) = store
        .reserve_live_work_session(agent, actor, &auth, &policy, "mock")
        .await
        .unwrap();
    store
        .live_request_stage(agent, session.id, false)
        .await
        .unwrap();
    assert!(
        store
            .live_request_stage(agent, session.id, false)
            .await
            .is_err()
    );
    sqlx::query(
        "UPDATE live_sessions SET lease_until=clock_timestamp()-interval '1 second' WHERE id=$1",
    )
    .bind(session.id)
    .execute(store.pool())
    .await
    .unwrap();
    assert!(
        store
            .live_request_stage(agent, session.id, true)
            .await
            .is_err()
    );
    store.recover_live_sessions().await.unwrap();
    let options = store.pool().connect_options();
    let reopened = Store::connect(sqlx::ConnectOptions::to_url_lossy(options.as_ref()).as_ref())
        .await
        .unwrap();
    let persisted = reopened.live_sessions(agent).await.unwrap();
    assert_eq!(persisted[0].status, "unknown");
    assert_eq!(persisted[0].generation_requests, 0);
    assert_eq!(
        persisted[0].accounted_microusd,
        policy.reservation().unwrap()
    );
    assert!(
        reopened
            .reserve_live_work_session(
                agent,
                actor,
                &consent(&store, &j, actor, &policy, false).await,
                &policy,
                "mock"
            )
            .await
            .is_err()
    );
    // A separate synthetic budget proves revocation fences output but does not erase known charges.
    let second = support::policy();
    sqlx::query("INSERT INTO live_budgets(id,limit_microusd) VALUES($1,$2)")
        .bind(second.budget_id)
        .bind(second.budget_limit_microusd)
        .execute(store.pool())
        .await
        .unwrap();
    let second_job = job(&store, agent, actor, &cases["cases"][1]).await;
    let auth = consent(&store, &second_job, actor, &second, false).await;
    let (session, _) = store
        .reserve_live_work_session(agent, actor, &auth, &second, "mock")
        .await
        .unwrap();
    store
        .live_request_stage(agent, session.id, false)
        .await
        .unwrap();
    store
        .live_request_stage(agent, session.id, true)
        .await
        .unwrap();
    let work = session.work.unwrap();
    let input = LiveWorkInput {
        request: work.request,
        kind: work.kind,
        approved_tools: auth.approved_tools,
        memories: vec![],
        deterministic_findings: work.baseline["findings"].clone(),
    };
    let provider = support::MockWorkProvider {
        policy: second.clone(),
        calls: AtomicUsize::new(0),
        mode: 0,
    };
    let result = provider.work(&input).await;
    sqlx::query("DELETE FROM agent_access WHERE agent_id=$1 AND principal_id=$2")
        .bind(agent)
        .bind(actor)
        .execute(store.pool())
        .await
        .unwrap();
    let settled = store
        .settle_live_work_session(
            agent,
            session.id,
            Some(result.result.unwrap()),
            result.usage,
            None,
            true,
        )
        .await
        .unwrap();
    assert_eq!(settled.status, "failed");
    assert_eq!(settled.accounted_microusd, 35);
    assert!(settled.work.unwrap().report.is_none());
    let (app, _) = make_app(&store, agent, actor, &second, 0);
    assert_eq!(
        request(&app, &format!("/agents/{agent}/live-work"), OWNER, None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL; one 26-second mock timeout, never OpenAI"]
async fn missing_usage_metadata_failure_and_timeout_do_not_replay_paid_work() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../config/work-evaluation-fixtures.json"
    ))
    .unwrap();
    for mode in [4, 5, 6] {
        let (store, agent, actor, policy) = setup().await;
        let j = job(&store, agent, actor, &cases["cases"][0]).await;
        let scope = consent(&store, &j, actor, &policy, false).await;
        let (app, provider) = make_app(&store, agent, actor, &policy, mode);
        let path = format!("/agents/{agent}/live-work");
        let (status, result) = request(&app, &path, OWNER, Some(json!(scope))).await;
        assert_eq!(status, StatusCode::OK, "{result}");
        if mode == 5 {
            assert_eq!(result["generation_requests"], 0);
            assert_eq!(result["accounted_microusd"], 0);
            assert_eq!(result["status"], "failed");
            assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
        } else {
            assert_eq!(result["status"], "unknown");
            assert_eq!(result["accounted_microusd"], policy.reservation().unwrap());
            assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
            assert!(result["work"]["report"].is_null());
        }
        assert_ne!(
            request(&app, &path, OWNER, Some(json!(scope))).await.0,
            StatusCode::OK
        );
    }
}

/// Mocked Claude origin tag exercises persistence only; never constructs an Anthropic client.
struct MockClaudeWork(support::MockWorkProvider, &'static str);
#[async_trait::async_trait]
impl supa_providers::controlled::ControlledProvider for MockClaudeWork {
    fn policy(&self) -> InferencePolicy {
        self.0.policy.clone()
    }
    fn project(&self) -> &str {
        self.1
    }
    fn work_evidence_kind(&self) -> &str {
        "anthropic"
    }
    async fn verify(&self) -> Result<()> {
        Ok(())
    }
    async fn verify_observed(&self) -> supa_providers::Invocation<()> {
        let mut d = ProviderDiagnostic::new(ProviderOperation::Metadata, ProviderStage::Completed);
        d.http_status = Some(200);
        d.request_id = Some("req_METADATA123".into());
        d.dispatch_intended = true;
        d.response_complete = true;
        supa_providers::Invocation {
            result: Ok(()),
            usage: None,
            diagnostic: Some(d),
        }
    }
    async fn reason(&self, _: &str, _: &[Memory]) -> supa_providers::Invocation<LiveProposal> {
        supa_providers::Invocation {
            diagnostic: None,
            result: Err(Error::Denied),
            usage: None,
        }
    }
    async fn work(&self, input: &LiveWorkInput) -> supa_providers::Invocation<WorkProposal> {
        if self.0.mode == 9 {
            self.0.calls.fetch_add(1, Ordering::SeqCst);
            let mut d = ProviderDiagnostic::new(
                ProviderOperation::Generation,
                ProviderStage::ResponseHeaders,
            );
            d.http_status = Some(429);
            d.request_id = Some("req_GENERATION123".into());
            d.failure = Some(ProviderFailure::HttpStatus);
            d.error_type = Some("rate_limit_error".into());
            d.dispatch_intended = true;
            d.response_complete = true;
            return supa_providers::Invocation {
                result: Err(Error::ProviderHttp(429)),
                usage: None,
                diagnostic: Some(d),
            };
        }
        supa_providers::controlled::ControlledProvider::work(&self.0, input).await
    }
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL supa_m10_test; synthetic provider only"]
async fn claude_work_accounting_is_audited_and_budget_cannot_switch_models() {
    let (store, agent, actor, mut policy) = setup().await;
    policy.model = "claude-haiku-5-5".into();
    let cases: Value = serde_json::from_str(include_str!(
        "../../../config/work-evaluation-fixtures.json"
    ))
    .unwrap();
    let j = job(&store, agent, actor, &cases["cases"][0]).await;
    let provider = Arc::new(MockClaudeWork(
        support::MockWorkProvider {
            policy: policy.clone(),
            calls: AtomicUsize::new(0),
            mode: 0,
        },
        "wrkspc_synthetic_fixture",
    ));
    let runtime = ControlledInference {
        store: store.clone(),
        provider: provider.clone(),
    };
    let c = consent(&store, &j, actor, &policy, false).await;
    let session = runtime.run_work(agent, actor, c.clone()).await.unwrap();
    assert_eq!(session.status, "awaiting_approval");
    assert_eq!(
        session.provider_scope.as_deref(),
        Some("wrkspc_synthetic_fixture")
    );
    assert_eq!(session.work.unwrap().provider_evidence, "anthropic");
    assert_eq!(provider.0.calls.load(Ordering::SeqCst), 1);
    assert!(runtime.run_work(agent, actor, c).await.is_err());
    let mut other = policy.clone();
    other.model = "gpt-6-luna".into();
    let c = consent(&store, &j, actor, &other, false).await;
    let rejected = ControlledInference {
        store: store.clone(),
        provider: Arc::new(MockClaudeWork(
            support::MockWorkProvider {
                policy: other,
                calls: AtomicUsize::new(0),
                mode: 0,
            },
            "wrkspc_synthetic_fixture",
        )),
    };
    assert!(matches!(
        rejected.run_work(agent, actor, c).await,
        Err(Error::LiveBudget)
    ));
    let c = consent(&store, &j, actor, &policy, false).await;
    let wrong_scope = ControlledInference {
        store: store.clone(),
        provider: Arc::new(MockClaudeWork(
            support::MockWorkProvider {
                policy,
                calls: AtomicUsize::new(0),
                mode: 0,
            },
            "wrkspc_other_account",
        )),
    };
    assert!(matches!(
        wrong_scope.run_work(agent, actor, c).await,
        Err(Error::LiveBudget)
    ));
    let saved = store.economic_job(j.id, actor).await.unwrap();
    assert_eq!(saved.state, JobState::Created);
    assert_eq!(saved.received_atoms, 0);
    assert_eq!(saved.earned_atoms, 0);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL supa_m10_test; synthetic provider only"]
async fn uncertain_diagnostics_survive_reconnect_and_preserve_replay_fences() {
    let (store, agent, actor, policy) = setup().await;
    let cases: Value = serde_json::from_str(include_str!(
        "../../../config/work-evaluation-fixtures.json"
    ))
    .unwrap();
    let j = job(&store, agent, actor, &cases["cases"][0]).await;
    let provider = Arc::new(MockClaudeWork(
        support::MockWorkProvider {
            policy: policy.clone(),
            calls: AtomicUsize::new(0),
            mode: 9,
        },
        "wrkspc_synthetic_fixture",
    ));
    let runtime = ControlledInference {
        store: store.clone(),
        provider: provider.clone(),
    };
    let c = consent(&store, &j, actor, &policy, false).await;
    let session = runtime.run_work(agent, actor, c.clone()).await.unwrap();
    assert_eq!(session.status, "unknown");
    assert_eq!(
        session.failure_code.as_deref(),
        Some("provider_charge_requires_review")
    );
    assert_eq!(
        session.technical_failure_code.as_deref(),
        Some("provider_http_429")
    );
    assert_eq!(session.provider_diagnostics.len(), 2);
    let d = &session.provider_diagnostics[1];
    assert_eq!(d.http_status, Some(429));
    assert_eq!(d.request_id.as_deref(), Some("req_GENERATION123"));
    let cost = policy.reservation().unwrap();
    assert_eq!(session.accounted_microusd, cost);
    let url = sqlx::ConnectOptions::to_url_lossy(&*store.pool().connect_options()).to_string();
    let reconnected = Store::connect(&url).await.unwrap();
    reconnected.recover_live_sessions().await.unwrap();
    let saved = reconnected.live_sessions(agent).await.unwrap().remove(0);
    assert_eq!(
        serde_json::to_value(&saved).unwrap(),
        serde_json::to_value(&session).unwrap()
    );
    assert!(runtime.run_work(agent, actor, c).await.is_err());
    let fresh = consent(&store, &j, actor, &policy, false).await;
    assert!(runtime.run_work(agent, actor, fresh).await.is_err());
    assert_eq!(provider.0.calls.load(Ordering::SeqCst), 1);
    assert!(
        reconnected
            .record_live_diagnostic(agent, saved.id, d)
            .await
            .is_err()
    );
    let committed: i64 =
        sqlx::query_scalar("SELECT committed_microusd FROM live_budgets WHERE id=$1")
            .bind(policy.budget_id)
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(committed, cost);
    let mut old = serde_json::to_value(&saved).unwrap();
    old.as_object_mut().unwrap().remove("provider_diagnostics");
    old.as_object_mut()
        .unwrap()
        .remove("technical_failure_code");
    let old: LiveSession = serde_json::from_value(old).unwrap();
    assert!(old.provider_diagnostics.is_empty());
    assert!(old.technical_failure_code.is_none());
}
#[tokio::test]
#[ignore = "requires isolated PostgreSQL supa_m10_test; synthetic provider only"]
async fn interrupted_reserved_diagnostics_survive_recovery_without_refund_or_replay() {
    let (store, agent, actor, policy) = setup().await;
    let cases: Value = serde_json::from_str(include_str!(
        "../../../config/work-evaluation-fixtures.json"
    ))
    .unwrap();
    let j = job(&store, agent, actor, &cases["cases"][0]).await;
    let c = consent(&store, &j, actor, &policy, false).await;
    let (session, _) = store
        .reserve_live_work_session_scoped(
            agent,
            actor,
            &c,
            &policy,
            ("anthropic", "wrkspc_synthetic_fixture"),
        )
        .await
        .unwrap();
    store
        .live_request_stage(agent, session.id, false)
        .await
        .unwrap();
    store
        .live_request_stage(agent, session.id, true)
        .await
        .unwrap();
    let mut d = ProviderDiagnostic::new(ProviderOperation::Generation, ProviderStage::ResponseBody);
    d.http_status = Some(200);
    d.request_id = Some("req_INTERRUPTED123".into());
    d.dispatch_intended = true;
    store
        .record_live_diagnostic(agent, session.id, &d)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE live_sessions SET lease_until=clock_timestamp()-interval '1 second' WHERE id=$1",
    )
    .bind(session.id)
    .execute(store.pool())
    .await
    .unwrap();
    store.recover_live_sessions().await.unwrap();
    let saved = store.live_sessions(agent).await.unwrap().remove(0);
    assert_eq!(saved.status, "unknown");
    assert_eq!(saved.provider_diagnostics[0].request_id, d.request_id);
    assert_eq!(saved.accounted_microusd, session.reserved_microusd);
    assert!(
        store
            .reserve_live_work_session_scoped(
                agent,
                actor,
                &c,
                &policy,
                ("anthropic", "wrkspc_synthetic_fixture")
            )
            .await
            .is_err()
    );
    let fresh = consent(&store, &j, actor, &policy, false).await;
    assert!(
        store
            .reserve_live_work_session_scoped(
                agent,
                actor,
                &fresh,
                &policy,
                ("anthropic", "wrkspc_synthetic_fixture")
            )
            .await
            .is_err()
    );
}
