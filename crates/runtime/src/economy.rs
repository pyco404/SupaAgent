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
        context.goals = vec![Goal {
            id: j.id,
            description: j.input.clone(),
            completed: false,
            priority: 0,
            suggested_progress_bps: 0,
        }];
        context.memory = if a.capabilities.contains(&Capability::ReadMemory) {
            relevant_memories(&a, &j.input, 4)
        } else {
            vec![]
        };
        context.verified_events.clear();
        context.permitted_tools.retain(|t| t == "read_memory");
        context.max_steps = 1;
        let claimed = self.store.claim_economic_job(id,actor,command.clone(),json!({"provider_model":policy.model,"live":false,"contract":j.spec.output_contract(),"estimated_simulated_cost_atoms":j.spec.cost(j.network),"reliability":"Bounded extractive/static analysis only; no completion probability claimed."})).await?;
        let outcome = tokio::time::timeout(Duration::from_secs(10), async {
            let invocation = self.reasoner.plan_metered(&context).await;
            let plan = invocation.result?;
            if plan.actions.len() > 1 {
                return Err(Error::Denied);
            }
            for action in &plan.actions {
                if action.tool != "read_memory" {
                    return Err(Error::Denied);
                }
                authorize(&self.store.get(claimed.agent_id).await?, action)?;
            }
            // The model only recommends; application-defined, pure analysis establishes delivery.
            let mut output = report(&claimed, &policy.model)?;
            output["provider_proposed_tools"] = json!(
                plan.actions
                    .iter()
                    .map(|a| a.tool.as_str())
                    .collect::<Vec<_>>()
            );
            Ok(output)
        })
        .await
        .unwrap_or(Err(Error::Timeout));
        match outcome {
            Ok(output) => {
                self.store
                    .finish_economic_job(id, actor, command.request_id, Some(output), None)
                    .await
            }
            Err(_) => {
                self.store
                    .finish_economic_job(
                        id,
                        actor,
                        command.request_id,
                        None,
                        Some("bounded_provider_or_contract_failure"),
                    )
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
fn report(j: &EconomicJob, model: &str) -> Result<Value> {
    let input = &j.input;
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
