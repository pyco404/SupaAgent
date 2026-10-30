//! Epoch identity and independently approved, fenced read-only transitions. No budget provisioning.
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
        let audited: bool = sqlx::query_scalar("SELECT epoch_guard.verify_active($1,$2)")
            .bind(epoch.id)
            .bind(approval)
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;
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
        plan_text: &str,
    ) -> Result<ApplicationEpoch> {
        let database: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;
        if !database.starts_with("supa_cutover_test_")
            || actor.is_nil()
            || approval.is_nil()
            || revision < 0
            || plan_text.len() > 32768
            || serde_json::from_str::<Value>(plan_text).is_err()
        {
            return Err(Error::Denied);
        }
        // The independent database authority owns the immutable approval. Caller hashes are insufficient.
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx::query("SET TRANSACTION READ WRITE")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let value: Value =
            sqlx::query_scalar("SELECT epoch_guard.execute_transition($1,$2,'owner',$3,$4,$5,$6)")
                .bind(approval)
                .bind(actor)
                .bind(expected)
                .bind(revision)
                .bind(operation)
                .bind(plan_text)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        let result = serde_json::from_value(value).map_err(|_| Error::Database)?;
        tx.commit().await.map_err(db)?;
        Ok(result)
    }
}
