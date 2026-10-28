//! Ownership is explicit. No migration, shared credential, or first login claims legacy agents.
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Viewer,
    Operator,
    Owner,
}
impl Role {
    pub fn permits(self, required: Self) -> bool {
        self as u8 >= required as u8
    }
    fn name(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Operator => "operator",
            Self::Owner => "owner",
        }
    }
}
impl Store {
    pub async fn require_access(&self, agent: Uuid, principal: Uuid, required: Role) -> Result<()> {
        let role: Option<String> = sqlx::query_scalar("SELECT x.role FROM agent_access x WHERE x.agent_id=$1 AND x.principal_id=$2 AND EXISTS(SELECT 1 FROM agent_access o WHERE o.agent_id=x.agent_id AND o.role='owner')")
            .bind(agent).bind(principal).fetch_optional(&self.pool).await.map_err(db)?;
        let role = match role.as_deref() {
            Some("owner") => Role::Owner,
            Some("operator") => Role::Operator,
            Some("viewer") => Role::Viewer,
            _ => return Err(Error::Denied),
        };
        if !role.permits(required) {
            return Err(Error::Denied);
        }
        Ok(())
    }
    /// Explicit offline provisioning only. Never invoked by a migration or login.
    pub async fn establish_owner(&self, agent: Uuid, principal: Uuid) -> Result<()> {
        if principal.is_nil() {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        if sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM agent_access WHERE agent_id=$1)",
        )
        .bind(agent)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?
        {
            return Err(Error::Denied);
        }
        assign(&mut tx, agent, principal, Role::Owner, principal).await?;
        tx.commit().await.map_err(db)
    }
    pub async fn grant_access(
        &self,
        agent: Uuid,
        actor: Uuid,
        target: Uuid,
        role: Role,
    ) -> Result<()> {
        if target.is_nil() || target == actor || role == Role::Owner {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        let owner: Option<Uuid> = sqlx::query_scalar(
            "SELECT principal_id FROM agent_access WHERE agent_id=$1 AND role='owner'",
        )
        .bind(agent)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if owner != Some(actor) {
            return Err(Error::Denied);
        }
        assign(&mut tx, agent, target, role, actor).await?;
        tx.commit().await.map_err(db)
    }
    pub async fn accessible_agents(&self, principal: Uuid) -> Result<Vec<Agent>> {
        let values:Vec<Value>=sqlx::query_scalar("SELECT a.data FROM agents a JOIN agent_access x ON x.agent_id=a.id WHERE x.principal_id=$1 AND EXISTS(SELECT 1 FROM agent_access o WHERE o.agent_id=a.id AND o.role='owner') ORDER BY a.id LIMIT 100").bind(principal).fetch_all(&self.pool).await.map_err(db)?;
        values.into_iter().map(decode).collect()
    }
}
pub(super) async fn assign(
    tx: &mut Tx<'_>,
    agent: Uuid,
    target: Uuid,
    role: Role,
    actor: Uuid,
) -> Result<()> {
    sqlx::query("INSERT INTO agent_access(agent_id,principal_id,role,granted_by) VALUES($1,$2,$3,$4) ON CONFLICT(agent_id,principal_id) DO UPDATE SET role=excluded.role,granted_by=excluded.granted_by,granted_at=now()")
        .bind(agent).bind(target).bind(role.name()).bind(actor).execute(&mut **tx).await.map_err(db)?;
    sqlx::query(
        "INSERT INTO access_audit(agent_id,principal_id,role,approved_by) VALUES($1,$2,$3,$4)",
    )
    .bind(agent)
    .bind(target)
    .bind(role.name())
    .bind(actor)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}
