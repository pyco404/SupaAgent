//! Bounded operational evidence, never credentials, response prose or hidden reasoning.
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderOperation {
    Metadata,
    Generation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStage {
    LocalValidation,
    RequestSend,
    ResponseHeaders,
    ResponseBody,
    ResponseParse,
    UsageValidation,
    OutputValidation,
    Completed,
    RuntimeDeadline,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFailure {
    LocalRejection,
    Connection,
    Timeout,
    Transport,
    InterruptedResponse,
    HttpStatus,
    ResponseTooLarge,
    MalformedJson,
    ModelMismatch,
    UnverifiedUsage,
    InvalidOutput,
    RuntimeDeadline,
}
/// Counts observed in a response, NOT validated/priced accounting usage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportedUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub cache_write_5m_tokens: Option<u64>,
    pub cache_write_1h_tokens: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderDiagnostic {
    pub operation: ProviderOperation,
    pub stage: ProviderStage,
    pub observed_at: DateTime<Utc>,
    pub response_headers_at: Option<DateTime<Utc>>,
    /// Intent only, never proof of provider receipt or billing.
    pub dispatch_intended: bool,
    pub response_complete: bool,
    pub http_status: Option<u16>,
    pub request_id: Option<String>,
    pub message_id: Option<String>,
    pub reported_model: Option<String>,
    pub failure: Option<ProviderFailure>,
    pub error_type: Option<String>,
    pub reported_usage: Option<ReportedUsage>,
}
impl ProviderDiagnostic {
    pub fn new(operation: ProviderOperation, stage: ProviderStage) -> Self {
        Self {
            operation,
            stage,
            observed_at: Utc::now(),
            response_headers_at: None,
            dispatch_intended: false,
            response_complete: false,
            http_status: None,
            request_id: None,
            message_id: None,
            reported_model: None,
            failure: None,
            error_type: None,
            reported_usage: None,
        }
    }
    pub fn observe_now(&mut self) {
        self.observed_at = Utc::now();
    }
    pub fn headers_received(&mut self, status: u16) {
        self.http_status = Some(status);
        self.response_headers_at = Some(Utc::now());
        self.observe_now();
    }
    pub fn identifier(value: &str, prefix: &str) -> Option<String> {
        let rest = value.strip_prefix(prefix)?;
        (!rest.is_empty() && rest.len() <= 96 && rest.bytes().all(|b| b.is_ascii_alphanumeric()))
            .then(|| value.to_owned())
    }
    pub fn validate(&self) -> Result<()> {
        if self.http_status.is_some_and(|n| !(100..=599).contains(&n))
            || self
                .request_id
                .as_ref()
                .is_some_and(|s| Self::identifier(s, "req_").is_none())
            || self
                .message_id
                .as_ref()
                .is_some_and(|s| Self::identifier(s, "msg_").is_none())
            || self.reported_model.as_ref().is_some_and(|s| {
                s.contains("sk-")
                    || !s.starts_with("claude-")
                    || s.len() > 96
                    || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
            || self
                .error_type
                .as_deref()
                .is_some_and(|s| !Self::safe_error_type(s))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    pub fn safe_error_type(value: &str) -> bool {
        matches!(
            value,
            "invalid_request_error"
                | "authentication_error"
                | "billing_error"
                | "permission_error"
                | "not_found_error"
                | "conflict_error"
                | "request_too_large"
                | "rate_limit_error"
                | "api_error"
                | "timeout_error"
                | "overloaded_error"
        )
    }
    pub fn deadline(operation: ProviderOperation) -> Self {
        let mut d = Self::new(operation, ProviderStage::RuntimeDeadline);
        d.dispatch_intended = true;
        d.failure = Some(ProviderFailure::RuntimeDeadline);
        d
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifiers_and_error_types_cannot_carry_arbitrary_response_prose() {
        for s in [
            "sk-ant-secret",
            "req_sk-ant-secret",
            "req_abc\nAuthorization",
            "req_",
            "req_hello world",
        ] {
            assert!(ProviderDiagnostic::identifier(s, "req_").is_none());
        }
        assert!(ProviderDiagnostic::identifier("req_123ABC", "req_").is_some());
        let mut d = ProviderDiagnostic::deadline(ProviderOperation::Generation);
        d.error_type = Some("private response prose".into());
        assert!(d.validate().is_err());
    }
}
