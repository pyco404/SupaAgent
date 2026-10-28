//! Authorization and execution claims share the agent row lock with every tool write.
use super::*;

impl Store {
    pub async fn authorization_history(&self, id: Uuid) -> Result<Vec<Value>> {
        sqlx::query_scalar("SELECT to_jsonb(x) FROM (SELECT id,approved_by,approved_at,policy,expires_at,closed_at,cycles_started,reserved_microusd,legacy FROM schedule_authorizations WHERE agent_id=$1 ORDER BY approved_at DESC,id LIMIT 100) x").bind(id).fetch_all(&self.pool).await.map_err(db)
    }
    pub async fn schedule_status(&self, id: Uuid) -> Result<ScheduleStatus> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let agent = locked(&mut tx, id).await?;
        let result = status(&mut tx, &agent).await?;
        tx.commit().await.map_err(db)?;
        Ok(result)
    }
    pub async fn schedule_control(
        &self,
        id: Uuid,
        command: &ScheduleCommand,
    ) -> Result<ScheduleStatus> {
        self.schedule_control_as(id, command, None).await
    }
    pub async fn schedule_control_as(
        &self,
        id: Uuid,
        command: &ScheduleCommand,
        actor: Option<Uuid>,
    ) -> Result<ScheduleStatus> {
        command.validate()?;
        if command.operation == ScheduleOperation::Renew && actor.is_none() {
            return Err(Error::Denied);
        }
        let encoded = serde_json::to_value(command).map_err(|_| Error::Invalid)?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let mut agent = locked(&mut tx, id).await?;
        command.validate()?; // A queued renewal must still carry fresh consent after acquiring the lock.
        if let Some(actor) = actor {
            let role: Option<String> = sqlx::query_scalar(
                "SELECT role FROM agent_access WHERE agent_id=$1 AND principal_id=$2",
            )
            .bind(id)
            .bind(actor)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let allowed = match command.operation {
                ScheduleOperation::Start | ScheduleOperation::Renew => {
                    role.as_deref() == Some("owner")
                }
                _ => matches!(role.as_deref(), Some("owner" | "operator")),
            };
            if !allowed {
                return Err(Error::Denied);
            }
        }
        // Existing lifecycle actor enum remains compatible; concrete identity is in schedule_commands.
        let actor_name = "operator";
        if let Some(prior) = sqlx::query(
            "SELECT agent_id,command,approved_by FROM schedule_commands WHERE request_id=$1",
        )
        .bind(command.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        {
            if prior.get::<Uuid, _>(0) != id
                || prior.get::<Value, _>(1) != encoded
                || prior.get::<Option<Uuid>, _>(2) != actor
            {
                return Err(Error::Invalid);
            }
            // Return current truth; never replay an old start/resume after a later pause/stop.
            return status(&mut tx, &agent).await;
        }
        let current = status(&mut tx, &agent).await?;
        if current.revision != command.expected_revision {
            return Err(Error::Stale);
        }
        match command.operation {
            ScheduleOperation::Start | ScheduleOperation::Renew => {
                let renew = command.operation == ScheduleOperation::Renew;
                if renew != current.policy.is_some() || agent.lifecycle == Lifecycle::Active {
                    return Err(Error::InvalidTransition);
                }
                if sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS(SELECT 1 FROM cycles WHERE agent_id=$1 AND status='running')",
                )
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?
                {
                    return Err(Error::Busy);
                }
                let policy = command.policy.as_ref().ok_or(Error::Invalid)?;
                safe_permissions(&agent)?;
                validate_goals(&agent, policy)?;
                agent.lifecycle.transition(Lifecycle::Active, true)?;
                // Explicit start authorizes these documented fixed simulated costs and limits.
                agent.config = CycleConfig {
                    scheduler_enabled: true,
                    interval_seconds: policy.interval_seconds,
                    max_steps: 3,
                    timeout_seconds: 5,
                    max_provider_calls: 1,
                    ..CycleConfig::default()
                };
                // Close the old epoch with its consumed allowance intact before opening another.
                sqlx::query("UPDATE schedule_authorizations SET closed_at=now() WHERE agent_id=$1 AND closed_at IS NULL").bind(id).execute(&mut *tx).await.map_err(db)?;
                let authorization = command.request_id;
                let expiry:chrono::DateTime<Utc>=sqlx::query_scalar("INSERT INTO schedule_authorizations(id,agent_id,approved_by,policy,expires_at,legacy) VALUES($1,$2,$3,$4,now()+make_interval(secs=>$5),$6) RETURNING expires_at")
                    .bind(authorization).bind(id).bind(actor).bind(serde_json::to_value(policy).map_err(|_|Error::Invalid)?).bind(policy.valid_for_seconds as f64).bind(actor.is_none()).fetch_one(&mut *tx).await.map_err(db)?;
                sqlx::query("INSERT INTO agent_schedules(agent_id,enabled,policy,expires_at,authorization_id) VALUES($1,true,$2,$3,$4) ON CONFLICT(agent_id) DO UPDATE SET enabled=true,policy=excluded.policy,expires_at=excluded.expires_at,authorization_id=excluded.authorization_id,cycles_started=0,reserved_microusd=0,last_error=NULL")
                    .bind(id).bind(serde_json::to_value(policy).map_err(|_|Error::Invalid)?).bind(expiry).bind(authorization).execute(&mut *tx).await.map_err(db)?;
                transition(
                    &mut tx,
                    &mut agent,
                    Lifecycle::Active,
                    &command.reason,
                    actor_name,
                )
                .await?;
            }
            ScheduleOperation::Resume => {
                if actor.is_some() && !sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM schedule_authorizations h JOIN agent_schedules s ON s.authorization_id=h.id WHERE s.agent_id=$1 AND h.approved_by IS NOT NULL)").bind(id).fetch_one(&mut *tx).await.map_err(db)? { return Err(Error::Denied); }
                let policy = current.policy.as_ref().ok_or(Error::InvalidTransition)?;
                safe_configuration(&agent)?;
                validate_goals(&agent, policy)?;
                if current.expires_at.is_none_or(|expiry| expiry <= Utc::now())
                    || current.cycles_started >= policy.max_cycles
                    || current.reserved_microusd + 1100 > policy.simulated_budget_microusd
                {
                    return Err(Error::ResourceBudget);
                }
                agent.lifecycle.transition(Lifecycle::Active, true)?;
                agent.config.scheduler_enabled = true;
                transition(
                    &mut tx,
                    &mut agent,
                    Lifecycle::Active,
                    &command.reason,
                    actor_name,
                )
                .await?;
                sqlx::query(
                    "UPDATE agent_schedules SET enabled=true,last_error=NULL WHERE agent_id=$1",
                )
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            ScheduleOperation::Pause | ScheduleOperation::Stop => {
                let target = if command.operation == ScheduleOperation::Stop {
                    Lifecycle::Terminated
                } else {
                    Lifecycle::Paused
                };
                agent.lifecycle.transition(target, true)?;
                agent.config.scheduler_enabled = false;
                transition(&mut tx, &mut agent, target, &command.reason, actor_name).await?;
                sqlx::query("UPDATE agent_schedules SET enabled=false WHERE agent_id=$1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                sqlx::query("UPDATE cycles SET lease_until=now()-interval '1 second' WHERE agent_id=$1 AND status='running'").bind(id).execute(&mut *tx).await.map_err(db)?;
                recover_agent(&mut tx, id).await?;
                // Recovery fences work; the explicit operator lifecycle remains authoritative.
                agent = locked(&mut tx, id).await?;
            }
        }
        sqlx::query(
            "UPDATE agent_schedules SET revision=revision+1,updated_at=now() WHERE agent_id=$1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("INSERT INTO schedule_commands(request_id,agent_id,command,approved_by) VALUES($1,$2,$3,$4)")
            .bind(command.request_id)
            .bind(id)
            .bind(encoded)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        event(&mut tx, id, None, "schedule", "operator_control", false).await?;
        let result = status(&mut tx, &agent).await?;
        tx.commit().await.map_err(db)?;
        Ok(result)
    }
}
fn safe_permissions(agent: &Agent) -> Result<()> {
    if !agent.capabilities.iter().any(|cap| {
        matches!(
            cap,
            Capability::ReadMemory | Capability::Remember | Capability::EvaluateGoal
        )
    }) {
        return Err(Error::Denied);
    }
    Ok(())
}
fn safe_configuration(agent: &Agent) -> Result<()> {
    safe_permissions(agent)?;
    let c = &agent.config;
    if c.max_steps != 3
        || c.retry_budget != 0
        || c.max_provider_calls != 1
        || c.inference_cost_microusd != 1000
        || c.operating_cost_microusd != 100
        || c.timeout_seconds != 5
    {
        return Err(Error::Denied);
    }
    Ok(())
}
fn validate_goals(agent: &Agent, policy: &SchedulePolicy) -> Result<()> {
    if !policy.goal_ids.iter().all(|id| {
        agent
            .goals
            .iter()
            .any(|goal| goal.id == *id && !goal.completed)
    }) {
        return Err(Error::Invalid);
    }
    Ok(())
}
async fn status(tx: &mut Tx<'_>, agent: &Agent) -> Result<ScheduleStatus> {
    let row=sqlx::query("SELECT revision,enabled,policy,cycles_started,reserved_microusd,expires_at,last_error FROM agent_schedules WHERE agent_id=$1")
        .bind(agent.id).fetch_optional(&mut **tx).await.map_err(db)?;
    let mut view = ScheduleStatus {
        agent_id: agent.id,
        lifecycle: agent.lifecycle,
        revision: 0,
        enabled: false,
        policy: None,
        cycles_started: 0,
        reserved_microusd: 0,
        expires_at: None,
        next_cycle_at: sqlx::query_scalar("SELECT next_cycle_at FROM agents WHERE id=$1")
            .bind(agent.id)
            .fetch_one(&mut **tx)
            .await
            .map_err(db)?,
        last_error: None,
        mode: "offline".into(),
        simulated: true,
        paid_requests_enabled: false,
    };
    if let Some(row) = row {
        view.revision = row.get(0);
        view.enabled = row.get::<bool, _>(1)
            && agent.lifecycle == Lifecycle::Active
            && agent.config.scheduler_enabled;
        view.policy = Some(serde_json::from_value(row.get(2)).map_err(|_| Error::Database)?);
        view.cycles_started = row.get(3);
        view.reserved_microusd = row.get(4);
        view.expires_at = Some(row.get(5));
        view.last_error = row.get(6);
    }
    Ok(view)
}
/// Returns a durable boundary error after disabling execution; caller must commit it.
pub(super) async fn claim(
    tx: &mut Tx<'_>,
    agent: &mut Agent,
    scheduled: bool,
) -> Result<Option<Error>> {
    if scheduled
        && !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM agent_access WHERE agent_id=$1 AND role='owner')",
        )
        .bind(agent.id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?
    {
        return Err(Error::Denied);
    }
    let view = status(tx, agent).await?;
    let Some(policy) = view.policy else {
        return if scheduled {
            Err(Error::Denied)
        } else {
            Ok(None)
        };
    };
    if !view.enabled {
        return Err(Error::Inactive);
    }
    if let Err(error) = safe_configuration(agent) {
        disable(tx, agent, Lifecycle::Suspended, "schedule_policy_changed").await?;
        return Ok(Some(error));
    }
    let error = if view.expires_at.is_none_or(|expiry| expiry <= Utc::now()) {
        Some((
            Lifecycle::Paused,
            "authorization_expired",
            Error::ResourceBudget,
        ))
    } else if view.cycles_started >= policy.max_cycles
        || view.reserved_microusd + 1100 > policy.simulated_budget_microusd
    {
        Some((
            Lifecycle::Paused,
            "schedule_limit_reached",
            Error::ResourceBudget,
        ))
    } else if !agent
        .goals
        .iter()
        .any(|goal| !goal.completed && policy.goal_ids.contains(&goal.id))
    {
        Some((
            Lifecycle::Paused,
            "approved_goals_completed",
            Error::Inactive,
        ))
    } else if sqlx::query_scalar::<_, i64>(
        "SELECT simulated_balance_microusd FROM agents WHERE id=$1",
    )
    .bind(agent.id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)?
        < 1100
    {
        Some((
            Lifecycle::Suspended,
            "insufficient_balance",
            Error::InsufficientBalance,
        ))
    } else {
        None
    };
    if let Some((state, code, error)) = error {
        disable(tx, agent, state, code).await?;
        return Ok(Some(error));
    }
    sqlx::query("UPDATE agent_schedules SET cycles_started=cycles_started+1,reserved_microusd=reserved_microusd+1100,updated_at=now() WHERE agent_id=$1")
        .bind(agent.id).execute(&mut **tx).await.map_err(db)?;
    sqlx::query("UPDATE schedule_authorizations h SET cycles_started=s.cycles_started,reserved_microusd=s.reserved_microusd FROM agent_schedules s WHERE s.agent_id=$1 AND h.id=s.authorization_id").bind(agent.id).execute(&mut **tx).await.map_err(db)?;
    Ok(None)
}
pub(super) async fn disable(
    tx: &mut Tx<'_>,
    agent: &mut Agent,
    state: Lifecycle,
    code: &str,
) -> Result<()> {
    agent.config.scheduler_enabled = false;
    sqlx::query("UPDATE agent_schedules SET enabled=false,last_error=$2,revision=revision+1,updated_at=now() WHERE agent_id=$1")
        .bind(agent.id).bind(code).execute(&mut **tx).await.map_err(db)?;
    if agent.lifecycle == Lifecycle::Active {
        transition(tx, agent, state, code, "system").await?;
    } else {
        save(tx, agent).await?;
    }
    Ok(())
}
pub(super) async fn finish(
    tx: &mut Tx<'_>,
    agent: &mut Agent,
    failure: Option<&Error>,
) -> Result<()> {
    let view = status(tx, agent).await?;
    if let Some(policy) = view.policy {
        if let Some(error) = failure {
            disable(tx, agent, Lifecycle::Suspended, &error.to_string()).await?;
        } else if view.cycles_started >= policy.max_cycles
            || view.reserved_microusd + 1100 > policy.simulated_budget_microusd
        {
            disable(tx, agent, Lifecycle::Paused, "schedule_limit_reached").await?;
        }
    }
    Ok(())
}
