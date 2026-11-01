//! Disposable-only work evaluation. No paid provider, retrieval, primary epoch or worker.
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use supa_domain::*;
use supa_persistence::Store;
use supa_providers::{Context, Offline, Reasoner};
use supa_runtime::Runtime;
use uuid::Uuid;

fn command(job: &EconomicJob, operation: &str) -> JobCommand {
    JobCommand {
        request_id: Uuid::new_v4(),
        expected_revision: job.revision,
        operation: operation.into(),
        authorize: true,
    }
}
async fn accepted(store: &Store, service: Uuid, owner: Uuid, input: String) -> EconomicJob {
    accepted_by(store, service, owner, owner, input).await
}
async fn accepted_by(
    store: &Store,
    service: Uuid,
    owner: Uuid,
    customer: Uuid,
    input: String,
) -> EconomicJob {
    let mut j = store
        .create_economic_job(
            customer,
            CreateJob {
                request_id: Uuid::new_v4(),
                service_id: service,
                network: EconomicNetwork::Simulated,
                input,
                valid_for_seconds: 3600,
            },
        )
        .await
        .unwrap();
    j = store
        .economic_command(j.id, customer, command(&j, "request_payment"))
        .await
        .unwrap();
    j = store
        .simulated_job_payment(
            j.id,
            customer,
            SimulatedReceipt {
                request_id: Uuid::new_v4(),
                amount_atoms: 5000,
                authorize_simulated: true,
            },
        )
        .await
        .unwrap();
    store
        .economic_command(j.id, owner, command(&j, "accept"))
        .await
        .unwrap()
}
struct Repair {
    calls: AtomicUsize,
    forbidden: bool,
}
#[async_trait::async_trait]
impl Reasoner for Repair {
    async fn plan(&self, c: &Context) -> Result<Plan> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if self.forbidden {
            return Ok(Plan {
                actions: vec![ProposedAction {
                    tool: "transfer".into(),
                    event_ids: vec![],
                    lesson: None,
                    evaluation: None,
                }],
            });
        }
        if call == 0 {
            return Ok(Plan { actions: vec![] });
        }
        Offline.plan(c).await
    }
    async fn draft(&self, _: &Context, _: &[Event]) -> Result<TweetDraft> {
        Err(Error::Denied)
    }
}
#[tokio::test]
#[ignore = "requires a dedicated disposable TEST_DATABASE_URL"]
async fn bounded_work_is_grounded_scoped_durable_and_never_earns_on_abstention() {
    let base = std::env::var("TEST_DATABASE_URL").unwrap();
    let options: sqlx::postgres::PgConnectOptions = base.parse().unwrap();
    assert!(
        matches!(options.get_host(), "localhost" | "127.0.0.1")
            && options.get_database() == Some("supa_m9_test")
    );
    let admin = Store::connect(&base).await.unwrap();
    let db = format!("supa_work_{}_test", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {db} TEMPLATE template0"))
        .execute(admin.pool())
        .await
        .unwrap();
    let url = sqlx::ConnectOptions::to_url_lossy(&options.database(&db)).to_string();
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let owner = Uuid::new_v4();
    let stranger = Uuid::new_v4();
    let mut agent = Agent::new(
        "Disposable bounded work".into(),
        Personality::default(),
        CycleConfig::default(),
        [Capability::ReadMemory, Capability::EvaluateGoal].into(),
    )
    .unwrap();
    agent.lifecycle = Lifecycle::Active;
    store.create_owned(&agent, owner).await.unwrap();
    store
        .economic_capital(agent.id, owner, Uuid::new_v4(), 100000)
        .await
        .unwrap();
    store
        .economic_policy(
            agent.id,
            owner,
            50000,
            1000,
            vec!["inference".into(), "infrastructure".into()],
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
                name: "Evidence synthesis".into(),
                kind: ServiceKind::ResearchReport,
                price_atoms: 5000,
                estimated_inference_atoms: 1000,
                estimated_infrastructure_atoms: 100,
                max_input_bytes: 8000,
                max_attempts: 2,
            },
        )
        .await
        .unwrap();
    // A genuine disposable completed cycle creates an evidence-backed memory.
    let cycle = store.begin_cycle(agent.id, false).await.unwrap();
    store.finish(agent.id, cycle, None).await.unwrap();
    let runtime = Runtime::new(store.clone(), Arc::new(Offline));
    let request = |required: &str, external: bool| {
        json!({"objective":"Review completed cycle storage","material":"storage uses replicated volumes","requirements":[required],"require_external_research":external}).to_string()
    };
    let j = accepted(&store, service, owner, request("absent", false)).await;
    assert!(
        runtime
            .execute_economic_job(j.id, stranger, command(&j, "execute"))
            .await
            .is_err()
    );
    let failed = runtime
        .execute_economic_job(j.id, owner, command(&j, "execute"))
        .await
        .unwrap();
    assert_eq!(failed.state, JobState::Failed);
    assert_eq!(failed.earned_atoms, 0);
    let failed_work = &failed.output.as_ref().unwrap()["work"];
    assert_eq!(failed_work["status"], "incomplete");
    assert!(failed_work["prior_outcomes"].as_array().unwrap().is_empty());
    assert_eq!(
        failed_work["memory"]["strategy"],
        "check_requirements_before_analysis"
    );
    let j = accepted(&store, service, owner, request("storage", false)).await;
    let execution = command(&j, "execute");
    let (a, b) = tokio::join!(
        runtime.execute_economic_job(j.id, owner, execution.clone()),
        runtime.execute_economic_job(j.id, owner, execution.clone())
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let completed = a.or(b).unwrap();
    assert_eq!(completed.state, JobState::Completed);
    assert_eq!(completed.earned_atoms, 5000);
    let output = completed.output.as_ref().unwrap();
    assert_eq!(
        output["work"]["memory"]["strategy"],
        "check_requirements_before_analysis"
    );
    assert_eq!(
        output["work"]["evidence"][0]["quote"],
        "storage uses replicated volumes"
    );
    assert_eq!(output["work"]["prior_outcomes"][0]["state"], "failed");
    assert_eq!(output["work"]["memory"]["verified_references_used"], 1);
    assert_eq!(output["work"]["steps"][0]["tool"], "read_memory");
    assert!(
        !output.to_string().contains("Verified cycle"),
        "private lesson must not leak into customer output"
    );
    assert!(
        runtime
            .execute_economic_job(j.id, owner, execution)
            .await
            .is_err()
    );
    let j = accepted(&store, service, owner, request("storage", true)).await;
    let abstained = runtime
        .execute_economic_job(j.id, owner, command(&j, "execute"))
        .await
        .unwrap();
    assert_eq!(abstained.state, JobState::Failed);
    assert_eq!(abstained.earned_atoms, 0);
    assert_eq!(abstained.output.unwrap()["work"]["status"], "abstained");
    let repair = Arc::new(Repair {
        calls: AtomicUsize::new(0),
        forbidden: false,
    });
    let j = accepted(&store, service, owner, request("storage", false)).await;
    let repaired = Runtime::new(store.clone(), repair.clone())
        .execute_economic_job(j.id, owner, command(&j, "execute"))
        .await
        .unwrap();
    assert_eq!(repaired.state, JobState::Completed);
    assert_eq!(repair.calls.load(Ordering::SeqCst), 2);
    assert_eq!(repaired.output.unwrap()["work"]["revision_count"], 1);
    let forbidden = Arc::new(Repair {
        calls: AtomicUsize::new(0),
        forbidden: true,
    });
    let j = accepted(&store, service, owner, request("storage", false)).await;
    let denied = Runtime::new(store.clone(), forbidden.clone())
        .execute_economic_job(j.id, owner, command(&j, "execute"))
        .await
        .unwrap();
    assert_eq!(denied.state, JobState::Failed);
    assert_eq!(forbidden.calls.load(Ordering::SeqCst), 1);
    // The customer can inspect their result, but cannot retrieve private memory provenance.
    let customer = Uuid::new_v4();
    let j = accepted_by(&store, service, owner, customer, request("storage", true)).await;
    runtime
        .execute_economic_job(j.id, owner, command(&j, "execute"))
        .await
        .unwrap();
    let operator_view = store.economic_job(j.id, owner).await.unwrap();
    assert_eq!(
        operator_view.assessment.unwrap()["private_memory_references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let customer_view = store.economic_job(j.id, customer).await.unwrap();
    assert!(
        customer_view
            .assessment
            .unwrap()
            .get("private_memory_references")
            .is_none()
    );
    let customer_list = store.economic_jobs(customer, Some(agent.id)).await.unwrap();
    assert_eq!(customer_list.len(), 1);
    assert!(
        customer_list[0]
            .assessment
            .as_ref()
            .unwrap()
            .get("private_memory_references")
            .is_none()
    );
    // A successful same-kind task teaches a validated sequence, not a reusable answer.
    let code_service = Uuid::new_v4();
    store
        .offer_service(
            agent.id,
            owner,
            code_service,
            ServiceSpec {
                name: "Static code review".into(),
                kind: ServiceKind::CodeAnalysis,
                price_atoms: 5000,
                estimated_inference_atoms: 1000,
                estimated_infrastructure_atoms: 100,
                max_input_bytes: 8000,
                max_attempts: 2,
            },
        )
        .await
        .unwrap();
    let source =
        json!({"objective":"Analyze unwrap safety","material":"fn main() { data.unwrap(); }"})
            .to_string();
    let j = accepted(&store, code_service, owner, source.clone()).await;
    let first = runtime
        .execute_economic_job(j.id, owner, command(&j, "execute"))
        .await
        .unwrap();
    assert_eq!(
        first.output.as_ref().unwrap()["work"]["memory"]["verified_references_used"],
        0
    );
    let j = accepted(&store, code_service, owner, source).await;
    let second = runtime
        .execute_economic_job(j.id, owner, command(&j, "execute"))
        .await
        .unwrap();
    assert_eq!(
        second.output.as_ref().unwrap()["work"]["learning"]["reused_tool_sequence_from_job"],
        json!(first.id)
    );
    assert_eq!(
        second.output.as_ref().unwrap()["work"]["learning"]["past_answer_reused"],
        false
    );
    let accounts = store.economic_accounts(agent.id, owner).await.unwrap();
    assert_eq!(accounts["networks"][0]["earned_atoms"], 20000);
    assert_eq!(accounts["networks"][0]["liability_atoms"], 20000);
    assert_eq!(accounts["networks"][0]["inference_atoms"], 4000);
    assert_eq!(accounts["networks"][1]["earned_atoms"], 0);
    let restarted = Store::connect(&url).await.unwrap();
    assert_eq!(
        restarted
            .economic_job(completed.id, owner)
            .await
            .unwrap()
            .output_sha256,
        completed.output_sha256
    );
    assert_eq!(
        restarted
            .economic_jobs(owner, Some(agent.id))
            .await
            .unwrap()
            .len(),
        8
    );
    println!(
        "M9: eight durable jobs; four deliveries; incomplete/abstained/forbidden earn zero; one bounded revision; duplicate and cross-user execution denied; simulated revenue 20000, costs 4400, liabilities 20000; paid calls zero"
    );
}
