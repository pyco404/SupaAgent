//! Opt-in live evaluation. No replayed or manually supplied prose is accepted.
mod live_eval_support;
use async_trait::async_trait;
use live_eval_support::environment::load_local_environment;
use live_eval_support::{EvalResult, Report, at};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
    time::Duration,
};
use supa_domain::*;
use supa_persistence::Store;
use supa_providers::{Context, Invocation, Offline, Reasoner, RoutedOpenAi};
use supa_runtime::{
    Runtime,
    evaluation::{ENVIRONMENT, readiness},
};
use uuid::Uuid;

struct Comparing {
    live: RoutedOpenAi,
    comparisons: Arc<Mutex<Vec<Value>>>,
}
#[async_trait]
impl Reasoner for Comparing {
    fn policy(&self) -> InferencePolicy {
        self.live.policy()
    }
    fn policy_for(&self, p: &str) -> InferencePolicy {
        self.live.policy_for(p)
    }
    async fn social_metered(&self, c: &SocialContext) -> Invocation<SocialDecision> {
        self.live.social_metered(c).await
    }
    async fn plan(&self, c: &Context) -> Result<Plan> {
        self.plan_metered(c).await.result
    }
    async fn draft(&self, c: &Context, e: &[Event]) -> Result<TweetDraft> {
        self.draft_metered(c, e).await.result
    }
    async fn plan_metered(&self, c: &Context) -> Invocation<Plan> {
        let deterministic = Offline.plan(c).await;
        let actual = self.live.plan_metered(c).await;
        let names = |p: &Plan| {
            p.actions
                .iter()
                .map(|a| supa_capabilities::audit_name(&a.tool))
                .collect::<Vec<_>>()
        };
        let comparison = json!({"same_model_context":true,"model":self.live.policy_for("plan").model,"deterministic_actions":deterministic.as_ref().ok().map(names),"live_actions":actual.result.as_ref().ok().map(names),"structured_response_valid":actual.result.is_ok(),"usage_received":actual.usage.is_some(),"failure_code":actual.result.as_ref().err().map(ToString::to_string),"interpretation":"Decision comparison, not a quality or intelligence score"});
        self.comparisons
            .lock()
            .expect("comparison lock")
            .push(comparison);
        actual
    }
    async fn draft_metered(&self, c: &Context, e: &[Event]) -> Invocation<TweetDraft> {
        self.live.draft_metered(c, e).await
    }
}
async fn wait_due<F, Fut>(mut operation: F) -> Result<Uuid>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Uuid>>,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    loop {
        match operation().await {
            Err(Error::Busy) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(1100)).await
            }
            result => return result,
        }
    }
}
/// Social variety must come from recorded work, never injected hypothetical earnings.
fn initial_simulated_funding(social: bool) -> Vec<(LedgerKind, i64)> {
    let mut funding = vec![(LedgerKind::Seed, 1_000_000)];
    if !social {
        funding.push((LedgerKind::Revenue, 12_500));
    }
    funding
}
#[cfg(test)]
mod setup_tests {
    use super::*;
    #[test]
    fn social_setup_never_injects_revenue_or_changes_legacy_funding() {
        let social = initial_simulated_funding(true);
        assert_eq!(social.len(), 1);
        assert!(matches!(social[0], (LedgerKind::Seed, 1_000_000)));
        let legacy = initial_simulated_funding(false);
        assert_eq!(legacy.len(), 2);
        assert!(matches!(legacy[1], (LedgerKind::Revenue, 12_500)));
    }
}
async fn evaluate(runtime: &Runtime, id: Uuid, social: bool) -> EvalResult<()> {
    // Actual planning calls; action choices are made by the live model.
    for _ in 0..3 {
        at(
            "autonomous_cycle",
            wait_due(|| runtime.cycle(id, false)).await,
        )?;
    }
    // Explicit operator-driven draft rounds, clearly separate from autonomous choices.
    for _ in 0..5 {
        if social {
            at(
                "social_cycle",
                wait_due(|| runtime.consider_tweet_cycle(id)).await,
            )?;
            continue;
        }
        let events = at("event_retrieval", runtime.store.events(id).await)?;
        let source = at(
            "event_retrieval",
            events
                .iter()
                .find(|e| e.simulated && verified_fact(e).is_some())
                .ok_or(Error::NotFound),
        )?
        .id;
        at(
            "tweet_cycle",
            wait_due(|| runtime.draft_cycle(id, vec![source])).await,
        )?;
    }
    // Social opportunities may abstain; creative quality is reported separately from protocol completion.
    if social {
        return Ok(());
    }
    let drafts = at(
        "draft_validation",
        runtime.store.history(id, "drafts").await,
    )?;
    let mut unique = BTreeSet::new();
    for row in &drafts {
        let d: TweetDraft = at(
            "draft_validation",
            serde_json::from_value(row["data"].clone()).map_err(|_| Error::Invalid),
        )?;
        at(
            "draft_validation",
            d.validate_facts(&at(
                "draft_validation",
                runtime.store.verified_events(id, &d.event_ids).await,
            )?),
        )?;
        unique.insert(d.text);
    }
    if unique.len() < 5 {
        return at("draft_validation", Err(Error::Invalid));
    }
    Ok(())
}
fn main() {
    let result = at("environment_loading", load_local_environment()).and_then(|_| {
        let runtime = at(
            "runtime_initialization",
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|_| Error::Invalid),
        )?;
        runtime.block_on(run())
    });
    if let Err(e) = result {
        eprintln!("{}", e.json());
        std::process::exit(1);
    }
}
async fn run() -> EvalResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| {
        !matches!(
            a.as_str(),
            "--check" | "--run" | "--authorize-paid" | "--social"
        )
    }) || (args.iter().any(|a| a == "--check") && args.iter().any(|a| a == "--run"))
    {
        return at("input_validation", Err(Error::Invalid));
    }
    let env: BTreeMap<String, String> = ENVIRONMENT
        .iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), v)))
        .collect();
    let social = args.iter().any(|a| a == "--social");
    let mut check = readiness(&args, &env);
    check["evaluation_mode"] = json!(if social { "social" } else { "legacy" });
    println!("{check}");
    if !args.iter().any(|a| a == "--run") {
        return Ok(());
    }
    if check["ready_for_paid"] != true {
        return at("authorization", Err(Error::Denied));
    }
    // No provider requests are sent by construction. All paid calls below use Runtime's durable guard.
    let provider = at("provider_configuration", RoutedOpenAi::from_env())?;
    let url = at(
        "database_configuration",
        env.get("SUPA_EVAL_DATABASE_URL").ok_or(Error::Invalid),
    )?;
    let options = at(
        "database_configuration",
        url.parse::<sqlx::postgres::PgConnectOptions>()
            .map_err(|_| Error::Invalid),
    )?;
    if !matches!(options.get_host(), "127.0.0.1" | "localhost" | "::1") {
        return at("database_configuration", Err(Error::Denied));
    }
    let s = at("database_connection", Store::connect(url).await)?;
    at("database_migration", s.migrate().await)?;
    let budget = at(
        "provider_configuration",
        provider.policy().budget_id.ok_or(Error::Invalid),
    )?;
    let explicit_path = std::env::var_os("SUPA_EVAL_OUTPUT").map(std::path::PathBuf::from);
    let mut report_file = Report::create(
        std::path::Path::new("artifacts/live-eval"),
        budget,
        explicit_path.as_deref(),
    )?;
    let a = at(
        "agent_setup",
        Agent::new(
            "Supa live evaluation".into(),
            Personality::default(),
            CycleConfig {
                scheduler_enabled: false,
                interval_seconds: 1,
                timeout_seconds: 30,
                retry_budget: 0,
                ..Default::default()
            },
            [
                Capability::ReadMemory,
                Capability::Remember,
                Capability::EvaluateGoal,
                Capability::Conserve,
                Capability::DraftTweet,
            ]
            .into(),
        ),
    )?;
    report_file.write(&json!({"status":"initializing","agent_id":a.id,"budget_id":budget,"live_evaluation_success":false}))?;
    at("agent_setup", s.create(&a).await)?;
    for (kind, amount) in initial_simulated_funding(social) {
        at(
            "agent_setup",
            s.credit(a.id, kind, amount, Uuid::new_v4()).await,
        )?;
    }
    at("agent_setup", s.add_goal(a.id,"Review verified simulated expenses, learn from outcomes, and assess progress without inventing achievements".into()).await)?;
    at(
        "agent_setup",
        s.transition(
            a.id,
            Lifecycle::Active,
            "operator authorized bounded live intelligence evaluation",
            true,
        )
        .await,
    )?;
    let comparisons = Arc::new(Mutex::new(vec![]));
    let r = Runtime::new(
        s.clone(),
        Arc::new(Comparing {
            live: provider,
            comparisons: comparisons.clone(),
        }),
    );
    let outcome = evaluate(&r, a.id, social).await;
    // Evaluation agents are excluded from the server scheduler, even after an abrupt crash.
    let stop = s
        .emergency_stop(
            a.id,
            "live evaluation finished or failed; no automatic continuation",
        )
        .await;
    let calls = at("report_collection", s.history(a.id, "inference").await)?;
    let validated = calls
        .iter()
        .filter(|c| c["status"] == "succeeded" && !c["usage"].is_null())
        .count();
    let stop = at("agent_stop", stop);
    let failure = outcome.as_ref().err().or(stop.as_ref().err());
    let social_decisions = at("report_collection", s.history(a.id, "social").await)?;
    let generated_social_drafts = social_decisions
        .iter()
        .filter(|row| row["disposition"] == "draft")
        .count();
    let abstentions = social_decisions
        .iter()
        .filter(|row| row["disposition"] == "abstain")
        .count();
    let minimum_responses = if social { 3 } else { 8 };
    let report = json!({"status":if outcome.is_ok()&&stop.is_ok(){"execution_complete"}else{"failed"},"live_evaluation_success":outcome.is_ok()&&stop.is_ok()&&validated>=minimum_responses&&(!social||generated_social_drafts>0),"evaluation_mode":if social{"social"}else{"legacy"},"social_decisions":social_decisions,"generated_social_drafts":generated_social_drafts,"abstentions":abstentions,"creative_quality":"not scored; requires human review of actual generated text","agent_id":a.id,"budget_id":budget,"validated_live_responses":validated,"failure_code":failure.map(|e| &e.code),"failure_stage":failure.map(|e| e.stage),"comparisons":*comparisons.lock().expect("comparison lock"),"drafts":at("report_collection", s.history(a.id,"drafts").await)?,"calls":calls,"actions":at("report_collection", s.history(a.id,"actions").await)?,"cycles":at("report_collection", s.history(a.id,"cycles").await)?,"simulated_financials":at("report_collection", s.financials(a.id).await)?,"semantic_factual_review":"required; canonical fact/source validation cannot prove absence of unsupported assertions in free-form humor","quality_assessment":"requires human review; not an automatic intelligence score"});
    report_file.write(&report)?;
    println!(
        "{}",
        json!({"report":report_file.path,"validated_live_responses":validated,"execution_complete":outcome.is_ok()&&stop.is_ok()})
    );
    outcome?;
    stop?;
    Ok(())
}
