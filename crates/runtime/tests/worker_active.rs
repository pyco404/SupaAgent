//! Real process scheduling against a fresh isolated PostgreSQL schema; deterministic provider only.
use std::{
    process::{Child, Command, Stdio},
    time::Duration,
};
use supa_domain::{
    Agent, Capability, CycleConfig, LedgerKind, Lifecycle, Personality, ScheduleCommand,
    ScheduleOperation, SchedulePolicy,
};
use supa_persistence::Store;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
impl Process {
    async fn terminate(&mut self) {
        assert!(
            Command::new("kill")
                .arg("-TERM")
                .arg(self.0.id().to_string())
                .status()
                .unwrap()
                .success()
        );
        for _ in 0..100 {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(status.success(), "worker did not shut down gracefully");
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("worker did not handle SIGTERM");
    }
}
async fn start(url: &str, enabled: bool) -> Process {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut command = Command::new(env!("CARGO_BIN_EXE_supa-runtime"));
    command
        .env_clear()
        .env("DATABASE_URL", url)
        .env("PORT", port.to_string())
        .env("SUPA_PROVIDER", "offline")
        .env("SUPA_WORKER_POLL_SECONDS", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if enabled {
        command.env("SUPA_WORKER_ENABLED", "true");
    }
    let mut process = Process(command.spawn().unwrap());
    for _ in 0..100 {
        assert!(
            process.0.try_wait().unwrap().is_none(),
            "worker exited before readiness"
        );
        if let Ok(mut socket) = tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
            socket
                .write_all(b"GET /ready HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
            let mut response = String::new();
            tokio::time::timeout(Duration::from_secs(5), socket.read_to_string(&mut response))
                .await
                .unwrap()
                .unwrap();
            if response.starts_with("HTTP/1.1 200") {
                return process;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("worker readiness timed out");
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL, local sockets and Unix SIGTERM"]
#[cfg(unix)]
async fn offline_worker_runs_persists_and_restarts_paused_without_reactivation() {
    let base_url = std::env::var("TEST_DATABASE_URL").expect("disposable database required");
    let admin = Store::connect(&base_url).await.unwrap();
    // Isolate the entire scheduler so it cannot select or charge another test's identities.
    let schema = format!("worker_active_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(admin.pool())
        .await
        .unwrap();
    let separator = if base_url.contains('?') { '&' } else { '?' };
    let url = format!("{base_url}{separator}options=-csearch_path%3D{schema}");
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let agent = Agent::new(
        "Offline process integration".into(),
        Personality::default(),
        CycleConfig {
            scheduler_enabled: true,
            interval_seconds: 1,
            ..Default::default()
        },
        [Capability::Remember].into(),
    )
    .unwrap();
    store.create(&agent).await.unwrap();
    store
        .credit(agent.id, LedgerKind::Seed, 100_000, Uuid::new_v4())
        .await
        .unwrap();
    let goal = store
        .add_goal(agent.id, "Review verified worker outcomes".into())
        .await
        .unwrap();
    store
        .schedule_control(
            agent.id,
            &ScheduleCommand {
                request_id: Uuid::new_v4(),
                expected_revision: 0,
                operation: ScheduleOperation::Start,
                authorize_offline: true,
                reason: "Explicit bounded worker test".into(),
                policy: Some(SchedulePolicy {
                    goal_ids: vec![goal.id],
                    interval_seconds: 5,
                    max_cycles: 3,
                    simulated_budget_microusd: 3300,
                    valid_for_seconds: 3600,
                }),
            },
        )
        .await
        .unwrap();
    let mut worker = start(&url, true).await;
    let mut completed = false;
    for _ in 0..150 {
        if store.completed_cycle_count(agent.id).await.unwrap() >= 3 {
            completed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(completed, "three scheduled cycles did not complete");
    assert!(store.get(agent.id).await.unwrap().memory.len() >= 3);
    let actions = store.history(agent.id, "actions").await.unwrap();
    assert!(
        actions
            .iter()
            .filter(|action| action["status"] == "succeeded"
                && action["authorization_outcome"] == "allowed")
            .count()
            >= 3
    );
    assert!(store.financials(agent.id).await.unwrap().balance_microusd < 100_000);
    assert_eq!(
        store.get(agent.id).await.unwrap().lifecycle,
        Lifecycle::Paused
    );
    worker.terminate().await;
    let before_cycles = store.history(agent.id, "cycles").await.unwrap().len();
    let before_balance = store.financials(agent.id).await.unwrap().balance_microusd;
    let before_memories = store.get(agent.id).await.unwrap().memory.len();
    let mut restarted = start(&url, false).await;
    tokio::time::sleep(Duration::from_millis(2200)).await;
    let reconnected = Store::connect(&url).await.unwrap();
    assert_eq!(
        reconnected.get(agent.id).await.unwrap().lifecycle,
        Lifecycle::Paused
    );
    assert_eq!(
        reconnected.history(agent.id, "cycles").await.unwrap().len(),
        before_cycles
    );
    assert_eq!(
        reconnected
            .financials(agent.id)
            .await
            .unwrap()
            .balance_microusd,
        before_balance
    );
    assert_eq!(
        reconnected.get(agent.id).await.unwrap().memory.len(),
        before_memories
    );
    restarted.terminate().await;
    store
        .transition(
            agent.id,
            Lifecycle::Terminated,
            "isolated test complete",
            true,
        )
        .await
        .unwrap();
    reconnected.pool().close().await;
    store.pool().close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(admin.pool())
        .await
        .unwrap();
}
