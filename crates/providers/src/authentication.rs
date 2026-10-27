//! Generation-free authentication checks. No inference policy, database, retry or paid endpoint.
use super::*;

pub(crate) fn key_format_valid(key: &str) -> bool {
    !key.is_empty()
        && key.is_ascii()
        && !key
            .chars()
            .any(|c| c.is_ascii_control() || c.is_whitespace() || c == '\'' || c == '"')
}
pub fn key_diagnostics(key: Option<&str>) -> Value {
    json!({"present":key.is_some_and(|k|!k.is_empty()),
        "basic_format_valid":key.is_some_and(key_format_valid),
        "expected_api_key_prefix":key.is_some_and(|k|k.starts_with("sk-")),
        "contains_whitespace":key.is_some_and(|k|k.chars().any(char::is_whitespace)),
        "ascii_only":key.is_some_and(str::is_ascii),
        "contains_quotes":key.is_some_and(|k|k.contains(['\'', '"'])),
        "contains_bearer_prefix":key.is_some_and(|k|k.to_ascii_lowercase().starts_with("bearer "))})
}
fn safe_code(body: &[u8]) -> &'static str {
    let value: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    match value["error"]["code"].as_str() {
        Some("invalid_api_key") => "invalid_api_key",
        Some("invalid_organization") => "invalid_organization",
        Some("organization_not_found") => "organization_not_found",
        Some("project_not_found") => "project_not_found",
        Some("mismatched_project") => "mismatched_project",
        Some("ip_not_authorized") => "ip_not_authorized",
        Some("insufficient_permissions") => "insufficient_permissions",
        Some("model_not_found") => "model_not_found",
        Some("rate_limit_exceeded") => "rate_limit_exceeded",
        Some("insufficient_quota") => "insufficient_quota",
        _ => "unclassified",
    }
}
impl OpenAi {
    /// Separate operator authorization is mandatory, even when paid inference is enabled.
    pub async fn authentication_probe(authorized: bool) -> Result<Value> {
        if !authorized {
            return Err(Error::Denied);
        }
        let model = configured_model(false)?;
        let key = std::env::var("OPENAI_API_KEY").map_err(|_| Error::Invalid)?;
        let provider = Self::new(key, model)?;
        Ok(provider.probe().await)
    }
    async fn probe(&self) -> Value {
        let endpoint = format!("https://api.openai.com/v1/models/{}", self.model);
        #[cfg(test)]
        let endpoint = self.endpoint.as_deref().unwrap_or(&endpoint);
        let result = self
            .client
            .get(endpoint)
            .bearer_auth(&self.api_key)
            .send()
            .await;
        let mut report = json!({"method":"GET","endpoint":"/v1/models/{routine_model}",
            "model":self.model,"request_attempts_started":1,"http_responses_received":0,
            "generation_requests":0,"automatic_retries":0,"redirects_followed":0,
            "inference_reservations_created":0,"evaluation_budget_modified":false,
            "authentication_verified":false,"model_metadata_access_verified":false,
            "inference_permission_verified":false,"intended_project_verified":false,
            "stage":"openai_transport","result":"transport_failed"});
        let mut response = match result {
            Ok(r) => r,
            Err(_) => return report,
        };
        let status = response.status().as_u16();
        report["http_responses_received"] = json!(1);
        report["http_status"] = json!(status);
        report["stage"] = json!("openai_response");
        report["result"] = json!(match status {
            200 => "metadata_response_pending_validation",
            401 => "authentication_rejected",
            403 => "access_denied",
            404 => "model_unavailable_or_inaccessible",
            429 => "rate_limited",
            _ => "http_error",
        });
        let mut body = vec![];
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) if body.len() + chunk.len() <= 16_384 => {
                    body.extend_from_slice(&chunk)
                }
                Ok(None) => break,
                _ => {
                    report["response_body_validated"] = json!(false);
                    return report;
                }
            }
        }
        if status == 200 {
            let value: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let valid = value["object"] == "model" && value["id"] == self.model;
            report["authentication_verified"] = json!(valid);
            report["model_metadata_access_verified"] = json!(valid);
            report["result"] = json!(if valid {
                "authenticated_model_metadata_access"
            } else {
                "invalid_metadata_response"
            });
        } else {
            report["safe_error_code"] = json!(safe_code(&body));
        }
        // Never return raw bodies, credentials, identity headers, model owners or request headers.
        report
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_format_checks_never_echo_credentials() {
        for value in [
            "",
            " sk-fake",
            "sk-fake\n",
            "Bearer sk-fake",
            "'sk-fake'",
            "sk-faké",
        ] {
            assert!(!key_format_valid(value));
            assert_eq!(key_diagnostics(Some(value))["basic_format_valid"], false);
        }
        assert!(key_format_valid(&format!(
            "sk-proj-{}",
            "fake-not-a-real-key"
        )));
        assert!(
            !key_diagnostics(Some("sk-fake-private"))
                .to_string()
                .contains("sk-fake-private")
        );
    }
    #[test]
    fn only_fixed_error_codes_survive_sanitization() {
        assert_eq!(
            safe_code(br#"{"error":{"code":"invalid_api_key","message":"private credential"}}"#),
            "invalid_api_key"
        );
        assert_eq!(
            safe_code(br#"{"error":{"code":"sk-private-code","message":"private credential"}}"#),
            "unclassified"
        );
    }
    #[tokio::test]
    async fn authentication_probe_requires_separate_authorization() {
        assert!(matches!(
            OpenAi::authentication_probe(false).await,
            Err(Error::Denied)
        ));
    }
    #[tokio::test]
    #[ignore = "requires local loopback sockets; no real API calls"]
    async fn probe_makes_one_get_and_never_exposes_private_responses() {
        for (status, body) in [
            (
                401,
                json!({"error":{"code":"invalid_api_key","message":"sk-private-upstream"}}),
            ),
            (
                200,
                json!({"id":"gpt-6-luna","object":"model","owned_by":"private-organization"}),
            ),
            (302, json!({"private":"sk-private-upstream"})),
            (
                403,
                json!({"error":{"code":"insufficient_permissions","message":"private-scope"}}),
            ),
        ] {
            let (endpoint, server) = crate::wire_tests::fixture(status, body.to_string()).await;
            let mut provider =
                OpenAi::new("sk-fake-probe-secret".into(), "gpt-6-luna".into()).unwrap();
            provider.endpoint = Some(endpoint);
            let report = provider.probe().await;
            assert_eq!(report["http_status"], status);
            assert_eq!(report["request_attempts_started"], 1);
            assert_eq!(report["generation_requests"], 0);
            assert_eq!(report["authentication_verified"], status == 200);
            for private in [
                "sk-fake-probe-secret",
                "sk-private-upstream",
                "private-organization",
                "private-scope",
            ] {
                assert!(!report.to_string().contains(private));
            }
            let request = server.await.unwrap();
            let (headers, body) = request.split_once("\r\n\r\n").unwrap();
            assert!(headers.starts_with("GET "));
            assert!(body.is_empty());
            assert!(headers.contains("Bearer sk-fake-probe-secret"));
            assert!(!headers.to_lowercase().contains("openai-organization:"));
            assert!(!headers.to_lowercase().contains("openai-project:"));
        }
    }
}
