//! Explicit disposable cluster only. HTTP requests traverse real auth middleware
//! and a feature-only PostgreSQL role. No listener, provider call or environment file.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use chrono::{Duration, Timelike, Utc};
use serde_json::{Value, json};
use sqlx::{ConnectOptions, postgres::PgConnectOptions};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};
use supa_api::{AppState, OperatorCredential, router};
use supa_domain::agent_identity::{
    AgentQualificationDecision, PurposeRevision, QualificationReview,
};
use supa_domain::*;
use supa_persistence::{Role, Store};
use supa_providers::Offline;
use supa_runtime::Runtime;
use tower::ServiceExt;
use uuid::Uuid;
const OWNER: &str = "fixture-ecosystem-owner-000000000000";
const OPERATOR: &str = "fixture-ecosystem-operator-000000000";
const VIEWER: &str = "fixture-ecosystem-viewer-00000000000";
const OTHER: &str = "fixture-ecosystem-other-000000000000";
const SERVICE: &str = "fixture-ecosystem-service-0000000000";
const EXPIRED: &str = "fixture-ecosystem-expired-0000000000";
fn permitted(o: &PgConnectOptions) -> bool {
    o.get_database() == Some("supa_ecosystem_test")
        && o.get_username() == "supa_ecosystem_test"
        && o.get_port() == 55479
        && o.get_socket().is_some_and(|s| {
            s.file_name().is_some_and(|n| n == "socket")
                && s.parent().is_some_and(|p| {
                    p.parent() == Some(std::path::Path::new("/tmp"))
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .and_then(|n| n.strip_prefix("supa-ecosystem-test-"))
                            .is_some_and(|n| {
                                n.len() == 32 && n.bytes().all(|b| b.is_ascii_hexdigit())
                            })
                })
        })
}
#[test]
fn rejects_unscoped_database_urls() {
    for url in [
        "postgresql://localhost/supa_staging",
        "postgresql://supa_ecosystem_test@localhost/supa_ecosystem_test?port=55479",
        "postgresql://localhost/postgres?host=/tmp/socket",
    ] {
        assert!(!permitted(&url.parse().unwrap()));
    }
}
async fn http(
    app: &Router,
    token: Option<&str>,
    agent: Uuid,
    endpoint: &str,
    body: Value,
) -> (StatusCode, Value) {
    let mut r = Request::builder()
        .method("POST")
        .uri(format!("/agents/{agent}/{endpoint}"))
        .header("content-type", "application/json");
    if let Some(token) = token {
        r = r.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(r.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let code = response.status();
    let data = to_bytes(response.into_body(), 32768).await.unwrap();
    (code, serde_json::from_slice(&data).unwrap())
}
async fn baseline(s: &Store) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('ledger',(SELECT count(*) FROM simulated_ledger),'entries',(SELECT count(*) FROM economic_entries),'holds',(SELECT count(*) FROM economic_holds),'sessions',(SELECT count(*) FROM live_sessions),'inference',(SELECT count(*) FROM inference_calls),'cycles',(SELECT count(*) FROM cycles),'reviews',(SELECT count(*) FROM work_evaluation_reviews),'balances',(SELECT sum(simulated_balance_microusd) FROM agents))").fetch_one(s.pool()).await.unwrap()
}
// Synthetic terminal receipts; no financial postings, no real jobs or execution.
async fn terminal(admin: &Store, agent: Uuid, valid: bool, failure: Option<&str>) -> Uuid {
    let job = Uuid::new_v4();
    let service = Uuid::new_v4();
    let spec = ServiceSpec {
        name: "Synthetic source provenance".into(),
        kind: ServiceKind::Summary,
        price_atoms: 2000,
        estimated_inference_atoms: 1000,
        estimated_infrastructure_atoms: 100,
        max_input_bytes: 8000,
        max_attempts: 1,
    };
    let input = "Private customer prose sentinel. A deterministic supplied statement.";
    let request = WorkRequest::parse(input, spec.kind).unwrap();
    let mut work = analyze_work(&request, spec.kind).unwrap();
    if failure.is_some() {
        work["status"] = json!("abstained");
    }
    if !valid {
        work["objective"] = json!("mismatched unsupported result");
    }
    let output =
        json!({"contract":"structured_report_v1","job_id":job,"live_inference":false,"work":work});
    let state = if failure.is_some() {
        "failed"
    } else {
        "completed"
    };
    let mut tx = admin.pool().begin().await.unwrap();
    sqlx::query("INSERT INTO economic_services(id,agent_id,spec) VALUES($1,$2,$3)")
        .bind(service)
        .bind(agent)
        .bind(json!(spec))
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO economic_jobs(id,service_id,agent_id,customer_id,network,spec,input,state,expires_at,output,output_sha256,failure_code,request) VALUES($1,$2,$3,$4,'simulated',$5,$6,$7,now()+interval '1 day',$8,encode(sha256(convert_to($8::jsonb::text,'UTF8')),'hex'),$9,'{}')")
        .bind(job).bind(service).bind(agent).bind(Uuid::new_v4()).bind(json!(spec)).bind(input).bind(state).bind(&output).bind(failure).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO economic_audit(job_id,agent_id,actor,operation,data) SELECT id,agent_id,$2,state,jsonb_build_object('output_sha256',output_sha256,'failure_code',failure_code) FROM economic_jobs WHERE id=$1")
        .bind(job).bind(Uuid::new_v4()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    job
}
#[tokio::test]
#[ignore = "requires newly created, verified PostgreSQL 18 ecosystem cluster"]
async fn authenticated_restricted_ecosystem_backend() {
    let (admin, options) = isolated_admin().await;
    sqlx::raw_sql("CREATE ROLE supa_ecosystem_runtime LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS; REVOKE CREATE ON SCHEMA public FROM PUBLIC;").execute(admin.pool()).await.unwrap();
    sqlx::raw_sql(include_str!(
        "../../../deployment/ecosystem-social-grants.sql"
    ))
    .execute(admin.pool())
    .await
    .unwrap();
    let restricted = Store::connect(
        options
            .clone()
            .username("supa_ecosystem_runtime")
            .to_url_lossy()
            .as_str(),
    )
    .await
    .unwrap();
    let owner = Uuid::new_v4();
    let operator = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    let other = Uuid::new_v4();
    let expired = Uuid::new_v4();
    let mut a = Agent::new(
        "Private agent name sentinel".into(),
        Personality::default(),
        CycleConfig::default(),
        BTreeSet::new(),
    )
    .unwrap();
    a.lifecycle = Lifecycle::Paused;
    a.created_at = (Utc::now() - Duration::seconds(3))
        .with_nanosecond(123456789)
        .unwrap();
    admin.create_owned(&a, owner).await.unwrap();
    sqlx::query("UPDATE activity SET simulated=true WHERE agent_id=$1")
        .bind(a.id)
        .execute(admin.pool())
        .await
        .unwrap();
    admin
        .grant_access(a.id, owner, operator, Role::Operator)
        .await
        .unwrap();
    admin
        .grant_access(a.id, owner, viewer, Role::Viewer)
        .await
        .unwrap();
    assert!(
        admin
            .grant_access(a.id, owner, expired, Role::Owner)
            .await
            .is_err()
    );
    let runtime = Runtime::new(restricted.clone(), Arc::new(Offline));
    let app = router(
        AppState::new(restricted.clone(), runtime, SERVICE.into())
            .unwrap()
            .with_credentials(vec![
                OperatorCredential {
                    id: owner,
                    role: Role::Owner,
                    token: OWNER.into(),
                },
                OperatorCredential {
                    id: operator,
                    role: Role::Operator,
                    token: OPERATOR.into(),
                },
                OperatorCredential {
                    id: viewer,
                    role: Role::Viewer,
                    token: VIEWER.into(),
                },
                OperatorCredential {
                    id: other,
                    role: Role::Owner,
                    token: OTHER.into(),
                },
                OperatorCredential {
                    id: expired,
                    role: Role::Owner,
                    token: EXPIRED.into(),
                },
            ])
            .unwrap()
            .with_credential_deadlines(BTreeMap::from([(expired, 1)]))
            .unwrap(),
    );
    let prepare = json!({"request_id":Uuid::new_v4()});
    for (token, code) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(EXPIRED), StatusCode::UNAUTHORIZED),
        (Some(SERVICE), StatusCode::FORBIDDEN),
        (Some(VIEWER), StatusCode::FORBIDDEN),
        (Some(OTHER), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            http(&app, token, a.id, "ecosystem-prepare", prepare.clone())
                .await
                .0,
            code
        );
    }
    let purpose = PurposeRevision {
        id: Uuid::new_v4(),
        agent_id: a.id,
        creator_id: owner,
        revision: 1,
        created_at: Utc::now(),
        objective: "Private objective sentinel".into(),
        permitted_tasks: vec![ServiceKind::Summary],
        success_criteria: vec!["Private criteria sentinel".into()],
        excluded_actions: vec!["No external activity".into()],
        public_summary: None,
    };
    for token in [OPERATOR, VIEWER, OTHER] {
        assert_eq!(
            http(&app, Some(token), a.id, "purpose", json!(purpose))
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    let mut forged = json!(purpose);
    forged["creator_id"] = json!(other);
    assert_eq!(
        http(&app, Some(OWNER), a.id, "purpose", forged).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        http(&app, Some(OWNER), a.id, "purpose", json!(purpose))
            .await
            .0,
        StatusCode::OK
    );
    let grant = json!({"request_id":Uuid::new_v4(),"expected_revision":0,"public":true,"revoked":false,"valid_from":a.created_at,"expires_at":Utc::now()+Duration::hours(1),"categories":["agent_created","agent_qualified","task_completed","task_failed","achievement"]});
    assert_eq!(
        http(
            &app,
            Some(OPERATOR),
            a.id,
            "ecosystem-consent",
            grant.clone()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        http(&app, Some(OWNER), a.id, "ecosystem-consent", grant.clone())
            .await
            .0,
        StatusCode::OK
    );
    let completed = terminal(&admin, a.id, true, None).await;
    let failed = terminal(&admin, a.id, true, Some("bounded_work_permission_denied")).await;
    let invalid = terminal(&admin, a.id, false, None).await;
    let ambiguous = terminal(
        &admin,
        a.id,
        true,
        Some("bounded_provider_or_contract_failure"),
    )
    .await;
    let evidence: Uuid = sqlx::query_scalar(
        "SELECT id FROM activity WHERE agent_id=$1 AND kind='lifecycle' AND code='created'",
    )
    .bind(a.id)
    .fetch_one(admin.pool())
    .await
    .unwrap();
    let policy = Uuid::new_v4();
    let review = QualificationReview {
        id: Uuid::new_v4(),
        purpose: purpose.clone(),
        policy_id: policy,
        policy_version: 1,
        reviewer_id: owner,
        decision: AgentQualificationDecision::Qualified,
        evidence_ids: vec![evidence],
        reviewed_at: Utc::now(),
        expires_at: Utc::now() + Duration::minutes(20),
    };
    // Ownership alone does not provision a reviewer authority.
    assert_eq!(
        http(
            &app,
            Some(OWNER),
            a.id,
            "qualification-review",
            json!(review)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    sqlx::query("INSERT INTO ecosystem_social.qualification_policies(id,version,enabled,reviewers,max_validity_seconds) VALUES($1,1,true,$2,3600)").bind(policy).bind(vec![owner]).execute(admin.pool()).await.unwrap();
    let mut bad = json!(review);
    bad["evidence_ids"] = json!([Uuid::new_v4()]);
    assert_eq!(
        http(&app, Some(OWNER), a.id, "qualification-review", bad)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        http(
            &app,
            Some(OPERATOR),
            a.id,
            "qualification-review",
            json!(review)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        http(
            &app,
            Some(OWNER),
            a.id,
            "qualification-review",
            json!(review)
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        http(
            &app,
            Some(OWNER),
            a.id,
            "qualification-review",
            json!(review)
        )
        .await
        .0,
        StatusCode::OK
    );
    let digest: String = sqlx::query_scalar("SELECT output_sha256 FROM economic_jobs WHERE id=$1")
        .bind(completed)
        .fetch_one(admin.pool())
        .await
        .unwrap();
    let achievement = json!({"id":Uuid::new_v4(),"result_id":completed,"result_sha256":digest,"decision":"accept","confirm_semantic_review":true});
    let mut bad = achievement.clone();
    bad["result_sha256"] = json!("0".repeat(64));
    assert_eq!(
        http(&app, Some(OWNER), a.id, "achievement-review", bad)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let mut evaluation = achievement.clone();
    evaluation["result_id"] = json!(Uuid::from_u128(41316));
    assert_eq!(
        http(&app, Some(OWNER), a.id, "achievement-review", evaluation)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        http(
            &app,
            Some(OPERATOR),
            a.id,
            "achievement-review",
            achievement.clone()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        http(
            &app,
            Some(OWNER),
            a.id,
            "achievement-review",
            achievement.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    // A completed-looking activity has no authoritative task receipt.
    sqlx::query("INSERT INTO activity(id,agent_id,kind,code) VALUES($1,$2,'task','completed')")
        .bind(Uuid::new_v4())
        .bind(a.id)
        .execute(admin.pool())
        .await
        .unwrap();
    let before = baseline(&admin).await;
    let mut spoof = prepare.clone();
    spoof["actor"] = json!(owner);
    assert_eq!(
        http(&app, Some(OPERATOR), a.id, "ecosystem-prepare", spoof)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let (code, payload) = http(
        &app,
        Some(OPERATOR),
        a.id,
        "ecosystem-prepare",
        prepare.clone(),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{payload}");
    let events = payload["events"].as_array().unwrap();
    assert_eq!(events.len(), 5, "{payload}");
    let text = payload.to_string();
    for private in [
        "Private agent",
        "Private customer",
        "Private objective",
        "Private criteria",
        "input",
        "creator_id",
    ] {
        assert!(!text.contains(private));
    }
    assert!(text.contains(&completed.to_string()) && text.contains(&failed.to_string()));
    assert!(!text.contains(&invalid.to_string()) && !text.contains(&ambiguous.to_string()));
    assert_eq!(payload["publishing_enabled"], false);
    assert_eq!(payload["requires_semantic_review"], true);
    assert_eq!(
        http(
            &app,
            Some(OPERATOR),
            a.id,
            "ecosystem-prepare",
            prepare.clone()
        )
        .await,
        (StatusCode::OK, payload)
    );
    let mut revoked_review = review.clone();
    revoked_review.id = Uuid::new_v4();
    revoked_review.decision = AgentQualificationDecision::Revoked;
    assert_eq!(
        http(
            &app,
            Some(OWNER),
            a.id,
            "qualification-review",
            json!(revoked_review)
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        http(
            &app,
            Some(OPERATOR),
            a.id,
            "ecosystem-prepare",
            prepare.clone()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // New disabled policy supersedes prior reviewer authority, never proving it by absence.
    sqlx::query("INSERT INTO ecosystem_social.qualification_policies(id,version,enabled,reviewers,max_validity_seconds) VALUES($1,2,false,$2,3600)").bind(policy).bind(vec![owner]).execute(admin.pool()).await.unwrap();
    assert_eq!(
        http(
            &app,
            Some(OPERATOR),
            a.id,
            "ecosystem-prepare",
            prepare.clone()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        http(
            &app,
            Some(OWNER),
            a.id,
            "qualification-review",
            json!(review)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut revoked_achievement = achievement;
    revoked_achievement["id"] = json!(Uuid::new_v4());
    revoked_achievement["decision"] = json!("revoke");
    assert_eq!(
        http(
            &app,
            Some(OWNER),
            a.id,
            "achievement-review",
            revoked_achievement
        )
        .await
        .0,
        StatusCode::OK
    );
    let fresh = json!({"request_id":Uuid::new_v4()});
    assert!(
        http(&app, Some(OWNER), a.id, "ecosystem-prepare", fresh)
            .await
            .1["events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // Every statement is tested separately so a failed statement cannot mask another.
    for sql in [
        "UPDATE agents SET data=data",
        "UPDATE agent_access SET role='owner'",
        "INSERT INTO application_epoch DEFAULT VALUES",
        "UPDATE economic_jobs SET earned_atoms=1",
        "DELETE FROM economic_entries",
        "SELECT * FROM wallet_identities",
        "SELECT * FROM live_budgets",
        "UPDATE economic_holds SET held=false",
        "DELETE FROM _sqlx_migrations",
        "CREATE TABLE public.forbidden(x int)",
        "CREATE SCHEMA forbidden",
        "GRANT supa_ecosystem_runtime TO supa_ecosystem_test",
        "ALTER ROLE supa_ecosystem_runtime SUPERUSER",
        "UPDATE ecosystem_social.consents SET revision=revision",
        "TRUNCATE ecosystem_social.preparations",
        "INSERT INTO ecosystem_social.qualification_policies(id,version,reviewers,max_validity_seconds) VALUES(gen_random_uuid(),1,ARRAY[gen_random_uuid()],60)",
    ] {
        let err = sqlx::raw_sql(sql)
            .execute(restricted.pool())
            .await
            .expect_err(sql);
        assert_eq!(
            err.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("42501"),
            "{sql}: {err}"
        );
    }
    restricted.verify_runtime_schema().await.unwrap();
    let counts: i64 = sqlx::query_scalar("SELECT count(*) FROM ecosystem_social.preparations")
        .fetch_one(admin.pool())
        .await
        .unwrap();
    let mut revoked = grant;
    revoked["request_id"] = json!(Uuid::new_v4());
    revoked["expected_revision"] = json!(1);
    revoked["revoked"] = json!(true);
    assert_eq!(
        http(&app, Some(OWNER), a.id, "ecosystem-consent", revoked)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        http(&app, Some(OPERATOR), a.id, "ecosystem-prepare", prepare)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        counts,
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ecosystem_social.preparations")
            .fetch_one(admin.pool())
            .await
            .unwrap()
    );
    assert_eq!(before, baseline(&admin).await);
    let saved = admin.get(a.id).await.unwrap();
    assert_eq!(saved.lifecycle, Lifecycle::Paused);
    assert!(saved.capabilities.is_empty());
    assert!(sqlx::query_scalar::<_,bool>("SELECT NOT rolcreaterole AND NOT rolsuper AND NOT rolcreatedb AND NOT rolbypassrls FROM pg_roles WHERE rolname=current_user").fetch_one(restricted.pool()).await.unwrap());
}

async fn isolated_admin() -> (Store, PgConnectOptions) {
    let options: PgConnectOptions = std::env::var("SUPA_ECOSYSTEM_TEST_URL")
        .unwrap()
        .parse()
        .unwrap();
    assert!(permitted(&options));
    let socket = options.get_socket().unwrap();
    assert_eq!(*socket, socket.canonicalize().unwrap());
    let admin = Store::connect(options.to_url_lossy().as_str())
        .await
        .unwrap();
    let (db,user,data,listen,address,version):(String,String,String,String,Option<String>,i32)=sqlx::query_as("SELECT current_database(),current_user::text,current_setting('data_directory'),current_setting('listen_addresses'),inet_server_addr()::text,current_setting('server_version_num')::integer").fetch_one(admin.pool()).await.unwrap();
    assert_eq!(db, "supa_ecosystem_test");
    assert_eq!(user, "supa_ecosystem_test");
    assert!(listen.is_empty() && address.is_none());
    assert!((180000..190000).contains(&version));
    assert_eq!(
        PathBuf::from(data).canonicalize().unwrap(),
        socket
            .parent()
            .unwrap()
            .join("data")
            .canonicalize()
            .unwrap()
    );
    (admin, options)
}
#[tokio::test]
#[ignore = "requires explicit restart of the verified PostgreSQL 18 ecosystem cluster"]
async fn ecosystem_backend_recovery() {
    let (admin, options) = isolated_admin().await;
    let before = baseline(&admin).await;
    let restricted = Store::connect(
        options
            .username("supa_ecosystem_runtime")
            .to_url_lossy()
            .as_str(),
    )
    .await
    .unwrap();
    restricted.verify_runtime_schema().await.unwrap();
    let (agent,owner):(Uuid,Uuid)=sqlx::query_as("SELECT agent_id,creator_id FROM ecosystem_social.owner_reviews WHERE kind='achievement' LIMIT 1").fetch_one(admin.pool()).await.unwrap();
    assert!(matches!(
        restricted
            .prepare_ecosystem_social(agent, owner, Uuid::new_v4())
            .await,
        Err(Error::Denied)
    ));
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM ecosystem_social.prepared_events e JOIN ecosystem_social.sources s ON e.source_id=s.source_id WHERE s.agent_id=$1").bind(agent).fetch_one(admin.pool()).await.unwrap();
    assert_eq!(count, 5);
    let reviews: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ecosystem_social.owner_reviews WHERE agent_id=$1")
            .bind(agent)
            .fetch_one(admin.pool())
            .await
            .unwrap();
    assert_eq!(reviews, 4);
    assert_eq!(before, baseline(&admin).await);
}
