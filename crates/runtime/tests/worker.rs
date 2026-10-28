//! Process-level paused startup/restart check. Never selects a paid provider.
use std::{
    process::{Child, Command, Stdio},
    time::Duration,
};
use supa_domain::{Agent, CycleConfig, LedgerKind, Lifecycle, Personality};
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
async fn await_ready(port: u16, process: &mut Process) {
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
            if response.starts_with("HTTP/1.1 200") && response.contains("\"status\":\"ready\"") {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("worker readiness timed out");
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL and local sockets"]
async fn paused_worker_restart_never_runs_even_an_active_due_agent() {
    let url = std::env::var("TEST_DATABASE_URL").expect("disposable database required");
    let store = Store::connect(&url).await.unwrap();
    store.migrate().await.unwrap();
    let agent = Agent::new(
        "Paused worker restart fixture".into(),
        Personality::default(),
        CycleConfig::default(),
        Default::default(),
    )
    .unwrap();
    store.create(&agent).await.unwrap();
    store
        .credit(agent.id, LedgerKind::Seed, 10_000, Uuid::new_v4())
        .await
        .unwrap();
    store
        .transition(agent.id, Lifecycle::Active, "isolated process test", true)
        .await
        .unwrap();
    for _ in 0..2 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let mut process = Process(
            Command::new(env!("CARGO_BIN_EXE_supa-runtime"))
                .env_clear()
                .env("DATABASE_URL", &url)
                .env("PORT", port.to_string())
                // Paused startup must not even initialize this unsupported provider.
                .env("SUPA_PROVIDER", "openai")
                .env("SUPA_WORKER_POLL_SECONDS", "1")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        await_ready(port, &mut process).await;
        tokio::time::sleep(Duration::from_millis(1200)).await;
        assert!(store.history(agent.id, "cycles").await.unwrap().is_empty());
        assert_eq!(
            store.financials(agent.id).await.unwrap().balance_microusd,
            10_000
        );
        assert_eq!(
            store.get(agent.id).await.unwrap().lifecycle,
            Lifecycle::Active
        );
    }
    // Cleanly retire this test's identity so subsequent offline scheduler tests cannot select it.
    store
        .transition(
            agent.id,
            Lifecycle::Terminated,
            "isolated test complete",
            true,
        )
        .await
        .unwrap();
}
