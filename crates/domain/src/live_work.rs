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
        self.validate_fields(input).map_err(|error| {
            WorkRejection::attach(error, &serde_json::to_value(self).unwrap_or(Value::Null))
        })
    }
    fn validate_fields(&self, input: &LiveWorkInput) -> Result<()> {
        use WorkRejectionCode as Code;
        use WorkValidationStage as Stage;
        let fail = |stage, code, path: &str| WorkRejection::error(stage, code, path);
        let text = |value: &str, max, path: &str| {
            clean_text(value, max).map_err(|_| fail(Stage::Domain, Code::InvalidText, path))
        };
        text(&self.interpretation, 600, "/interpretation")?;
        text(&self.summary, 1000, "/summary")?;
        if !matches!(
            self.status.as_str(),
            "supported" | "incomplete" | "abstained"
        ) {
            return Err(fail(Stage::Completeness, Code::InvalidStatus, "/status"));
        }
        if self.tools.len() != input.approved_tools.len()
            || self.tools.iter().collect::<BTreeSet<_>>().len() != self.tools.len()
            || self.tools.iter().any(|t| !input.approved_tools.contains(t))
        {
            return Err(fail(Stage::Tools, Code::UnauthorizedTool, "/tools"));
        }
        for (path, size, max) in [
            ("/findings", self.findings.len(), 6),
            ("/totals", self.totals.len(), 8),
            ("/proposed_steps", self.proposed_steps.len(), 4),
            ("/missing_information", self.missing_information.len(), 8),
            ("/memory_ids", self.memory_ids.len(), 2),
        ] {
            if size > max {
                return Err(fail(Stage::Domain, Code::BoundsExceeded, path));
            }
        }
        if self.memory_ids.iter().collect::<BTreeSet<_>>().len() != self.memory_ids.len()
            || self.memory_ids.iter().any(|id| {
                !input
                    .memories
                    .iter()
                    .any(|m| m.id == *id && m.kind == MemoryKind::VerifiedObservation)
            })
        {
            return Err(fail(
                Stage::Evidence,
                Code::InvalidMemoryReference,
                "/memory_ids",
            ));
        }
        let baseline = analyze_work(&input.request, input.kind)
            .map_err(|_| fail(Stage::Domain, Code::OtherDomainFailure, "/"))?;
        if (input.request.require_external_research && self.status != "abstained")
            || (baseline["status"] != "completed" && self.status == "supported")
        {
            return Err(fail(Stage::Completeness, Code::IncompleteWork, "/status"));
        }
        if self.status == "supported" && self.findings.is_empty() {
            return Err(fail(Stage::Evidence, Code::MissingEvidence, "/findings"));
        }
        if self.status == "incomplete" && self.missing_information.is_empty() {
            return Err(fail(
                Stage::Completeness,
                Code::IncompleteWork,
                "/missing_information",
            ));
        }
        for (name, values) in [
            ("proposed_steps", &self.proposed_steps),
            ("missing_information", &self.missing_information),
        ] {
            for (i, value) in values.iter().enumerate() {
                text(value, 300, &format!("/{name}/{i}"))?;
            }
        }
        for (i, finding) in self.findings.iter().enumerate() {
            let path = format!("/findings/{i}");
            text(&finding.observation, 300, &format!("{path}/observation"))?;
            text(
                &finding.interpretation,
                600,
                &format!("{path}/interpretation"),
            )?;
            if finding.evidence.is_empty() {
                return Err(fail(
                    Stage::Evidence,
                    Code::MissingEvidence,
                    &format!("{path}/evidence"),
                ));
            }
            if finding.evidence.len() > 3 {
                return Err(fail(
                    Stage::Evidence,
                    Code::BoundsExceeded,
                    &format!("{path}/evidence"),
                ));
            }
            let mut supports = false;
            for (j, citation) in finding.evidence.iter().enumerate() {
                let path = format!("{path}/evidence/{j}");
                text(&citation.quote, 300, &format!("{path}/quote"))?;
                let source = if citation.source == "material" {
                    &input.request.material
                } else {
                    &input
                        .memories
                        .iter()
                        .find(|m| {
                            m.id.to_string() == citation.source && self.memory_ids.contains(&m.id)
                        })
                        .ok_or_else(|| {
                            fail(
                                Stage::Evidence,
                                Code::InvalidCitation,
                                &format!("{path}/source"),
                            )
                        })?
                        .lesson
                };
                if citation.line == 0 || source.lines().nth(citation.line - 1).is_none() {
                    return Err(fail(
                        Stage::Evidence,
                        Code::InvalidCitation,
                        &format!("{path}/line"),
                    ));
                }
                if !source
                    .lines()
                    .nth(citation.line - 1)
                    .is_some_and(|line| line.contains(&citation.quote))
                {
                    return Err(fail(
                        Stage::Evidence,
                        Code::InvalidCitation,
                        &format!("{path}/quote"),
                    ));
                }
                supports |= citation.quote.contains(&finding.observation);
            }
            if !supports {
                return Err(fail(
                    Stage::Evidence,
                    Code::UnsupportedClaim,
                    &format!("{path}/observation"),
                ));
            }
        }
        let mut fields = BTreeSet::new();
        for (i, total) in self.totals.iter().enumerate() {
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
                return Err(fail(
                    Stage::Evidence,
                    Code::InvalidTotal,
                    &format!("/totals/{i}"),
                ));
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
