//! Synthetic provider for tests/examples only. No HTTP client or real credential is constructed.
use std::sync::atomic::{AtomicUsize, Ordering};
use supa_domain::*;
use supa_providers::{Invocation, controlled::ControlledProvider};
pub struct MockWorkProvider {
    pub policy: InferencePolicy,
    pub calls: AtomicUsize,
    pub mode: u8,
}
#[async_trait::async_trait]
impl ControlledProvider for MockWorkProvider {
    fn policy(&self) -> InferencePolicy {
        self.policy.clone()
    }
    fn project(&self) -> &str {
        "proj_synthetic_work_fixture"
    }
    async fn verify(&self) -> Result<()> {
        if self.mode == 5 {
            Err(Error::ProviderHttp(401))
        } else {
            Ok(())
        }
    }
    async fn reason(&self, _: &str, _: &[Memory]) -> Invocation<LiveProposal> {
        Invocation {
            diagnostic: None,
            result: Err(Error::Denied),
            usage: None,
        }
    }
    async fn work(&self, input: &LiveWorkInput) -> Invocation<WorkProposal> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        if self.mode == 3 {
            return Invocation {
                diagnostic: None,
                result: Err(Error::ProviderTransport),
                usage: None,
            };
        }
        if self.mode == 6 {
            std::future::pending::<()>().await;
        }
        let baseline = analyze_work(&input.request, input.kind).unwrap();
        let quote = input
            .request
            .material
            .lines()
            .next()
            .unwrap()
            .chars()
            .take(250)
            .collect::<String>();
        let mut report = WorkProposal {
            interpretation: "Mock interpretation of supplied evaluation input".into(),
            summary: format!(
                "Synthetic provider report using {} permitted verified memories; not an actual GPT result.",
                input.memories.len()
            ),
            status: if baseline["status"] == "completed" {
                "supported"
            } else if baseline["status"] == "incomplete" {
                "incomplete"
            } else {
                "abstained"
            }
            .into(),
            tools: input.approved_tools.clone(),
            findings: vec![WorkFinding {
                observation: quote.clone(),
                interpretation: "Mock analysis only; independently review the supplied statement."
                    .into(),
                evidence: vec![WorkCitation {
                    source: "material".into(),
                    line: 1,
                    quote,
                }],
            }],
            totals: baseline["findings"]["totals"]
                .as_array()
                .map(|rows| {
                    rows.iter()
                        .filter_map(|row| {
                            Some(WorkTotal {
                                field: row["field"].as_str()?.into(),
                                value: row["integer_sum"].as_i64()?,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            proposed_steps: if input.kind == ServiceKind::DelegationSimulation {
                vec![
                    "Inspect supplied sources".into(),
                    "Draft a comparison with evidence".into(),
                    "Review missing information".into(),
                ]
            } else {
                vec![]
            },
            missing_information: if baseline["status"] != "completed" {
                vec!["Required evidence was not supplied".into()]
            } else {
                vec![]
            },
            memory_ids: vec![],
        };
        if let Some(m) = input.memories.first() {
            report.memory_ids.push(m.id);
            let quote = m
                .lesson
                .lines()
                .next()
                .unwrap()
                .chars()
                .take(250)
                .collect::<String>();
            report.findings.push(WorkFinding {
                observation: quote.clone(),
                interpretation:
                    "Observed disposable cycle outcome, not proof that a new goal is complete."
                        .into(),
                evidence: vec![WorkCitation {
                    source: m.id.to_string(),
                    line: 1,
                    quote,
                }],
            });
        }
        if self.mode == 1 {
            report.findings[0].observation = "Invented guaranteed revenue".into();
        }
        if self.mode == 2 {
            report.tools[0] = "transfer".into();
        }
        let usage = if self.mode == 4 {
            None
        } else {
            Some(TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cached_input_tokens: 0,
                cache_write_tokens: 0,
            })
        };
        Invocation {
            diagnostic: None,
            result: Ok(report),
            usage,
        }
    }
}
pub fn policy() -> InferencePolicy {
    InferencePolicy {
        model: "mock-work".into(),
        live: true,
        budget_id: Some(uuid::Uuid::new_v4()),
        budget_limit_microusd: 100000,
        input_rate: 100000,
        cached_input_rate: 10000,
        cache_write_rate: 125000,
        output_rate: 500000,
        input_token_bound: 24192,
        max_output_tokens: 1024,
    }
}
