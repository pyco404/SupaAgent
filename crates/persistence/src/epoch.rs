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
