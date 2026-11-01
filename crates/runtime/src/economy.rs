//! Bounded, supplied-input services. No shell, HTTP tool, signing, paid inference or scheduler.
use super::*;
use serde_json::{Value, json};
use supa_providers::solana::{RpcMethod, SolanaRpc, verify_usdc_mint};

impl Runtime {
    pub async fn execute_economic_job(
        &self,
        id: Uuid,
        actor: Uuid,
        command: JobCommand,
    ) -> Result<EconomicJob> {
        let j = self.store.economic_job(id, actor).await?;
        self.store
            .require_access(j.agent_id, actor, supa_persistence::Role::Operator)
            .await?;
        let policy = self.reasoner.policy();
        policy.validate()?;
        // This milestone never reserves or invokes a paid provider, including on retries.
        if policy.live || policy.input_rate != 0 || policy.output_rate != 0 {
            return Err(Error::FinancialPolicy);
        }
        let (a, mut context) = self.context(j.agent_id).await?;
        j.spec.input(&j.input)?;
        let request = WorkRequest::parse(&j.input, j.spec.kind)?;
        // Use only same-customer outcomes; never disclose another customer's work.
        let prior = self.store.economic_jobs(actor, Some(j.agent_id)).await?;
        let outcomes: Vec<_> = prior
            .iter()
            .filter(|old| {
                old.id != j.id
                    && old.customer_id == j.customer_id
                    && old.spec.kind == j.spec.kind
                    && matches!(old.state, JobState::Completed | JobState::Failed)
            })
            .take(8)
            .collect();
        let failures = outcomes
            .iter()
            .filter(|old| old.state == JobState::Failed)
            .count();
        let mut verified = vec![];
        let memories = if a.capabilities.contains(&Capability::ReadMemory) {
            relevant_memories(&a, &request.objective, 4)
        } else {
            vec![]
        };
        for memory in &memories {
            let relevant = request
                .objective
                .split_whitespace()
                .filter(|term| term.len() > 3)
                .any(|term| memory.lesson.to_lowercase().contains(&term.to_lowercase()));
            if relevant
                && memory.kind == MemoryKind::VerifiedObservation
                && !memory.source_event_ids.is_empty()
                && self
                    .store
                    .verified_events(a.id, &memory.source_event_ids)
                    .await?
                    .len()
                    == memory.source_event_ids.len()
            {
                verified.push(memory.clone());
            }
        }
        let cautious = failures > 0 || !verified.is_empty();
        context.goals = vec![Goal {
            id: j.id,
            description: request.objective.clone(),
            completed: false,
            priority: 0,
            suggested_progress_bps: 0,
        }];
        context.memory = verified.clone();
        context.verified_events.clear();
        context.permitted_tools = request
            .tools(j.spec.kind, cautious)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut reused_plan = None;
        // A previous delivery is evidence for a tool sequence, never for this task's answer.
        if !cautious {
            for old in outcomes
                .iter()
                .filter(|old| old.state == JobState::Completed)
            {
                let Some(steps) = old
                    .output
                    .as_ref()
                    .and_then(|v| v["work"]["steps"].as_array())
                else {
                    continue;
                };
                let actions: Vec<_> = steps
                    .iter()
                    .filter_map(|step| step["tool"].as_str())
                    .map(|tool| ProposedAction {
                        tool: tool.into(),
                        event_ids: vec![],
                        lesson: None,
                        evaluation: None,
                    })
                    .collect();
                if validate_work_plan(
                    &Plan {
                        actions: actions.clone(),
                    },
                    &context.permitted_tools,
                )
                .is_ok()
                {
                    context.permitted_tools =
                        actions.into_iter().map(|action| action.tool).collect();
                    reused_plan = Some(old.id);
                    break;
                }
            }
        }
        if !verified.is_empty() {
            context.permitted_tools.insert(0, "read_memory".into());
        }
        if context.permitted_tools.len() > a.config.max_steps as usize {
            return Err(Error::ResourceBudget);
        }
        context.max_steps = context.permitted_tools.len();
        let max_requests = a.config.max_provider_calls.min(2);
        let accounts = self.store.economic_accounts(a.id, actor).await?;
        let claimed = self.store.claim_economic_job(id,actor,command.clone(),json!({"provider_model":policy.model,"live":false,"contract":j.spec.output_contract(),"estimated_simulated_cost_atoms":j.spec.cost(j.network),"max_provider_requests":max_requests,"max_tool_steps":context.max_steps,"private_memory_references":verified.iter().map(|m|json!({"memory_id":m.id,"event_ids":m.source_event_ids})).collect::<Vec<_>>(),"reliability":"Supplied-input analysis only; no external truth or completion probability claimed."})).await?;
        let mut attempts = 0;
        let mut trace = vec![];
        let outcome = tokio::time::timeout(Duration::from_secs(10), async {
            let plan = loop {
                attempts += 1;
                let invocation = self.reasoner.plan_metered(&context).await;
                // A zero-price policy may not smuggle metered/ambiguous charges into simulation.
                if invocation.usage.is_some() { return Err(Error::FinancialPolicy); }
                let plan = invocation.result?;
                match validate_work_plan(&plan, &context.permitted_tools) {
                    Ok(()) => break plan,
                    Err(Error::Invalid) if attempts < max_requests => {
                        context.goals[0].description = format!("Revise the incomplete tool plan: select each permitted tool exactly once. {}", request.objective);
                    }
                    Err(error) => return Err(error),
                }
            };
            let mut work = None;
            for action in &plan.actions {
                self.store.require_access(claimed.agent_id, actor, supa_persistence::Role::Operator).await?;
                let current = self.store.get(claimed.agent_id).await?;
                if current.lifecycle != Lifecycle::Active || !current.capabilities.contains(&Capability::EvaluateGoal) {
                    return Err(Error::Denied);
                }
                if action.tool == "read_memory" { authorize(&current, action)?; }
                if action.tool == "inspect_input" { WorkRequest::parse(&claimed.input, claimed.spec.kind)?; }
                if action.tool == "check_requirements" { analyze_work(&request, claimed.spec.kind)?; }
                // Every tool is application-owned, read-only and bound to this supplied material.
                if !matches!(action.tool.as_str(), "inspect_input" | "read_memory" | "check_requirements") {
                    work = Some(analyze_work(&request, claimed.spec.kind)?);
                }
                trace.push(json!({"tool":action.tool,"state":"completed","external_mutations":0}));
            }
            let mut work = work.ok_or(Error::Invalid)?;
            work["steps"] = json!(trace);
            work["provider_requests"] = json!(attempts);
            work["revision_count"] = json!(attempts - 1);
            work["memory"] = json!({"verified_references_used":verified.len(),"hypotheses_excluded":memories.iter().filter(|m|m.kind==MemoryKind::Hypothesis).count(),"unsupported_verified_excluded":memories.iter().filter(|m|m.kind==MemoryKind::VerifiedObservation).count()-verified.len(),"contents":"private; not included in customer reports","strategy":if cautious {"check_requirements_before_analysis"} else {"analyze_then_check_requirements"}});
            work["learning"] = json!({"reused_tool_sequence_from_job":reused_plan,"past_answer_reused":false,"prior_successful_tasks":outcomes.len()-failures,"prior_failed_tasks":failures,"policy":"Prior failures or relevant verified observations require preflight checks; otherwise a validated successful tool sequence may be reused."});
            work["prior_outcomes"] = json!(outcomes.iter().map(|old|json!({"job_id":old.id,"state":old.state})).collect::<Vec<_>>());
            work["economics"] = economic_recommendation(&claimed, &accounts, failures, &work);
            let mut output = report(&claimed, &policy.model)?;
            output["work"] = work;
            output["provider_proposed_tools"] = json!(plan.actions.iter().map(|a|a.tool.as_str()).collect::<Vec<_>>());
            if output.to_string().len() > 16000 { return Err(Error::Invalid); }
            Ok(output)
        }).await.unwrap_or(Err(Error::Timeout));
        match outcome {
            Ok(output) => {
                self.store
                    .finish_economic_job(
                        id,
                        actor,
                        command.request_id,
                        Some(output.clone()),
                        if work_delivered(&output)? {
                            None
                        } else {
                            Some("incomplete_or_unsupported_work")
                        },
                    )
                    .await
            }
            Err(error) => {
                let failure = match error {
                    Error::Timeout => "bounded_work_timeout",
                    Error::Denied => "bounded_work_permission_denied",
                    Error::FinancialPolicy => "bounded_work_metering_rejected",
                    _ => "bounded_provider_or_contract_failure",
                };
                let output = json!({"contract":"structured_report_v1","job_id":id,"live_inference":false,
                    "work":{"contract":"bounded_work_v1","status":"abstained","objective":request.objective,
                        "steps":trace,"evidence":[],"provider_requests":attempts,"revision_count":attempts.saturating_sub(1),
                        "abstention_reason":failure,"limitations":["No delivered result; no earned revenue."]}});
                self.store
                    .finish_economic_job(id, actor, command.request_id, Some(output), Some(failure))
                    .await
            }
        }
    }
    pub async fn verify_economic_payment(
        &self,
        id: Uuid,
        actor: Uuid,
        signature: &str,
        rpc: &dyn SolanaRpc,
    ) -> Result<EconomicJob> {
        validate_solana_signature(signature)?;
        let j = self.store.economic_job(id, actor).await?;
        if j.network != EconomicNetwork::Devnet {
            return Err(Error::FinancialPolicy);
        }
        let wallet = self.store.wallet(j.agent_id).await?;
        wallet.network.require_devnet()?;
        verify_usdc_mint(rpc, wallet.network).await?;
        let status = rpc
            .read(
                RpcMethod::SignatureStatus,
                json!([[signature],{"searchTransactionHistory":true}]),
            )
            .await?;
        let statuses = status["value"]
            .as_array()
            .filter(|s| s.len() == 1)
            .ok_or(Error::Rpc)?;
        if statuses[0].is_null() || statuses[0]["confirmationStatus"] != "finalized" {
            return Err(Error::Unconfirmed);
        }
        let transaction=rpc.read(RpcMethod::Transaction,json!([signature,{"encoding":"jsonParsed","commitment":"finalized","maxSupportedTransactionVersion":0}])).await?;
        if transaction.is_null() {
            return Err(Error::Unconfirmed);
        }
        let instructions = transaction["transaction"]["message"]["instructions"]
            .as_array()
            .ok_or(Error::InvalidPayment)?;
        let memos: Vec<_> = instructions
            .iter()
            .filter(|v| v["programId"] == "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
            .collect();
        if memos.len() != 1 || memos[0]["parsed"] != j.id.to_string() {
            return Err(Error::InvalidPayment);
        }
        let proof = VerifiedPayment::verify(
            rpc.evidence(),
            wallet.network,
            &wallet.address,
            signature,
            &statuses[0],
            &transaction,
        )?;
        self.store.verified_job_payment(id, actor, &proof).await
    }
}
/// A retry may repair an incomplete plan, but never an unauthorized tool proposal.
fn validate_work_plan(plan: &Plan, permitted: &[String]) -> Result<()> {
    let mut selected = std::collections::BTreeSet::new();
    if plan.actions.len() > 4 {
        return Err(Error::Denied);
    }
    for action in &plan.actions {
        if !permitted.contains(&action.tool)
            || !selected.insert(action.tool.as_str())
            || action.lesson.is_some()
            || action.evaluation.is_some()
            || !action.event_ids.is_empty()
        {
            return Err(Error::Denied);
        }
    }
    if selected.len() != permitted.len() {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn economic_recommendation(
    job: &EconomicJob,
    accounts: &Value,
    failures: usize,
    work: &Value,
) -> Value {
    let available = accounts["networks"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["network"] == job.network.name()))
        .and_then(|row| row["available_atoms"].as_i64());
    let cost = job.spec.cost(job.network);
    let margin = job.spec.price_atoms.checked_sub(cost);
    let supported = work["status"] == "completed";
    json!({"recommendation":if !supported || margin.is_none_or(|m|m<=0) || available.is_some_and(|n|n<cost) {"reject"} else if failures>0 || available.is_none() {"review"} else {"consider"},
        "offered_revenue_atoms":job.spec.price_atoms,"estimated_cost_atoms":cost,"estimated_margin_atoms":margin,
        "available_operating_atoms_snapshot":available,"network":job.network,
        "cost_basis":if job.network == EconomicNetwork::Simulated {"simulated configured estimates; not provider usage"}else{"offline analysis only; no paid provider or transfer"},
        "risk_factors":{"missing_evidence":!supported,"prior_failed_same_customer_tasks":failures},
        "application_limits_enforced":true,"acceptance_authorized":false,"completion_probability":null})
}
fn report(j: &EconomicJob, model: &str) -> Result<Value> {
    let request = WorkRequest::parse(&j.input, j.spec.kind)?;
    let input = &request.material;
    let analysis = match j.spec.kind {
        ServiceKind::Summary => {
            json!({"method":"extractive_first_sentences","sentences":input.split_terminator(['.','!','?']).filter(|s|!s.trim().is_empty()).take(3).map(|s|s.trim().chars().take(700).collect::<String>()).collect::<Vec<_>>() })
        }
        ServiceKind::ResearchReport => {
            json!({"method":"supplied_material_inventory","sources":[{"reference":"customer_input","excerpt":input.chars().take(1200).collect::<String>()}],"limitations":"No external research performed; claims in supplied material are unverified."})
        }
        ServiceKind::CodeAnalysis => {
            json!({"method":"static_text_inspection","lines":input.lines().count(),"todo_lines":input.lines().enumerate().filter(|(_,s)|s.contains("TODO")).take(20).map(|(i,_)|i+1).collect::<Vec<_>>(),"limitations":"No code was executed, compiled or certified secure."})
        }
        ServiceKind::ApiReport => {
            let v: Value = serde_json::from_str(input).map_err(|_| Error::Invalid)?;
            json!({"method":"supplied_json_structure","keys":v.as_object().ok_or(Error::Invalid)?.keys().take(100).collect::<Vec<_>>(),"limitations":"Supplied response only; no API request or freshness claim."})
        }
        ServiceKind::DelegationSimulation => {
            json!({"method":"simulated_delegation","tasks":["inspect supplied input","produce bounded report","verify delivery contract"],"child_agents_created":0,"external_actions":0})
        }
    };
    Ok(
        json!({"contract":"structured_report_v1","job_id":j.id,"kind":j.spec.kind,"provider_model":model,"live_inference":false,"input_bytes":input.len(),"word_count":input.split_whitespace().count(),"analysis":analysis,"verification":"Generated by the allowlisted application tool from supplied input; provider recommendations cannot mutate accounting."}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_plans_cannot_escalate_or_bypass_bounded_checks() {
        let tools = vec![
            "inspect_input".to_owned(),
            "review_code".into(),
            "check_requirements".into(),
        ];
        let action = |tool: &str| ProposedAction {
            tool: tool.into(),
            event_ids: vec![],
            lesson: None,
            evaluation: None,
        };
        assert!(matches!(
            validate_work_plan(&Plan { actions: vec![] }, &tools),
            Err(Error::Invalid)
        ));
        for names in [
            vec!["transfer"],
            vec!["review_code", "review_code"],
            vec![
                "inspect_input",
                "review_code",
                "check_requirements",
                "remember",
            ],
        ] {
            assert!(matches!(
                validate_work_plan(
                    &Plan {
                        actions: names.into_iter().map(action).collect()
                    },
                    &tools
                ),
                Err(Error::Denied)
            ));
        }
        assert!(
            validate_work_plan(
                &Plan {
                    actions: tools.iter().map(|t| action(t)).collect()
                },
                &tools
            )
            .is_ok()
        );
        let mut injected = action("inspect_input");
        injected.lesson = Some("unsupported memory".into());
        assert!(
            validate_work_plan(
                &Plan {
                    actions: vec![injected]
                },
                &tools
            )
            .is_err()
        );
    }
    #[test]
    fn reports_never_execute_code_or_claim_external_research() {
        let mut j:EconomicJob=serde_json::from_value(json!({"id":Uuid::new_v4(),"service_id":Uuid::new_v4(),"agent_id":Uuid::new_v4(),"customer_id":Uuid::new_v4(),"network":"simulated","spec":{"name":"Analysis","kind":"code_analysis","price_atoms":5000,"estimated_inference_atoms":1000,"estimated_infrastructure_atoms":100,"max_input_bytes":8000,"max_attempts":2},"input":"TODO: review\nprint('sample')","state":"executing","revision":1,"attempts":1,"received_atoms":5000,"earned_atoms":0,"expires_at":chrono::Utc::now(),"created_at":chrono::Utc::now()})).unwrap();
        let result = report(&j, "deterministic").unwrap();
        assert_eq!(result["analysis"]["lines"], 2);
        assert_eq!(result["live_inference"], false);
        for kind in [
            ServiceKind::Summary,
            ServiceKind::ResearchReport,
            ServiceKind::CodeAnalysis,
            ServiceKind::ApiReport,
            ServiceKind::DelegationSimulation,
        ] {
            j.spec.kind = kind;
            j.input = if kind == ServiceKind::ApiReport {
                "{\"sample\":1}".into()
            } else {
                "Supplied material. Second sentence.".into()
            };
            j.spec.input(&j.input).unwrap();
            let output = report(&j, "deterministic").unwrap();
            assert_eq!(output["contract"], "structured_report_v1");
            assert!(output["analysis"]["method"].is_string());
            assert!(output.to_string().len() < 16000);
        }
    }
}
