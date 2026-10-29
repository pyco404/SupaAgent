//! Independent provider accounting. Never creates, resets, or increases an existing budget.
use super::*;
impl Store {
    pub async fn live_sessions(&self, agent: Uuid) -> Result<Vec<LiveSession>> {
        let rows: Vec<Value> = sqlx::query_scalar(
            "SELECT CASE WHEN status='reserved' AND lease_until<clock_timestamp() THEN jsonb_set(jsonb_set(data,'{status}','\"unknown\"'::jsonb),'{failure_code}','\"interrupted_request_requires_review\"'::jsonb) ELSE data END FROM live_sessions WHERE agent_id=$1 ORDER BY created_at DESC LIMIT 20",
        )
        .bind(agent)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        rows.into_iter()
            .map(|v| serde_json::from_value(v).map_err(|_| Error::Database))
            .collect()
    }
    pub async fn reserve_live_session(
        &self,
        agent: Uuid,
        actor: Uuid,
        consent: &LiveConsent,
        policy: &InferencePolicy,
    ) -> Result<(LiveSession, Vec<Memory>)> {
        consent.validate()?;
        policy.validate()?;
        if !policy.live
            || consent.model != policy.model
            || consent.max_input_tokens != policy.input_token_bound
            || consent.max_output_tokens != policy.max_output_tokens
            || policy.reservation()? > consent.max_cost_microusd
        {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        let a = locked(&mut tx, agent).await?;
        live_owner(&mut tx, agent, actor).await?;
        consent.validate()?;
        if matches!(a.lifecycle, Lifecycle::Terminated | Lifecycle::Suspended)
            || !a.capabilities.contains(&Capability::ReadMemory)
        {
            return Err(Error::Denied);
        }
        let budget = policy.budget_id.ok_or(Error::Invalid)?;
        let r = sqlx::query(
            "SELECT limit_microusd,committed_microusd FROM live_budgets WHERE id=$1 FOR UPDATE",
        )
        .bind(budget)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(Error::LiveBudget)?;
        // Fence ambiguous prior requests, including historical evaluator evidence.
        let uncertain:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM live_sessions WHERE budget_id=$1 AND status IN ('reserved','unknown')) OR EXISTS(SELECT 1 FROM inference_calls WHERE budget_id=$1 AND status IN ('reserved','unknown'))").bind(budget).fetch_one(&mut *tx).await.map_err(db)?;
        if uncertain {
            return Err(Error::Busy);
        }
        let duplicate: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM live_sessions WHERE id=$1)")
                .bind(consent.request_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        if duplicate {
            return Err(Error::Stale);
        }
        let reserved = policy.reservation()?;
        if r.get::<i64, _>(0) != policy.budget_limit_microusd
            || r.get::<i64, _>(1) + reserved > r.get::<i64, _>(0)
        {
            return Err(Error::LiveBudget);
        }
        let memory = relevant_memories(&a, &consent.goal, 12);
        let session = LiveSession {
            id: consent.request_id,
            agent_id: agent,
            approved_by: actor,
            consent: consent.clone(),
            policy: policy.clone(),
            status: "reserved".into(),
            memory_ids: memory.iter().map(|m| m.id).collect(),
            proposal: None,
            usage: None,
            reserved_microusd: reserved,
            accounted_microusd: reserved,
            failure_code: None,
            metadata_requests: 0,
            generation_requests: 0,
            project_verified: false,
            tool_result: None,
            created_at: Utc::now(),
            finished_at: None,
        };
        sqlx::query("UPDATE live_budgets SET committed_microusd=committed_microusd+$2 WHERE id=$1")
            .bind(budget)
            .bind(reserved)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("INSERT INTO live_sessions(id,agent_id,approved_by,budget_id,status,lease_until,data) VALUES($1,$2,$3,$4,'reserved',clock_timestamp()+interval '65 seconds',$5)").bind(session.id).bind(agent).bind(actor).bind(budget).bind(serde_json::to_value(&session).map_err(|_|Error::Invalid)?).execute(&mut *tx).await.map_err(db)?;
        live_audit(&mut tx, session.id, actor, "authorize").await?;
        tx.commit().await.map_err(db)?;
        Ok((session, memory))
    }
    pub async fn live_request_stage(&self, agent: Uuid, id: Uuid, generation: bool) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let a = locked(&mut tx, agent).await?;
        if matches!(a.lifecycle, Lifecycle::Terminated | Lifecycle::Suspended)
            || !a.capabilities.contains(&Capability::ReadMemory)
        {
            return Err(Error::Denied);
        }
        let mut s = live_locked(&mut tx, agent, id).await?;
        live_owner(&mut tx, agent, s.approved_by).await?;
        if s.status != "reserved" || s.consent.expires_at <= Utc::now() {
            return Err(Error::Stale);
        }
        if generation {
            if s.generation_requests != 0 || s.metadata_requests != 1 {
                return Err(Error::Stale);
            }
            s.generation_requests = 1;
            s.project_verified = true;
        } else {
            if s.metadata_requests != 0 {
                return Err(Error::Stale);
            }
            s.metadata_requests = 1;
        }
        save_live(&mut tx, &s).await?;
        tx.commit().await.map_err(db)
    }
    pub async fn settle_live_session(
        &self,
        agent: Uuid,
        id: Uuid,
        proposal: Option<LiveProposal>,
        usage: Option<TokenUsage>,
        failure: Option<&Error>,
        generation_started: bool,
    ) -> Result<LiveSession> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        let mut s = live_locked(&mut tx, agent, id).await?;
        if s.status != "reserved" {
            return Err(Error::Stale);
        }
        let valid_usage = usage.filter(|u| u.validate().is_ok());
        let cost = valid_usage.and_then(|u| s.policy.cost(u).ok());
        let bounded = valid_usage.is_some_and(|u| {
            u.input_tokens <= s.consent.max_input_tokens
                && u.output_tokens <= s.consent.max_output_tokens
        }) && cost.is_some_and(|c| c <= s.reserved_microusd);
        let uncertain = generation_started && (!bounded || cost.is_none());
        let accounted = if !generation_started {
            0
        } else if uncertain {
            s.reserved_microusd.max(cost.unwrap_or(0))
        } else {
            cost.unwrap_or(s.reserved_microusd)
        };
        sqlx::query("UPDATE live_budgets SET committed_microusd=committed_microusd+$2 WHERE id=$1")
            .bind(s.policy.budget_id)
            .bind(accounted - s.accounted_microusd)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        s.accounted_microusd = accounted;
        s.usage = valid_usage;
        s.finished_at = Some(Utc::now());
        s.status = if uncertain {
            "unknown"
        } else if failure.is_some() {
            "failed"
        } else {
            "awaiting_approval"
        }
        .into();
        s.failure_code = if uncertain {
            Some("provider_charge_requires_review".into())
        } else {
            failure.map(ToString::to_string)
        };
        if s.status == "awaiting_approval" {
            s.proposal = proposal;
        }
        save_live(&mut tx, &s).await?;
        live_audit(&mut tx, id, s.approved_by, "settle").await?;
        tx.commit().await.map_err(db)?;
        Ok(s)
    }
    pub async fn approve_live_tools(
        &self,
        agent: Uuid,
        id: Uuid,
        actor: Uuid,
        approve: bool,
    ) -> Result<LiveSession> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let a = locked(&mut tx, agent).await?;
        live_owner(&mut tx, agent, actor).await?;
        let mut s = live_locked(&mut tx, agent, id).await?;
        if s.approved_by != actor
            || s.status != "awaiting_approval"
            || s.consent.expires_at <= Utc::now()
            || matches!(a.lifecycle, Lifecycle::Terminated | Lifecycle::Suspended)
        {
            return Err(Error::Stale);
        }
        let p = s.proposal.as_ref().ok_or(Error::Invalid)?;
        // Revalidate the exact persisted proposal and current capability at the effect boundary.
        let memory: Vec<_> = a
            .memory
            .iter()
            .filter(|m| s.memory_ids.contains(&m.id))
            .cloned()
            .collect();
        p.validate(&memory)?;
        if approve && !a.capabilities.contains(&Capability::ReadMemory) {
            return Err(Error::Denied);
        }
        s.tool_result = Some(if approve {
            json!({"operation":"read_memory","executed_actions":p.actions.len(),"memory_ids":if p.actions.is_empty(){vec![]}else{memory.iter().map(|m|m.id).collect::<Vec<_>>()},"result":"completed"})
        } else {
            json!({"executed_actions":0,"result":"declined"})
        });
        s.status = "completed".into();
        s.finished_at = Some(Utc::now());
        save_live(&mut tx, &s).await?;
        live_audit(
            &mut tx,
            id,
            actor,
            if approve {
                "approve_tools"
            } else {
                "decline_tools"
            },
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(s)
    }
    /// Read-only endpoints project expired work as unknown; this recovery also persists that fence.
    pub async fn recover_live_sessions(&self) -> Result<()> {
        sqlx::query("UPDATE live_sessions SET status='unknown',data=jsonb_set(jsonb_set(data,'{status}','\"unknown\"'::jsonb),'{failure_code}','\"interrupted_request_requires_review\"'::jsonb) WHERE status='reserved' AND lease_until<clock_timestamp()")
            .execute(&self.pool).await.map_err(db)?;
        Ok(())
    }
}
async fn live_owner(tx: &mut Tx<'_>, agent: Uuid, actor: Uuid) -> Result<()> {
    let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_access WHERE agent_id=$1 AND principal_id=$2 AND role='owner')").bind(agent).bind(actor).fetch_one(&mut **tx).await.map_err(db)?;
    if !allowed {
        return Err(Error::Denied);
    }
    Ok(())
}
async fn live_locked(tx: &mut Tx<'_>, agent: Uuid, id: Uuid) -> Result<LiveSession> {
    let v: Value =
        sqlx::query_scalar("SELECT data FROM live_sessions WHERE agent_id=$1 AND id=$2 FOR UPDATE")
            .bind(agent)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db)?
            .ok_or(Error::NotFound)?;
    serde_json::from_value(v).map_err(|_| Error::Database)
}
async fn save_live(tx: &mut Tx<'_>, s: &LiveSession) -> Result<()> {
    sqlx::query("UPDATE live_sessions SET status=$2,data=$3 WHERE id=$1")
        .bind(s.id)
        .bind(&s.status)
        .bind(serde_json::to_value(s).map_err(|_| Error::Invalid)?)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    Ok(())
}
async fn live_audit(tx: &mut Tx<'_>, id: Uuid, actor: Uuid, operation: &str) -> Result<()> {
    sqlx::query("INSERT INTO live_session_audit(session_id,actor,operation) VALUES($1,$2,$3)")
        .bind(id)
        .bind(actor)
        .bind(operation)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    Ok(())
}
