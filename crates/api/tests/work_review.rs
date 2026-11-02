//! Reviews only restored disposable copies. Never loads provider/owner credentials.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use supa_api::{AppState, OperatorCredential, router};
use supa_domain::*;
use supa_persistence::{Role, Store};
use supa_providers::Offline;
use supa_runtime::Runtime;
use tower::ServiceExt;
use uuid::{Uuid, uuid};
const AGENT: Uuid = uuid!("82c254ff-4659-4404-94c4-c497565567e6");
const SESSION: Uuid = uuid!("41316d3e-40b8-4b7d-8605-83db7aab36b8");
const ACTOR: Uuid = uuid!("acede749-746b-4541-b178-0085434dcd39");
const HASH: &str = "01fcfc0576fb7d8cebfc09666c060eb1bff5c85a08b2682d2b17842cfe5abc42";
const OWNER: &str = "fixture-review-owner-credential-000000000";
const PUBLIC: &str = "fixture-review-public-credential-00000000";
fn path() -> String {
    format!("/agents/{AGENT}/live-work/{SESSION}/review")
}
async fn unmigrated_clone() -> Store {
    let base = std::env::var("TEST_DATABASE_URL").unwrap();
    let options: sqlx::postgres::PgConnectOptions = base.parse().unwrap();
    assert_eq!(options.get_host(), "localhost");
    assert_eq!(options.get_port(), 55680);
    assert_eq!(options.get_database(), Some("supa_m10_test"));
    let admin = Store::connect(&base).await.unwrap();
    let name = format!("supa_review_{}_test", Uuid::new_v4().simple());
    sqlx::query(&format!(
        "CREATE DATABASE {name} TEMPLATE supa_review_seed_test"
    ))
    .execute(admin.pool())
    .await
    .unwrap();
    let url = sqlx::ConnectOptions::to_url_lossy(&options.database(&name)).to_string();
    Store::connect(&url).await.unwrap()
}
async fn setup() -> Store {
    let store = unmigrated_clone().await;
    store.migrate().await.unwrap();
    assert!(store.application_epoch().await.unwrap().is_none());
    store
}

