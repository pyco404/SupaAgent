//! Process liveness is independent of authorization and of cycle lease validity.
use super::*;
impl Store {
    pub async fn register_worker(
        &self,
        kind: &str,
        enabled: bool,
        poll_seconds: i64,
    ) -> Result<Uuid> {
        if !matches!(kind, "api" | "worker") || !(1..=3600).contains(&poll_seconds) {
            return Err(Error::Invalid);
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO worker_health(id,kind,enabled,poll_seconds) VALUES($1,$2,$3,$4)")
            .bind(id)
            .bind(kind)
            .bind(enabled)
            .bind(poll_seconds)
            .execute(&self.pool)
            .await
            .map_err(db)?;
        Ok(id)
    }
    pub async fn heartbeat(&self, worker: Uuid) -> Result<()> {
        let n=sqlx::query("UPDATE worker_health SET heartbeat_at=now() WHERE id=$1 AND stopped_at IS NULL AND heartbeat_at>clock_timestamp()-interval '15 seconds'").bind(worker).execute(&self.pool).await.map_err(db)?.rows_affected();
        if n != 1 {
            return Err(Error::Stale);
        }
        Ok(())
    }
    pub async fn worker_tick(
        &self,
        worker: Uuid,
        finished: bool,
        error: Option<&Error>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        live_worker(&mut tx, worker).await?;
        if finished {
            sqlx::query(
                "UPDATE worker_health SET tick_finished_at=now(),last_error=$2 WHERE id=$1",
            )
            .bind(worker)
            .bind(error.map(ToString::to_string))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        } else {
            sqlx::query("UPDATE worker_health SET tick_started_at=now() WHERE id=$1")
                .bind(worker)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        tx.commit().await.map_err(db)
    }
    pub async fn stop_worker(&self, worker: Uuid) -> Result<()> {
        sqlx::query("UPDATE worker_health SET stopped_at=now() WHERE id=$1 AND stopped_at IS NULL")
            .bind(worker)
            .execute(&self.pool)
            .await
            .map_err(db)?;
        Ok(())
    }
    pub async fn execution_health(&self, agent: Uuid) -> Result<Value> {
        // Agent lock provides one snapshot consistent with lifecycle and lease changes.
        let mut tx = self.pool.begin().await.map_err(db)?;
        let a = locked(&mut tx, agent).await?;
        let lease=sqlx::query("SELECT id,worker_id,lease_until,lease_until>clock_timestamp() AS valid FROM cycles WHERE agent_id=$1 AND status='running' ORDER BY started_at DESC LIMIT 1").bind(agent).fetch_optional(&mut *tx).await.map_err(db)?;
        let worker_id = lease
            .as_ref()
            .and_then(|r| r.get::<Option<Uuid>, _>("worker_id"));
        let w=sqlx::query("SELECT *, heartbeat_at>clock_timestamp()-interval '15 seconds' AND stopped_at IS NULL AS fresh, tick_started_at IS NOT NULL AND (tick_finished_at IS NULL OR tick_started_at>tick_finished_at) AND tick_started_at<clock_timestamp()-interval '30 seconds' AS delayed, tick_finished_at IS NULL OR tick_finished_at<clock_timestamp()-make_interval(secs=>poll_seconds+15) AS overdue FROM worker_health WHERE ($1::uuid IS NULL OR id=$1) ORDER BY (enabled AND heartbeat_at>clock_timestamp()-interval '15 seconds' AND stopped_at IS NULL) DESC,heartbeat_at DESC LIMIT 1").bind(worker_id).fetch_optional(&mut *tx).await.map_err(db)?;
        let recovery = a.lifecycle == Lifecycle::Suspended
            || lease.as_ref().is_some_and(|r| !r.get::<bool, _>("valid"));
        let fresh = w
            .as_ref()
            .is_some_and(|r| r.get::<bool, _>("fresh") && r.get::<bool, _>("enabled"));
        let delayed = w.as_ref().is_some_and(|r| {
            r.get::<bool, _>("delayed")
                || r.get::<bool, _>("overdue")
                || r.get::<Option<String>, _>("last_error").is_some()
        });
        let state = if recovery {
            "recovery_required"
        } else if !fresh {
            "unavailable"
        } else if delayed {
            "delayed"
        } else {
            "healthy"
        };
        let authorized:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_schedules WHERE agent_id=$1 AND enabled AND expires_at>clock_timestamp() AND EXISTS(SELECT 1 FROM schedule_authorizations h WHERE h.id=authorization_id AND h.approved_by IS NOT NULL))").bind(agent).fetch_one(&mut *tx).await.map_err(db)?;
        let running = lease.as_ref().is_some_and(|r| r.get::<bool, _>("valid"))
            && worker_id.is_some()
            && fresh
            && !recovery
            && authorized
            && a.lifecycle == Lifecycle::Active;
        let result = json!({"agent_id":agent,"status":state,"execution_running":running,"lifecycle":a.lifecycle,
            "worker_enabled":w.as_ref().is_some_and(|r|r.get::<bool,_>("enabled")),"checked_at":Utc::now(),"heartbeat_at":w.as_ref().map(|r|r.get::<chrono::DateTime<Utc>,_>("heartbeat_at")),
            "tick_finished_at":w.as_ref().and_then(|r|r.get::<Option<chrono::DateTime<Utc>>,_>("tick_finished_at")),
            "lease_until":lease.as_ref().map(|r|r.get::<chrono::DateTime<Utc>,_>("lease_until")),
            "cycle_id":lease.as_ref().map(|r|r.get::<Uuid,_>("id"))});
        tx.commit().await.map_err(db)?;
        Ok(result)
    }
}
pub(super) async fn live_worker(tx: &mut Tx<'_>, worker: Uuid) -> Result<()> {
    let valid:Option<bool>=sqlx::query_scalar("SELECT enabled AND stopped_at IS NULL AND heartbeat_at>clock_timestamp()-interval '15 seconds' FROM worker_health WHERE id=$1 FOR SHARE").bind(worker).fetch_optional(&mut **tx).await.map_err(db)?;
    if valid != Some(true) {
        return Err(Error::Stale);
    }
    Ok(())
}
