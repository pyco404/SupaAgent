//! Paid evaluation of supplied work, separate from commercial delivery and simulated capital.
use crate::*;
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveWorkConsent {
    pub job_id: Uuid,
    pub expected_revision: i64,
    pub input_sha256: String,
    pub approved_tools: Vec<String>,
    pub use_memory: bool,
    pub consent: LiveConsent,
}
impl LiveWorkConsent {
    pub fn validate(&self, request: &WorkRequest, kind: ServiceKind) -> Result<()> {
        self.consent.validate()?;
        validate_work_source(&request.objective)?;
        validate_work_source(&request.material)?;
        for requirement in &request.requirements {
            validate_work_source(requirement)?;
        }
        let mut permitted = request
            .tools(kind, false)
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if self.use_memory {
            permitted.push("read_memory".into());
        }
        if self.job_id.is_nil()
            || self.expected_revision < 0
            || self.input_sha256.len() != 64
            || !self
                .input_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.approved_tools.len() != permitted.len()
            || self.approved_tools.iter().collect::<BTreeSet<_>>().len() != permitted.len()
            || self
                .approved_tools
                .iter()
                .any(|tool| !permitted.contains(tool))
            || request.material.len() > 4000
            || request.objective.len() > 500
            || self.consent.goal != request.objective
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
}
/// Conservative rejection of recognizable secret material; approved inputs still require human review.
/// This is not a general-purpose secret detector or a claim that arbitrary text is public.
pub fn validate_work_source(text: &str) -> Result<()> {
    let lower = text.to_ascii_lowercase();
    if [
        "-----begin ",
        "sk-proj-",
        "sk-svcacct-",
        "github_pat_",
        "ghp_",
        "api_key",
        "api-key:",
        "authorization: bearer",
        "private_key",
        "seed phrase",
        "password=",
        "password:",
        "postgres://",
        "postgresql://",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return Err(Error::Denied);
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkCitation {
    pub source: String,
    pub line: usize,
    pub quote: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkFinding {
    /// A source observation must be verbatim. Interpretation is separately marked for human review.
    pub observation: String,
    pub interpretation: String,
    pub evidence: Vec<WorkCitation>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkTotal {
    pub field: String,
    pub value: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkProposal {
    pub interpretation: String,
    pub summary: String,
    pub status: String,
    pub tools: Vec<String>,
    pub findings: Vec<WorkFinding>,
    pub totals: Vec<WorkTotal>,
    pub proposed_steps: Vec<String>,
    pub missing_information: Vec<String>,
    pub memory_ids: Vec<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveWorkInput {
    pub request: WorkRequest,
    pub kind: ServiceKind,
    pub approved_tools: Vec<String>,
    pub memories: Vec<Memory>,
    pub deterministic_findings: Value,
}
impl WorkProposal {
    pub fn validate(&self, input: &LiveWorkInput) -> Result<()> {
        clean_text(&self.interpretation, 600)?;
        clean_text(&self.summary, 1000)?;
        if !matches!(
            self.status.as_str(),
            "supported" | "incomplete" | "abstained"
        ) || self.tools.len() != input.approved_tools.len()
            || self.tools.iter().collect::<BTreeSet<_>>().len() != self.tools.len()
            || self
                .tools
                .iter()
                .any(|tool| !input.approved_tools.contains(tool))
            || self.findings.len() > 6
            || self.totals.len() > 8
            || self.proposed_steps.len() > 4
            || self.missing_information.len() > 8
            || self.memory_ids.len() > 2
            || self.memory_ids.iter().collect::<BTreeSet<_>>().len() != self.memory_ids.len()
            || self.memory_ids.iter().any(|id| {
                !input
                    .memories
                    .iter()
                    .any(|m| m.id == *id && m.kind == MemoryKind::VerifiedObservation)
            })
        {
            return Err(Error::Denied);
        }
        let baseline = analyze_work(&input.request, input.kind)?;
        if (input.request.require_external_research && self.status != "abstained")
            || (baseline["status"] != "completed" && self.status == "supported")
            || (self.status == "supported" && self.findings.is_empty())
            || (self.status == "incomplete" && self.missing_information.is_empty())
        {
            return Err(Error::Invalid);
        }
        for text in self.proposed_steps.iter().chain(&self.missing_information) {
            clean_text(text, 300)?;
        }
        for finding in &self.findings {
            clean_text(&finding.observation, 300)?;
            clean_text(&finding.interpretation, 600)?;
            if finding.evidence.is_empty() || finding.evidence.len() > 3 {
                return Err(Error::Invalid);
            }
            let mut supports_observation = false;
            for citation in &finding.evidence {
                clean_text(&citation.quote, 300)?;
                let source = if citation.source == "material" {
                    &input.request.material
                } else {
                    let memory = input
                        .memories
                        .iter()
                        .find(|m| {
                            m.id.to_string() == citation.source && self.memory_ids.contains(&m.id)
                        })
                        .ok_or(Error::Denied)?;
                    &memory.lesson
                };
                if citation.line == 0
                    || !source
                        .lines()
                        .nth(citation.line - 1)
                        .is_some_and(|line| line.contains(&citation.quote))
                {
                    return Err(Error::Invalid);
                }
                supports_observation |= citation.quote.contains(&finding.observation);
            }
            if !supports_observation {
                return Err(Error::Invalid);
            }
        }
        let mut fields = BTreeSet::new();
        for total in &self.totals {
            if !fields.insert(&total.field)
                || !baseline["findings"]["totals"]
                    .as_array()
                    .is_some_and(|rows| {
                        rows.iter().any(|row| {
                            row["field"] == total.field
                                && row["exact"] == true
                                && row["integer_sum"].as_i64() == Some(total.value)
                        })
                    })
            {
                return Err(Error::Invalid);
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveWorkRecord {
    pub provider_evidence: String,
    pub authorization: LiveWorkConsent,
    pub request: WorkRequest,
    pub kind: ServiceKind,
    pub baseline: Value,
    pub report: Option<WorkProposal>,
    pub tool_results: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> LiveWorkInput {
        LiveWorkInput {
            request: WorkRequest {
                objective: "Analyze replicas".into(),
                material: "Storage uses replicas.\nEvidence is limited.".into(),
                requirements: vec![],
                require_external_research: false,
            },
            kind: ServiceKind::ResearchReport,
            approved_tools: vec![
                "inspect_input".into(),
                "extract_evidence".into(),
                "check_requirements".into(),
            ],
            memories: vec![],
            deterministic_findings: Value::Null,
        }
    }
    fn proposal() -> WorkProposal {
        WorkProposal {
            interpretation: "Analyze supplied storage claims".into(),
            summary: "Replicas may improve resilience; independent verification is needed.".into(),
            status: "supported".into(),
            tools: input().approved_tools,
            findings: vec![WorkFinding {
                observation: "Storage uses replicas.".into(),
                interpretation: "This is a supplied design claim, not a verified deployment."
                    .into(),
                evidence: vec![WorkCitation {
                    source: "material".into(),
                    line: 1,
                    quote: "Storage uses replicas.".into(),
                }],
            }],
            totals: vec![],
            proposed_steps: vec![],
            missing_information: vec![],
            memory_ids: vec![],
        }
    }
    #[test]
    fn recognizable_credentials_are_rejected_before_prompt_construction() {
        for value in [
            "sk-proj-example",
            "API_KEY = omitted",
            "postgresql://not-a-live-target",
            "Authorization: Bearer omitted",
        ] {
            assert!(validate_work_source(value).is_err());
        }
        assert!(validate_work_source("Review supplied non-sensitive recovery notes.").is_ok());
    }
    #[test]
    fn grounding_rejects_invented_sources_claims_tools_and_totals() {
        let i = input();
        let good = proposal();
        assert!(good.validate(&i).is_ok());
        let mut p = good.clone();
        p.findings[0].observation = "Revenue is guaranteed".into();
        assert!(p.validate(&i).is_err());
        let mut p = good.clone();
        p.findings[0].evidence[0].line = 2;
        assert!(p.validate(&i).is_err());
        let mut p = good.clone();
        p.tools[0] = "transfer".into();
        assert!(p.validate(&i).is_err());
        let mut p = good.clone();
        p.memory_ids.push(Uuid::new_v4());
        assert!(p.validate(&i).is_err());
        let mut p = good;
        p.totals.push(WorkTotal {
            field: "revenue".into(),
            value: 5000,
        });
        assert!(p.validate(&i).is_err());
    }
    #[test]
    fn model_cannot_override_missing_information_or_external_research() {
        let mut i = input();
        i.request.requirements.push("missing measurement".into());
        assert!(proposal().validate(&i).is_err());
        i.request.requirements.clear();
        i.request.require_external_research = true;
        assert!(proposal().validate(&i).is_err());
        let mut p = proposal();
        p.status = "abstained".into();
        assert!(p.validate(&i).is_ok());
    }
}
