//! Isolated browser-validation fixture, never a selectable production provider.
use async_trait::async_trait;
use std::{sync::Arc, time::Duration};
use supa_api::{AppState, router};
use supa_domain::*;
use supa_persistence::Store;
use supa_providers::{Invocation, Offline, controlled::ControlledProvider};
use supa_runtime::{Runtime, controlled::ControlledInference};
use uuid::Uuid;
struct Fixture {
    policy: InferencePolicy,
}
#[async_trait]
impl ControlledProvider for Fixture {
    fn policy(&self) -> InferencePolicy {
        self.policy.clone()
    }
    fn project(&self) -> &str {
        "proj_browser_fixture"
    }
    async fn verify(&self) -> Result<()> {
        Ok(())
    }
    async fn reason(&self, _: &str, memory: &[Memory]) -> Invocation<LiveProposal> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Invocation{usage:Some(TokenUsage{input_tokens:120,output_tokens:60,cached_input_tokens:0,cache_write_tokens:0}),result:Ok(LiveProposal{summary:"MOCK PROVIDER: inspect the supplied hypothesis before taking further action. This is not a GPT intelligence result.".into(),memory_ids:memory.iter().map(|m|m.id).collect(),actions:vec![ProposedAction{tool:"read_memory".into(),event_ids:vec![],lesson:None,evaluation:None}]})}
    }
}
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() != Some("--authorize-isolated-fixture") {
        return Err("explicit fixture authorization required".into());
    }
    let url = std::env::var("DATABASE_URL")?;
    let parsed = url.parse::<sqlx::postgres::PgConnectOptions>()?;
    if !["127.0.0.1", "localhost", "::1"].contains(&parsed.get_host())
        || !parsed.get_database().is_some_and(|d| d.ends_with("_test"))
        || !url.contains("search_path%3Dcontrolled_browser_")
    {
        return Err("isolated local test schema required".into());
    }
    let store = Store::connect(&url).await?;
    store.migrate().await?;
    let owner: Uuid = std::env::var("SUPA_OPERATOR_ID")?.parse()?;
    let mut agent = Agent::new(
        "MOCK / Controlled inference validation".into(),
        Personality::default(),
        CycleConfig::default(),
        [Capability::ReadMemory].into_iter().collect(),
    )?;
    agent.memory.push(Memory{id:Uuid::new_v4(),lesson:"Fixture hypothesis: review past outcomes before another action. Not a verified achievement.".into(),kind:MemoryKind::Hypothesis,source_event_ids:vec![],cycle_id:Uuid::nil(),created_at:chrono::Utc::now()});
    store.create_owned(&agent, owner).await?;
    let budget = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO live_budgets(id,limit_microusd,committed_microusd) VALUES($1,100000,7948)",
    )
    .bind(budget)
    .execute(store.pool())
    .await?;
    let policy = InferencePolicy {
        model: "gpt-6-luna".into(),
        live: true,
        budget_id: Some(budget),
        budget_limit_microusd: 100000,
        input_rate: 100000,
        cached_input_rate: 10000,
        cache_write_rate: 125000,
        output_rate: 500000,
        input_token_bound: 24192,
        max_output_tokens: 1024,
    };
    let state = AppState::new(
        store.clone(),
        Runtime::new(store.clone(), Arc::new(Offline)),
        std::env::var("SUPA_API_TOKEN")?,
    )?
    .with_operator(owner)?
    .with_controlled_inference(ControlledInference {
        store,
        provider: Arc::new(Fixture { policy }),
    });
    let bind = supa_api::local_address(&std::env::var("SUPA_BIND")?)?;
    axum::serve(tokio::net::TcpListener::bind(bind).await?, router(state)).await?;
    Ok(())
}
