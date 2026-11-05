//! Opt-in, socket-only disposable PostgreSQL. No production URLs or dotenv loading.
use chrono::{Duration, Timelike, Utc};
use serde_json::{Value, json};
use sqlx::{ConnectOptions, postgres::PgConnectOptions};
use std::{collections::BTreeSet, path::PathBuf};
use supa_domain::agent_identity::PurposeRevision;
use supa_domain::{Agent, CycleConfig, Error, Lifecycle, Personality, ServiceKind};
use supa_persistence::{EcosystemConsentRequest, Role, Store};
use uuid::Uuid;
fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}
fn permitted(o: &PgConnectOptions) -> bool {
    let Some(socket) = o.get_socket() else {
        return false;
    };
    let Some(parent) = socket.parent() else {
        return false;
    };
    let Some(name) = parent.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    o.get_database() == Some("supa_ecosystem_test")
        && o.get_username() == "supa_ecosystem_test"
        && o.get_port() == 55479
        && parent.parent() == Some(std::path::Path::new("/tmp"))
        && socket.file_name().is_some_and(|s| s == "socket")
        && name
            .strip_prefix("supa-ecosystem-test-")
            .is_some_and(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()))
}
async fn connect() -> Store {
    let url =
        std::env::var("SUPA_ECOSYSTEM_TEST_URL").expect("explicit_disposable_test_url_required");
    let o: PgConnectOptions = url.parse().unwrap();
    assert!(permitted(&o), "unexpected_disposable_target");
    let socket = o.get_socket().unwrap();
    assert_eq!(socket.canonicalize().unwrap(), *socket);
    let s = Store::connect(o.to_url_lossy().as_str()).await.unwrap();
    let (db,user,data,listen,address):(String,String,String,String,Option<String>)=sqlx::query_as("SELECT current_database(),current_user::text,current_setting('data_directory'),current_setting('listen_addresses'),inet_server_addr()::text")
        .fetch_one(s.pool()).await.unwrap();
    assert_eq!(db, "supa_ecosystem_test");
    assert_eq!(user, "supa_ecosystem_test");
    assert!(listen.is_empty());
    assert!(address.is_none());
    assert_eq!(
        PathBuf::from(data).canonicalize().unwrap(),
        socket
            .parent()
            .unwrap()
            .join("data")
            .canonicalize()
            .unwrap()
    );
    s
}
async fn agent(s: &Store, n: u128, owner: Uuid) -> Agent {
    let mut a = Agent::new(
        "private-name-sentinel".into(),
        Personality::default(),
        CycleConfig::default(),
        BTreeSet::new(),
    )
    .unwrap();
    a.id = id(n);
    a.created_at = (Utc::now() - Duration::seconds(5))
        .with_nanosecond(0)
        .unwrap();
    a.lifecycle = Lifecycle::Paused;
    s.create_owned(&a, owner).await.unwrap();
    sqlx::query("UPDATE activity SET simulated=true WHERE agent_id=$1")
        .bind(a.id)
        .execute(s.pool())
        .await
        .unwrap();
    a
}
fn purpose(a: &Agent, owner: Uuid, n: u128) -> PurposeRevision {
    PurposeRevision {
        id: id(n),
        agent_id: a.id,
        creator_id: owner,
        revision: 1,
        created_at: a.created_at,
        objective: "private-goal-sentinel".into(),
        permitted_tasks: vec![ServiceKind::Summary],
        success_criteria: vec!["private-customer-criteria".into()],
        excluded_actions: vec!["No money movement".into()],
        public_summary: None,
    }
}
fn consent(a: &Agent, n: u128, expected: u32) -> EcosystemConsentRequest {
    EcosystemConsentRequest {
        request_id: id(n),
        expected_revision: expected,
        public: true,
        revoked: false,
        valid_from: a.created_at,
        expires_at: Utc::now() + Duration::days(1),
        categories: vec!["agent_created".into()],
    }
}
async fn baseline(s: &Store) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('balances',(SELECT coalesce(sum(simulated_balance_microusd),0) FROM agents),'ledger',(SELECT count(*) FROM simulated_ledger),'entries',(SELECT count(*) FROM economic_entries),'holds',(SELECT count(*) FROM economic_holds),'live',(SELECT count(*) FROM live_sessions),'cycles',(SELECT count(*) FROM cycles),'wallets',(SELECT count(*) FROM wallet_identities))")
        .fetch_one(s.pool()).await.unwrap()
}
fn assert_payload(p: &Value) {
    assert_eq!(p["publishing_enabled"], false);
    assert_eq!(p["requires_semantic_review"], true);
    for key in [
        "private-name-sentinel",
        "private-goal-sentinel",
        "private-customer-criteria",
        "creator_id",
        "objective",
        "wallet",
        "private_key",
    ] {
        assert!(!p.to_string().contains(key));
    }
}
#[test]
fn rejects_unexpected_database_transport_port_and_primary_path() {
    let valid = "postgresql://supa_ecosystem_test@localhost/supa_ecosystem_test?host=/tmp/supa-ecosystem-test-00000000000000000000000000000000/socket&port=55479";
    assert!(permitted(&valid.parse().unwrap()));
    for bad in [
        valid.replace("55479", "5432"),
        valid.replace("/tmp/", "/home/pikoo/"),
        valid.replace("supa_ecosystem_test?", "primary?"),
        "postgresql://supa_ecosystem_test@localhost/supa_ecosystem_test?port=55479".into(),
        valid.replace("00000000000000000000000000000000", "unexpected"),
    ] {
        assert!(!permitted(&bad.parse().unwrap()));
    }
}
#[tokio::test]
#[ignore = "requires the explicit newly created socket-only ecosystem test cluster"]
async fn ecosystem_lifecycle() {
    let s = connect().await;
    s.migrate().await.unwrap();
    sqlx::raw_sql(include_str!("../../../deployment/ecosystem-social.sql"))
        .execute(s.pool())
        .await
        .unwrap();
    let owner = id(2);
    let operator = id(3);
    let viewer = id(4);
    let outsider = id(5);
    let a = agent(&s, 1, owner).await;
    s.grant_access(a.id, owner, operator, Role::Operator)
        .await
        .unwrap();
    s.grant_access(a.id, owner, viewer, Role::Viewer)
        .await
        .unwrap();
    let before = baseline(&s).await;
    let p = purpose(&a, owner, 20);
    let c = consent(&a, 21, 0);
    assert!(matches!(
        s.record_ecosystem_purpose(operator, &p).await,
        Err(Error::Denied)
    ));
    s.record_ecosystem_purpose(owner, &p).await.unwrap();
    s.record_ecosystem_purpose(owner, &p).await.unwrap();
    let mut changed = p.clone();
    changed.objective = "changed private objective".into();
    assert!(matches!(
        s.record_ecosystem_purpose(owner, &changed).await,
        Err(Error::Stale)
    ));
    for actor in [operator, viewer, outsider, Uuid::nil()] {
        assert!(s.record_ecosystem_consent(a.id, actor, &c).await.is_err());
    }
    assert_eq!(
        s.record_ecosystem_consent(a.id, owner, &c).await.unwrap(),
        1
    );
    assert_eq!(
        s.record_ecosystem_consent(a.id, owner, &c).await.unwrap(),
        1
    );
    for actor in [viewer, outsider, Uuid::nil()] {
        assert!(
            s.prepare_ecosystem_social(a.id, actor, id(100))
                .await
                .is_err()
        );
    }
    let first = s
        .prepare_ecosystem_social(a.id, owner, id(101))
        .await
        .unwrap();
    assert_payload(&first);
    assert_eq!(first["events"].as_array().unwrap().len(), 1);
    assert!(
        first["events"][0]["public_fact"]
            .as_str()
            .unwrap()
            .contains("In a simulation")
    );
    assert_eq!(
        first,
        s.prepare_ecosystem_social(a.id, owner, id(101))
            .await
            .unwrap()
    );
    assert!(matches!(
        s.prepare_ecosystem_social(a.id, operator, id(101)).await,
        Err(Error::Stale)
    ));
    assert!(
        s.prepare_ecosystem_social(a.id, operator, id(102))
            .await
            .unwrap()["events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for code in ["completed", "accept_evaluation_only"] {
        sqlx::query("INSERT INTO activity(id,agent_id,kind,code,simulated) VALUES($1,$2,'work_review',$3,true)").bind(Uuid::new_v4()).bind(a.id).bind(code).execute(s.pool()).await.unwrap();
    }
    assert!(
        s.prepare_ecosystem_social(a.id, owner, id(103))
            .await
            .unwrap()["events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    sqlx::query("UPDATE activity SET simulated=false WHERE agent_id=$1 AND kind='lifecycle'")
        .bind(a.id)
        .execute(s.pool())
        .await
        .unwrap();
    assert!(matches!(
        s.prepare_ecosystem_social(a.id, owner, id(101)).await,
        Err(Error::Stale)
    ));
    sqlx::query("UPDATE activity SET simulated=true WHERE agent_id=$1 AND kind='lifecycle'")
        .bind(a.id)
        .execute(s.pool())
        .await
        .unwrap();
    for table in [
        "sources",
        "preparations",
        "prepared_events",
        "consents",
        "purposes",
    ] {
        assert!(
            sqlx::query(&format!("DELETE FROM ecosystem_social.{table}"))
                .execute(s.pool())
                .await
                .is_err()
        );
    }
    let mut revoke = consent(&a, 22, 1);
    revoke.revoked = true;
    assert_eq!(
        s.record_ecosystem_consent(a.id, owner, &revoke)
            .await
            .unwrap(),
        2
    );
    assert!(matches!(
        s.prepare_ecosystem_social(a.id, owner, id(101)).await,
        Err(Error::Denied)
    ));
    assert_eq!(
        s.record_ecosystem_consent(a.id, owner, &consent(&a, 23, 2))
            .await
            .unwrap(),
        3
    );
    assert!(matches!(
        s.prepare_ecosystem_social(a.id, owner, id(101)).await,
        Err(Error::Stale)
    ));
    assert!(
        s.prepare_ecosystem_social(a.id, owner, id(104))
            .await
            .unwrap()["events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut wrong = consent(&a, 24, 0);
    assert!(matches!(
        s.record_ecosystem_consent(a.id, owner, &wrong).await,
        Err(Error::Stale)
    ));
    wrong.categories = vec!["private_memory".into()];
    assert!(matches!(
        s.record_ecosystem_consent(a.id, owner, &wrong).await,
        Err(Error::Invalid)
    ));
    let mut raw = serde_json::to_value(&wrong).unwrap();
    raw["actor"] = json!(owner);
    assert!(serde_json::from_value::<EcosystemConsentRequest>(raw).is_err());
    sqlx::query("DELETE FROM agent_access WHERE agent_id=$1 AND principal_id=$2")
        .bind(a.id)
        .bind(operator)
        .execute(s.pool())
        .await
        .unwrap();
    assert!(matches!(
        s.prepare_ecosystem_social(a.id, operator, id(102)).await,
        Err(Error::Denied)
    ));
    let other = agent(&s, 11, owner).await;
    s.record_ecosystem_purpose(owner, &purpose(&other, owner, 30))
        .await
        .unwrap();
    s.record_ecosystem_consent(other.id, owner, &consent(&other, 31, 0))
        .await
        .unwrap();
    assert!(matches!(
        s.prepare_ecosystem_social(other.id, owner, id(101)).await,
        Err(Error::Stale)
    ));
    let (one, two) = tokio::join!(
        s.prepare_ecosystem_social(other.id, owner, id(201)),
        s.prepare_ecosystem_social(other.id, owner, id(202))
    );
    let sizes = [
        one.unwrap()["events"].as_array().unwrap().len(),
        two.unwrap()["events"].as_array().unwrap().len(),
    ];
    assert_eq!(sizes.iter().sum::<usize>(), 1);
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM ecosystem_social.prepared_events e JOIN ecosystem_social.sources s ON s.source_id=e.source_id WHERE s.agent_id=$1").bind(other.id).fetch_one(s.pool()).await.unwrap();
    assert_eq!(count, 1);
    let mut forged = s
        .prepare_ecosystem_social(a.id, owner, id(401))
        .await
        .unwrap();
    forged["guidance"] = json!("untrusted-guidance");
    sqlx::query("INSERT INTO ecosystem_social.preparations(request_id,agent_id,actor,consent_revision,payload,payload_sha256,expires_at) VALUES($1,$2,$3,3,$4,encode(sha256(convert_to(public.work_review_canonical($4::jsonb),'UTF8')),'hex'),clock_timestamp()+interval '30 seconds')")
        .bind(id(402)).bind(a.id).bind(owner).bind(forged).execute(s.pool()).await.unwrap();
    assert!(matches!(
        s.prepare_ecosystem_social(a.id, owner, id(402)).await,
        Err(Error::Stale)
    ));
    assert_eq!(baseline(&s).await, before);
    for id in [a.id, other.id] {
        let a = s.get(id).await.unwrap();
        assert_eq!(a.lifecycle, Lifecycle::Paused);
        assert!(a.capabilities.is_empty());
    }
    let states: Vec<(String, bool)> =
        sqlx::query_as("SELECT status,publishing_enabled FROM ecosystem_social.preparations")
            .fetch_all(s.pool())
            .await
            .unwrap();
    assert!(
        states
            .iter()
            .all(|(s, p)| s == "prepared_requires_review" && !*p)
    );
    s.pool().close().await;
}
#[tokio::test]
#[ignore = "requires restart of the same disposable cluster after lifecycle"]
async fn ecosystem_recovery() {
    let s = connect().await;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM ecosystem_social.prepared_events")
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(before, 2);
    let p = s
        .prepare_ecosystem_social(id(1), id(2), id(301))
        .await
        .unwrap();
    assert_payload(&p);
    assert!(p["events"].as_array().unwrap().is_empty());
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM ecosystem_social.prepared_events")
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(before, after);
    assert!(
        s.prepare_ecosystem_social(id(1), id(5), id(101))
            .await
            .is_err()
    );
    s.pool().close().await;
}
