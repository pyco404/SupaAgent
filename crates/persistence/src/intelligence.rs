use super::*;

impl Store {
    pub async fn reserve_inference(
        &self,
        id: Uuid,
        cycle: Uuid,
        purpose: &str,
        attempt: u8,
        policy: &InferencePolicy,
        simulated_cost: i64,
    ) -> Result<Uuid> {
        if !matches!(purpose, "plan" | "draft") {
            return Err(Error::Invalid);
        }
        let reservation = policy.reservation()?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let a = live(&mut tx, id, cycle).await?;
        let rows = sqlx::query("SELECT policy,usage,status FROM inference_calls WHERE cycle_id=$1")
            .bind(cycle)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut token_total = 0u64;
        for r in &rows {
            let p: InferencePolicy =
                serde_json::from_value(r.get(0)).map_err(|_| Error::Database)?;
            let usage: Option<Value> = r.get(1);
            let status: String = r.get(2);
            token_total += if status == "rejected" {
                0
            } else if let Some(u) = usage {
                serde_json::from_value::<TokenUsage>(u)
                    .map_err(|_| Error::Database)?
                    .total()
            } else {
                p.token_bound()
            };
        }
        if rows.len() >= a.config.max_provider_calls as usize
            || token_total.saturating_add(policy.token_bound()) > a.config.max_cycle_tokens
        {
            return Err(Error::ResourceBudget);
        }
        let call = Uuid::new_v4();
        if simulated_cost > 0 {
            ledger(
                &mut tx,
                id,
                Some(cycle),
                LedgerKind::Inference,
                simulated_cost,
                call,
            )
            .await?;
        }
        if policy.live {
            let budget = policy.budget_id.ok_or(Error::Invalid)?;
            sqlx::query(
                "INSERT INTO live_budgets(id,limit_microusd) VALUES($1,$2) ON CONFLICT DO NOTHING",
            )
            .bind(budget)
            .bind(policy.budget_limit_microusd)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let n=sqlx::query("UPDATE live_budgets SET committed_microusd=committed_microusd+$2 WHERE id=$1 AND limit_microusd=$3 AND committed_microusd+$2<=limit_microusd").bind(budget).bind(reservation).bind(policy.budget_limit_microusd).execute(&mut *tx).await.map_err(db)?.rows_affected();
            if n != 1 {
                return Err(Error::LiveBudget);
            }
        }
        sqlx::query("INSERT INTO inference_calls(id,agent_id,cycle_id,purpose,attempt,policy,budget_id,reserved_microusd,accounted_microusd,status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$8,'reserved')").bind(call).bind(id).bind(cycle).bind(purpose).bind(attempt as i32).bind(serde_json::to_value(policy).map_err(|_|Error::Invalid)?).bind(policy.budget_id.filter(|_|policy.live)).bind(reservation).execute(&mut *tx).await.map_err(db)?;
        event(
            &mut tx,
            id,
            Some(cycle),
            "inference",
            "reserved",
            !policy.live,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(call)
    }
    /// Billing evidence may be recorded after a pause; this does not authorize tool effects.
    pub async fn finish_inference(
        &self,
        id: Uuid,
        call: Uuid,
        usage: Option<TokenUsage>,
        failure: Option<&Error>,
    ) -> Result<()> {
        if let Some(u) = usage {
            u.validate()?;
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, id).await?;
        let r=sqlx::query("SELECT policy,reserved_microusd,cycle_id,status FROM inference_calls WHERE id=$1 AND agent_id=$2 FOR UPDATE").bind(call).bind(id).fetch_optional(&mut *tx).await.map_err(db)?.ok_or(Error::NotFound)?;
        if r.get::<String, _>(3) != "reserved" {
            return Err(Error::Stale);
        }
        let p: InferencePolicy = serde_json::from_value(r.get(0)).map_err(|_| Error::Database)?;
        let reserved: i64 = r.get(1);
        let cycle: Uuid = r.get(2);
        let rejected = matches!(failure, Some(Error::ProviderHttp(429)));
        let accounted = if rejected {
            0
        } else if let Some(u) = usage {
            p.cost(u)?
        } else {
            reserved
        };
        if let Some(budget) = p.budget_id.filter(|_| p.live) {
            sqlx::query(
                "UPDATE live_budgets SET committed_microusd=committed_microusd+$2 WHERE id=$1",
            )
            .bind(budget)
            .bind(accounted - reserved)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let status = if rejected {
            "rejected"
        } else if p.live && usage.is_none() {
            "unknown"
        } else if failure.is_some() {
            "failed"
        } else {
            "succeeded"
        };
        sqlx::query("UPDATE inference_calls SET status=$2,usage=$3,usage_source=$4,accounted_microusd=$5,failure_code=$6,finished_at=now() WHERE id=$1").bind(call).bind(status).bind(usage.map(|u|serde_json::to_value(u).expect("token usage serializes"))).bind(if usage.is_some(){"provider_reported"}else{"unavailable"}).bind(accounted).bind(failure.map(ToString::to_string)).execute(&mut *tx).await.map_err(db)?;
        event(&mut tx, id, Some(cycle), "inference", status, !p.live).await?;
        tx.commit().await.map_err(db)?;
        if accounted > reserved && p.live {
            return Err(Error::LiveBudget);
        }
        Ok(())
    }
    pub async fn completed_cycle_count(&self, id: Uuid) -> Result<i64> {
        sqlx::query_scalar("SELECT count(*) FROM cycles WHERE agent_id=$1 AND status='completed'")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(db)
    }
    pub async fn set_goal_priority(&self, id: Uuid, goal: Uuid, priority: u8) -> Result<()> {
        if priority > 10 {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        let mut a = locked(&mut tx, id).await?;
        if a.lifecycle == Lifecycle::Terminated {
            return Err(Error::Inactive);
        }
        a.goals
            .iter_mut()
            .find(|g| g.id == goal)
            .ok_or(Error::NotFound)?
            .priority = priority;
        save(&mut tx, &a).await?;
        tx.commit().await.map_err(db)
    }
    pub async fn evaluate_goal(
        &self,
        id: Uuid,
        cycle: Uuid,
        action: Uuid,
        evaluation: &GoalEvaluation,
        evidence: &[Uuid],
    ) -> Result<()> {
        if evaluation.progress_bps > 10000 || evidence.is_empty() || evidence.len() > 8 {
            return Err(Error::Invalid);
        }
        clean_text(&evaluation.assessment, 500)?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let mut a = live(&mut tx, id, cycle).await?;
        let verified = events_by_ids_tx(&mut tx, id, evidence).await?;
        if evidence
            .iter()
            .any(|id| !verified.iter().any(|e| e.id == *id))
        {
            return Err(Error::Invalid);
        }
        let goal = a
            .goals
            .iter_mut()
            .find(|g| g.id == evaluation.goal_id && !g.completed)
            .ok_or(Error::NotFound)?;
        goal.suggested_progress_bps = evaluation.progress_bps;
        let n=sqlx::query("UPDATE actions SET status='succeeded',finished_at=now() WHERE id=$1 AND cycle_id=$2 AND status='running' AND authorization_outcome='allowed'").bind(action).bind(cycle).execute(&mut *tx).await.map_err(db)?.rows_affected();
        if n != 1 {
            return Err(Error::Stale);
        }
        sqlx::query("INSERT INTO goal_evaluations(id,agent_id,cycle_id,action_id,data,evidence_event_ids) VALUES($1,$2,$3,$4,$5,$6)").bind(Uuid::new_v4()).bind(id).bind(cycle).bind(action).bind(serde_json::to_value(evaluation).map_err(|_|Error::Invalid)?).bind(evidence).execute(&mut *tx).await.map_err(db)?;
        save(&mut tx, &a).await?;
        event(&mut tx, id, Some(cycle), "goal", "evaluated", false).await?;
        tx.commit().await.map_err(db)
    }
    pub async fn conserve(&self, id: Uuid, cycle: Uuid, action: Uuid) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let a = live(&mut tx, id, cycle).await?;
        let n=sqlx::query("UPDATE actions SET status='succeeded',finished_at=now() WHERE id=$1 AND cycle_id=$2 AND status='running' AND authorization_outcome='allowed'").bind(action).bind(cycle).execute(&mut *tx).await.map_err(db)?.rows_affected();
        if n != 1 {
            return Err(Error::Stale);
        }
        let seconds = (a.config.interval_seconds.saturating_mul(2)).clamp(60, 86400) as f64;
        sqlx::query("UPDATE agents SET next_cycle_at=GREATEST(next_cycle_at,now()+make_interval(secs => $2)) WHERE id=$1").bind(id).bind(seconds).execute(&mut *tx).await.map_err(db)?;
        event(&mut tx, id, Some(cycle), "economy", "cycle_deferred", true).await?;
        tx.commit().await.map_err(db)
    }
    pub async fn emergency_stop(&self, id: Uuid, reason: &str) -> Result<()> {
        clean_text(reason, 500)?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let mut a = locked(&mut tx, id).await?;
        if a.lifecycle == Lifecycle::Terminated {
            return Err(Error::Inactive);
        }
        if a.lifecycle != Lifecycle::Suspended {
            transition(&mut tx, &mut a, Lifecycle::Suspended, reason, "operator").await?;
        }
        sqlx::query("UPDATE wallet_identities SET policy=jsonb_set(policy,'{locked}','true'::jsonb) WHERE agent_id=$1").bind(id).execute(&mut *tx).await.map_err(db)?;
        // Fence even a paused-then-resumed worker immediately; uncertain billing stays reserved.
        let cycles:Vec<Uuid>=sqlx::query_scalar("UPDATE cycles SET lease_until=now()-interval '1 second' WHERE agent_id=$1 AND status='running' RETURNING id").bind(id).fetch_all(&mut *tx).await.map_err(db)?;
        if !cycles.is_empty() {
            recover_agent(&mut tx, id).await?;
        }
        tx.commit().await.map_err(db)
    }
}

impl Store {
    pub async fn verified_events(&self, id: Uuid, ids: &[Uuid]) -> Result<Vec<Event>> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let result = events_by_ids_tx(&mut tx, id, ids).await?;
        tx.commit().await.map_err(db)?;
        Ok(result)
    }
}
