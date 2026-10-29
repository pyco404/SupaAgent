//! Identity only. There is deliberately no activation, adoption, or budget provisioning method.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplicationEpoch {
    pub id: Uuid,
    pub accounting_namespace: Uuid,
    pub label: String,
    pub kind: String,
    pub state: String,
    pub historical_database: String,
    #[serde(default)]
    pub revision: i64,
    #[serde(default)]
    pub approval_id: Option<Uuid>,
}
impl Store {
    pub async fn application_epoch(&self) -> Result<Option<ApplicationEpoch>> {
        // Read safely before migrations. Missing metadata is unidentified, never recovered.
        let exists: bool =
            sqlx::query_scalar("SELECT to_regclass('application_epoch') IS NOT NULL")
                .fetch_one(&self.pool)
                .await
                .map_err(db)?;
        if !exists {
            return Ok(None);
        }
        let value: Option<Value> = sqlx::query_scalar(
            "SELECT to_jsonb(e)-'singleton'-'created_at' FROM application_epoch e",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?;
        value
            .map(|v| serde_json::from_value(v).map_err(|_| Error::Database))
            .transpose()
    }
    pub async fn require_epoch_preview(&self, expected: Uuid) -> Result<ApplicationEpoch> {
        let epoch = self.application_epoch().await?.ok_or(Error::Denied)?;
        let frozen: String = sqlx::query_scalar("SHOW default_transaction_read_only")
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;
        if expected.is_nil()
            || epoch.id != expected
            || epoch.state != "prepared"
            || epoch.kind != "development"
            || epoch.historical_database != "unrecovered"
            || frozen != "on"
        {
            return Err(Error::Denied);
        }
        Ok(epoch)
    }
}

impl Store {
    pub async fn require_read_only_epoch(
        &self,
        expected: Uuid,
        mode: &str,
        approval: Option<Uuid>,
    ) -> Result<ApplicationEpoch> {
        if mode == "preview" {
            return self.require_epoch_preview(expected).await;
        }
        if mode != "active_read_only" {
            return Err(Error::Denied);
        }
        let epoch = self.application_epoch().await?.ok_or(Error::Denied)?;
        let frozen: String = sqlx::query_scalar("SHOW default_transaction_read_only")
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;
        if epoch.id != expected
            || epoch.state != "active_read_only"
            || epoch.historical_database != "unrecovered"
            || epoch.kind != "development"
            || epoch.revision < 1
            || approval.is_none()
            || epoch.approval_id != approval
            || frozen != "on"
        {
            return Err(Error::Denied);
        }
        let audited: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM epoch_transitions WHERE epoch_id=$1 AND revision=$2 AND approval_id=$3 AND operation='activate_read_only' AND to_state='active_read_only')")
            .bind(epoch.id).bind(epoch.revision).bind(approval).fetch_one(&self.pool).await.map_err(db)?;
        if !audited {
            return Err(Error::Denied);
        }
        Ok(epoch)
    }
    /// Review exercise only. This cannot transition the real development candidate.
    pub async fn exercise_epoch_transition(
        &self,
        expected: Uuid,
        actor: Uuid,
        approval: Uuid,
        revision: i64,
        operation: &str,
        plan_sha256: &str,
    ) -> Result<ApplicationEpoch> {
        let database: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;
        if !database.starts_with("supa_cutover_test_")
            || actor.is_nil()
            || approval.is_nil()
            || revision < 0
            || plan_sha256.len() != 64
            || !plan_sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(Error::Denied);
        }
        let next_revision = revision.checked_add(1).ok_or(Error::Invalid)?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let value: Value=sqlx::query_scalar("SELECT to_jsonb(e)-'singleton'-'created_at' FROM application_epoch e WHERE id=$1 FOR UPDATE")
            .bind(expected).fetch_optional(&mut *tx).await.map_err(db)?.ok_or(Error::Denied)?;
        let current: ApplicationEpoch =
            serde_json::from_value(value).map_err(|_| Error::Database)?;
        let next = match (operation, current.state.as_str()) {
            ("activate_read_only", "prepared") => "active_read_only",
            ("rollback", "active_read_only") => "prepared",
            _ => return Err(Error::Stale),
        };
        if current.revision != revision {
            return Err(Error::Stale);
        }
        let used: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM epoch_transitions WHERE approval_id=$1)",
        )
        .bind(approval)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if used {
            return Err(Error::Stale);
        }
        sqlx::query("INSERT INTO epoch_transitions(approval_id,epoch_id,revision,operation,approved_by,plan_sha256,from_state,to_state) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(approval).bind(expected).bind(revision+1).bind(operation).bind(actor).bind(plan_sha256).bind(&current.state).bind(next)
            .execute(&mut *tx).await.map_err(db)?;
        sqlx::query(
            "UPDATE application_epoch SET state=$2,revision=revision+1,approval_id=$3 WHERE id=$1",
        )
        .bind(expected)
        .bind(next)
        .bind(approval)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let mut result = current;
        result.state = next.into();
        result.revision = next_revision;
        result.approval_id = Some(approval);
        tx.commit().await.map_err(db)?;
        Ok(result)
    }
}
