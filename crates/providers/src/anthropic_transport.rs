//! One transport attempt. Never persists raw bodies, error prose or authentication headers.
use super::*;

pub(super) fn body_evidence(d: &mut ProviderDiagnostic, bytes: &[u8]) {
    let Ok(v) = serde_json::from_slice::<Value>(bytes) else {
        return;
    };
    if d.request_id.is_none() {
        d.request_id = v["request_id"]
            .as_str()
            .and_then(|s| ProviderDiagnostic::identifier(s, "req_"));
    }
    d.message_id = v["id"]
        .as_str()
        .and_then(|s| ProviderDiagnostic::identifier(s, "msg_"));
    d.reported_model = v["model"]
        .as_str()
        .or_else(|| {
            if d.operation == ProviderOperation::Metadata {
                v["id"].as_str()
            } else {
                None
            }
        })
        .filter(|s| {
            !s.contains("sk-")
                && s.starts_with("claude-")
                && s.len() <= 96
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        .map(str::to_owned);
    d.error_type = v["error"]["type"]
        .as_str()
        .filter(|s| ProviderDiagnostic::safe_error_type(s))
        .map(str::to_owned);
    if v["usage"].is_object() {
        let u = &v["usage"];
        d.reported_usage = Some(ReportedUsage {
            input_tokens: u["input_tokens"].as_u64(),
            output_tokens: u["output_tokens"].as_u64(),
            cache_read_tokens: u["cache_read_input_tokens"].as_u64(),
            cache_write_tokens: u["cache_creation_input_tokens"].as_u64(),
            cache_write_5m_tokens: u["cache_creation"]["ephemeral_5m_input_tokens"].as_u64(),
            cache_write_1h_tokens: u["cache_creation"]["ephemeral_1h_input_tokens"].as_u64(),
        });
    }
}
fn outcome(result: Result<Vec<u8>>, mut d: ProviderDiagnostic) -> Invocation<Vec<u8>> {
    d.observe_now();
    Invocation {
        result,
        usage: None,
        diagnostic: Some(d),
    }
}
impl Anthropic {
    pub(super) async fn transport_observed(
        &self,
        metadata: bool,
        body: Option<&Value>,
    ) -> Invocation<Vec<u8>> {
        let operation = if metadata {
            ProviderOperation::Metadata
        } else {
            ProviderOperation::Generation
        };
        let mut d = ProviderDiagnostic::new(operation, ProviderStage::RequestSend);
        let url = if metadata {
            format!("https://api.anthropic.com/v1/models/{}", self.policy.model)
        } else {
            "https://api.anthropic.com/v1/messages".into()
        };
        #[cfg(test)]
        let url = self.endpoint.as_ref().unwrap_or(&url);
        let mut request = if metadata {
            self.client.get(url)
        } else {
            self.client.post(url)
        }
        .bearer_auth(self.key.as_str())
        .header("anthropic-version", "2026-10-21")
        .header("anthropic-workspace-id", &self.workspace);
        if let Some(value) = body {
            request = request.json(value);
        }
        d.dispatch_intended = true;
        let mut response = match request.send().await {
            Ok(v) => v,
            Err(e) => {
                d.failure = Some(if e.is_timeout() {
                    ProviderFailure::Timeout
                } else if e.is_connect() {
                    ProviderFailure::Connection
                } else {
                    ProviderFailure::Transport
                });
                return outcome(
                    Err(if e.is_timeout() {
                        Error::Timeout
                    } else {
                        Error::ProviderTransport
                    }),
                    d,
                );
            }
        };
        let status = response.status();
        d.headers_received(status.as_u16());
        d.request_id = response
            .headers()
            .get("request-id")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| ProviderDiagnostic::identifier(s, "req_"));
        d.stage = ProviderStage::ResponseBody;
        // Error responses need no large body; status/header evidence survives all body failures.
        let bound = if metadata || !status.is_success() {
            16384
        } else {
            128000
        };
        let mut bytes = vec![];
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    if bytes.len() + chunk.len() > bound {
                        d.failure = Some(ProviderFailure::ResponseTooLarge);
                        return outcome(Err(Error::Provider), d);
                    }
                    bytes.extend_from_slice(&chunk);
                }
                Ok(None) => break,
                Err(e) => {
                    body_evidence(&mut d, &bytes);
                    d.failure = Some(if e.is_timeout() {
                        ProviderFailure::Timeout
                    } else {
                        ProviderFailure::InterruptedResponse
                    });
                    return outcome(
                        Err(if e.is_timeout() {
                            Error::Timeout
                        } else {
                            Error::ProviderTransport
                        }),
                        d,
                    );
                }
            }
        }
        d.response_complete = true;
        body_evidence(&mut d, &bytes);
        if !status.is_success() {
            d.stage = ProviderStage::ResponseHeaders;
            d.failure = Some(ProviderFailure::HttpStatus);
            return outcome(Err(Error::ProviderHttp(status.as_u16())), d);
        }
        d.stage = ProviderStage::ResponseParse;
        outcome(Ok(bytes), d)
    }
}
