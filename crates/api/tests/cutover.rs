//! Synthetic transition approvals in a disposable database. No agents, grants, or budgets.
use serde_json::json;
use supa_domain::Error;
use supa_persistence::Store;
use uuid::Uuid;
#[tokio::test]
#[ignore = "requires local TEST_DATABASE_URL; creates only disposable review metadata"]
async fn reviewed_read_only_transition_and_rollback_are_fenced_and_append_only() {
    let base = std::env::var("TEST_DATABASE_URL").unwrap();
    assert!(base.contains("_test"));
    let admin = Store::connect(&base).await.unwrap();
    // Real candidate and generic test databases are forbidden transition targets.
    assert!(matches!(
        admin
            .exercise_epoch_transition(
                Uuid::new_v4(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                0,
                "activate_read_only",
                &"a".repeat(64)
            )
            .await,
        Err(Error::Denied)
    ));
    let db = format!("supa_cutover_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {db} TEMPLATE template0"))
        .execute(admin.pool())
        .await
        .unwrap();
    let (head, query) = base.split_once('?').unwrap_or((&base, ""));
    let prefix = head.rsplit_once('/').unwrap().0;
    let url = format!("{prefix}/{db}?{query}");
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let id = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let approval = Uuid::new_v4();
    sqlx::query("INSERT INTO application_epoch(id,accounting_namespace,label,kind,state,historical_database) VALUES($1,$2,'Disposable review fixture','development','prepared','unrecovered')").bind(id).bind(Uuid::new_v4()).execute(store.pool()).await.unwrap();
    sqlx::query("CREATE TABLE review_probe (id integer PRIMARY KEY,value jsonb NOT NULL)")
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO review_probe VALUES(1,$1)")
        .bind(json!({"fixture":"preserve across rollback"}))
        .execute(store.pool())
        .await
        .unwrap();
    let frozen_url = format!(
        "{url}{}options=-cdefault_transaction_read_only%3Don",
        if query.is_empty() { "" } else { "&" }
    );
    let frozen = Store::connect(&frozen_url).await.unwrap();
    assert!(
        frozen
            .require_read_only_epoch(id, "active_read_only", Some(approval))
            .await
            .is_err()
    );
    assert!(matches!(
        store
            .exercise_epoch_transition(
                id,
                Uuid::nil(),
                approval,
                0,
                "activate_read_only",
                &"a".repeat(64)
            )
            .await,
        Err(Error::Denied)
    ));
    let plan_digest = "a".repeat(64);
    let (a, b) = tokio::join!(
        store.exercise_epoch_transition(id, actor, approval, 0, "activate_read_only", &plan_digest),
        store.exercise_epoch_transition(id, actor, approval, 0, "activate_read_only", &plan_digest)
    );
    assert_eq!(a.is_ok() as u8 + b.is_ok() as u8, 1);
    assert_eq!(
        frozen
            .require_read_only_epoch(id, "active_read_only", Some(approval))
            .await
            .unwrap()
            .revision,
        1
    );
    assert!(
        frozen
            .require_read_only_epoch(id, "active_read_only", Some(Uuid::new_v4()))
            .await
            .is_err()
    );
    assert!(
        frozen
            .require_read_only_epoch(Uuid::new_v4(), "active_read_only", Some(approval))
            .await
            .is_err()
    );
    assert!(
        store
            .require_read_only_epoch(id, "active_read_only", Some(approval))
            .await
            .is_err()
    );
    assert!(
        store
            .exercise_epoch_transition(id, actor, approval, 1, "rollback", &"a".repeat(64))
            .await
            .is_err()
    );
    assert!(
        store
            .exercise_epoch_transition(id, actor, Uuid::new_v4(), 0, "rollback", &"a".repeat(64))
            .await
            .is_err()
    );
    let rollback = store
        .exercise_epoch_transition(id, actor, Uuid::new_v4(), 1, "rollback", &"b".repeat(64))
        .await
        .unwrap();
    assert_eq!(rollback.state, "prepared");
    assert_eq!(rollback.revision, 2);
    assert!(
        frozen
            .require_read_only_epoch(id, "active_read_only", Some(approval))
            .await
            .is_err()
    );
    assert_eq!(frozen.require_epoch_preview(id).await.unwrap().revision, 2);
    for sql in [
        "UPDATE epoch_transitions SET operation='rollback'",
        "DELETE FROM epoch_transitions",
        "TRUNCATE epoch_transitions",
    ] {
        assert!(sqlx::query(sql).execute(store.pool()).await.is_err());
    }
    let counts:(i64,i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM epoch_transitions),(SELECT count(*) FROM agents),(SELECT count(*) FROM agent_access),(SELECT count(*) FROM live_budgets),(SELECT count(*) FROM schedule_authorizations)").fetch_one(store.pool()).await.unwrap();
    assert_eq!(counts, (2, 0, 0, 0, 0));
    let probe: serde_json::Value = sqlx::query_scalar("SELECT value FROM review_probe WHERE id=1")
        .fetch_one(frozen.pool())
        .await
        .unwrap();
    assert_eq!(probe, json!({"fixture":"preserve across rollback"}));
    frozen.pool().close().await;
    store.pool().close().await;
    admin.pool().close().await;
}
