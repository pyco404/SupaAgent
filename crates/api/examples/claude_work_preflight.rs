//! Disposable preparation only. No provider, API listener, credential loader or budget activation.
use serde_json::{Value, json};
use supa_domain::*;
use supa_persistence::Store;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    if run().await.is_err() {
        eprintln!("disposable_work_preflight_failed_no_private_details");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err(Error::Invalid);
    }
    if std::path::Path::new("postgres-data/infrastructure-v1/data/postmaster.pid").exists() {
        return Err(Error::Denied);
    }
    let plan: Value = serde_json::from_slice(&std::fs::read(&args[1]).map_err(|_| Error::Invalid)?)
        .map_err(|_| Error::Invalid)?;
    let p = &plan["preflight"];
    let db = &p["database"];
    let name = db["name"].as_str().ok_or(Error::Invalid)?;
    let directory = db["data_directory"].as_str().ok_or(Error::Invalid)?;
    let socket = db["socket_directory"].as_str().ok_or(Error::Invalid)?;
    if plan["provider"] != "anthropic"
        || !name.starts_with("supa_claude_")
        || !name.ends_with("_test")
        || !directory.starts_with("/tmp/supa-claude-pg-")
        || !directory.ends_with("/data")
        || socket
            != directory
                .strip_suffix("data")
                .ok_or(Error::Invalid)?
                .to_owned()
                + "s"
        || db["port"] != 55683
        || plan["budget_limit_microusd"] != 5000
        || plan["max_cost_microusd"] != 4000
        || plan["max_requests"] != 1
        || plan["model"] != "claude-haiku-5-5"
        || plan["use_memory"] != false
    {
        return Err(Error::Denied);
    }
    // SQLx's reviewed socket form keeps a nonempty URL authority; socket host lives in the query.
    let url = format!("postgresql://localhost/{name}?host={socket}&port=55683");
    let store = Store::connect(&url).await?;
    let actual: (String,String,String) = sqlx::query_as("SELECT current_database(),current_setting('data_directory'),system_identifier::text FROM pg_control_system()")
        .fetch_one(store.pool()).await.map_err(|_| Error::Invalid)?;
    if actual.0 != name || actual.1 != directory || db["system_identifier"] != actual.2 {
        return Err(Error::Denied);
    }
    let tables: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pg_tables WHERE schemaname='public'")
            .fetch_one(store.pool())
            .await
            .map_err(|_| Error::Invalid)?;
    if tables != 0 {
        return Err(Error::Denied);
    }
    store.migrate().await?;
    let id = |v: &Value| -> Result<Uuid> {
        v.as_str()
            .ok_or(Error::Invalid)?
            .parse()
            .map_err(|_| Error::Invalid)
    };
    let actor = id(&p["test_owner_id"])?;
    let mut agent = Agent::new(
        "Claude supplied-evidence evaluation".into(),
        Personality::default(),
        CycleConfig::default(),
        [Capability::ReadMemory, Capability::EvaluateGoal].into(),
    )?;
    agent.id = id(&plan["agent_id"])?;
    agent.lifecycle = Lifecycle::Paused;
    store.create_owned(&agent, actor).await?;
    let service = id(&p["service_id"])?;
    store
        .offer_service(
            agent.id,
            actor,
            service,
            ServiceSpec {
                name: "Claude unpaid synthesis evaluation".into(),
                kind: ServiceKind::ResearchReport,
                price_atoms: 5000,
                estimated_inference_atoms: 1000,
                estimated_infrastructure_atoms: 100,
                max_input_bytes: 8000,
                max_attempts: 1,
            },
        )
        .await?;
    let input = p["task_input"].as_str().ok_or(Error::Invalid)?.to_string();
    let parsed = WorkRequest::parse(&input, ServiceKind::ResearchReport)?;
    validate_work_source(&parsed.material)?;
    let job = store
        .create_economic_job(
            actor,
            CreateJob {
                request_id: id(&plan["job_id"])?,
                service_id: service,
                network: EconomicNetwork::Simulated,
                input,
                valid_for_seconds: 86400,
            },
        )
        .await?;
    let hash = store.work_input_sha256(job.id, actor).await?;
    if plan["input_sha256"] != hash || job.state != JobState::Created || job.earned_atoms != 0 {
        return Err(Error::Denied);
    }
    let budget = id(&plan["budget_id"])?;
    if [
        "ad91e475-f1da-495a-8247-139643d2d8b1",
        "e6076529-e302-403e-b846-1ff902c1f641",
    ]
    .iter()
    .any(|old| budget.to_string() == *old)
    {
        return Err(Error::Denied);
    }
    sqlx::query("INSERT INTO live_budgets(id,limit_microusd) VALUES($1,5000)")
        .bind(budget)
        .execute(store.pool())
        .await
        .map_err(|_| Error::Database)?;
    // Fresh independent test budget only: no spending consent or request reservation.
    println!(
        "{}",
        json!({"prepared":true,"agent_id":agent.id,"lifecycle":"paused","job_id":job.id,"input_sha256":hash,"expires_at":job.expires_at,"test_owner_id":actor,"budget_installed":true,"credentials_installed":false,"live_requests":0,"earned_revenue":0})
    );
    Ok(())
}
