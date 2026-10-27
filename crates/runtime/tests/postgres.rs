//! Explicit opt-in tests against a disposable PostgreSQL database. No real provider calls.
use async_trait::async_trait;
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

async fn database() -> Store {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must point to a disposable PostgreSQL database");
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    store
}
async fn agent(s: &Store, balance: i64) -> Agent {
    let a = Agent::new(
        "Supa test".into(),
        Personality::default(),
        CycleConfig::default(),
        [
            Capability::ReadMemory,
            Capability::Remember,
            Capability::DraftTweet,
        ]
        .into(),
    )
    .unwrap();
    s.create(&a).await.unwrap();
    if balance > 0 {
        s.credit(a.id, LedgerKind::Seed, balance, Uuid::new_v4())
            .await
            .unwrap();
    }
    s.transition(a.id, Lifecycle::Active, "test authorized activation", true)
        .await
        .unwrap();
    a
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn persistence_runtime_memory_goals_and_treasury_survive_reconnect() {
    let s = database().await;
    let a = agent(&s, 10_000).await;
    let goal = s
        .add_goal(a.id, "Learn from the last execution".into())
        .await
        .unwrap();
    let r = Runtime::new(s.clone(), Arc::new(Offline));
    let cycle = r.cycle(a.id, false).await.unwrap();
    let reopened = database().await;
    let saved = reopened.get(a.id).await.unwrap();
    assert_eq!(saved.lifecycle, Lifecycle::Active);
    assert_eq!(saved.goals[0].id, goal.id);
    assert_eq!(saved.memory.len(), 2);
    assert_eq!(saved.memory[0].cycle_id, cycle);
    let f = reopened.financials(a.id).await.unwrap();
    assert!(f.simulated);
    assert_eq!(f.balance_microusd, 8900);
    assert_eq!(f.inference_microusd, 1000);
    assert_eq!(f.operating_microusd, 100);
    assert_eq!(f.runway_days, None);
    s.complete_goal(a.id, goal.id).await.unwrap();
    assert!(s.get(a.id).await.unwrap().goals[0].completed);
    let actions = s.history(a.id, "actions").await.unwrap();
    assert_eq!(actions.len(), 2);
    assert!(
        actions
            .iter()
            .all(|a| a["status"] == "succeeded" && a["authorization_outcome"] == "allowed")
    );
    assert!(
        actions
            .iter()
            .all(|a| a["proposal"].get("lesson").is_none())
    );
    assert!(
        s.events(a.id)
            .await
            .unwrap()
            .iter()
            .any(|e| e.cycle_id == Some(cycle) && e.code == "completed")
    );
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn lifecycle_authorization_and_restart_never_reactivate_inactive_agents() {
    let s = database().await;
    for state in [
        Lifecycle::Paused,
        Lifecycle::Suspended,
        Lifecycle::Terminated,
    ] {
        let a = agent(&s, 10_000).await;
        s.transition(a.id, state, "operator requested stop", true)
            .await
            .unwrap();
        assert!(matches!(
            s.transition(a.id, Lifecycle::Active, "unauthorized", false)
                .await,
            Err(Error::Unauthorized)
        ));
        let reconnected = database().await;
        reconnected.recover().await.unwrap();
        assert_eq!(reconnected.get(a.id).await.unwrap().lifecycle, state);
        assert!(matches!(
            Runtime::new(reconnected, Arc::new(Offline))
                .cycle(a.id, false)
                .await,
            Err(Error::Inactive)
        ));
        assert!(!s.due_agents().await.unwrap().contains(&a.id));
        assert!(
            s.history(a.id, "transitions")
                .await
                .unwrap()
                .iter()
                .any(|t| t["reason"] == "operator requested stop" && t.get("created_at").is_some())
        );
        let result = s
            .transition(a.id, Lifecycle::Active, "authorized restart", true)
            .await;
        assert_eq!(result.is_ok(), state != Lifecycle::Terminated);
    }
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn paid_inference_is_blocked_before_provider_and_agent_is_suspended() {
    let s = database().await;
    let a = agent(&s, 100).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let r = Runtime::new(
        s.clone(),
        Arc::new(Failing {
            calls: calls.clone(),
            mode: 0,
        }),
    );
    assert!(matches!(
        r.cycle(a.id, false).await,
        Err(Error::InsufficientBalance)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(s.financials(a.id).await.unwrap().balance_microusd, 0);
    assert_eq!(s.get(a.id).await.unwrap().lifecycle, Lifecycle::Suspended);
    s.credit(a.id, LedgerKind::Revenue, 20_000, Uuid::new_v4())
        .await
        .unwrap();
    assert_eq!(s.get(a.id).await.unwrap().lifecycle, Lifecycle::Suspended);
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn financial_idempotency_independent_treasuries_and_concurrent_debits() {
    let s = database().await;
    let a = agent(&s, 1500).await;
    let b = agent(&s, 5000).await;
    let key = Uuid::new_v4();
    s.credit(a.id, LedgerKind::Revenue, 500, key).await.unwrap();
    s.credit(a.id, LedgerKind::Revenue, 500, key).await.unwrap();
    assert!(s.credit(a.id, LedgerKind::Revenue, 501, key).await.is_err());
    let cycle = s.begin_cycle(a.id, false).await.unwrap();
    let k1 = Uuid::new_v4();
    let k2 = Uuid::new_v4();
    let (one, two) = tokio::join!(
        s.charge(a.id, cycle, LedgerKind::Inference, 1500, k1),
        s.charge(a.id, cycle, LedgerKind::Inference, 1500, k2)
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    let successful_key = if one.is_ok() { k1 } else { k2 };
    s.charge(a.id, cycle, LedgerKind::Inference, 1500, successful_key)
        .await
        .unwrap();
    assert_eq!(s.financials(a.id).await.unwrap().balance_microusd, 500);
    assert_eq!(s.financials(b.id).await.unwrap().balance_microusd, 5000);
    s.finish(a.id, cycle, None).await.unwrap();
}
struct Failing {
    calls: Arc<AtomicUsize>,
    mode: u8,
}
#[async_trait]
impl Reasoner for Failing {
    async fn plan(&self, _: &Context) -> Result<Plan> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.mode == 1 {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
        Err(Error::Provider)
    }
    async fn draft(&self, _: &Context, _: &[Event]) -> Result<TweetDraft> {
        Err(Error::Provider)
    }
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn provider_failure_and_timeout_leave_durable_history_and_allow_next_cycle() {
    let s = database().await;
    for mode in [0, 1] {
        let mut a = Agent::new(
            "Supa failures".into(),
            Personality::default(),
            CycleConfig {
                timeout_seconds: 1,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap();
        a.capabilities.insert(Capability::ReadMemory);
        s.create(&a).await.unwrap();
        s.credit(a.id, LedgerKind::Seed, 10_000, Uuid::new_v4())
            .await
            .unwrap();
        s.transition(a.id, Lifecycle::Active, "test", true)
            .await
            .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let r = Runtime::new(
            s.clone(),
            Arc::new(Failing {
                calls: calls.clone(),
                mode,
            }),
        );
        let result = r.cycle(a.id, false).await;
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let history = s.history(a.id, "cycles").await.unwrap();
        assert_eq!(history[0]["status"], "failed");
        assert_eq!(
            history[0]["failure_code"],
            if mode == 0 {
                "provider_failed"
            } else {
                "provider_timeout"
            }
        );
        allow_next(&s, a.id).await;
        Runtime::new(s.clone(), Arc::new(Offline))
            .cycle(a.id, false)
            .await
            .unwrap();
    }
}
struct Malicious;
#[async_trait]
impl Reasoner for Malicious {
    async fn plan(&self, _: &Context) -> Result<Plan> {
        Ok(Plan {
            actions: vec![
                ProposedAction {
                    evaluation: None,
                    tool: "shell sk-never-log-this".into(),
                    event_ids: vec![],
                    lesson: Some("sk-never-store-this".into()),
                },
                ProposedAction {
                    evaluation: None,
                    tool: "remember".into(),
                    event_ids: vec![],
                    lesson: Some("I checked the permission denial before continuing.".into()),
                },
            ],
        })
    }
    async fn draft(&self, _: &Context, _: &[Event]) -> Result<TweetDraft> {
        Err(Error::Denied)
    }
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn permission_denial_is_durable_and_does_not_stop_permitted_actions() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    Runtime::new(s.clone(), Arc::new(Malicious))
        .cycle(a.id, false)
        .await
        .unwrap();
    let actions = s.history(a.id, "actions").await.unwrap();
    assert!(actions.iter().any(|a| a["status"] == "denied"
        && a["authorization_outcome"] == "denied"
        && a["proposal"]["tool"] == "unknown"));
    assert!(
        !serde_json::to_string(&actions)
            .unwrap()
            .contains("sk-never")
    );
    assert_eq!(
        s.get(a.id)
            .await
            .unwrap()
            .memory
            .iter()
            .filter(|m| m.kind == MemoryKind::Hypothesis)
            .count(),
        1
    );
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn expired_lease_recovers_without_replaying_costs_or_reactivating_agent() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    let cycle = s.begin_cycle(a.id, false).await.unwrap();
    assert!(matches!(s.begin_cycle(a.id, false).await, Err(Error::Busy)));
    s.charge(a.id, cycle, LedgerKind::Inference, 1000, Uuid::new_v4())
        .await
        .unwrap();
    let ids = s
        .propose(a.id, cycle, &[json!({"tool":"remember"})])
        .await
        .unwrap();
    s.authorize_action(a.id, cycle, ids[0], true).await.unwrap();
    s.transition(a.id, Lifecycle::Paused, "pause during work", true)
        .await
        .unwrap();
    sql_expire(&s, cycle).await;
    let restarted = database().await;
    restarted.recover().await.unwrap();
    restarted.recover().await.unwrap();
    assert_eq!(
        restarted.get(a.id).await.unwrap().lifecycle,
        Lifecycle::Paused
    );
    assert_eq!(
        restarted.history(a.id, "cycles").await.unwrap()[0]["status"],
        "interrupted"
    );
    assert_eq!(
        restarted.history(a.id, "actions").await.unwrap()[0]["status"],
        "failed"
    );
    assert_eq!(
        restarted.financials(a.id).await.unwrap().balance_microusd,
        9000
    );
    assert!(
        restarted
            .execute_action(a.id, cycle, ids[0], Some("should never be written"), None)
            .await
            .is_err()
    );
    assert!(restarted.get(a.id).await.unwrap().memory.is_empty());
}
async fn sql_expire(s: &Store, cycle: Uuid) {
    // Use pool through sqlx re-export-free dev dependency below.
    sqlx::query("UPDATE cycles SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(cycle)
        .execute(s.pool())
        .await
        .unwrap();
}
struct Drafting;
#[async_trait]
impl Reasoner for Drafting {
    async fn plan(&self, c: &Context) -> Result<Plan> {
        Ok(Plan {
            actions: vec![ProposedAction {
                evaluation: None,
                tool: "draft_tweet".into(),
                event_ids: c
                    .verified_events
                    .iter()
                    .filter(|e| e.simulated)
                    .take(1)
                    .map(|e| e.id)
                    .collect(),
                lesson: None,
            }],
        })
    }
    async fn draft(&self, _: &Context, e: &[Event]) -> Result<TweetDraft> {
        Ok(TweetDraft {
            claims: vec![],
            text: format!(
                "My simulated treasury recorded {}. Economic independence remains a work in progress.",
                e[0].code
            ),
            event_ids: e.iter().map(|e| e.id).collect(),
        })
    }
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn event_grounded_draft_is_persisted_and_never_published() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    Runtime::new(s.clone(), Arc::new(Drafting))
        .cycle(a.id, false)
        .await
        .unwrap();
    let drafts = s.history(a.id, "drafts").await.unwrap();
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0]["status"], "draft");
    assert!(
        drafts[0]["data"]["text"]
            .as_str()
            .unwrap()
            .contains("simulated")
    );
    assert_eq!(s.financials(a.id).await.unwrap().balance_microusd, 7900);
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn scheduling_respects_interval_and_lease_fences_old_worker() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    let cycle = s.begin_cycle(a.id, true).await.unwrap();
    s.finish(a.id, cycle, None).await.unwrap();
    assert!(matches!(s.begin_cycle(a.id, true).await, Err(Error::Busy)));
    assert!(matches!(s.begin_cycle(a.id, false).await, Err(Error::Busy)));
    allow_next(&s, a.id).await;
    let old = s.begin_cycle(a.id, false).await.unwrap();
    sql_expire(&s, old).await;
    allow_next(&s, a.id).await;
    let new = s.begin_cycle(a.id, false).await.unwrap();
    assert!(matches!(
        s.charge(a.id, old, LedgerKind::Inference, 1000, Uuid::new_v4())
            .await,
        Err(Error::Stale)
    ));
    s.finish(a.id, new, None).await.unwrap();
}

struct Blocking {
    entered: Arc<tokio::sync::Notify>,
    released: Arc<tokio::sync::Notify>,
}
#[async_trait]
impl Reasoner for Blocking {
    async fn plan(&self, _: &Context) -> Result<Plan> {
        self.entered.notify_one();
        self.released.notified().await;
        Ok(Plan {
            actions: vec![ProposedAction {
                evaluation: None,
                tool: "remember".into(),
                event_ids: vec![],
                lesson: Some("This result must be discarded after a pause.".into()),
            }],
        })
    }
    async fn draft(&self, _: &Context, _: &[Event]) -> Result<TweetDraft> {
        Err(Error::Provider)
    }
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn pause_during_inference_prevents_subsequent_tool_effects() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    let entered = Arc::new(tokio::sync::Notify::new());
    let released = Arc::new(tokio::sync::Notify::new());
    let r = Runtime::new(
        s.clone(),
        Arc::new(Blocking {
            entered: entered.clone(),
            released: released.clone(),
        }),
    );
    let task = tokio::spawn(async move { r.cycle(a.id, false).await });
    entered.notified().await;
    s.transition(a.id, Lifecycle::Paused, "pause in-flight inference", true)
        .await
        .unwrap();
    released.notify_one();
    assert!(matches!(
        task.await.unwrap(),
        Err(Error::Inactive | Error::Stale)
    ));
    assert!(s.get(a.id).await.unwrap().memory.is_empty());
    assert_eq!(s.get(a.id).await.unwrap().lifecycle, Lifecycle::Paused);
    assert_eq!(
        s.history(a.id, "cycles").await.unwrap()[0]["status"],
        "interrupted"
    );
}
struct DraftCounter(Arc<AtomicUsize>);
#[async_trait]
impl Reasoner for DraftCounter {
    async fn plan(&self, c: &Context) -> Result<Plan> {
        Drafting.plan(c).await
    }
    async fn draft(&self, c: &Context, e: &[Event]) -> Result<TweetDraft> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Drafting.draft(c, e).await
    }
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn insufficient_draft_balance_blocks_second_paid_inference() {
    let s = database().await;
    let a = agent(&s, 1500).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let r = Runtime::new(s.clone(), Arc::new(DraftCounter(calls.clone())));
    assert!(matches!(
        r.cycle(a.id, false).await,
        Err(Error::InsufficientBalance)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(s.history(a.id, "drafts").await.unwrap().is_empty());
    assert_eq!(s.financials(a.id).await.unwrap().balance_microusd, 400);
    assert_eq!(s.get(a.id).await.unwrap().lifecycle, Lifecycle::Suspended);
    assert_eq!(
        s.history(a.id, "actions").await.unwrap()[0]["status"],
        "failed"
    );
}

async fn allow_next(s: &Store, id: Uuid) {
    // Fault/setup control only: production callers must wait for the configured interval.
    sqlx::query("UPDATE agents SET next_cycle_at=now() WHERE id=$1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

struct RateLimited {
    calls: Arc<AtomicUsize>,
}
#[async_trait]
impl Reasoner for RateLimited {
    async fn plan(&self, _: &Context) -> Result<Plan> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(Error::ProviderHttp(429));
        }
        Ok(Plan { actions: vec![] })
    }
    async fn draft(&self, _: &Context, _: &[Event]) -> Result<TweetDraft> {
        Err(Error::Denied)
    }
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn retry_budget_records_each_attempt_and_never_retries_uncertain_failures() {
    let s = database().await;
    let a = Agent::new(
        "Retry test".into(),
        Personality::default(),
        CycleConfig {
            retry_budget: 1,
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    s.create(&a).await.unwrap();
    s.credit(a.id, LedgerKind::Seed, 10000, Uuid::new_v4())
        .await
        .unwrap();
    s.transition(a.id, Lifecycle::Active, "test", true)
        .await
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    Runtime::new(
        s.clone(),
        Arc::new(RateLimited {
            calls: calls.clone(),
        }),
    )
    .cycle(a.id, false)
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let history = s.history(a.id, "inference").await.unwrap();
    assert_eq!(history.len(), 2);
    assert!(history.iter().any(|c| c["status"] == "rejected"));
    assert_eq!(s.financials(a.id).await.unwrap().inference_microusd, 2000);
    allow_next(&s, a.id).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let r = Runtime::new(
        s.clone(),
        Arc::new(Failing {
            calls: calls.clone(),
            mode: 0,
        }),
    );
    assert!(r.cycle(a.id, false).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
fn live_policy(budget: Uuid) -> InferencePolicy {
    InferencePolicy {
        model: "mock-only-no-network".into(),
        live: true,
        budget_id: Some(budget),
        budget_limit_microusd: 300,
        input_rate: 100000,
        cached_input_rate: 50000,
        cache_write_rate: 125000,
        output_rate: 100000,
        input_token_bound: 1000,
        max_output_tokens: 1024,
    }
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn durable_live_reservations_survive_crashes_and_enforce_shared_budget() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    let b = agent(&s, 10000).await;
    let p = live_policy(Uuid::new_v4());
    let cycle = s.begin_cycle(a.id, false).await.unwrap();
    let other = s.begin_cycle(b.id, false).await.unwrap();
    let call = s
        .reserve_inference(a.id, cycle, "plan", 0, &p, 1000)
        .await
        .unwrap();
    assert!(matches!(
        s.reserve_inference(b.id, other, "plan", 0, &p, 1000).await,
        Err(Error::LiveBudget)
    ));
    assert_eq!(s.financials(b.id).await.unwrap().balance_microusd, 10000);
    sql_expire(&s, cycle).await;
    let restarted = database().await;
    restarted.recover().await.unwrap();
    assert_eq!(
        restarted.history(a.id, "inference").await.unwrap()[0]["status"],
        "unknown"
    );
    assert!(matches!(
        restarted
            .reserve_inference(b.id, other, "plan", 0, &p, 1000)
            .await,
        Err(Error::LiveBudget)
    ));
    assert!(
        restarted
            .finish_inference(a.id, call, None, None)
            .await
            .is_err()
    );
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn token_settlement_releases_only_confirmed_unused_reservation() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    let p = live_policy(Uuid::new_v4());
    let cycle = s.begin_cycle(a.id, false).await.unwrap();
    let call = s
        .reserve_inference(a.id, cycle, "plan", 0, &p, 1000)
        .await
        .unwrap();
    s.finish_inference(
        a.id,
        call,
        Some(TokenUsage {
            cache_write_tokens: 0,
            input_tokens: 10,
            cached_input_tokens: 0,
            output_tokens: 10,
        }),
        Some(&Error::Provider),
    )
    .await
    .unwrap();
    let history = s.history(a.id, "inference").await.unwrap();
    assert_eq!(history[0]["accounted_microusd"], 2);
    assert_eq!(history[0]["usage"]["output_tokens"], 10);
    assert_eq!(history[0]["status"], "failed");
    s.reserve_inference(a.id, cycle, "plan", 1, &p, 1000)
        .await
        .unwrap();
    assert!(s.finish_inference(a.id, call, None, None).await.is_err());
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn resource_limits_block_calls_before_charging_or_contacting_provider() {
    let s = database().await;
    let a = Agent::new(
        "Budgets".into(),
        Personality::default(),
        CycleConfig {
            max_provider_calls: 1,
            max_cycle_tokens: 1000,
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    s.create(&a).await.unwrap();
    s.credit(a.id, LedgerKind::Seed, 10000, Uuid::new_v4())
        .await
        .unwrap();
    s.transition(a.id, Lifecycle::Active, "test", true)
        .await
        .unwrap();
    let cycle = s.begin_cycle(a.id, false).await.unwrap();
    let p = live_policy(Uuid::new_v4());
    assert!(matches!(
        s.reserve_inference(a.id, cycle, "plan", 0, &p, 1000).await,
        Err(Error::ResourceBudget)
    ));
    assert_eq!(s.financials(a.id).await.unwrap().balance_microusd, 10000);
    let p = InferencePolicy::default();
    s.reserve_inference(a.id, cycle, "plan", 0, &p, 1000)
        .await
        .unwrap();
    assert!(matches!(
        s.reserve_inference(a.id, cycle, "draft", 0, &p, 1000).await,
        Err(Error::ResourceBudget)
    ));
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn evaluations_keep_model_assessments_separate_from_goal_completion_and_facts() {
    let s = database().await;
    // Create a separate agent with the new explicit capability set.
    let a = Agent::new(
        "Learning".into(),
        Personality::default(),
        CycleConfig::default(),
        [Capability::Remember, Capability::EvaluateGoal].into(),
    )
    .unwrap();
    s.create(&a).await.unwrap();
    s.credit(a.id, LedgerKind::Seed, 100000, Uuid::new_v4())
        .await
        .unwrap();
    s.transition(a.id, Lifecycle::Active, "test", true)
        .await
        .unwrap();
    let low = s
        .add_goal(a.id, "Review operating expenses".into())
        .await
        .unwrap();
    let high = s
        .add_goal(a.id, "Evaluate execution evidence".into())
        .await
        .unwrap();
    s.set_goal_priority(a.id, high.id, 10).await.unwrap();
    let r = Runtime::new(s.clone(), Arc::new(Offline));
    r.cycle(a.id, false).await.unwrap();
    allow_next(&s, a.id).await;
    r.cycle(a.id, false).await.unwrap();
    let saved = s.get(a.id).await.unwrap();
    assert!(saved.goals.iter().all(|g| !g.completed));
    assert_eq!(
        saved
            .goals
            .iter()
            .find(|g| g.id == low.id)
            .unwrap()
            .suggested_progress_bps,
        0
    );
    assert!(
        saved
            .goals
            .iter()
            .find(|g| g.id == high.id)
            .unwrap()
            .suggested_progress_bps
            > 0
    );
    assert!(
        saved
            .memory
            .iter()
            .any(|m| m.kind == MemoryKind::Hypothesis)
    );
    assert!(
        saved
            .memory
            .iter()
            .any(|m| m.kind == MemoryKind::VerifiedObservation && !m.source_event_ids.is_empty())
    );
    assert_eq!(
        s.history(a.id, "evaluations").await.unwrap()[0]["verification"],
        "model_assessment"
    );
    let foreign = agent(&s, 10000).await;
    let evidence = s.events(foreign.id).await.unwrap()[0].id;
    assert!(s.verified_events(a.id, &[evidence]).await.is_err());
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn emergency_stop_fences_worker_even_after_explicit_resume() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    let old = s.begin_cycle(a.id, false).await.unwrap();
    let p = InferencePolicy::default();
    s.reserve_inference(a.id, old, "plan", 0, &p, 1000)
        .await
        .unwrap();
    s.emergency_stop(a.id, "operator emergency stop")
        .await
        .unwrap();
    s.emergency_stop(a.id, "idempotent repeated stop")
        .await
        .unwrap();
    assert_eq!(s.get(a.id).await.unwrap().lifecycle, Lifecycle::Suspended);
    s.transition(a.id, Lifecycle::Active, "operator reauthorizes", true)
        .await
        .unwrap();
    assert!(matches!(
        s.charge(a.id, old, LedgerKind::Operating, 100, Uuid::new_v4())
            .await,
        Err(Error::Stale)
    ));
    assert_eq!(
        s.history(a.id, "cycles").await.unwrap()[0]["status"],
        "interrupted"
    );
    assert_eq!(
        s.history(a.id, "inference").await.unwrap()[0]["status"],
        "unknown"
    );
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn conservation_can_only_decrease_frequency_and_is_audited() {
    let s = database().await;
    let a = Agent::new(
        "Conserving Supa".into(),
        Personality::default(),
        CycleConfig::default(),
        [Capability::Conserve].into(),
    )
    .unwrap();
    s.create(&a).await.unwrap();
    s.credit(a.id, LedgerKind::Seed, 5000, Uuid::new_v4())
        .await
        .unwrap();
    s.transition(a.id, Lifecycle::Active, "test", true)
        .await
        .unwrap();
    Runtime::new(s.clone(), Arc::new(Offline))
        .cycle(a.id, false)
        .await
        .unwrap();
    assert!(matches!(s.begin_cycle(a.id, false).await, Err(Error::Busy)));
    assert!(
        s.events(a.id)
            .await
            .unwrap()
            .iter()
            .any(|e| e.code == "cycle_deferred")
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn cache_write_settlement_uses_durable_ceiling_and_never_double_counts_tokens() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    let p = live_policy(Uuid::new_v4());
    let cycle = s.begin_cycle(a.id, false).await.unwrap();
    let call = s
        .reserve_inference(a.id, cycle, "plan", 0, &p, 1000)
        .await
        .unwrap();
    let u = TokenUsage {
        input_tokens: 100,
        cached_input_tokens: 20,
        cache_write_tokens: 40,
        output_tokens: 10,
    };
    s.finish_inference(a.id, call, Some(u), None).await.unwrap();
    let row = &s.history(a.id, "inference").await.unwrap()[0];
    assert_eq!(row["accounted_microusd"], 11);
    assert_eq!(row["usage"]["cache_write_tokens"], 40);
    let reserved = row["reserved_microusd"].as_i64().unwrap();
    assert!(reserved >= 11);
    s.reserve_inference(a.id, cycle, "draft", 0, &p, 1000)
        .await
        .unwrap();
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn explicitly_requested_drafts_still_require_permission_and_resource_budgets() {
    let s = database().await;
    let a = Agent::new(
        "Draft permission check".into(),
        Personality::default(),
        CycleConfig {
            scheduler_enabled: false,
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    s.create(&a).await.unwrap();
    s.credit(a.id, LedgerKind::Seed, 10000, Uuid::new_v4())
        .await
        .unwrap();
    s.transition(a.id, Lifecycle::Active, "test", true)
        .await
        .unwrap();
    let event = s.events(a.id).await.unwrap()[0].id;
    assert!(!s.due_agents().await.unwrap().contains(&a.id));
    Runtime::new(s.clone(), Arc::new(Offline))
        .draft_cycle(a.id, vec![event])
        .await
        .unwrap();
    assert!(s.history(a.id, "inference").await.unwrap().is_empty());
    assert!(s.history(a.id, "drafts").await.unwrap().is_empty());
    assert_eq!(
        s.history(a.id, "actions").await.unwrap()[0]["authorization_outcome"],
        "denied"
    );
}

// Deterministic social fixtures test policy, not model creativity. No real provider is contacted.
struct SocialFixture {
    calls: Arc<AtomicUsize>,
    abstain: bool,
    foreign_event: Option<Uuid>,
}
#[async_trait]
impl Reasoner for SocialFixture {
    async fn plan(&self, _: &Context) -> Result<Plan> {
        Ok(Plan {
            actions: vec![ProposedAction {
                tool: "consider_tweet".into(),
                event_ids: vec![],
                lesson: None,
                evaluation: None,
            }],
        })
    }
    async fn draft(&self, _: &Context, _: &[Event]) -> Result<TweetDraft> {
        Err(Error::Provider)
    }
    async fn social_metered(
        &self,
        c: &SocialContext,
    ) -> supa_providers::Invocation<SocialDecision> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let decision = if self.abstain {
            SocialDecision::abstain(SocialReason::NothingNew)
        } else {
            let candidate = &c.candidates[0];
            let id = self.foreign_event.unwrap_or(candidate.event.id);
            SocialDecision {
                choice: SocialChoice::Draft,
                reason: SocialReason::WorthSharing,
                style: Some(PostStyle::Reflection),
                text: format!(
                    "My ambitions have a spreadsheet: {}.",
                    candidate.public_fact
                ),
                event_ids: vec![id],
                memory_ids: vec![],
                claims: vec![SocialClaim {
                    event_id: id,
                    fact: candidate.canonical_fact.clone(),
                    wording: candidate.public_fact.clone(),
                }],
            }
        };
        supa_providers::Invocation {
            result: Ok(decision),
            usage: None,
        }
    }
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn social_persists_clean_public_text_separate_review_and_rejects_repeated_openings() {
    let s = database().await;
    let a = agent(&s, 100000).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let r = Runtime::new(
        s.clone(),
        Arc::new(SocialFixture {
            calls: calls.clone(),
            abstain: false,
            foreign_event: None,
        }),
    );
    r.cycle(a.id, false).await.unwrap();
    let rows = s.history(a.id, "social").await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["disposition"], "draft");
    assert_eq!(rows[0]["review_status"], "requires_human_review");
    assert_eq!(rows[0]["publishing_enabled"], false);
    assert!(!rows[0]["data"]["text"].as_str().unwrap().contains("review"));
    allow_next(&s, a.id).await;
    let reopened = database().await;
    let again = Runtime::new(
        reopened.clone(),
        Arc::new(SocialFixture {
            calls: calls.clone(),
            abstain: false,
            foreign_event: None,
        }),
    );
    assert!(matches!(
        again.consider_tweet_cycle(a.id).await,
        Err(Error::SocialRepetition)
    ));
    let rows = reopened.history(a.id, "social").await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["disposition"], "rejected");
    assert!(rows[0]["data"].get("text").is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(reopened.social_history(a.id).await.unwrap().len(), 1);
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn social_abstention_is_durable_and_does_not_create_a_public_draft() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    let calls = Arc::new(AtomicUsize::new(0));
    Runtime::new(
        s.clone(),
        Arc::new(SocialFixture {
            calls: calls.clone(),
            abstain: true,
            foreign_event: None,
        }),
    )
    .consider_tweet_cycle(a.id)
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        s.history(a.id, "social").await.unwrap()[0]["disposition"],
        "abstain"
    );
    assert!(s.social_history(a.id).await.unwrap().is_empty());
    assert!(s.history(a.id, "drafts").await.unwrap().is_empty());
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn social_foreign_evidence_and_missing_permission_are_rejected() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    let foreign = agent(&s, 10000).await;
    let event = s.events(foreign.id).await.unwrap()[0].id;
    let calls = Arc::new(AtomicUsize::new(0));
    assert!(matches!(
        Runtime::new(
            s.clone(),
            Arc::new(SocialFixture {
                calls: calls.clone(),
                abstain: false,
                foreign_event: Some(event)
            })
        )
        .consider_tweet_cycle(a.id)
        .await,
        Err(Error::Invalid)
    ));
    assert_eq!(
        s.history(a.id, "social").await.unwrap()[0]["disposition"],
        "rejected"
    );
    let denied = Agent::new(
        "No social permission".into(),
        Personality::default(),
        CycleConfig::default(),
        Default::default(),
    )
    .unwrap();
    s.create(&denied).await.unwrap();
    s.credit(denied.id, LedgerKind::Seed, 10000, Uuid::new_v4())
        .await
        .unwrap();
    s.transition(denied.id, Lifecycle::Active, "test", true)
        .await
        .unwrap();
    let before = calls.load(Ordering::SeqCst);
    Runtime::new(
        s.clone(),
        Arc::new(SocialFixture {
            calls: calls.clone(),
            abstain: false,
            foreign_event: None,
        }),
    )
    .consider_tweet_cycle(denied.id)
    .await
    .unwrap();
    assert_eq!(before, calls.load(Ordering::SeqCst));
    assert!(s.history(denied.id, "social").await.unwrap().is_empty());
    assert_eq!(
        s.history(denied.id, "actions").await.unwrap()[0]["authorization_outcome"],
        "denied"
    );
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn social_empty_candidate_set_skips_provider_and_preserves_legacy_history() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    // A legacy draft must cool its topic in the new path, including after reconnect.
    Runtime::new(s.clone(), Arc::new(Drafting))
        .cycle(a.id, false)
        .await
        .unwrap();
    let history = s.social_history(a.id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert!(!history[0].topics.is_empty());
    let empty = Agent::new(
        "Quiet Supa".into(),
        Personality::default(),
        CycleConfig {
            operating_cost_microusd: 0,
            ..Default::default()
        },
        [Capability::DraftTweet].into(),
    )
    .unwrap();
    s.create(&empty).await.unwrap();
    s.transition(empty.id, Lifecycle::Active, "test", true)
        .await
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    Runtime::new(
        s.clone(),
        Arc::new(SocialFixture {
            calls: calls.clone(),
            abstain: false,
            foreign_event: None,
        }),
    )
    .consider_tweet_cycle(empty.id)
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(s.history(empty.id, "inference").await.unwrap().is_empty());
    assert_eq!(
        s.history(empty.id, "social").await.unwrap()[0]["disposition"],
        "abstain"
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn social_insufficient_treasury_blocks_generation_before_provider() {
    let s = database().await;
    let a = agent(&s, 100).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let result = Runtime::new(
        s.clone(),
        Arc::new(SocialFixture {
            calls: calls.clone(),
            abstain: false,
            foreign_event: None,
        }),
    )
    .consider_tweet_cycle(a.id)
    .await;
    assert!(matches!(result, Err(Error::InsufficientBalance)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(s.history(a.id, "inference").await.unwrap().is_empty());
    assert_eq!(s.get(a.id).await.unwrap().lifecycle, Lifecycle::Suspended);
}
struct BlockingSocial {
    entered: Arc<tokio::sync::Notify>,
    released: Arc<tokio::sync::Notify>,
}
#[async_trait]
impl Reasoner for BlockingSocial {
    async fn plan(&self, _: &Context) -> Result<Plan> {
        Ok(Plan { actions: vec![] })
    }
    async fn draft(&self, _: &Context, _: &[Event]) -> Result<TweetDraft> {
        Err(Error::Provider)
    }
    async fn social_metered(
        &self,
        c: &SocialContext,
    ) -> supa_providers::Invocation<SocialDecision> {
        self.entered.notify_one();
        self.released.notified().await;
        SocialFixture {
            calls: Arc::new(AtomicUsize::new(0)),
            abstain: false,
            foreign_event: None,
        }
        .social_metered(c)
        .await
    }
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn social_pause_during_generation_fences_draft_persistence() {
    let s = database().await;
    let a = agent(&s, 10000).await;
    let entered = Arc::new(tokio::sync::Notify::new());
    let released = Arc::new(tokio::sync::Notify::new());
    let r = Runtime::new(
        s.clone(),
        Arc::new(BlockingSocial {
            entered: entered.clone(),
            released: released.clone(),
        }),
    );
    let worker = tokio::spawn(async move { r.consider_tweet_cycle(a.id).await });
    entered.notified().await;
    s.transition(
        a.id,
        Lifecycle::Paused,
        "operator pauses social generation",
        true,
    )
    .await
    .unwrap();
    released.notify_one();
    assert!(matches!(
        worker.await.unwrap(),
        Err(Error::Inactive | Error::Stale)
    ));
    assert!(s.social_history(a.id).await.unwrap().is_empty());
    assert_eq!(s.get(a.id).await.unwrap().lifecycle, Lifecycle::Paused);
}
