//! One process incarnation, never reused on restart. Expired heartbeats cannot revive it.
use std::time::Duration;
use supa_domain::Result;
use supa_persistence::Store;
use uuid::Uuid;
pub struct WorkerMonitor {
    pub id: Uuid,
    store: Store,
    task: tokio::task::JoinHandle<()>,
}
impl WorkerMonitor {
    pub async fn start(store: Store, kind: &str, enabled: bool, poll: i64) -> Result<Self> {
        let id = store.register_worker(kind, enabled, poll).await?;
        let heartbeat = store.clone();
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                if let Err(error) = heartbeat.heartbeat(id).await {
                    tracing::warn!(worker_id=%id,error_code=%error,"worker_heartbeat_fenced");
                    break;
                }
            }
        });
        Ok(Self { id, store, task })
    }
    pub async fn shutdown(self) -> Result<()> {
        self.task.abort();
        self.store.stop_worker(self.id).await
    }
}
