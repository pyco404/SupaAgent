//! Controlled offline demo. Draft prose is supplied as a run-specific recorded model-output artifact.
//! No real provider call, shell tool, publishing or payment path exists here.
use async_trait::async_trait;
use serde_json::{Value, json};
use std::{path::Path, sync::Arc, time::Duration};
use supa_capabilities::REGISTRY;
use supa_domain::*;
use supa_persistence::Store;
use supa_providers::{Context, Offline, Reasoner};
use supa_runtime::Runtime;
use uuid::Uuid;

struct Replay {
    drafts: Vec<TweetDraft>,
}
#[async_trait]
impl Reasoner for Replay {
    async fn plan(&self, c: &Context) -> Result<Plan> {
        let mut p = Offline.plan(c).await?;
        if let Some(d) = self.drafts.get((c.completed_cycles - 10).max(0) as usize) {
            p.actions.push(ProposedAction {
                tool: "draft_tweet".into(),
                event_ids: d.event_ids.clone(),
                lesson: None,
                evaluation: None,
            });
        }
        if p.actions.len() > c.max_steps {
            return Err(Error::Invalid);
        }
        Ok(p)
    }
    async fn draft(&self, _: &Context, events: &[Event]) -> Result<TweetDraft> {
        self.drafts
            .iter()
            .find(|d| {
                d.event_ids.len() == events.len()
                    && d.event_ids
                        .iter()
                        .all(|id| events.iter().any(|e| e.id == *id))
            })
            .cloned()
            .ok_or(Error::Invalid)
    }
}
fn read_json(path: &str) -> Result<Value> {
    serde_json::from_slice(&std::fs::read(path).map_err(|_| Error::Invalid)?)
        .map_err(|_| Error::Invalid)
}
fn write_json(path: &str, v: &Value) -> Result<()> {
    std::fs::write(
        path,
        serde_json::to_vec_pretty(v).map_err(|_| Error::Invalid)?,
    )
    .map_err(|_| Error::Invalid)
}
async fn snapshot(s: &Store, id: Uuid, phase: &str) -> Result<()> {
    let rows:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(t) FROM (SELECT * FROM activity WHERE agent_id=$1 ORDER BY created_at,id) t").bind(id).fetch_all(s.pool()).await.map_err(|_|Error::Database)?;
    write_json(
        &format!("artifacts/demo/{phase}.json"),
        &json!({"mode":"deterministic_offline_no_live_calls","agent":s.get(id).await?,"financials":s.financials(id).await?,"events":rows,"cycles":s.history(id,"cycles").await?,"actions":s.history(id,"actions").await?,"inference":s.history(id,"inference").await?,"evaluations":s.history(id,"evaluations").await?,"drafts":s.history(id,"drafts").await?,"ledger":s.history(id,"ledger").await?,"transitions":s.history(id,"transitions").await?}),
    )
}
async fn cycles(r: &Runtime, id: Uuid, target: i64) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        let count = r.store.completed_cycle_count(id).await?;
        if count >= target {
            return Ok(());
        }
        if tokio::time::Instant::now() > deadline {
            return Err(Error::Timeout);
        }
        match r.cycle(id, true).await {
            Ok(_) => {}
            Err(Error::Busy) => {}
            Err(e) => return Err(e),
        }
        tokio::time::sleep(Duration::from_millis(1100)).await;
    }
}
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter("supa_runtime=info")
        .with_writer(std::io::stderr)
        .init();
    let stage = std::env::args().nth(1).ok_or(Error::Invalid)?;
    let url = std::env::var("TEST_DATABASE_URL").map_err(|_| Error::Invalid)?;
    let s = Store::connect(&url).await?;
    s.migrate().await?;
    s.recover().await?;
    std::fs::create_dir_all("artifacts/demo")?;
    let id = if stage == "init" {
        if Path::new("artifacts/demo/identity.json").exists() {
            return Err(Error::Invalid.into());
        }
        let a = Agent::new(
            "Supa".into(),
            Personality::default(),
            CycleConfig {
                interval_seconds: 1,
                max_steps: 4,
                ..Default::default()
            },
            REGISTRY.iter().map(|t| t.capability).collect(),
        )?;
        s.create(&a).await?;
        s.credit(a.id, LedgerKind::Seed, 1_000_000, Uuid::new_v4())
            .await?;
        s.credit(a.id, LedgerKind::Revenue, 5000, Uuid::new_v4())
            .await?;
        let goal=s.add_goal(a.id,"Learn from verified execution outcomes while tracking simulated operating expenses".into()).await?;
        s.set_goal_priority(a.id, goal.id, 10).await?;
        s.transition(
            a.id,
            Lifecycle::Active,
            "operator authorizes controlled offline demonstration",
            true,
        )
        .await?;
        write_json("artifacts/demo/identity.json", &json!({"agent_id":a.id}))?;
        a.id
    } else {
        serde_json::from_value(read_json("artifacts/demo/identity.json")?["agent_id"].clone())
            .map_err(|_| Error::Invalid)?
    };
    match stage.as_str() {
        "init" => {
            cycles(&Runtime::new(s.clone(), Arc::new(Offline)), id, 5).await?;
            snapshot(&s, id, "before-restart").await?;
        }
        "continue" => {
            if s.completed_cycle_count(id).await? != 5 {
                return Err(Error::Invalid.into());
            }
            s.credit(id, LedgerKind::Revenue, 7500, Uuid::new_v4())
                .await?;
            cycles(&Runtime::new(s.clone(), Arc::new(Offline)), id, 10).await?;
            snapshot(&s, id, "ten-cycles").await?;
        }
        "crash" => {
            let cycle = s.begin_cycle(id, true).await?;
            s.reserve_inference(id, cycle, "plan", 0, &InferencePolicy::default(), 1000)
                .await?;
            write_json(
                "artifacts/demo/crash-marker.json",
                &json!({"agent_id":id,"cycle_id":cycle}),
            )?;
            // The supervising demonstration script kills this process after seeing the marker.
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
        "recover-test" => {
            let marker = read_json("artifacts/demo/crash-marker.json")?;
            let cycle: Uuid =
                serde_json::from_value(marker["cycle_id"].clone()).map_err(|_| Error::Invalid)?;
            // Explicit test-only clock/lease fault injection avoids waiting 240s. Never a public API.
            sqlx::query("UPDATE cycles SET lease_until=now()-interval '1 second' WHERE id=$1 AND agent_id=$2").bind(cycle).bind(id).execute(s.pool()).await.map_err(|_|Error::Database)?;
            s.recover().await?;
            snapshot(&s, id, "after-crash-recovery").await?;
        }
        "drafts" => {
            let value = read_json("artifacts/demo/generated-drafts.json")?;
            let drafts: Vec<TweetDraft> =
                serde_json::from_value(value["drafts"].clone()).map_err(|_| Error::Invalid)?;
            if drafts.len() != 5 || s.completed_cycle_count(id).await? != 10 {
                return Err(Error::Invalid.into());
            }
            for draft in &drafts {
                draft.validate(&s.verified_events(id, &draft.event_ids).await?)?;
            }
            cycles(
                &Runtime::new(s.clone(), Arc::new(Replay { drafts })),
                id,
                15,
            )
            .await?;
            s.transition(
                id,
                Lifecycle::Paused,
                "controlled demonstration complete; remain paused after restart",
                true,
            )
            .await?;
            snapshot(&s, id, "final").await?;
        }
        "verify-paused" => {
            if s.get(id).await?.lifecycle != Lifecycle::Paused {
                return Err(Error::Invalid.into());
            }
            if !matches!(
                Runtime::new(s.clone(), Arc::new(Offline))
                    .cycle(id, true)
                    .await,
                Err(Error::Inactive)
            ) {
                return Err(Error::Invalid.into());
            }
            snapshot(&s, id, "paused-after-restart").await?;
        }
        _ => return Err(Error::Invalid.into()),
    }
    println!("{stage}: controlled offline demonstration complete for {id}");
    Ok(())
}
