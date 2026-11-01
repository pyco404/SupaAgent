use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
fn provider() -> Anthropic {
    Anthropic::new(
        "sk-ant-fixture-secret-only".into(),
        "wrkspc_fixture".into(),
        "claude-haiku-5-5".into(),
        uuid::Uuid::new_v4(),
        5000,
        1024,
    )
    .unwrap()
}
fn input() -> LiveWorkInput {
    LiveWorkInput {
        request: WorkRequest {
            objective: "Compare supplied documents".into(),
            material: "Replicas reduce downtime.".into(),
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
fn report() -> Value {
    json!({"interpretation":"Compare supplied evidence","summary":"The supplied document states replicas reduce downtime; no measured recovery times are provided.","status":"supported","tools":input().approved_tools,"findings":[{"observation":"Replicas reduce downtime.","interpretation":"A supplied claim, not independently verified","evidence":[{"source":"material","line":1,"quote":"Replicas reduce downtime."}]}],"totals":[],"proposed_steps":[],"missing_information":[],"memory_ids":[]})
}
fn envelope(output: Value) -> Value {
    json!({"type":"message","role":"assistant","model":"claude-haiku-5-5","stop_reason":"end_turn","content":[{"type":"text","text":output.to_string()}],"usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":20,"cache_creation_input_tokens":10,"cache_creation":{"ephemeral_5m_input_tokens":10,"ephemeral_1h_input_tokens":0},"service_tier":"standard","inference_geo":"global","speed":"standard"}})
}
#[test]
fn independent_prices_and_nonoverlapping_cache_usage() {
    let p = provider();
    let u = usage(envelope(report()).to_string().as_bytes()).unwrap();
    assert_eq!(u.input_tokens, 130);
    assert_eq!(u.cached_input_tokens, 20);
    assert_eq!(u.cache_write_tokens, 10);
    assert_eq!(p.policy.cost(u).unwrap(), 37);
    assert_eq!(p.policy.reservation().unwrap(), 3536);
    assert_eq!(
        rates("claude-sonnet-5-5").unwrap(),
        [2_000_000, 100_000, 2_500_000, 10_000_000]
    );
    assert!(rates("unverified-model").is_err());
    assert!(
        Anthropic::new(
            "sk-openai-fixture".into(),
            "wrkspc_fixture".into(),
            "claude-haiku-5-5".into(),
            uuid::Uuid::new_v4(),
            5000,
            1024
        )
        .is_err()
    );
    assert!(
        Anthropic::new(
            "sk-ant-fixture-secret-only".into(),
            "project_not_workspace".into(),
            "claude-haiku-5-5".into(),
            uuid::Uuid::new_v4(),
            5000,
            1024
        )
        .is_err()
    );
}
#[test]
fn unknown_pricing_missing_usage_and_one_hour_cache_fail_closed() {
    for field in ["input_tokens", "output_tokens"] {
        let mut value = envelope(report());
        value["usage"].as_object_mut().unwrap().remove(field);
        assert!(usage(value.to_string().as_bytes()).is_err());
    }
    for (field, value) in [
        ("input_tokens", json!(-1)),
        ("cache_read_input_tokens", json!("20")),
        (
            "cache_creation",
            json!({"ephemeral_1h_input_tokens":10,"ephemeral_5m_input_tokens":0}),
        ),
        ("service_tier", json!("priority")),
        ("speed", json!("fast")),
        ("inference_geo", json!("us")),
        ("server_tool_use", json!({"web_search_requests":1})),
    ] {
        let mut response = envelope(report());
        response["usage"][field] = value;
        assert!(usage(response.to_string().as_bytes()).is_err(), "{field}");
    }
}
#[tokio::test]
#[ignore = "local mock HTTP only; never calls Anthropic"]
async fn messages_wire_schema_evidence_and_failure_usage() {
    for mode in 0..9 {
        let mut output = report();
        let mut response = envelope(output.clone());
        match mode {
            1 => {
                output["findings"][0]["observation"] = json!("Unsubstantiated recovery time");
                response = envelope(output);
            }
            2 => {
                output["tools"] = json!(["transfer"]);
                response = envelope(output);
            }
            3 => {
                output["extra"] = json!(true);
                response = envelope(output);
            }
            4 => response["stop_reason"] = json!("max_tokens"),
            5 => response["stop_reason"] = json!("refusal"),
            6 => response["model"] = json!("unapproved-model"),
            7 => response["content"][0]["text"] = json!("invalid JSON"),
            8 => response["content"][0]["type"] = json!("tool_use"),
            _ => {}
        }
        let (url, server) = crate::wire_tests::fixture(200, response.to_string()).await;
        let mut p = provider();
        p.endpoint = Some(url);
        let invocation = p.work(&input()).await;
        assert_eq!(invocation.usage.is_some(), mode != 6);
        assert_eq!(invocation.result.is_ok(), mode == 0);
        let request = server.await.unwrap();
        let (headers, body) = request.split_once("\r\n\r\n").unwrap();
        assert!(
            headers
                .to_lowercase()
                .contains("anthropic-workspace-id: wrkspc_fixture")
        );
        assert!(headers.contains("Bearer sk-ant-fixture-secret-only"));
        let b: Value = serde_json::from_str(body).unwrap();
        assert!(!body.contains("sk-ant-fixture-secret-only"));
        assert!(!body.contains("wrkspc_fixture"));
        assert_eq!(b["service_tier"], "standard_only");
        assert_eq!(b["max_tokens"], 1024);
        assert_eq!(b["thinking"]["type"], "disabled");
        assert_eq!(b["output_config"]["format"]["type"], "json_schema");
        assert!(b.get("tools").is_none());
        assert!(b.get("cache_control").is_none());
    }
}
#[tokio::test]
#[ignore = "local mock HTTP only; never calls Anthropic"]
async fn metadata_and_http_errors_have_no_generation_or_secret_leak() {
    for valid in [false, true] {
        let (url,server)=crate::wire_tests::fixture(200,json!({"type":"model","id":"claude-haiku-5-5","capabilities":{"structured_outputs":{"supported":valid}},"max_input_tokens":100000,"max_tokens":4096}).to_string()).await;
        let mut p = provider();
        p.endpoint = Some(url);
        assert_eq!(p.verify().await.is_ok(), valid);
        assert!(server.await.unwrap().starts_with("GET "));
    }
    for code in [401, 403, 429, 500] {
        let (url, server) =
            crate::wire_tests::fixture(code, "private upstream detail excluded".into()).await;
        let mut p = provider();
        p.endpoint = Some(url);
        let result = p.work(&input()).await;
        assert!(result.usage.is_none());
        assert_eq!(
            result.result.unwrap_err().to_string(),
            format!("provider_http_{code}")
        );
        server.await.unwrap();
    }
}
#[tokio::test]
#[ignore = "local mock HTTP only; never calls Anthropic"]
async fn timeout_is_single_attempt_and_oversized_input_never_dispatches() {
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = [0; 32768];
        let _ = socket.read(&mut bytes).await;
        observed.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    let mut p = provider();
    p.endpoint = Some(format!("http://{address}"));
    p.client = reqwest::Client::builder()
        .timeout(Duration::from_millis(25))
        .retry(reqwest::retry::never())
        .no_proxy()
        .build()
        .unwrap();
    let result = p.work(&input()).await;
    assert!(matches!(result.result, Err(Error::ProviderTransport)));
    assert!(result.usage.is_none());
    server.await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let oversized = p
        .request("test", json!({"text":"x".repeat(16001)}), json!({}))
        .await;
    assert!(matches!(oversized.result, Err(Error::ProviderRequest)));
}

#[tokio::test]
#[ignore = "local mock HTTP only; never calls Anthropic"]
async fn bounded_goal_reasoning_cannot_expand_tools_or_invent_memories() {
    for allowed in [false, true] {
        let mut proposal =
            json!({"summary":"Inspect supplied memories only","memory_ids":[],"actions":[]});
        if !allowed {
            proposal["memory_ids"] = json!([uuid::Uuid::new_v4()]);
        }
        let (url, server) = crate::wire_tests::fixture(200, envelope(proposal).to_string()).await;
        let mut p = provider();
        p.endpoint = Some(url);
        let result = p.reason("Review memory", &[]).await;
        assert_eq!(result.result.is_ok(), allowed);
        assert!(result.usage.is_some());
        server.await.unwrap();
    }
}

#[tokio::test]
async fn social_empty_context_abstains_without_transport_or_usage() {
    let context = SocialContext {
        personality: Personality::default(),
        candidates: vec![],
        memories: vec![],
        previous_drafts: vec![],
    };
    let result = provider().social(&context).await;
    assert!(result.usage.is_none());
    assert_eq!(result.result.unwrap().reason, SocialReason::NothingNew);
}

#[tokio::test]
#[ignore = "local mock HTTP only; never calls either provider"]
async fn social_transports_share_personality_evidence_and_rejection_contract() {
    let config: Value = serde_json::from_str(include_str!("../../../config/supa.json")).unwrap();
    let event: Event = serde_json::from_value(json!({"id":"e4650e8b-78db-42a5-82db-187869a1eb77","cycle_id":null,"kind":"cycle","code":"completed","simulated":false,"created_at":"2026-10-27T00:00:00Z"})).unwrap();
    let memory: Memory = serde_json::from_value(json!({"id":"8e00bb1b-1e56-4b70-a6fb-fd01ee220afc","cycle_id":event.id,"kind":"verified_observation","lesson":"A completed cycle is evidence, not a commercial achievement.","source_event_ids":[event.id],"created_at":"2026-10-27T00:00:00Z"})).unwrap();
    let context = social_context(
        serde_json::from_value(config["personality"].clone()).unwrap(),
        &[event],
        &[memory],
        &[],
    );
    assert_eq!(context.memories.len(), 1);
    let candidate = &context.candidates[0];
    let valid = json!({"choice":"draft","reason":"worth_sharing","style":"reflection","text":"A completed reasoning cycle. Confidence may take a seat; evidence gets the microphone.","event_ids":[candidate.event.id],"memory_ids":[context.memories[0].id],"claims":[{"event_id":candidate.event.id,"fact":candidate.canonical_fact,"wording":candidate.public_fact}]});
    for mode in 0..5 {
        let mut output = valid.clone();
        match mode {
            1 => output["memory_ids"] = json!([uuid::Uuid::nil()]),
            2 => output["claims"][0]["fact"] = json!("An unsupported accomplishment"),
            3 => output["text"] = json!("A completed reasoning cycle. Draft for human review."),
            4 => {
                output =
                    serde_json::to_value(SocialDecision::abstain(SocialReason::Conserve)).unwrap()
            }
            _ => {}
        }
        let (url, server) =
            crate::wire_tests::fixture(200, envelope(output.clone()).to_string()).await;
        let mut claude = provider();
        claude.endpoint = Some(url);
        let result = claude.social(&context).await;
        assert_eq!(result.result.is_ok(), mode == 0 || mode == 4);
        assert!(result.usage.is_some());
        let request = server.await.unwrap();
        let (_, body) = request.split_once("\r\n\r\n").unwrap();
        let claude_body: Value = serde_json::from_str(body).unwrap();
        let response = json!({"status":"completed","usage":{"input_tokens":100,"output_tokens":50},"output":[{"type":"message","content":[{"type":"output_text","text":output.to_string()}]}]});
        let (url, server) = crate::wire_tests::fixture(200, response.to_string()).await;
        let mut openai =
            OpenAi::new("sk-fake-social-comparison".into(), "gpt-6-luna".into()).unwrap();
        openai.endpoint = Some(url);
        let result = controlled::ControlledProvider::social(&openai, &context).await;
        assert_eq!(result.result.is_ok(), mode == 0 || mode == 4);
        assert!(result.usage.is_some());
        let request = server.await.unwrap();
        let (_, body) = request.split_once("\r\n\r\n").unwrap();
        let openai_body: Value = serde_json::from_str(body).unwrap();
        assert_eq!(claude_body["system"], openai_body["instructions"]);
        assert_eq!(claude_body["messages"][0]["content"], openai_body["input"]);
        assert_eq!(
            claude_body["output_config"]["format"]["schema"],
            openai_body["text"]["format"]["schema"]
        );
        let input: Value = serde_json::from_str(openai_body["input"].as_str().unwrap()).unwrap();
        assert_eq!(input["personality"], config["personality"]);
        assert_eq!(input["memories"][0]["id"], json!(context.memories[0].id));
        assert!(claude_body.get("tools").is_none());
    }
}
