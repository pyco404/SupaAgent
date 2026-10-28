//! Genuine isolated PostgreSQL contracts, deterministic inference only, no existing agent changes.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::sync::Arc;
use supa_api::{AppState, OperatorCredential, router};
use supa_domain::*;
use supa_persistence::{Role, Store};
use supa_providers::Offline;
use supa_runtime::Runtime;
use tower::ServiceExt;
use uuid::Uuid;
const OWNER: &str = "fixture-owner-credential-000000000000";
const OTHER: &str = "fixture-other-credential-000000000000";
const READER: &str = "fixture-reader-credential-00000000000";
const OPERATOR: &str = "fixture-operator-credential-000000000";
const SERVICE: &str = "fixture-service-credential-0000000000";
struct Fixture {
    store: Store,
    admin: Store,
    schema: String,
    url: String,
    app: Router,
    id: Uuid,
    owner: Uuid,
    other: Uuid,
    reader: Uuid,
    operator: Uuid,
    goal: Uuid,
}
impl Fixture {
    async fn new() -> Self {
        let base = std::env::var("TEST_DATABASE_URL").expect("dedicated test database required");
        let admin = Store::connect(&base).await.unwrap();
        let schema = format!("authorization_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE SCHEMA {schema}"))
            .execute(admin.pool())
            .await
            .unwrap();
        let url = format!(
            "{base}{}options=-csearch_path%3D{schema}",
            if base.contains('?') { '&' } else { '?' }
        );
        let store = Store::connect(&url).await.unwrap();
        store.migrate().await.unwrap();
        let owner = Uuid::new_v4();
        let other = Uuid::new_v4();
        let reader = Uuid::new_v4();
        let operator = Uuid::new_v4();
        let runtime = Runtime::new(store.clone(), Arc::new(Offline));
        let state = AppState::new(store.clone(), runtime, SERVICE.into())
            .unwrap()
            .with_credentials(vec![
                OperatorCredential {
                    id: owner,
                    role: Role::Owner,
                    token: OWNER.into(),
                },
                OperatorCredential {
                    id: other,
                    role: Role::Owner,
                    token: OTHER.into(),
                },
                OperatorCredential {
                    id: reader,
                    role: Role::Viewer,
                    token: READER.into(),
                },
                OperatorCredential {
                    id: operator,
                    role: Role::Operator,
                    token: OPERATOR.into(),
                },
            ])
            .unwrap();
        let app = router(state);
        let (code,agent)=http(&app,OWNER,"POST","/agents",Some(json!({"name":"Isolated renewal contract","capabilities":["read_memory","remember","evaluate_goal"]}))).await;
        assert_eq!(code, StatusCode::OK);
        let id = Uuid::parse_str(agent["id"].as_str().unwrap()).unwrap();
        let goal = store
            .add_goal(id, "Review verified outcomes and retain lessons".into())
            .await
            .unwrap()
            .id;
        store
            .credit(id, LedgerKind::Seed, 10000, Uuid::new_v4())
            .await
            .unwrap();
        Self {
            store,
            admin,
            schema,
            url,
            app,
            id,
            owner,
            other,
            reader,
            operator,
            goal,
        }
    }
    fn path(&self, resource: &str) -> String {
        format!("/agents/{}/{}", self.id, resource)
    }
    fn command(&self, operation: ScheduleOperation, revision: i64) -> ScheduleCommand {
        ScheduleCommand {
            request_id: Uuid::new_v4(),
            expected_revision: revision,
            operation,
            authorize_offline: true,
            consented_at: Some(sqlx::types::chrono::Utc::now()),
            reason: "Explicit isolated test consent".into(),
            policy: if matches!(
                operation,
                ScheduleOperation::Start | ScheduleOperation::Renew
            ) {
                Some(SchedulePolicy {
                    goal_ids: vec![self.goal],
                    interval_seconds: 5,
                    max_cycles: 1,
                    simulated_budget_microusd: 1100,
                    valid_for_seconds: 60,
                })
            } else {
                None
            },
        }
    }
    async fn control(&self, token: &str, command: &ScheduleCommand) -> (StatusCode, Value) {
        http(
            &self.app,
            token,
            "POST",
            &self.path("schedule"),
            Some(json!(command)),
        )
        .await
    }
    async fn close(self) {
        self.store.pool().close().await;
        sqlx::query(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .execute(self.admin.pool())
            .await
            .unwrap();
    }
}
async fn http(
    app: &Router,
    token: &str,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .header("x-supa-principal", Uuid::nil().to_string())
                .header("x-supa-role", "owner")
                .header("content-type", "application/json")
                .body(Body::from(body.map(|b| b.to_string()).unwrap_or_default()))
                .unwrap(),
        )
        .await
        .unwrap();
    let code = response.status();
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    (code, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn identities_roles_and_unknown_ownership_fail_closed() {
    let f = Fixture::new().await;
    for resource in [
        "",
        "memory",
        "goals",
        "activity",
        "cycles",
        "actions",
        "financials",
        "simulated-ledger",
        "schedule",
        "authorizations",
        "execution-health",
        "wallet",
    ] {
        let path = if resource.is_empty() {
            format!("/agents/{}", f.id)
        } else {
            f.path(resource)
        };
        assert_eq!(
            http(&f.app, OTHER, "GET", &path, None).await.0,
            StatusCode::FORBIDDEN,
            "cross-user {resource}"
        );
        assert_eq!(
            http(&f.app, SERVICE, "GET", &path, None).await.0,
            StatusCode::FORBIDDEN,
            "shared service {resource}"
        );
    }
    assert_eq!(
        http(&f.app, OTHER, "GET", "/agents", None).await.1,
        json!([])
    );
    let directory = http(&f.app, SERVICE, "GET", "/directory", None).await.1;
    assert_eq!(directory[0].as_object().unwrap().len(), 4);
    assert!(directory[0].get("memory").is_none());
    let legacy = Agent::new(
        "Unowned legacy identity".into(),
        Personality::default(),
        CycleConfig::default(),
        Default::default(),
    )
    .unwrap();
    f.store.create(&legacy).await.unwrap();
    assert_eq!(
        http(
            &f.app,
            OWNER,
            "GET",
            &format!("/agents/{}/memory", legacy.id),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.store.get(legacy.id).await.unwrap().lifecycle,
        Lifecycle::Created
    );
    for (principal, role) in [(f.reader, "viewer"), (f.operator, "operator")] {
        assert_eq!(
            http(
                &f.app,
                OTHER,
                "POST",
                &f.path("access"),
                Some(json!({"principal_id":principal,"role":role}))
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            http(
                &f.app,
                OWNER,
                "POST",
                &f.path("access"),
                Some(json!({"principal_id":principal,"role":role}))
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    assert_eq!(
        http(&f.app, READER, "GET", &f.path("memory"), None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        http(&f.app, READER, "GET", &f.path("access"), None).await.1,
        json!({"role":"viewer"})
    );
    assert_eq!(
        f.control(READER, &f.command(ScheduleOperation::Start, 0))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.control(OPERATOR, &f.command(ScheduleOperation::Start, 0))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        http(
            &f.app,
            OPERATOR,
            "POST",
            &f.path("access"),
            Some(json!({"principal_id":f.other,"role":"owner"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        http(
            &f.app,
            OWNER,
            "POST",
            &f.path("access"),
            Some(json!({"principal_id":f.other,"role":"owner"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // A credential's viewer ceiling still wins even if trusted provisioning grants operator ACL.
    f.store
        .grant_access(f.id, f.owner, f.reader, Role::Operator)
        .await
        .unwrap();
    assert_eq!(
        f.control(READER, &f.command(ScheduleOperation::Stop, 0))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    f.close().await;
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn explicit_renewal_preserves_accounting_and_fences_duplicates() {
    let f = Fixture::new().await;
    let start = f.command(ScheduleOperation::Start, 0);
    assert_eq!(f.control(OWNER, &start).await.0, StatusCode::OK);
    let runtime = Runtime::new(f.store.clone(), Arc::new(Offline));
    runtime.cycle(f.id, true).await.unwrap();
    let before = f.store.financials(f.id).await.unwrap();
    let ledger = f.store.history(f.id, "ledger").await.unwrap();
    let current = f.store.schedule_status(f.id).await.unwrap();
    assert_eq!(current.lifecycle, Lifecycle::Paused);
    assert_eq!(current.reserved_microusd, 1100);
    let mut renew = f.command(ScheduleOperation::Renew, current.revision);
    renew.policy.as_mut().unwrap().max_cycles = 2;
    renew.policy.as_mut().unwrap().simulated_budget_microusd = 2200;
    let mut no_consent = renew.clone();
    no_consent.authorize_offline = false;
    assert_eq!(f.control(OWNER, &no_consent).await.0, StatusCode::FORBIDDEN);
    let (code, result) = f.control(OWNER, &renew).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(result["cycles_started"], 0);
    assert_eq!(f.control(OWNER, &renew).await.0, StatusCode::OK);
    assert_eq!(f.control(OTHER, &renew).await.0, StatusCode::FORBIDDEN);
    assert_eq!(
        f.control(
            OWNER,
            &f.command(
                ScheduleOperation::Renew,
                result["revision"].as_i64().unwrap()
            )
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let history = f.store.authorization_history(f.id).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(
        history
            .iter()
            .map(|a| a["reserved_microusd"].as_i64().unwrap())
            .sum::<i64>(),
        1100
    );
    assert!(history.iter().all(|a| a["approved_by"] == json!(f.owner)));
    assert_eq!(
        history.iter().filter(|a| a["closed_at"].is_null()).count(),
        1
    );
    assert_eq!(f.store.history(f.id, "ledger").await.unwrap(), ledger);
    assert_eq!(
        f.store.financials(f.id).await.unwrap().balance_microusd,
        before.balance_microusd
    );
    let restarted = Store::connect(&f.url).await.unwrap();
    assert_eq!(
        restarted.schedule_status(f.id).await.unwrap().revision,
        result["revision"].as_i64().unwrap()
    );
    restarted.pool().close().await;
    let pause = f.command(
        ScheduleOperation::Pause,
        result["revision"].as_i64().unwrap(),
    );
    assert_eq!(f.control(OWNER, &pause).await.0, StatusCode::OK);
    // Old renewal replay returns current pause; it cannot reactivate or replenish limits.
    assert_eq!(f.control(OWNER, &renew).await.1["lifecycle"], "paused");
    let rev = f.store.schedule_status(f.id).await.unwrap().revision;
    let a = f.command(ScheduleOperation::Renew, rev);
    let b = f.command(ScheduleOperation::Renew, rev);
    let (a, b) = tokio::join!(f.control(OWNER, &a), f.control(OWNER, &b));
    assert_eq!(
        [a.0, b.0].iter().filter(|c| **c == StatusCode::OK).count(),
        1
    );
    assert_eq!(f.store.authorization_history(f.id).await.unwrap().len(), 3);
    assert_eq!(f.store.history(f.id, "ledger").await.unwrap(), ledger);
    f.close().await;
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn heartbeat_stalls_restarts_and_cycle_lease_fencing_are_measured() {
    let f = Fixture::new().await;
    assert_eq!(
        f.store.execution_health(f.id).await.unwrap()["status"],
        "unavailable"
    );
    let mut start = f.command(ScheduleOperation::Start, 0);
    start.policy.as_mut().unwrap().max_cycles = 3;
    start.policy.as_mut().unwrap().simulated_budget_microusd = 3300;
    assert_eq!(f.control(OWNER, &start).await.0, StatusCode::OK);
    assert_eq!(
        f.store.execution_health(f.id).await.unwrap()["execution_running"],
        false
    );
    let worker = f.store.register_worker("worker", true, 1).await.unwrap();
    f.store.worker_tick(worker, false, None).await.unwrap();
    f.store.worker_tick(worker, true, None).await.unwrap();
    assert_eq!(
        f.store.execution_health(f.id).await.unwrap()["status"],
        "healthy"
    );
    let cycle = f
        .store
        .begin_cycle_for(f.id, true, Some(worker))
        .await
        .unwrap();
    assert_eq!(
        f.store.execution_health(f.id).await.unwrap()["execution_running"],
        true
    );
    assert!(matches!(
        f.store.begin_cycle_for(f.id, true, Some(worker)).await,
        Err(Error::Busy)
    ));
    sqlx::query("UPDATE worker_health SET tick_started_at=now()-interval '40 seconds',tick_finished_at=now()-interval '45 seconds' WHERE id=$1").bind(worker).execute(f.store.pool()).await.unwrap();
    assert_eq!(
        f.store.execution_health(f.id).await.unwrap()["status"],
        "delayed"
    );
    sqlx::query("UPDATE worker_health SET heartbeat_at=now()-interval '20 seconds' WHERE id=$1")
        .bind(worker)
        .execute(f.store.pool())
        .await
        .unwrap();
    assert_eq!(
        f.store.execution_health(f.id).await.unwrap()["status"],
        "unavailable"
    );
    assert_eq!(
        f.store.execution_health(f.id).await.unwrap()["execution_running"],
        false
    );
    assert!(matches!(f.store.heartbeat(worker).await, Err(Error::Stale)));
    assert!(matches!(
        f.store
            .charge(f.id, cycle, LedgerKind::Operating, 100, Uuid::new_v4())
            .await,
        Err(Error::Stale)
    ));
    sqlx::query("UPDATE cycles SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(cycle)
        .execute(f.store.pool())
        .await
        .unwrap();
    assert_eq!(
        f.store.execution_health(f.id).await.unwrap()["status"],
        "recovery_required"
    );
    f.store.recover().await.unwrap();
    assert_eq!(
        f.store.get(f.id).await.unwrap().lifecycle,
        Lifecycle::Suspended
    );
    let replacement = f.store.register_worker("worker", true, 1).await.unwrap();
    assert_ne!(worker, replacement);
    assert!(matches!(
        f.store.finish(f.id, cycle, None).await,
        Err(Error::Stale) | Err(Error::Inactive)
    ));
    assert_eq!(
        f.store.schedule_status(f.id).await.unwrap().cycles_started,
        1
    );
    assert_eq!(
        f.store.financials(f.id).await.unwrap().balance_microusd,
        10000
    );
    f.store.stop_worker(replacement).await.unwrap();
    assert!(f.store.heartbeat(replacement).await.is_err());
    f.close().await;
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn renewal_rejects_old_consent_and_unresolved_leases() {
    let f = Fixture::new().await;
    let start = f.command(ScheduleOperation::Start, 0);
    assert_eq!(f.control(OWNER, &start).await.0, StatusCode::OK);
    let revision = f.store.schedule_status(f.id).await.unwrap().revision;
    assert_eq!(
        f.control(OWNER, &f.command(ScheduleOperation::Pause, revision))
            .await
            .0,
        StatusCode::OK
    );
    let revision = f.store.schedule_status(f.id).await.unwrap().revision;
    let mut command = f.command(ScheduleOperation::Renew, revision);
    command.consented_at =
        Some(sqlx::types::chrono::Utc::now() - std::time::Duration::from_secs(180));
    assert_eq!(f.control(OWNER, &command).await.0, StatusCode::CONFLICT);
    command.consented_at = Some(sqlx::types::chrono::Utc::now());
    let cycle = Uuid::new_v4();
    sqlx::query("INSERT INTO cycles(id,agent_id,status,lease_until) VALUES($1,$2,'running',now()+interval '60 seconds')").bind(cycle).bind(f.id).execute(f.store.pool()).await.unwrap();
    assert_eq!(f.control(OWNER, &command).await.0, StatusCode::CONFLICT);
    assert_eq!(f.store.authorization_history(f.id).await.unwrap().len(), 1);
    assert_eq!(
        f.store.schedule_status(f.id).await.unwrap().cycles_started,
        0
    );
    sqlx::query("UPDATE cycles SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(cycle)
        .execute(f.store.pool())
        .await
        .unwrap();
    assert_eq!(f.control(OWNER, &command).await.0, StatusCode::CONFLICT); // Expiry alone is not resolution.
    f.store.recover().await.unwrap();
    command.expected_revision = f.store.schedule_status(f.id).await.unwrap().revision;
    assert_eq!(f.control(OWNER, &command).await.0, StatusCode::OK);
    f.close().await;
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn additive_migration_preserves_unowned_legacy_records_and_allowances() {
    let base = std::env::var("TEST_DATABASE_URL").unwrap();
    let admin = Store::connect(&base).await.unwrap();
    let schema = format!("legacy_upgrade_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(admin.pool())
        .await
        .unwrap();
    let store = Store::connect(&format!(
        "{base}{}options=-csearch_path%3D{schema}",
        if base.contains('?') { '&' } else { '?' }
    ))
    .await
    .unwrap();
    let mut old = sqlx::migrate!("../../migrations");
    old.migrations = std::borrow::Cow::Owned(
        old.migrations
            .into_owned()
            .into_iter()
            .filter(|m| m.version < 6)
            .collect(),
    );
    old.run(store.pool()).await.unwrap();
    let agent = Agent::new(
        "Preserved legacy identity".into(),
        Personality::default(),
        CycleConfig {
            scheduler_enabled: true,
            ..CycleConfig::default()
        },
        [Capability::ReadMemory].into(),
    )
    .unwrap();
    store.create(&agent).await.unwrap();
    store
        .transition(
            agent.id,
            Lifecycle::Active,
            "Legacy explicit approval",
            true,
        )
        .await
        .unwrap();
    let goal = store
        .add_goal(agent.id, "Preserved legacy goal".into())
        .await
        .unwrap();
    store
        .credit(agent.id, LedgerKind::Seed, 10000, Uuid::new_v4())
        .await
        .unwrap();
    let policy = SchedulePolicy {
        goal_ids: vec![goal.id],
        interval_seconds: 5,
        max_cycles: 3,
        simulated_budget_microusd: 3300,
        valid_for_seconds: 3600,
    };
    sqlx::query("INSERT INTO agent_schedules(agent_id,policy,expires_at,cycles_started,reserved_microusd,enabled) VALUES($1,$2,now()+interval '1 hour',2,2200,true)").bind(agent.id).bind(json!(policy)).execute(store.pool()).await.unwrap();
    let before = json!(store.get(agent.id).await.unwrap());
    let ledger = store.history(agent.id, "ledger").await.unwrap();
    store.migrate().await.unwrap();
    assert_eq!(json!(store.get(agent.id).await.unwrap()), before);
    assert_eq!(store.history(agent.id, "ledger").await.unwrap(), ledger);
    let history = store.authorization_history(agent.id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["legacy"], true);
    assert_eq!(history[0]["approved_by"], Value::Null);
    assert_eq!(history[0]["approved_at"], Value::Null);
    assert_eq!(history[0]["reserved_microusd"], 2200);
    assert!(matches!(
        store
            .require_access(agent.id, Uuid::new_v4(), Role::Viewer)
            .await,
        Err(Error::Denied)
    ));
    assert!(store.due_agents().await.unwrap().is_empty());
    let owner = Uuid::new_v4();
    store.establish_owner(agent.id, owner).await.unwrap();
    assert!(store.due_agents().await.unwrap().is_empty()); // Ownership alone is not execution consent.
    let status = store.schedule_status(agent.id).await.unwrap();
    let command = ScheduleCommand {
        request_id: Uuid::new_v4(),
        expected_revision: status.revision,
        operation: ScheduleOperation::Resume,
        authorize_offline: true,
        consented_at: None,
        reason: "Cannot adopt an unknown legacy approval".into(),
        policy: None,
    };
    assert!(matches!(
        store
            .schedule_control_as(agent.id, &command, Some(owner))
            .await,
        Err(Error::Denied)
    ));
    assert_eq!(json!(store.get(agent.id).await.unwrap()), before);
    store.pool().close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(admin.pool())
        .await
        .unwrap();
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn lease_expiry_is_rechecked_after_waiting_for_agent_lock() {
    let f = Fixture::new().await;
    assert_eq!(
        f.control(OWNER, &f.command(ScheduleOperation::Start, 0))
            .await
            .0,
        StatusCode::OK
    );
    let worker = f.store.register_worker("worker", true, 1).await.unwrap();
    let cycle = f
        .store
        .begin_cycle_for(f.id, true, Some(worker))
        .await
        .unwrap();
    let mut lock = f.store.pool().begin().await.unwrap();
    sqlx::query("SELECT id FROM agents WHERE id=$1 FOR UPDATE")
        .bind(f.id)
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    // The charge starts its transaction while the lease is valid, then blocks on the agent lock.
    sqlx::query("UPDATE cycles SET lease_until=clock_timestamp()+interval '1 second' WHERE id=$1")
        .bind(cycle)
        .execute(&mut *lock)
        .await
        .unwrap();
    let store = f.store.clone();
    let id = f.id;
    let charge = tokio::spawn(async move {
        store
            .charge(id, cycle, LedgerKind::Operating, 100, Uuid::new_v4())
            .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    lock.commit().await.unwrap();
    assert!(matches!(charge.await.unwrap(), Err(Error::Stale)));
    assert_eq!(
        f.store.financials(f.id).await.unwrap().balance_microusd,
        10000
    );
    f.close().await;
}
