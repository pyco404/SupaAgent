//! Append-only semantic review. Never edits the generation or touches accounting.
use super::*;

impl Store {
    pub async fn work_review_context(
        &self,
        agent: Uuid,
        session: Uuid,
        actor: Uuid,
    ) -> Result<Value> {
        self.require_access(agent, actor, Role::Owner).await?;
        let value: Value =
            sqlx::query_scalar("SELECT data FROM live_sessions WHERE id=$1 AND agent_id=$2")
                .bind(session)
                .bind(agent)
                .fetch_optional(&self.pool)
                .await
                .map_err(db)?
                .ok_or(Error::NotFound)?;
        let s: LiveSession = serde_json::from_value(value).map_err(|_| Error::Database)?;
        let work = s.work.as_ref().ok_or(Error::Invalid)?;
        let report = work.report.as_ref().ok_or(Error::Invalid)?;
        let hash = review_hash(&self.pool, report).await?;
        let review: Option<Value> = sqlx::query_scalar("SELECT to_jsonb(r)-'confirmation' FROM work_evaluation_reviews r WHERE session_id=$1 AND agent_id=$2")
            .bind(session).bind(agent).fetch_optional(&self.pool).await.map_err(db)?;
        let review_matches = review.as_ref().is_none_or(|r| r["report_sha256"] == hash);
        let evaluation_status = if !review_matches {
            "review_requires_reconciliation"
        } else {
            review
                .as_ref()
                .and_then(|r| r["decision"].as_str())
                .unwrap_or("awaiting_human_review")
        };
        self.require_access(agent, actor, Role::Owner).await?;
        Ok(
            json!({"session_id":session,"agent_id":agent,"session_status":s.status,
            "report_sha256":hash,"report":report,"usage":s.usage,"estimated_cost_microusd":s.accounted_microusd,
            "source_material":work.request.material,"objective":work.request.objective,"input_sha256":work.authorization.input_sha256,"job_id":work.authorization.job_id,"server_time":Utc::now(),
            "review":review,"evaluation_status":evaluation_status,"review_matches_current_report":review_matches,
            "paid_consent_required":false,"review_effects":{"commercial_delivery":false,"job_completion":false,"earned_revenue_atoms":0,"financial_mutation":false,"agent_activation":false,"additional_generation_requests":0}}),
        )
    }

    pub async fn review_work_evaluation(
        &self,
        agent: Uuid,
        session: Uuid,
        actor: Uuid,
        input: WorkReviewRequest,
    ) -> Result<WorkEvaluationReview> {
        input.validate_at(Utc::now())?;
        // No blanket writable-epoch scope is inferred from owner authentication.
        if self.application_epoch().await?.is_some() {
            return Err(Error::Denied);
        }
        let mut tx = self.operational_transaction().await?;
        locked(&mut tx, agent).await?;
        let role: Option<String> = sqlx::query_scalar(
            "SELECT role FROM agent_access WHERE agent_id=$1 AND principal_id=$2 FOR UPDATE",
        )
        .bind(agent)
        .bind(actor)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if role.as_deref() != Some("owner") {
            return Err(Error::Denied);
        }
        let (status, value): (String, Value) = sqlx::query_as(
            "SELECT status,data FROM live_sessions WHERE id=$1 AND agent_id=$2 FOR UPDATE",
        )
        .bind(session)
        .bind(agent)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(Error::NotFound)?;
        let s: LiveSession = serde_json::from_value(value).map_err(|_| Error::Database)?;
        if status != "awaiting_approval"
            || s.status != status
            || s.approved_by != actor
            || s.generation_requests != 1
            || s.usage.is_none()
            || s.finished_at.is_none()
            || input.request_id == session
        {
            return Err(Error::Stale);
        }
        let report = s
            .work
            .as_ref()
            .and_then(|w| w.report.as_ref())
            .ok_or(Error::Invalid)?;
        let value = serde_json::to_value(report).map_err(|_| Error::Invalid)?;
        let hash: String = sqlx::query_scalar(
            "SELECT encode(sha256(convert_to(public.work_review_canonical($1),'UTF8')),'hex')",
        )
        .bind(value)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if hash != input.report_sha256 {
            return Err(Error::Stale);
        }
        // Fresh semantic confirmation is independent of the expired generation consent.
        input.validate_at(Utc::now())?;
        let confirmation = serde_json::to_value(&input).map_err(|_| Error::Invalid)?;
        let decision = serde_json::to_value(input.decision).map_err(|_| Error::Invalid)?;
        let row:Value=sqlx::query_scalar("INSERT INTO work_evaluation_reviews(request_id,session_id,agent_id,reviewer_principal_id,decision,report_sha256,confirmation) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING to_jsonb(work_evaluation_reviews)-'confirmation'")
            .bind(input.request_id).bind(session).bind(agent).bind(actor).bind(decision.as_str().ok_or(Error::Invalid)?)
            .bind(hash).bind(confirmation).fetch_one(&mut *tx).await.map_err(|e|match e {
                sqlx::Error::Database(ref d) if d.is_unique_violation() || d.is_check_violation()=>Error::Stale,
                _=>db(e),
            })?;
        let review = serde_json::from_value(row).map_err(|_| Error::Database)?;
        tx.commit().await.map_err(db)?;
        Ok(review)
    }
}
async fn review_hash(pool: &PgPool, report: &WorkProposal) -> Result<String> {
    sqlx::query_scalar(
        "SELECT encode(sha256(convert_to(public.work_review_canonical($1),'UTF8')),'hex')",
    )
    .bind(serde_json::to_value(report).map_err(|_| Error::Invalid)?)
    .fetch_one(pool)
    .await
    .map_err(db)
}
