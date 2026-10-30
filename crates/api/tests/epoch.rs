//! Isolated databases only. These fixtures are never adopted by the prepared candidate.
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::sync::Arc;
use supa_api::{AppState, router};
use supa_domain::*;
use supa_persistence::{Role, Store};
use supa_providers::Offline;
use supa_runtime::Runtime;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires dedicated TEST_DATABASE_URL; no external calls"]
async fn prepared_epoch_is_identified_read_only_and_never_adopts_owners_or_consent() {
    let base = std::env::var("TEST_DATABASE_URL").unwrap();
    assert!(base.contains("_test"));
    let admin = Store::connect(&base).await.unwrap();
    let schema = format!("supa_test_epoch_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {schema} TEMPLATE template0"))
        .execute(admin.pool())
        .await
        .unwrap();
    let url = sqlx::ConnectOptions::to_url_lossy(
        &base
            .parse::<sqlx::postgres::PgConnectOptions>()
            .unwrap()
            .database(&schema),
    )
    .to_string();
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    assert!(store.application_epoch().await.unwrap().is_none());
    // Explicit test-only ownership procedure. No implicit claim on first login.
    let agent = Agent::new(
        "New fixture, not historical".into(),
        Personality::default(),
        CycleConfig::default(),
        Default::default(),
    )
    .unwrap();
    store.create(&agent).await.unwrap();
    let owner = Uuid::new_v4();
    let outsider = Uuid::new_v4();
    assert!(
        store
            .require_access(agent.id, owner, Role::Owner)
            .await
            .is_err()
    );
    store.establish_owner(agent.id, owner).await.unwrap();
    assert!(store.establish_owner(agent.id, outsider).await.is_err());
    assert!(
        store
            .require_access(agent.id, outsider, Role::Viewer)
            .await
            .is_err()
    );
    assert!(store.get(Uuid::new_v4()).await.is_err()); // Missing historical identity stays missing.
    assert!(!store.schedule_status(agent.id).await.unwrap().enabled);
    let id = Uuid::new_v4();
    let namespace = Uuid::new_v4();
    sqlx::query("INSERT INTO application_epoch(id,accounting_namespace,label,kind,state,historical_database) VALUES($1,$2,'Isolated epoch fixture','development','prepared','unrecovered')").bind(id).bind(namespace).execute(store.pool()).await.unwrap();
    let frozen = Store::connect(&format!(
        "{url}&options=-cdefault_transaction_read_only%3Don"
    ))
    .await
    .unwrap();
    assert_eq!(frozen.require_epoch_preview(id).await.unwrap().id, id);
    assert!(frozen.require_epoch_preview(Uuid::new_v4()).await.is_err());
    assert!(store.require_epoch_preview(id).await.is_err()); // Correct ID alone is insufficient.
    assert!(
        frozen
            .create_owned(
                &Agent::new(
                    "Blocked".into(),
                    Personality::default(),
                    CycleConfig::default(),
                    Default::default()
                )
                .unwrap(),
                owner
            )
            .await
            .is_err()
    );
    let token = "fixture-epoch-credential-0000000000000000";
    let app = router(
        AppState::new(
            store.clone(),
            Runtime::new(store.clone(), Arc::new(Offline)),
            token.into(),
        )
        .unwrap()
        .with_operator(owner)
        .unwrap(),
    );
    let health = app
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
    assert_eq!(health.status(), StatusCode::OK);
    let v: Value =
        serde_json::from_slice(&to_bytes(health.into_body(), 10000).await.unwrap()).unwrap();
    assert_eq!(v["epoch"]["id"], json!(id));
    assert_eq!(v["epoch"]["state"], "prepared");
    assert_eq!(v["historical_database"], "unrecovered");
    for path in [
        "/agents".to_string(),
        format!("/agents/{}/access", agent.id),
        format!("/agents/{}/schedule", agent.id),
        format!("/agents/{}/live", agent.id),
        "/autonomy".into(),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
    }
    let r = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM schedule_authorizations),(SELECT count(*) FROM live_budgets),(SELECT count(*) FROM live_sessions)").fetch_one(store.pool()).await.unwrap();
    assert_eq!(counts, (0, 0, 0));
    assert_eq!(
        store.get(agent.id).await.unwrap().lifecycle,
        Lifecycle::Created
    );
    frozen.pool().close().await;
    store.pool().close().await;
    sqlx::query(&format!("DROP DATABASE {schema} WITH (FORCE)"))
        .execute(admin.pool())
        .await
        .unwrap();
}
