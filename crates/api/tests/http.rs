use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use std::sync::Arc;
use supa_api::{AppState, router};
use supa_persistence::Store;
use supa_providers::Offline;
use supa_runtime::Runtime;
use tower::ServiceExt;

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn authenticated_http_workflow_and_safe_errors() {
    let s = Store::connect(
        &std::env::var("TEST_DATABASE_URL").expect("disposable test database required"),
    )
    .await
    .unwrap();
    s.migrate().await.unwrap();
    let token = "test-token-with-at-least-32-characters";
    let app = router(
        AppState::new(s.clone(), Runtime::new(s, Arc::new(Offline)), token.into())
            .unwrap()
            .with_operator(uuid::Uuid::from_u128(1))
            .unwrap(),
    );
    let ready = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ready.status(), StatusCode::OK);
    let body = to_bytes(ready.into_body(), 1024).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        serde_json::json!({"status":"ready"})
    );
    let private_health = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(private_health.status(), StatusCode::UNAUTHORIZED);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/agents")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Supa HTTP"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 20000).await.unwrap()).unwrap();
    assert_eq!(value["lifecycle"], "created");
    let id = value["id"].as_str().unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/agents/{id}/financials"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let f: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 20000).await.unwrap()).unwrap();
    assert_eq!(f["simulated"], true);
    assert_eq!(f["balance_microusd"], 0);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/agents/{id}/social"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let social: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 20000).await.unwrap()).unwrap();
    assert_eq!(social, serde_json::json!([]));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/agents")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"sk-sensitive","unknown":"sk-private"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = String::from_utf8(
        to_bytes(response.into_body(), 20000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(!body.contains("sk-"));
}

#[tokio::test]
#[ignore = "requires disposable TEST_DATABASE_URL; RPC is mocked"]
async fn wallet_routes_authenticate_hide_keys_and_reject_signing() {
    use async_trait::async_trait;
    use supa_domain::*;
    use supa_providers::{
        solana::{RpcMethod, SolanaRpc},
        vault::KeyVault,
    };
    use supa_runtime::wallet::WalletService;
    use uuid::Uuid;
    struct NoNetwork;
    #[async_trait]
    impl SolanaRpc for NoNetwork {
        async fn read(&self, _: RpcMethod, _: serde_json::Value) -> Result<serde_json::Value> {
            Err(Error::Rpc)
        }
    }
    let s = Store::connect(&std::env::var("TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    s.migrate().await.unwrap();
    let a = Agent::new(
        "Wallet HTTP".into(),
        Personality::default(),
        CycleConfig::default(),
        Default::default(),
    )
    .unwrap();
    s.create(&a).await.unwrap();
    let token = "wallet-test-token-with-32-characters";
    let mut state = AppState::new(
        s.clone(),
        Runtime::new(s.clone(), Arc::new(Offline)),
        token.into(),
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let operator = Uuid::new_v4();
    s.establish_owner(a.id, operator).await.unwrap();
    state = state.with_operator(operator).unwrap();
    state.wallets = Some(WalletService::new(
        s.clone(),
        KeyVault::open(&dir.path().join("vault"), [6; 32]).unwrap(),
        Arc::new(NoNetwork),
        operator,
    ));
    let mut read_state = state.clone();
    let app = router(state);
    let request = |method: &str, path: &str, authenticated: bool, body: &str| {
        let mut b = Request::builder()
            .method(method)
            .uri(format!("/agents/{}/wallet{}", a.id, path));
        if authenticated {
            b = b.header("authorization", format!("Bearer {token}"));
        }
        b.header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap()
    };
    assert_eq!(
        app.clone()
            .oneshot(request("POST", "", false, ""))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = app
        .clone()
        .oneshot(request("POST", "", true, ""))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let public = String::from_utf8(
        to_bytes(response.into_body(), 20000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(!public.contains("ciphertext"));
    assert!(!public.contains("nonce"));
    assert!(!public.contains("private_key"));
    let w: WalletIdentity = serde_json::from_str(&public).unwrap();
    assert_eq!(w.owner_id, operator);
    for route in [
        "",
        "/balances",
        "/payments",
        "/payment-checks",
        "/treasury",
        "/policy",
        "/reservations",
        "/audit",
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request("GET", route, true, ""))
                .await
                .unwrap()
                .status(),
            StatusCode::OK,
            "{route}"
        );
    }
    assert_eq!(
        app.clone()
            .oneshot(request("POST", "/recover", true, ""))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone()
            .oneshot(request("POST", "/sign", true, ""))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(request(
                "POST",
                "/payments",
                true,
                r#"{"signature":"sk-private"}"#
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        app.clone()
            .oneshot(request("POST", "/sync", true, ""))
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert!(
        s.wallet_history(a.id, "audit")
            .await
            .unwrap()
            .iter()
            .any(|v| v["operation"] == "sign")
    );
    // Saved observations remain available after removing both the service and its vault files.
    drop(app);
    drop(dir);
    read_state.wallets = None;
    read_state.wallet_read_operator = Some(operator);
    let reader = router(read_state.clone());
    for route in ["", "/balances", "/payments", "/payment-checks", "/treasury"] {
        let response = reader
            .clone()
            .oneshot(request("GET", route, true, ""))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "read-only scope {route}");
        let body = String::from_utf8(
            to_bytes(response.into_body(), 20000)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(
            !body.contains("ciphertext")
                && !body.contains("private_key")
                && !body.contains("nonce")
        );
        assert_eq!(
            reader
                .clone()
                .oneshot(request("GET", route, false, ""))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    for route in [
        "",
        "/recover",
        "/sync",
        "/payments",
        "/policy",
        "/reservations",
        "/sign",
    ] {
        assert_eq!(
            reader
                .clone()
                .oneshot(request("POST", route, true, "{}"))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN,
            "write scope {route}"
        );
    }
    let release = format!("/reservations/{}/release", Uuid::new_v4());
    assert_eq!(
        reader
            .oneshot(request("POST", &release, true, "{}"))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    read_state.wallet_read_operator = Some(Uuid::new_v4());
    let wrong_reader = router(read_state);
    for route in ["", "/balances", "/payments", "/payment-checks", "/treasury"] {
        assert_eq!(
            wrong_reader
                .clone()
                .oneshot(request("GET", route, true, ""))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN,
            "wrong owner {route}"
        );
    }
}
