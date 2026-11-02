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
        self.reserve_live_session_bound(agent, actor, consent, policy, None, None)
            .await
    }
    pub async fn reserve_live_work_session(
        &self,
        agent: Uuid,
        actor: Uuid,
        authorization: &LiveWorkConsent,
        policy: &InferencePolicy,
        provider_evidence: &str,
    ) -> Result<(LiveSession, Vec<Memory>)> {
        self.reserve_live_session_bound(
            agent,
            actor,
            &authorization.consent,
            policy,
            Some((authorization, provider_evidence)),
            None,
        )
        .await
    }
    pub async fn reserve_live_session_scoped(
        &self,
        agent: Uuid,
        actor: Uuid,
        consent: &LiveConsent,
        policy: &InferencePolicy,
        scope: &str,
    ) -> Result<(LiveSession, Vec<Memory>)> {
        self.reserve_live_session_bound(agent, actor, consent, policy, None, Some(scope))
            .await
    }
    pub async fn reserve_live_work_session_scoped(
        &self,
        agent: Uuid,
        actor: Uuid,
        authorization: &LiveWorkConsent,
        policy: &InferencePolicy,
        provider: (&str, &str),
    ) -> Result<(LiveSession, Vec<Memory>)> {
        self.reserve_live_session_bound(
            agent,
            actor,
            &authorization.consent,
            policy,
            Some((authorization, provider.0)),
            Some(provider.1),
        )
        .await
    }
    async fn reserve_live_session_bound(
        &self,
        agent: Uuid,
        actor: Uuid,
        consent: &LiveConsent,
        policy: &InferencePolicy,
        work: Option<(&LiveWorkConsent, &str)>,
        scope: Option<&str>,
    ) -> Result<(LiveSession, Vec<Memory>)> {
        if let Some(scope) = scope {
            clean_text(scope, 120)?;
        }
        writable_inference_budget(policy.budget_id)?;
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
        // A live budget cannot be repurposed for a different provider/model after use.
        let mismatched: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM live_sessions WHERE budget_id=$1 AND (data->'policy'->>'model' IS DISTINCT FROM $2 OR ($3::text IS NOT NULL AND data->>'provider_scope' IS DISTINCT FROM $3))) OR EXISTS(SELECT 1 FROM inference_calls WHERE budget_id=$1 AND policy->>'model' IS DISTINCT FROM $2)")
            .bind(budget).bind(&policy.model).bind(scope).fetch_one(&mut *tx).await.map_err(db)?;
        if mismatched {
            return Err(Error::LiveBudget);
        }
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
        let mut memory = relevant_memories(&a, &consent.goal, 12);
        let work_record = if let Some((authorization, provider_evidence)) = work {
            if !a.capabilities.contains(&Capability::EvaluateGoal) {
                return Err(Error::Denied);
            }
            let job = work_job(&mut tx, agent, actor, authorization).await?;
            let request = WorkRequest::parse(&job.input, job.spec.kind)?;
            authorization.validate(&request, job.spec.kind)?;
            if authorization.approved_tools.len() > a.config.max_steps as usize {
                return Err(Error::ResourceBudget);
            }
            let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM live_sessions WHERE agent_id=$1 AND status IN ('reserved','unknown') AND data->'work'->'authorization'->>'job_id'=$2)").bind(agent).bind(job.id.to_string()).fetch_one(&mut *tx).await.map_err(db)?;
            if pending {
                return Err(Error::Busy);
            }
            if authorization.use_memory {
                memory.retain(|m| {
                    m.kind == MemoryKind::VerifiedObservation
                        && !m.source_event_ids.is_empty()
                        && request
                            .objective
                            .split_whitespace()
                            .filter(|t| t.len() > 3)
                            .any(|t| m.lesson.to_lowercase().contains(&t.to_lowercase()))
                });
                memory.truncate(2);
                for m in &memory {
                    clean_text(&m.lesson, 1000)?;
                    validate_work_source(&m.lesson)?;
                    events_by_ids_tx(&mut tx, agent, &m.source_event_ids).await?;
                }
            } else {
                memory.clear();
            }
            if !matches!(provider_evidence, "mock" | "openai" | "anthropic") {
                return Err(Error::Denied);
            }
            Some(LiveWorkRecord {
                provider_evidence: provider_evidence.into(),
                authorization: authorization.clone(),
                request: request.clone(),
                kind: job.spec.kind,
                baseline: analyze_work(&request, job.spec.kind)?,
                report: None,
                tool_results: None,
            })
        } else {
            None
        };
        let session = LiveSession {
            provider_diagnostics: vec![],
            technical_failure_code: None,
            provider_scope: scope.map(str::to_owned),
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
            work: work_record,
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
        writable_inference_budget(s.policy.budget_id)?;
        live_owner(&mut tx, agent, s.approved_by).await?;
        if s.status != "reserved" || s.consent.expires_at <= Utc::now() {
            return Err(Error::Stale);
        }
        let lease_live: bool = sqlx::query_scalar(
            "SELECT lease_until>clock_timestamp() FROM live_sessions WHERE id=$1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !lease_live {
            return Err(Error::Stale);
        }
        if let Some(work) = &s.work {
            if !a.capabilities.contains(&Capability::EvaluateGoal) {
                return Err(Error::Denied);
            }
            work_job(&mut tx, agent, s.approved_by, &work.authorization).await?;
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
        self.settle_live_session_bound(
            agent,
            id,
            (proposal, None),
            usage,
            failure,
            generation_started,
        )
        .await
    }
    /// Append bounded evidence to an in-flight request only. Never touches budgets or old outcomes.
    pub async fn record_live_diagnostic(
        &self,
        agent: Uuid,
        id: Uuid,
        diagnostic: &ProviderDiagnostic,
    ) -> Result<()> {
        diagnostic.validate()?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let mut session = live_locked(&mut tx, agent, id).await?;
        writable_inference_budget(session.policy.budget_id)?;
        if session.status != "reserved"
            || session.provider_diagnostics.len() >= 8
            || match diagnostic.operation {
                ProviderOperation::Metadata => session.metadata_requests != 1,
                ProviderOperation::Generation => session.generation_requests != 1,
            }
        {
            return Err(Error::Stale);
        }
        session.provider_diagnostics.push(diagnostic.clone());
        save_live(&mut tx, &session).await?;
        tx.commit().await.map_err(db)
    }
    pub async fn settle_live_work_session(
        &self,
        agent: Uuid,
        id: Uuid,
        report: Option<WorkProposal>,
        usage: Option<TokenUsage>,
        failure: Option<&Error>,
        generation_started: bool,
    ) -> Result<LiveSession> {
        let proposal = report.as_ref().map(|r| LiveProposal {
            summary: r.summary.clone(),
            memory_ids: r.memory_ids.clone(),
            actions: vec![],
        });
        self.settle_live_session_bound(
            agent,
            id,
            (proposal, report),
            usage,
            failure,
            generation_started,
        )
        .await
    }
    async fn settle_live_session_bound(
        &self,
        agent: Uuid,
        id: Uuid,
        reports: (Option<LiveProposal>, Option<WorkProposal>),
        usage: Option<TokenUsage>,
        failure: Option<&Error>,
        generation_started: bool,
    ) -> Result<LiveSession> {
        let (proposal, work_report) = reports;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let current = locked(&mut tx, agent).await?;
        let mut s = live_locked(&mut tx, agent, id).await?;
        if s.status != "reserved" {
            return Err(Error::Stale);
        }
        let mut work_rejection = None;
        if let Some(report) = work_report.as_ref() {
            let work = s.work.as_ref().ok_or(Error::Invalid)?;
            let permitted = current
                .memory
                .iter()
                .filter(|m| s.memory_ids.contains(&m.id))
                .cloned()
                .collect::<Vec<_>>();
            let input = LiveWorkInput {
                request: work.request.clone(),
                kind: work.kind,
                approved_tools: work.authorization.approved_tools.clone(),
                memories: permitted,
                deterministic_findings: work.baseline["findings"].clone(),
            };
            work_rejection = report.validate(&input).err();
            let permission_changed = s.consent.expires_at <= Utc::now()
                || matches!(
                    current.lifecycle,
                    Lifecycle::Terminated | Lifecycle::Suspended
                )
                || !current.capabilities.contains(&Capability::ReadMemory)
                || !current.capabilities.contains(&Capability::EvaluateGoal)
                || live_owner(&mut tx, agent, s.approved_by).await.is_err()
                || work_job(&mut tx, agent, s.approved_by, &work.authorization)
                    .await
                    .is_err();
            if permission_changed && work_rejection.is_none() {
                work_rejection = Some(Error::Denied);
            }
        }
        let failure = work_rejection.as_ref().or(failure);
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
        // Financial uncertainty must not erase the independently safe technical cause.
        s.technical_failure_code = failure.map(ToString::to_string);
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
            if let Some(work) = s.work.as_mut() {
                let quality=work_report.as_ref().map(|report|serde_json::json!({"verbatim_observations_checked":report.findings.len(),"exact_citations_checked":report.findings.iter().map(|f|f.evidence.len()).sum::<usize>(),"integer_totals_checked":report.totals.len(),"semantic_quality":"human_review_pending","commercial_success_verified":false}));
                work.report = work_report;
                work.tool_results = Some(
                    serde_json::json!({"contract":"live_work_evaluation_v1","external_mutations":0,"commercial_delivery":false,"earned_revenue_atoms":0,"executed_tools":work.authorization.approved_tools,"quality_checks":quality,"verification":"Exact citations and integer totals checked; model interpretations require human review."}),
                );
            }
        }
        save_live(&mut tx, &s).await?;
        if let (Some(work), Some(error)) = (s.work.as_ref(), failure) {
            let created_at = Utc::now();
            let diagnostic = WorkDiagnostic {
                version: 1,
                response_schema: "bounded_work_proposal_v1".into(),
                retention_policy: "structure_only_v1".into(),
                provider: work.provider_evidence.clone(),
                model: s.policy.model.clone(),
                agent_id: s.agent_id,
                evaluation_id: work.authorization.job_id,
                request_id: s.id,
                input_sha256: work.authorization.input_sha256.clone(),
                rejection: WorkRejection::from_error(error),
                usage: s.usage,
                estimated_cost_microusd: s.accounted_microusd,
                outcome: s.status.clone(),
                provider_request_id: s
                    .provider_diagnostics
                    .iter()
                    .rev()
                    .find(|d| d.operation == ProviderOperation::Generation)
                    .and_then(|d| d.request_id.clone()),
                created_at,
                expires_at: created_at + chrono::Duration::days(7),
            };
            diagnostic.validate()?;
            // Private JSON extension: LiveSession's public serialization never includes it.
            // It commits atomically with the failed outcome and usage-based accounting.
            sqlx::query(
                "UPDATE live_sessions SET data=jsonb_set(data,'{work_diagnostic}',$2) WHERE id=$1",
            )
            .bind(s.id)
            .bind(serde_json::to_value(diagnostic).map_err(|_| Error::Invalid)?)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query("UPDATE live_sessions SET data=data-'work_diagnostic' WHERE agent_id=$1 AND (data->'work_diagnostic'->>'expires_at')::timestamptz<=clock_timestamp()")
                .bind(s.agent_id).execute(&mut *tx).await.map_err(db)?;
        }
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
        if s.work.is_some()
            || s.approved_by != actor
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
    /// No raw prompts/prose. Recheck current ownership in SQL; expiry fails closed.
    pub async fn work_diagnostic(
        &self,
        agent: Uuid,
        actor: Uuid,
        request: Uuid,
    ) -> Result<WorkDiagnostic> {
        self.require_access(agent, actor, Role::Owner).await?;
        let value:Option<Value> = sqlx::query_scalar("SELECT data->'work_diagnostic' FROM live_sessions WHERE agent_id=$1 AND id=$2 AND data ? 'work_diagnostic' AND (data->'work_diagnostic'->>'expires_at')::timestamptz>clock_timestamp() AND EXISTS(SELECT 1 FROM agent_access WHERE agent_id=$1 AND principal_id=$3 AND role='owner')")
            .bind(agent).bind(request).bind(actor).fetch_optional(&self.pool).await.map_err(db)?;
        let diagnostic: WorkDiagnostic =
            serde_json::from_value(value.ok_or(Error::NotFound)?).map_err(|_| Error::Database)?;
        diagnostic.validate()?;
        Ok(diagnostic)
    }
    /// Explicit owner maintenance; deletes only expired diagnostic extensions, never accounting.
    pub async fn purge_work_diagnostics(&self, agent: Uuid, actor: Uuid) -> Result<u64> {
        self.require_access(agent, actor, Role::Owner).await?;
        sqlx::query("UPDATE live_sessions SET data=data-'work_diagnostic' WHERE agent_id=$1 AND (data->'work_diagnostic'->>'expires_at')::timestamptz<=clock_timestamp() AND EXISTS(SELECT 1 FROM agent_access WHERE agent_id=$1 AND principal_id=$2 AND role='owner')")
            .bind(agent).bind(actor).execute(&self.pool).await.map(|r|r.rows_affected()).map_err(db)
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
    let row = sqlx::query(
        "SELECT data,budget_id FROM live_sessions WHERE agent_id=$1 AND id=$2 FOR UPDATE",
    )
    .bind(agent)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .ok_or(Error::NotFound)?;
    writable_inference_budget(row.get(1))?;
    let s: LiveSession = serde_json::from_value(row.get(0)).map_err(|_| Error::Database)?;
    writable_inference_budget(s.policy.budget_id)?;
    Ok(s)
}
async fn save_live(tx: &mut Tx<'_>, s: &LiveSession) -> Result<()> {
    sqlx::query("UPDATE live_sessions SET status=$2,data=$3 || CASE WHEN data ? 'work_diagnostic' THEN jsonb_build_object('work_diagnostic',data->'work_diagnostic') ELSE '{}'::jsonb END WHERE id=$1")
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

/// Evaluations inspect unpaid owner-supplied jobs only; they never accept payment or settle revenue.
async fn work_job(
    tx: &mut Tx<'_>,
    agent: Uuid,
    actor: Uuid,
    authorization: &LiveWorkConsent,
) -> Result<EconomicJob> {
    let value: Option<Value> =
        sqlx::query_scalar("SELECT to_jsonb(j) FROM economic_jobs j WHERE id=$1 FOR UPDATE")
            .bind(authorization.job_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db)?;
    let job: EconomicJob =
        serde_json::from_value(value.ok_or(Error::NotFound)?).map_err(|_| Error::Database)?;
    let digest: String = sqlx::query_scalar(
        "SELECT encode(sha256(convert_to(input,'UTF8')),'hex') FROM economic_jobs WHERE id=$1",
    )
    .bind(job.id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)?;
    if job.agent_id != agent
        || job.customer_id != actor
        || job.state != JobState::Created
        || job.received_atoms != 0
        || job.earned_atoms != 0
        || job.network != EconomicNetwork::Simulated
        || job.revision != authorization.expected_revision
        || digest != authorization.input_sha256
        || job.expires_at <= Utc::now()
    {
        return Err(Error::Denied);
    }
    Ok(job)
}
