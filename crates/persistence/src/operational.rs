//! One-use database-authorized admission. Secrets exist only in this process's pool closure.
use super::*;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize)]
pub struct OperationalContext {
    pub epoch_id: Uuid,
    pub accounting_namespace: Uuid,
    pub principal_id: Uuid,
    pub agent_id: Uuid,
    pub scope: String,
    pub approval_id: Uuid,
}
impl Store {
    /// Acquire the permit fence before any agent/worker row lock, avoiding lock-order inversions.
    pub(super) async fn operational_transaction(&self) -> Result<Tx<'_>> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        if self.operational.is_some() {
            sqlx::query("SELECT epoch_guard.current_operation()")
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        Ok(tx)
    }
    pub fn operational_context(&self) -> Option<&OperationalContext> {
        self.operational.as_deref()
    }
    /// Only an independently issued, unused database approval admits a process.
    /// Reconnecting/restarting PostgreSQL does not replay admission.
    pub async fn admit_operation(
        &self,
        approval: Uuid,
        epoch: Uuid,
        namespace: Uuid,
    ) -> Result<Self> {
        if self.operational.is_some() || approval.is_nil() || epoch.is_nil() || namespace.is_nil() {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx::query("SET TRANSACTION READ WRITE")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let value: Value = sqlx::query_scalar("SELECT epoch_guard.claim_operation($1,$2,$3)")
            .bind(approval)
            .bind(epoch)
            .bind(namespace)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let secret = value
            .get("secret")
            .and_then(Value::as_str)
            .ok_or(Error::Denied)?
            .to_owned();
        let context: OperationalContext =
            serde_json::from_value(value).map_err(|_| Error::Denied)?;
        // Commit once. Ambiguous admission is never retried; independent review/new approval is required.
        tx.commit().await.map_err(db)?;
        let options = self
            .pool
            .connect_options()
            .as_ref()
            .clone()
            .options([("default_transaction_read_only", "off")]);
        let id = approval.to_string();
        let pool = PgPoolOptions::new().max_connections(8)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .after_connect(move |connection, _| {
                let id=id.clone(); let secret=secret.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('supa.operation_id',$1,false),set_config('supa.operation_secret',$2,false)")
                        .bind(id).bind(secret).execute(&mut *connection).await?;
                    sqlx::query("SELECT epoch_guard.current_operation()").execute(connection).await?;
                    Ok(())
                })
            }).connect_with(options).await.map_err(db)?;
        Ok(Self {
            pool,
            operational: Some(Arc::new(context)),
        })
    }
    /// Authorization is rechecked by SQL on each mutation and again at commit.
    pub async fn require_operation(
        &self,
        principal: Uuid,
        scope: &str,
        agent: Option<Uuid>,
    ) -> Result<()> {
        let context = self.operational_context().ok_or(Error::Denied)?;
        if context.principal_id != principal
            || context.scope != scope
            || agent.is_some_and(|id| id != context.agent_id)
        {
            return Err(Error::Denied);
        }
        sqlx::query("SELECT epoch_guard.current_operation()")
            .execute(&self.pool)
            .await
            .map_err(db)?;
        Ok(())
    }
}
