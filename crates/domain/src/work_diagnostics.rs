//! No model prose or source text is approved for diagnostic retention.
use crate::*;
use serde::de::IntoDeserializer;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkValidationStage {
    StructuredOutput,
    Schema,
    Evidence,
    Tools,
    Completeness,
    Domain,
    Execution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkRejectionCode {
    InvalidJson,
    InvalidStructuredOutput,
    SchemaMismatch,
    MissingEvidence,
    UnsupportedClaim,
    InvalidCitation,
    UnauthorizedTool,
    IncompleteWork,
    InvalidMemoryReference,
    InvalidStatus,
    BoundsExceeded,
    InvalidText,
    InvalidTotal,
    PermissionChanged,
    OtherDomainFailure,
}
impl WorkRejectionCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidJson => "work_invalid_json",
            Self::InvalidStructuredOutput => "work_invalid_structured_output",
            Self::SchemaMismatch => "work_schema_mismatch",
            Self::MissingEvidence => "work_missing_evidence",
            Self::UnsupportedClaim => "work_unsupported_claim",
            Self::InvalidCitation => "work_invalid_citation",
            Self::UnauthorizedTool => "work_unauthorized_tool",
            Self::IncompleteWork => "work_incomplete",
            Self::InvalidMemoryReference => "work_invalid_memory_reference",
            Self::InvalidStatus => "work_invalid_status",
            Self::BoundsExceeded => "work_bounds_exceeded",
            Self::InvalidText => "work_invalid_text",
            Self::InvalidTotal => "work_invalid_total",
            Self::PermissionChanged => "work_permission_changed",
            Self::OtherDomainFailure => "work_domain_failure",
        }
    }
}
fn safe_component(s: &str) -> bool {
    matches!(
        s,
        "interpretation"
            | "summary"
            | "status"
            | "tools"
            | "findings"
            | "observation"
            | "evidence"
            | "source"
            | "line"
            | "quote"
            | "totals"
            | "field"
            | "value"
            | "proposed_steps"
            | "missing_information"
            | "memory_ids"
    ) || (!s.is_empty() && s.len() <= 6 && s.bytes().all(|b| b.is_ascii_digit()))
}
fn safe_path(path: &str) -> String {
    if path == "/"
        || (path.starts_with('/') && path.len() <= 160 && path[1..].split('/').all(safe_component))
    {
        path.to_owned()
    } else {
        "/".into()
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputFieldShape {
    pub path: String,
    pub kind: OutputKind,
    pub length: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RejectedWorkShape {
    pub fields: Vec<OutputFieldShape>,
    pub truncated: bool,
}
impl RejectedWorkShape {
    pub fn from_output(value: &Value) -> Self {
        fn visit(
            value: &Value,
            path: String,
            fields: &mut Vec<OutputFieldShape>,
            truncated: &mut bool,
        ) {
            if fields.len() == 32 {
                *truncated = true;
                return;
            }
            let (kind, length) = match value {
                Value::Null => (OutputKind::Null, None),
                Value::Bool(_) => (OutputKind::Boolean, None),
                Value::Number(_) => (OutputKind::Number, None),
                Value::String(s) => (OutputKind::String, Some(s.chars().count())),
                Value::Array(a) => (OutputKind::Array, Some(a.len())),
                Value::Object(o) => (OutputKind::Object, Some(o.len())),
            };
            fields.push(OutputFieldShape {
                path: path.clone(),
                kind,
                length,
            });
            if path.len() > 120 {
                return;
            }
            match value {
                Value::Object(o) => {
                    for (key, v) in o {
                        if safe_component(key) && !key.bytes().all(|b| b.is_ascii_digit()) {
                            visit(
                                v,
                                format!("{}/{key}", path.trim_end_matches('/')),
                                fields,
                                truncated,
                            );
                        }
                    }
                }
                Value::Array(a) => {
                    for (i, v) in a.iter().take(7).enumerate() {
                        visit(
                            v,
                            format!("{}/{i}", path.trim_end_matches('/')),
                            fields,
                            truncated,
                        );
                    }
                }
                _ => {}
            }
        }
        let mut output = Self {
            fields: vec![],
            truncated: false,
        };
        visit(value, "/".into(), &mut output.fields, &mut output.truncated);
        output
    }
    pub fn validate(&self) -> Result<()> {
        if self.fields.len() > 32 || self.fields.iter().any(|f| safe_path(&f.path) != f.path) {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkRejection {
    pub stage: WorkValidationStage,
    pub code: WorkRejectionCode,
    pub field_path: String,
    pub rejected_output: Option<RejectedWorkShape>,
}
impl std::fmt::Display for WorkRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code.as_str())
    }
}
impl WorkRejection {
    pub fn error(stage: WorkValidationStage, code: WorkRejectionCode, path: &str) -> Error {
        Error::WorkRejected(Box::new(Self {
            stage,
            code,
            field_path: safe_path(path),
            rejected_output: None,
        }))
    }
    pub fn attach(error: Error, value: &Value) -> Error {
        match error {
            Error::WorkRejected(mut rejection) => {
                rejection.rejected_output = Some(RejectedWorkShape::from_output(value));
                Error::WorkRejected(rejection)
            }
            other => other,
        }
    }
    pub fn from_error(error: &Error) -> Self {
        if let Error::WorkRejected(rejection) = error {
            return (**rejection).clone();
        }
        let (stage, code) = match error {
            Error::Provider => (
                WorkValidationStage::StructuredOutput,
                WorkRejectionCode::InvalidStructuredOutput,
            ),
            Error::Denied | Error::Unauthorized => (
                WorkValidationStage::Execution,
                WorkRejectionCode::PermissionChanged,
            ),
            _ => (
                WorkValidationStage::Domain,
                WorkRejectionCode::OtherDomainFailure,
            ),
        };
        Self {
            stage,
            code,
            field_path: "/".into(),
            rejected_output: None,
        }
    }
}
impl WorkProposal {
    pub fn decode_output(value: Value) -> Result<Self> {
        if let Some(object) = value.as_object() {
            for key in [
                "interpretation",
                "summary",
                "status",
                "tools",
                "findings",
                "totals",
                "proposed_steps",
                "missing_information",
                "memory_ids",
            ] {
                if !object.contains_key(key) {
                    return Err(WorkRejection::attach(
                        WorkRejection::error(
                            WorkValidationStage::Schema,
                            WorkRejectionCode::SchemaMismatch,
                            &format!("/{key}"),
                        ),
                        &value,
                    ));
                }
            }
        }
        serde_path_to_error::deserialize(value.clone().into_deserializer()).map_err(|e| {
            let mut path = String::new();
            for part in e.path() {
                match part {
                    serde_path_to_error::Segment::Seq { index } => {
                        path.push_str(&format!("/{index}"))
                    }
                    serde_path_to_error::Segment::Map { key } if safe_component(key) => {
                        path.push_str(&format!("/{key}"))
                    }
                    _ => {
                        path = "/".into();
                        break;
                    }
                }
            }
            WorkRejection::attach(
                WorkRejection::error(
                    WorkValidationStage::Schema,
                    WorkRejectionCode::SchemaMismatch,
                    if path.is_empty() { "/" } else { &path },
                ),
                &value,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn input() -> LiveWorkInput {
        LiveWorkInput {
            request: WorkRequest {
                objective: "Review supplied facts".into(),
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
    fn output() -> Value {
        json!({"interpretation":"Review supplied facts","summary":"Supplied statements only.","status":"supported","tools":input().approved_tools,
            "findings":[{"observation":"Storage uses replicas.","interpretation":"Unverified supplied statement.","evidence":[{"source":"material","line":1,"quote":"Storage uses replicas."}]}],
            "totals":[],"proposed_steps":[],"missing_information":[],"memory_ids":[]})
    }
    fn reason(value: Value, input: &LiveWorkInput) -> WorkRejection {
        let result = WorkProposal::decode_output(value).and_then(|p| p.validate(input));
        WorkRejection::from_error(&result.unwrap_err())
    }
    #[test]
    fn exact_rejection_codes_and_paths_cover_grounding_tools_and_completeness() {
        use WorkRejectionCode as C;
        let mut cases = vec![];
        let mut v = output();
        v["findings"][0]["observation"] = json!("Invented revenue");
        cases.push((v, C::UnsupportedClaim, "/findings/0/observation"));
        let mut v = output();
        v["findings"][0]["evidence"] = json!([]);
        cases.push((v, C::MissingEvidence, "/findings/0/evidence"));
        let mut v = output();
        v["findings"][0]["evidence"][0]["source"] = json!("unknown");
        cases.push((v, C::InvalidCitation, "/findings/0/evidence/0/source"));
        let mut v = output();
        v["findings"][0]["evidence"][0]["line"] = json!(20);
        cases.push((v, C::InvalidCitation, "/findings/0/evidence/0/line"));
        let mut v = output();
        v["findings"][0]["evidence"][0]["quote"] = json!("Fabricated quote");
        cases.push((v, C::InvalidCitation, "/findings/0/evidence/0/quote"));
        let mut v = output();
        v["tools"][0] = json!("transfer");
        cases.push((v, C::UnauthorizedTool, "/tools"));
        let mut v = output();
        v["status"] = json!("incomplete");
        cases.push((v, C::IncompleteWork, "/missing_information"));
        let mut v = output();
        v["memory_ids"] = json!([Uuid::new_v4()]);
        cases.push((v, C::InvalidMemoryReference, "/memory_ids"));
        let mut v = output();
        v["summary"] = json!("");
        cases.push((v, C::InvalidText, "/summary"));
        let mut v = output();
        v["summary"] = json!(7);
        cases.push((v, C::SchemaMismatch, "/summary"));
        let mut v = output();
        v.as_object_mut().unwrap().remove("summary");
        cases.push((v, C::SchemaMismatch, "/summary"));
        let mut v = output();
        v["totals"] = json!([{"field":"revenue","value":700}]);
        cases.push((v, C::InvalidTotal, "/totals/0"));
        let mut v = output();
        v["status"] = json!("commercial_success");
        cases.push((v, C::InvalidStatus, "/status"));
        let mut v = output();
        v["proposed_steps"] = json!(["a", "b", "c", "d", "e"]);
        cases.push((v, C::BoundsExceeded, "/proposed_steps"));
        for (v, code, path) in cases {
            let rejection = reason(v, &input());
            assert_eq!(rejection.code, code);
            assert_eq!(rejection.field_path, path);
            rejection.rejected_output.unwrap().validate().unwrap();
        }
        let mut i = input();
        i.request.requirements.push("missing measurement".into());
        assert_eq!(reason(output(), &i).code, C::IncompleteWork);
        let mut i = input();
        i.request.require_external_research = true;
        assert_eq!(reason(output(), &i).code, C::IncompleteWork);
    }
    #[test]
    fn projection_never_retains_values_unknown_keys_or_error_prose() {
        let secret = ["sk-", "synthetic-not-a-real-key"].concat();
        let mut v = output();
        v["summary"] = json!(secret);
        v["customer@example.invalid"] = json!("private customer input");
        let rejected = reason(v, &input());
        let encoded = serde_json::to_string(&rejected).unwrap();
        for s in [
            &secret,
            "customer@example.invalid",
            "private customer input",
            "Storage uses replicas.",
        ] {
            assert!(!encoded.contains(s));
        }
        let invalid =
            WorkProposal::decode_output(json!({"summary":{},secret.clone():secret})).unwrap_err();
        assert!(!format!("{invalid:?}").contains("synthetic-not-a-real-key"));
        assert_eq!(safe_path("/secret@example.invalid"), "/");
        let v = json!({"findings":vec![output();100]});
        let shape = RejectedWorkShape::from_output(&v);
        assert!(shape.fields.len() <= 32);
        assert!(shape.truncated);
        shape.validate().unwrap();
    }
    #[test]
    fn honest_abstention_and_incomplete_work_remain_valid() {
        for status in ["supported", "incomplete", "abstained"] {
            let mut v = output();
            v["status"] = json!(status);
            if status == "incomplete" {
                v["missing_information"] = json!(["No measured recovery times supplied"]);
            }
            WorkProposal::decode_output(v)
                .unwrap()
                .validate(&input())
                .unwrap();
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkDiagnostic {
    pub version: u8,
    pub response_schema: String,
    pub retention_policy: String,
    pub provider: String,
    pub model: String,
    pub agent_id: Uuid,
    pub evaluation_id: Uuid,
    pub request_id: Uuid,
    pub input_sha256: String,
    pub rejection: WorkRejection,
    pub usage: Option<TokenUsage>,
    pub estimated_cost_microusd: i64,
    pub outcome: String,
    pub provider_request_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
impl WorkDiagnostic {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1
            || self.response_schema != "bounded_work_proposal_v1"
            || self.retention_policy != "structure_only_v1"
            || !matches!(self.provider.as_str(), "mock" | "openai" | "anthropic")
            || self.model.is_empty()
            || self.model.len() > 100
            || self.model.contains("sk-")
            || !self
                .model
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'.')
            || self.input_sha256.len() != 64
            || !self.input_sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || self.rejection.field_path != safe_path(&self.rejection.field_path)
            || !matches!(self.outcome.as_str(), "failed" | "unknown")
            || self.estimated_cost_microusd < 0
            || self.expires_at != self.created_at + chrono::Duration::days(7)
            || self
                .provider_request_id
                .as_deref()
                .is_some_and(|id| ProviderDiagnostic::identifier(id, "req_").is_none())
            || serde_json::to_vec(self).map_err(|_| Error::Invalid)?.len() > 8192
        {
            return Err(Error::Invalid);
        }
        if let Some(output) = &self.rejection.rejected_output {
            output.validate()?;
        }
        if let Some(usage) = self.usage {
            usage.validate()?;
        }
        Ok(())
    }
}