#[tokio::test]
#[ignore = "isolated restored test database only"]
async fn additive_migration_and_transaction_rollback_preserve_source_tables() {
    let store = unmigrated_clone().await;
    let before = snapshot(&store)
        .await
        .into_iter()
        .filter(|(t, _)| t != "_sqlx_migrations")
        .collect::<Vec<_>>();
    let version: i64 = sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(version, 12);
    let mut tx = store.pool().begin().await.unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/0013_work_evaluation_reviews.sql"
    ))
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    let exists: bool =
        sqlx::query_scalar("SELECT to_regclass('work_evaluation_reviews') IS NOT NULL")
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert!(!exists);
    store.migrate().await.unwrap();
    assert_eq!(
        snapshot(&store)
            .await
            .into_iter()
            .filter(|(t, _)| t != "_sqlx_migrations")
            .collect::<Vec<_>>(),
        before
    );
    let matches: bool = sqlx::query_scalar(
        "SELECT success AND checksum=sha384($1) FROM _sqlx_migrations WHERE version=13",
    )
    .bind(include_bytes!("../../../migrations/0013_work_evaluation_reviews.sql").as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(matches);
    // Read and write access remain revoked from the public database role.
    let acl: Option<String> = sqlx::query_scalar(
        "SELECT relacl::text FROM pg_class WHERE oid='work_evaluation_reviews'::regclass",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(
        acl.unwrap()
            .split(',')
            .all(|entry| !entry.trim_start_matches('{').starts_with('='))
    );
}
fn app(store: &Store, extra: Vec<OperatorCredential>, expired: bool) -> Router {
    let mut credentials = vec![OperatorCredential {
        id: ACTOR,
        role: Role::Owner,
        token: OWNER.into(),
    }];
    credentials.extend(extra);
    let mut state = AppState::new(
        store.clone(),
        Runtime::new(store.clone(), Arc::new(Offline)),
        PUBLIC.into(),
    )
    .unwrap()
    .with_credentials(credentials)
    .unwrap();
    if expired {
        state = state
            .with_credential_deadlines(BTreeMap::from([(ACTOR, 1)]))
            .unwrap();
    }
    router(state) // No provider and no paid-inference enablement.
}
fn confirmation(decision: &str) -> Value {
    let now = Utc::now();
    json!({"request_id":Uuid::new_v4(),"decision":decision,"report_sha256":HASH,"confirm_review":true,"consented_at":now,"expires_at":now+Duration::minutes(5)})
}
async fn call(
    app: &Router,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let body = if let Some(body) = body {
        request = request
            .method("POST")
            .header("content-type", "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    let response = app
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    if status == StatusCode::OK {
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    let bytes = to_bytes(response.into_body(), 100000).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
async fn snapshot(store: &Store) -> Vec<(String, Value)> {
    let tables:Vec<String>=sqlx::query_scalar("SELECT tablename FROM pg_tables WHERE schemaname='public' AND tablename<>'work_evaluation_reviews' ORDER BY tablename").fetch_all(store.pool()).await.unwrap();
    let mut result = vec![];
    for table in tables {
        assert!(table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        let value:Value=sqlx::query_scalar(&format!("SELECT coalesce(jsonb_agg(to_jsonb(x) ORDER BY to_jsonb(x)::text),'[]') FROM public.\"{table}\" x")).fetch_one(store.pool()).await.unwrap();
        result.push((table, value));
    }
    result
}
#[tokio::test]
#[ignore = "requires isolated restored supa_review_seed_test; no provider calls"]
async fn acceptance_preserves_every_generation_and_financial_record_and_survives_restart() {
    let store = setup().await;
    let app = app(&store, vec![], false);
    let before = snapshot(&store).await;
    let (status, context) = call(&app, &path(), Some(OWNER), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(context["report_sha256"], HASH);
    assert!(context["review"].is_null());
    let input = confirmation("accept_evaluation_only");
    let (status, event) = call(&app, &path(), Some(OWNER), Some(input.clone())).await;
    assert_eq!(status, StatusCode::OK, "{event}");
    assert_eq!(event["reviewer_principal_id"], ACTOR.to_string());
    assert_eq!(event["decision"], "accept_evaluation_only");
    assert_eq!(snapshot(&store).await, before); // Includes session, usage, cost, audits, job, agent and all ledgers.
    assert_eq!(
        call(&app, &path(), Some(OWNER), Some(input)).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        call(
            &app,
            &path(),
            Some(OWNER),
            Some(confirmation("reject_evaluation_only"))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let url = sqlx::ConnectOptions::to_url_lossy(&*store.pool().connect_options()).to_string();
    let reopened = Store::connect(&url).await.unwrap();
    let context = reopened
        .work_review_context(AGENT, SESSION, ACTOR)
        .await
        .unwrap();
    assert_eq!(context["evaluation_status"], "accept_evaluation_only");
    assert_eq!(context["session_status"], "awaiting_approval");
    assert_eq!(context["review"]["request_id"], event["request_id"]);
    assert_eq!(context["review_effects"]["financial_mutation"], false);
    assert_eq!(context["review_effects"]["earned_revenue_atoms"], 0);
    assert!(matches!(
        reopened
            .approve_live_tools(AGENT, SESSION, ACTOR, true)
            .await,
        Err(Error::Stale)
    ));
    assert_eq!(snapshot(&store).await, before);
    // An existing decision must not silently attest to later tampered clone output.
    sqlx::query("UPDATE live_sessions SET data=jsonb_set(data,'{work,report,summary}','\"changed after clone review\"') WHERE id=$1")
        .bind(SESSION).execute(store.pool()).await.unwrap();
    let changed = reopened
        .work_review_context(AGENT, SESSION, ACTOR)
        .await
        .unwrap();
    assert_eq!(
        changed["evaluation_status"],
        "review_requires_reconciliation"
    );
    assert_eq!(changed["review_matches_current_report"], false);
}
#[tokio::test]
#[ignore = "isolated restored test database only"]
async fn rejection_is_evaluation_only_and_audit_is_immutable() {
    let store = setup().await;
    let before = snapshot(&store).await;
    let input: WorkReviewRequest =
        serde_json::from_value(confirmation("reject_evaluation_only")).unwrap();
    store
        .review_work_evaluation(AGENT, SESSION, ACTOR, input)
        .await
        .unwrap();
    for sql in [
        "UPDATE work_evaluation_reviews SET decision='accept_evaluation_only'",
        "DELETE FROM work_evaluation_reviews",
        "TRUNCATE work_evaluation_reviews",
    ] {
        assert!(sqlx::query(sql).execute(store.pool()).await.is_err());
    }
    assert_eq!(
        store
            .work_review_context(AGENT, SESSION, ACTOR)
            .await
            .unwrap()["evaluation_status"],
        "reject_evaluation_only"
    );
    assert_eq!(snapshot(&store).await, before);
}
#[tokio::test]
#[ignore = "isolated restored test database only"]
async fn public_other_owner_operator_viewer_expired_and_spoofed_identities_are_denied() {
    let store = setup().await;
    let operator = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    let other = Uuid::new_v4();
    store
        .grant_access(AGENT, ACTOR, operator, Role::Operator)
        .await
        .unwrap();
    store
        .grant_access(AGENT, ACTOR, viewer, Role::Viewer)
        .await
        .unwrap();
    let app = app(
        &store,
        vec![
            OperatorCredential {
                id: operator,
                role: Role::Operator,
                token: "fixture-review-operator-credential-00".into(),
            },
            OperatorCredential {
                id: viewer,
                role: Role::Viewer,
                token: "fixture-review-viewer-credential-0000".into(),
            },
            OperatorCredential {
                id: other,
                role: Role::Owner,
                token: "fixture-review-other-credential-00000".into(),
            },
        ],
        false,
    );
    let before = snapshot(&store).await;
    for token in [
        None,
        Some(PUBLIC),
        Some("fixture-review-operator-credential-00"),
        Some("fixture-review-viewer-credential-0000"),
        Some("fixture-review-other-credential-00000"),
    ] {
        let status = call(
            &app,
            &path(),
            token,
            Some(confirmation("accept_evaluation_only")),
        )
        .await
        .0;
        assert!(matches!(
            status,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ));
        assert!(matches!(
            call(&app, &path(), token, None).await.0,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ));
    }
    let mut spoof = confirmation("accept_evaluation_only");
    spoof["reviewer_principal_id"] = json!(ACTOR);
    assert_eq!(
        call(&app, &path(), Some(OWNER), Some(spoof)).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let expired = super_app_expired(&store);
    assert_eq!(
        call(
            &expired,
            &path(),
            Some(OWNER),
            Some(confirmation("accept_evaluation_only"))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let short = router(
        AppState::new(
            store.clone(),
            Runtime::new(store.clone(), Arc::new(Offline)),
            PUBLIC.into(),
        )
        .unwrap()
        .with_credentials(vec![OperatorCredential {
            id: ACTOR,
            role: Role::Owner,
            token: OWNER.into(),
        }])
        .unwrap()
        .with_credential_deadlines(BTreeMap::from([(
            ACTOR,
            (Utc::now().timestamp() + 60) as u64,
        )]))
        .unwrap(),
    );
    assert_eq!(
        call(
            &short,
            &path(),
            Some(OWNER),
            Some(confirmation("accept_evaluation_only"))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let wrong = format!("/agents/{}/live-work/{SESSION}/review", Uuid::new_v4());
    assert_eq!(
        call(&app, &wrong, Some(OWNER), None).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(snapshot(&store).await, before);
}
fn super_app_expired(store: &Store) -> Router {
    app(store, vec![], true)
}
#[tokio::test]
#[ignore = "isolated restored test database only"]
async fn stale_mismatched_unconfirmed_and_changed_reports_fail_closed() {
    let store = setup().await;
    let app = app(&store, vec![], false);
    let before = snapshot(&store).await;
    let mut cases = vec![];
    let mut v = confirmation("accept_evaluation_only");
    v["report_sha256"] = json!("0".repeat(64));
    cases.push(v);
    let mut v = confirmation("accept_evaluation_only");
    v["expires_at"] = json!(Utc::now() - Duration::seconds(1));
    cases.push(v);
    let mut v = confirmation("accept_evaluation_only");
    v["consented_at"] = json!(Utc::now() + Duration::minutes(1));
    cases.push(v);
    let mut v = confirmation("accept_evaluation_only");
    v["expires_at"] = json!(Utc::now() + Duration::minutes(6));
    cases.push(v);
    let mut v = confirmation("accept_evaluation_only");
    v["confirm_review"] = json!(false);
    cases.push(v);
    let mut v = confirmation("accept_evaluation_only");
    v["request_id"] = json!(SESSION);
    cases.push(v);
    for v in cases {
        assert_ne!(
            call(&app, &path(), Some(OWNER), Some(v)).await.0,
            StatusCode::OK
        );
    }
    assert_eq!(snapshot(&store).await, before);
    // Only this disposable clone is tampered with to test a stale report hash.
    sqlx::query("UPDATE live_sessions SET data=jsonb_set(data,'{work,report,summary}','\"changed clone report\"') WHERE id=$1").bind(SESSION).execute(store.pool()).await.unwrap();
    assert_eq!(
        call(
            &app,
            &path(),
            Some(OWNER),
            Some(confirmation("accept_evaluation_only"))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
}
#[tokio::test]
#[ignore = "isolated restored test database only"]
async fn concurrent_reviews_record_one_event_and_revocation_blocks_new_decisions() {
    let store = setup().await;
    let app = app(&store, vec![], false);
    let before = snapshot(&store).await;
    let route = path();
    let a = call(
        &app,
        &route,
        Some(OWNER),
        Some(confirmation("accept_evaluation_only")),
    );
    let b = call(
        &app,
        &route,
        Some(OWNER),
        Some(confirmation("reject_evaluation_only")),
    );
    let (a, b) = tokio::join!(a, b);
    assert!(matches!(
        (a.0, b.0),
        (StatusCode::OK, StatusCode::CONFLICT) | (StatusCode::CONFLICT, StatusCode::OK)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM work_evaluation_reviews")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(snapshot(&store).await, before);
    sqlx::query("DELETE FROM agent_access WHERE agent_id=$1 AND principal_id=$2")
        .bind(AGENT)
        .bind(ACTOR)
        .execute(store.pool())
        .await
        .unwrap();
    assert_eq!(
        call(
            &app,
            &path(),
            Some(OWNER),
            Some(confirmation("accept_evaluation_only"))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&app, &path(), Some(OWNER), None).await.0,
        StatusCode::FORBIDDEN
    );
}
#[tokio::test]
#[ignore = "isolated restored test database only"]
async fn database_guard_rejects_forged_null_and_duplicate_request_events() {
    let store = setup().await;
    let valid = confirmation("accept_evaluation_only");
    for (actor, hash, confirm) in [
        (Uuid::new_v4(), HASH.to_string(), valid.clone()),
        (ACTOR, "0".repeat(64), valid.clone()),
        (ACTOR, HASH.into(), {
            let mut v = valid.clone();
            v["confirm_review"] = Value::Null;
            v
        }),
    ] {
        let result=sqlx::query("INSERT INTO work_evaluation_reviews(request_id,session_id,agent_id,reviewer_principal_id,decision,report_sha256,confirmation) VALUES($1,$2,$3,$4,'accept_evaluation_only',$5,$6)").bind(Uuid::parse_str(valid["request_id"].as_str().unwrap()).unwrap()).bind(SESSION).bind(AGENT).bind(actor).bind(hash).bind(confirm).execute(store.pool()).await;
        assert!(result.is_err());
    }
    let canonical: String = sqlx::query_scalar("SELECT public.work_review_canonical($1)")
        .bind(json!({"z":[1,"é"],"a":"quote\""}))
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(canonical, "{\"a\":\"quote\\\"\",\"z\":[1,\"é\"]}");
}

#[tokio::test]
#[ignore = "isolated restored test database only"]
async fn revoked_owner_and_application_epochs_cannot_admit_a_review() {
    let store = setup().await;
    let app = app(&store, vec![], false);
    sqlx::query("DELETE FROM agent_access WHERE agent_id=$1 AND principal_id=$2")
        .bind(AGENT)
        .bind(ACTOR)
        .execute(store.pool())
        .await
        .unwrap();
    assert_eq!(
        call(
            &app,
            &path(),
            Some(OWNER),
            Some(confirmation("accept_evaluation_only"))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM work_evaluation_reviews")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
    let epoch = setup().await;
    sqlx::query("INSERT INTO application_epoch(id,accounting_namespace,label,kind,state,historical_database) VALUES($1,$2,'Disposable review denial','development','prepared','unrecovered')")
        .bind(Uuid::new_v4()).bind(Uuid::new_v4()).execute(epoch.pool()).await.unwrap();
    let input: WorkReviewRequest =
        serde_json::from_value(confirmation("accept_evaluation_only")).unwrap();
    assert!(matches!(
        epoch
            .review_work_evaluation(AGENT, SESSION, ACTOR, input)
            .await,
        Err(Error::Denied)
    ));
}
