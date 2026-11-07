//! Private normalized image storage. No filesystem paths, network, launches or keys.
use super::*;
impl Store {
    pub async fn avatar_command(
        &self,
        actor: Option<Uuid>,
        operation: &str,
        body: &Value,
    ) -> Result<Value> {
        if self.operational.is_some() {
            return Err(Error::Denied);
        }
        sqlx::query_scalar("SELECT agent_avatars.command($1,$2,$3)")
            .bind(actor)
            .bind(operation)
            .bind(body)
            .fetch_one(&self.pool)
            .await
            .map_err(
                |e| match e.as_database_error().and_then(|e| e.code()).as_deref() {
                    Some("42501") => Error::Denied,
                    Some("40001") => Error::Stale,
                    Some("P0002") => Error::NotFound,
                    Some("54000" | "23514") => Error::ResourceBudget,
                    _ => db(e),
                },
            )
    }
}
