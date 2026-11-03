//! SELECT-only disposable role regression. No live provider or existing evaluation writes.
#[path = "support/disposable.rs"]
mod disposable;
#[path = "support/work_provider.rs"]
mod support;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use supa_api::{AppState, OperatorCredential, router};
use supa_domain::*;
use supa_persistence::{Role, Store};
use supa_runtime::Runtime;
use tower::ServiceExt;
use uuid::Uuid;

const OWNER: &str = "wallet-read-owner-fixture-credential-01";
const OTHER: &str = "wallet-read-other-fixture-credential-02";
const PUBLIC: &str = "wallet-read-public-fixture-credential-03";

async fn get(app: &Router, path: &str, token: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 256000).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
#[ignore = "requires verified disposable PostgreSQL; no provider calls"]
async fn select_only_job_reads_preserve_wallet_locks_and_owner_boundaries() {
    let base = std::env::var("TEST_DATABASE_URL").unwrap();
    let options: sqlx::postgres::PgConnectOptions = base.parse().unwrap();
    let admin = disposable::connect(&options).await;
    let suffix = Uuid::new_v4().simple().to_string();
    let database = format!("supa_read_{suffix}_test");
    let role = format!("supa_read_{suffix}");
    sqlx::query(&format!("CREATE DATABASE {database} TEMPLATE template0"))
        .execute(admin.pool())
        .await
        .unwrap();
    let db_options = options.database(&database);
    let url = sqlx::ConnectOptions::to_url_lossy(&db_options).to_string();
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let owner = Uuid::new_v4();
    let other = Uuid::new_v4();
    let mut agent = Agent::new(
        "Read permission fixture".into(),
        Personality::default(),
        CycleConfig::default(),
        [Capability::ReadMemory, Capability::EvaluateGoal].into(),
    )
    .unwrap();
    agent.lifecycle = Lifecycle::Active;
    store.create_owned(&agent, owner).await.unwrap();
    let service = Uuid::new_v4();
    store
        .offer_service(
            agent.id,
            owner,
            service,
            ServiceSpec {
                name: "Supplied summary".into(),
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
    let job = store.create_economic_job(owner, CreateJob {
        request_id: Uuid::new_v4(), service_id: service, network: EconomicNetwork::Simulated,
        input: json!({"objective":"Compare supplied facts", "material":"A: snapshot. B: replication.", "requirements":[]}).to_string(),
        valid_for_seconds: 3600,
    }).await.unwrap();
    store
        .transition(agent.id, Lifecycle::Paused, "Read-only regression", true)
        .await
        .unwrap();
    let password = Uuid::new_v4().simple().to_string();
    sqlx::query(&format!(
        "CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{password}'"
    ))
    .execute(admin.pool())
    .await
    .unwrap();
    sqlx::raw_sql(&format!("GRANT CONNECT ON DATABASE {database} TO {role}; GRANT USAGE ON SCHEMA public TO {role}; GRANT SELECT ON ALL TABLES IN SCHEMA public TO {role}; ALTER ROLE {role} SET default_transaction_read_only=on;"))
        .execute(store.pool()).await.unwrap();
    let reader_url =
        sqlx::ConnectOptions::to_url_lossy(&db_options.username(&role).password(&password))
            .to_string();
    let reader = Store::connect(&reader_url).await.unwrap();
    let app = router(
        AppState::new(
            reader.clone(),
            Runtime::new(reader.clone(), Arc::new(supa_providers::Offline)),
            PUBLIC.into(),
        )
        .unwrap()
        .with_credentials(vec![
            OperatorCredential {
                id: owner,
                role: Role::Owner,
                token: OWNER.into(),
            },
            OperatorCredential {
                id: other,
                role: Role::Owner,
                token: OTHER.into(),
            },
        ])
        .unwrap(),
    );
    let path = format!("/jobs/{}", job.id);
    let (status, body) = get(&app, &path, OWNER).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["payment_request"]["recipient"].is_null());
    assert_eq!(get(&app, &path, OTHER).await.0, StatusCode::FORBIDDEN);
    assert_eq!(get(&app, &path, PUBLIC).await.0, StatusCode::FORBIDDEN);

    // An existing wallet must also be readable, not just the missing-wallet case.
    let wallet = WalletIdentity {
        id: Uuid::new_v4(),
        agent_id: agent.id,
        network: SolanaNetwork::Devnet,
        address: "11111111111111111111111111111112".into(),
        owner_id: owner,
        origin: IdentityOrigin::HumanOperator,
    };
    store.register_wallet(&wallet, owner).await.unwrap();
    let writable: bool =
        sqlx::query_scalar("SELECT has_table_privilege(current_user,'wallet_identities','UPDATE')")
            .fetch_one(reader.pool())
            .await
            .unwrap();
    assert!(!writable);
    assert_eq!(reader.wallet(agent.id).await.unwrap().id, wallet.id);
    assert!(reader.wallet_balance(agent.id).await.unwrap().is_none());
    assert_eq!(
        reader.monitored_addresses(agent.id).await.unwrap(),
        vec![wallet.address.clone()]
    );
    assert_eq!(
        reader.scan_cursor(agent.id, &wallet.address).await.unwrap(),
        json!({})
    );
    let (status, body) = get(&app, &path, OWNER).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["payment_request"]["recipient"], wallet.address);
    assert_eq!(body["job"]["state"], "created");
    assert_eq!(body["job"]["received_atoms"], 0);
    assert_eq!(body["job"]["earned_atoms"], 0);
    assert_eq!(get(&app, &path, OTHER).await.0, StatusCode::FORBIDDEN);
    assert!(
        reader
            .set_spending_policy(agent.id, owner, &SpendingPolicy::default())
            .await
            .is_err()
    );
    assert!(
        store
            .set_spending_policy(agent.id, other, &SpendingPolicy::default())
            .await
            .is_err()
    );

    // A read passes an existing row lock; a financial policy write waits for it.
    let mut lock = store.pool().begin().await.unwrap();
    sqlx::query("SELECT id FROM wallet_identities WHERE id=$1 FOR UPDATE")
        .bind(wallet.id)
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), reader.wallet(agent.id))
            .await
            .unwrap()
            .unwrap()
            .id,
        wallet.id
    );
    let writer = store.clone();
    let mut pending = tokio::spawn(async move {
        writer
            .set_spending_policy(agent.id, owner, &SpendingPolicy::default())
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(150), &mut pending)
            .await
            .is_err()
    );
    lock.rollback().await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), pending)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let agent = store.get(agent.id).await.unwrap();
    assert_eq!(agent.lifecycle, Lifecycle::Paused);
    let count: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM live_sessions)+(SELECT count(*) FROM inference_calls)+(SELECT count(*) FROM operating_reservations)+(SELECT count(*) FROM economic_entries)")
        .fetch_one(store.pool()).await.unwrap();
    assert_eq!(count, 0);
    // Mirror the reviewed execution grants in this throwaway database only.
    // The real evaluation login and its budget are never changed by this test.
    let execution_role = format!("supa_exec_{suffix}");
    let execution_password = Uuid::new_v4().simple().to_string();
    sqlx::query(&format!("CREATE ROLE {execution_role} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{execution_password}'"))
        .execute(admin.pool()).await.unwrap();
    sqlx::raw_sql(&format!("GRANT CONNECT ON DATABASE {database} TO {execution_role}; GRANT USAGE ON SCHEMA public TO {execution_role}; GRANT SELECT ON ALL TABLES IN SCHEMA public TO {execution_role}; GRANT UPDATE(id) ON agents,economic_jobs TO {execution_role}; GRANT UPDATE(committed_microusd) ON live_budgets TO {execution_role}; GRANT INSERT,UPDATE ON live_sessions TO {execution_role}; GRANT INSERT ON live_session_audit TO {execution_role}; GRANT USAGE ON SEQUENCE live_session_audit_id_seq TO {execution_role};"))
        .execute(store.pool()).await.unwrap();
    let options: sqlx::postgres::PgConnectOptions = url.parse().unwrap();
    let executor = Store::connect(
        sqlx::ConnectOptions::to_url_lossy(
            &options
                .username(&execution_role)
                .password(&execution_password),
        )
        .as_str(),
    )
    .await
    .unwrap();
    let mut policy = support::policy();
    policy.budget_limit_microusd = 5000;
    sqlx::query("INSERT INTO live_budgets(id,limit_microusd) VALUES($1,5000)")
        .bind(policy.budget_id)
        .execute(store.pool())
        .await
        .unwrap();
    let provider = Arc::new(support::MockWorkProvider {
        policy: policy.clone(),
        calls: AtomicUsize::new(0),
        mode: 0,
    });
    let inference = supa_runtime::controlled::ControlledInference {
        store: executor.clone(),
        provider: provider.clone(),
    };
    let authorization = LiveWorkConsent {
        job_id: job.id,
        expected_revision: 0,
        input_sha256: executor.work_input_sha256(job.id, owner).await.unwrap(),
        approved_tools: vec![
            "inspect_input".into(),
            "extract_evidence".into(),
            "check_requirements".into(),
        ],
        use_memory: false,
        consent: LiveConsent {
            request_id: Uuid::new_v4(),
            goal: "Compare supplied facts".into(),
            model: policy.model.clone(),
            authorize_paid: true,
            consented_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(2),
            max_requests: 1,
            max_input_tokens: 24192,
            max_output_tokens: 1024,
            max_cost_microusd: 4000,
        },
    };
    let result = inference
        .run_work(agent.id, owner, authorization.clone())
        .await
        .unwrap();
    assert_eq!(result.status, "awaiting_approval");
    assert_eq!(result.generation_requests, 1);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert!(
        inference
            .run_work(agent.id, owner, authorization)
            .await
            .is_err()
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let wallet_writable: bool = sqlx::query_scalar(
        "SELECT has_any_column_privilege(current_user,'wallet_identities','UPDATE')",
    )
    .fetch_one(executor.pool())
    .await
    .unwrap();
    assert!(!wallet_writable);
    assert_eq!(
        store.get(agent.id).await.unwrap().lifecycle,
        Lifecycle::Paused
    );
    assert_eq!(
        store.economic_job(job.id, owner).await.unwrap().state,
        JobState::Created
    );
    executor.pool().close().await;
    drop(app);
    reader.pool().close().await;
    store.pool().close().await;
    sqlx::query(&format!("DROP DATABASE {database}"))
        .execute(admin.pool())
        .await
        .unwrap();
    sqlx::query(&format!("DROP ROLE {role}"))
        .execute(admin.pool())
        .await
        .unwrap();
    sqlx::query(&format!("DROP ROLE {execution_role}"))
        .execute(admin.pool())
        .await
        .unwrap();
    admin.pool().close().await;
}
