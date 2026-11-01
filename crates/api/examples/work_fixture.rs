//! Disposable HTTP fixture. No OpenAI object, dotenv loader, wallet, scheduler or external transport.
#[path = "../tests/support/work_provider.rs"]
mod support;
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicUsize};
use supa_api::{AppState, OperatorCredential, router};
use supa_domain::*;
use supa_persistence::{Role, Store};
use supa_providers::Offline;
use supa_runtime::{Runtime, controlled::ControlledInference};
use uuid::Uuid;
#[tokio::main]
async fn main() {
    let url = std::env::var("TEST_DATABASE_URL").expect("isolated test database required");
    let options: sqlx::postgres::PgConnectOptions = url.parse().unwrap();
    assert!(
        matches!(options.get_host(), "localhost" | "127.0.0.1")
            && options.get_port() == 55680
            && options.get_database() == Some("supa_m10_test")
    );
    let store = Store::connect(&url).await.unwrap();
    let directory: String = sqlx::query_scalar("SHOW data_directory")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert!(directory.starts_with("/tmp/supa-m10-pg-"));
    store.migrate().await.unwrap();
    let epochs: i64 = sqlx::query_scalar("SELECT count(*) FROM application_epoch")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(epochs, 0);
    let actor: Uuid = std::env::var("SUPA_TEST_OWNER_ID")
        .unwrap()
        .parse()
        .unwrap();
    let mut agent = Agent::new(
        "Disposable mocked GPT work".into(),
        Personality::default(),
        CycleConfig::default(),
        [Capability::ReadMemory, Capability::EvaluateGoal].into(),
    )
    .unwrap();
    agent.lifecycle = Lifecycle::Active;
    store.create_owned(&agent, actor).await.unwrap();
    let cycle = store.begin_cycle(agent.id, false).await.unwrap();
    store.finish(agent.id, cycle, None).await.unwrap();
    store
        .transition(
            agent.id,
            Lifecycle::Paused,
            "Temporary mock evaluation only",
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
    let cases: Value = serde_json::from_str(include_str!(
        "../../../config/work-evaluation-fixtures.json"
    ))
    .unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let service = Uuid::new_v4();
        let kind = serde_json::from_value(case["kind"].clone()).unwrap();
        store
            .offer_service(
                agent.id,
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
        store.create_economic_job(actor,CreateJob{request_id:Uuid::new_v4(),service_id:service,network:EconomicNetwork::Simulated,input:json!({"objective":case["objective"],"material":case["material"],"requirements":case["requirements"]}).to_string(),valid_for_seconds:3600}).await.unwrap();
    }
    let provider = Arc::new(support::MockWorkProvider {
        policy,
        calls: AtomicUsize::new(0),
        mode: 0,
    });
    let state = AppState::new(
        store.clone(),
        Runtime::new(store.clone(), Arc::new(Offline)),
        std::env::var("SUPA_TEST_SERVICE_TOKEN").unwrap(),
    )
    .unwrap()
    .with_credentials(vec![OperatorCredential {
        id: actor,
        role: Role::Owner,
        token: std::env::var("SUPA_TEST_OWNER_TOKEN").unwrap(),
    }])
    .unwrap()
    .with_controlled_inference(ControlledInference { store, provider })
    .with_live_work_evaluation();
    let bind: std::net::SocketAddr = std::env::var("SUPA_TEST_BIND").unwrap().parse().unwrap();
    assert!(bind.ip().is_loopback());
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    println!(
        "{}",
        json!({"ready":true,"agent_id":agent.id,"provider_evidence":"mock","real_provider_requests":0})
    );
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let mut signal =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
            signal.recv().await;
        })
        .await
        .unwrap();
}
