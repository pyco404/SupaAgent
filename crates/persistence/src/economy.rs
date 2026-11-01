//! Transactional contracts and immutable double-entry postings; never a signing service.
use super::*;

fn decode_job(v: Value) -> Result<EconomicJob> {
    serde_json::from_value(v).map_err(|_| Error::Database)
}
async fn job(tx: &mut Tx<'_>, id: Uuid) -> Result<EconomicJob> {
    let value: Option<Value> =
        sqlx::query_scalar("SELECT to_jsonb(j) FROM economic_jobs j WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db)?;
    decode_job(value.ok_or(Error::NotFound)?)
}
// A posting names both accounts explicitly; grouping would obscure audit call sites.
#[allow(clippy::too_many_arguments)]
async fn posting(
    tx: &mut Tx<'_>,
    agent: Uuid,
    job: Option<Uuid>,
    network: EconomicNetwork,
    key: &str,
    debit: &str,
    credit: &str,
    amount: i64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let n=sqlx::query("INSERT INTO economic_entries(agent_id,job_id,network,event_key,debit,credit,amount_atoms) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(agent_id,network,event_key) DO NOTHING")
        .bind(agent).bind(job).bind(network.name()).bind(key).bind(debit).bind(credit).bind(amount).execute(&mut **tx).await.map_err(db)?.rows_affected();
    if n == 0 {
        let same:bool=sqlx::query_scalar("SELECT debit=$4 AND credit=$5 AND amount_atoms=$6 FROM economic_entries WHERE agent_id=$1 AND network=$2 AND event_key=$3")
            .bind(agent).bind(network.name()).bind(key).bind(debit).bind(credit).bind(amount).fetch_one(&mut **tx).await.map_err(db)?;
        if !same {
            return Err(Error::Stale);
        }
    }
    Ok(())
}
async fn audit(
    tx: &mut Tx<'_>,
    j: &EconomicJob,
    actor: Uuid,
    operation: &str,
    data: Value,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO economic_audit(job_id,agent_id,actor,operation,data) VALUES($1,$2,$3,$4,$5)",
    )
    .bind(j.id)
    .bind(j.agent_id)
    .bind(actor)
    .bind(operation)
    .bind(data)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}
async fn balance(
    tx: &mut Tx<'_>,
    agent: Uuid,
    network: EconomicNetwork,
    account: &str,
) -> Result<i64> {
    sqlx::query_scalar::<_,i64>("SELECT coalesce(sum(CASE WHEN debit=$3 THEN amount_atoms WHEN credit=$3 THEN -amount_atoms ELSE 0 END),0)::bigint FROM economic_entries WHERE agent_id=$1 AND network=$2")
        .bind(agent).bind(network.name()).bind(account).fetch_one(&mut **tx).await.map_err(db)
}
async fn available(tx: &mut Tx<'_>, agent: Uuid, network: EconomicNetwork) -> Result<i64> {
    let cash = balance(tx, agent, network, "cash").await?;
    let liability = -balance(tx, agent, network, "liability").await?;
    let held:i64=sqlx::query_scalar("SELECT coalesce(sum(amount_atoms) FILTER(WHERE held),0)::bigint FROM economic_holds WHERE agent_id=$1 AND network=$2").bind(agent).bind(network.name()).fetch_one(&mut **tx).await.map_err(db)?;
    let reserve: i64 = if network == EconomicNetwork::Simulated {
        sqlx::query_scalar("SELECT reserve_atoms FROM economic_spending_policies WHERE agent_id=$1")
            .bind(agent)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db)?
            .unwrap_or(0)
    } else {
        0
    };
    Ok(cash - liability - held - reserve)
}
async fn spending_allowed(
    tx: &mut Tx<'_>,
    agent: Uuid,
    amount: i64,
    required: &[&str],
) -> Result<()> {
    let row = sqlx::query(
        "SELECT destinations,ceiling_atoms FROM economic_spending_policies WHERE agent_id=$1",
    )
    .bind(agent)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .ok_or(Error::FinancialPolicy)?;
    let destinations: Vec<String> = row.try_get("destinations").map_err(db)?;
    let ceiling: i64 = row.try_get("ceiling_atoms").map_err(db)?;
    let used = balance(tx, agent, EconomicNetwork::Simulated, "inference_expense").await?
        + balance(
            tx,
            agent,
            EconomicNetwork::Simulated,
            "infrastructure_expense",
        )
        .await?;
    let held:i64=sqlx::query_scalar("SELECT coalesce(sum(amount_atoms) FILTER(WHERE held),0)::bigint FROM economic_holds WHERE agent_id=$1 AND network='simulated'").bind(agent).fetch_one(&mut **tx).await.map_err(db)?;
    if !required.iter().all(|v| destinations.iter().any(|d| d == v))
        || used + held + amount > ceiling
        || available(tx, agent, EconomicNetwork::Simulated).await? < amount
    {
        return Err(Error::ResourceBudget);
    }
    Ok(())
}
impl Store {
    async fn economic_writable(&self) -> Result<()> {
        if self.application_epoch().await?.is_some() {
            Err(Error::Denied)
        } else {
            Ok(())
        }
    }
    pub async fn economic_service(&self, id: Uuid) -> Result<EconomicService> {
        let row: Option<Value> =
            sqlx::query_scalar("SELECT to_jsonb(s) FROM economic_services s WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(db)?;
        serde_json::from_value(row.ok_or(Error::NotFound)?).map_err(|_| Error::Database)
    }
    pub async fn economic_services(&self, agent: Option<Uuid>) -> Result<Vec<EconomicService>> {
        let rows:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(s) FROM economic_services s WHERE ($1::uuid IS NULL OR agent_id=$1) ORDER BY created_at,id LIMIT 100").bind(agent).fetch_all(&self.pool).await.map_err(db)?;
        rows.into_iter()
            .map(|v| serde_json::from_value(v).map_err(|_| Error::Database))
            .collect()
    }
    pub async fn offer_service(
        &self,
        agent: Uuid,
        actor: Uuid,
        id: Uuid,
        spec: ServiceSpec,
    ) -> Result<EconomicService> {
        self.economic_writable().await?;
        self.require_access(agent, actor, Role::Owner).await?;
        spec.validate()?;
        if id.is_nil()
            || !self
                .get(agent)
                .await?
                .capabilities
                .contains(&Capability::EvaluateGoal)
        {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        sqlx::query("INSERT INTO economic_services(id,agent_id,spec) VALUES($1,$2,$3) ON CONFLICT(id) DO NOTHING").bind(id).bind(agent).bind(json!(spec)).execute(&mut *tx).await.map_err(db)?;
        tx.commit().await.map_err(db)?;
        let saved = self.economic_service(id).await?;
        if saved.agent_id != agent || saved.spec != spec {
            return Err(Error::Stale);
        }
        Ok(saved)
    }
    /// Only scoped callers can obtain the exact input binding used by live-work consent.
    pub async fn work_input_sha256(&self, id: Uuid, actor: Uuid) -> Result<String> {
        self.economic_job(id, actor).await?;
        sqlx::query_scalar(
            "SELECT encode(sha256(convert_to(input,'UTF8')),'hex') FROM economic_jobs WHERE id=$1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(db)
    }
    pub async fn economic_job(&self, id: Uuid, actor: Uuid) -> Result<EconomicJob> {
        let v: Option<Value> =
            sqlx::query_scalar("SELECT CASE WHEN EXISTS(SELECT 1 FROM agent_access a WHERE a.agent_id=j.agent_id AND a.principal_id=$2 AND a.role IN ('owner','operator')) THEN to_jsonb(j) ELSE to_jsonb(j)||jsonb_build_object('assessment',j.assessment-'private_memory_references') END FROM economic_jobs j WHERE id=$1")
                .bind(id).bind(actor)
                .fetch_optional(&self.pool)
                .await
                .map_err(db)?;
        let j = decode_job(v.ok_or(Error::NotFound)?)?;
        if actor != j.customer_id {
            self.require_access(j.agent_id, actor, Role::Operator)
                .await?;
        }
        Ok(j)
    }
    pub async fn create_economic_job(
        &self,
        actor: Uuid,
        request: CreateJob,
    ) -> Result<EconomicJob> {
        self.economic_writable().await?;
        request.network.enabled()?;
        if actor.is_nil()
            || request.request_id.is_nil()
            || !(60..=86400).contains(&request.valid_for_seconds)
        {
            return Err(Error::Invalid);
        }
        let service = self.economic_service(request.service_id).await?;
        service.spec.input(&request.input)?;
        if request.network == EconomicNetwork::Devnet {
            let w = self.wallet(service.agent_id).await?;
            w.network.require_devnet()?;
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, service.agent_id).await?;
        let n=sqlx::query("INSERT INTO economic_jobs(id,service_id,agent_id,customer_id,network,spec,input,state,expires_at,request) VALUES($1,$2,$3,$4,$5,$6,$7,'created',now()+make_interval(secs=>$8),$9) ON CONFLICT(id) DO NOTHING")
            .bind(request.request_id).bind(request.service_id).bind(service.agent_id).bind(actor).bind(request.network.name()).bind(json!(service.spec)).bind(&request.input).bind(request.valid_for_seconds as f64).bind(json!(request)).execute(&mut *tx).await.map_err(db)?.rows_affected();
        let saved = job(&mut tx, request.request_id).await?;
        let prior: Value = sqlx::query_scalar("SELECT request FROM economic_jobs WHERE id=$1")
            .bind(saved.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if saved.customer_id != actor {
            return Err(Error::Denied);
        }
        if prior != json!(request) {
            return Err(Error::Stale);
        }
        if n == 1 {
            audit(
                &mut tx,
                &saved,
                actor,
                "created",
                json!({"network":request.network}),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(saved)
    }
    pub async fn economic_jobs(
        &self,
        actor: Uuid,
        agent: Option<Uuid>,
    ) -> Result<Vec<EconomicJob>> {
        let rows:Vec<Value>=sqlx::query_scalar("SELECT CASE WHEN EXISTS(SELECT 1 FROM agent_access a WHERE a.agent_id=j.agent_id AND a.principal_id=$1 AND a.role IN ('owner','operator')) THEN to_jsonb(j) ELSE to_jsonb(j)||jsonb_build_object('assessment',j.assessment-'private_memory_references') END FROM economic_jobs j WHERE ($2::uuid IS NULL OR agent_id=$2) AND (customer_id=$1 OR EXISTS(SELECT 1 FROM agent_access a WHERE a.agent_id=j.agent_id AND a.principal_id=$1 AND a.role IN ('owner','operator'))) ORDER BY created_at DESC,id LIMIT 100").bind(actor).bind(agent).fetch_all(&self.pool).await.map_err(db)?;
        rows.into_iter().map(decode_job).collect()
    }
    pub async fn economic_command(
        &self,
        id: Uuid,
        actor: Uuid,
        c: JobCommand,
    ) -> Result<EconomicJob> {
        self.economic_writable().await?;
        if !c.authorize || c.request_id.is_nil() {
            return Err(Error::Denied);
        }
        let before = self.economic_job(id, actor).await?;
        if !["request_payment", "cancel", "dispute", "expire"].contains(&c.operation.as_str()) {
            self.require_access(before.agent_id, actor, Role::Operator)
                .await?;
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, before.agent_id).await?;
        let j = job(&mut tx, id).await?;
        let replay:Option<Value>=sqlx::query_scalar("SELECT jsonb_build_object('job_id',job_id,'request',request,'result',result) FROM economic_commands WHERE actor=$1 AND request_id=$2").bind(actor).bind(c.request_id).fetch_optional(&mut *tx).await.map_err(db)?;
        if let Some(v) = replay {
            if v["job_id"] != json!(id) || v["request"] != json!(c) {
                return Err(Error::Stale);
            }
            return decode_job(v["result"].clone());
        }
        if j.revision != c.expected_revision {
            return Err(Error::Stale);
        }
        let expired = j.expires_at <= Utc::now();
        let next = match c.operation.as_str() {
            "request_payment" if j.state == JobState::Created && !expired => {
                if j.received_atoms >= j.spec.price_atoms {
                    JobState::Funded
                } else {
                    JobState::AwaitingPayment
                }
            }
            "cancel"
                if matches!(
                    j.state,
                    JobState::Created
                        | JobState::AwaitingPayment
                        | JobState::Funded
                        | JobState::Accepted
                ) =>
            {
                JobState::Cancelled
            }
            "expire"
                if expired
                    && !matches!(
                        j.state,
                        JobState::Completed
                            | JobState::Executing
                            | JobState::Disputed
                            | JobState::Cancelled
                    ) =>
            {
                JobState::Expired
            }
            "dispute"
                if matches!(
                    j.state,
                    JobState::Completed | JobState::Failed | JobState::Funded
                ) =>
            {
                JobState::Disputed
            }
            "retry"
                if j.state == JobState::Failed
                    && j.attempts < i32::from(j.spec.max_attempts)
                    && !expired =>
            {
                JobState::Funded
            }
            "recover"
                if j.state == JobState::Executing
                    && j.lease_until.is_some_and(|t| t <= Utc::now()) =>
            {
                JobState::Failed
            }
            "accept" if j.state == JobState::Funded && !expired => {
                let agent = self.get(j.agent_id).await?;
                if agent.lifecycle != Lifecycle::Active
                    || !agent.capabilities.contains(&Capability::EvaluateGoal)
                {
                    return Err(Error::Denied);
                }
                let cost = j.spec.cost(j.network);
                if cost > 0 {
                    spending_allowed(&mut tx, j.agent_id, cost, &["inference", "infrastructure"])
                        .await?;
                }
                sqlx::query("INSERT INTO economic_holds(job_id,agent_id,network,amount_atoms) VALUES($1,$2,$3,$4) ON CONFLICT(job_id) DO UPDATE SET held=true,amount_atoms=excluded.amount_atoms").bind(id).bind(j.agent_id).bind(j.network.name()).bind(cost).execute(&mut *tx).await.map_err(db)?;
                JobState::Accepted
            }
            _ => return Err(Error::InvalidTransition),
        };
        if matches!(
            next,
            JobState::Cancelled | JobState::Expired | JobState::Failed
        ) {
            sqlx::query("UPDATE economic_holds SET held=false WHERE job_id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        sqlx::query("UPDATE economic_jobs SET state=$2,revision=revision+1,execution_token=NULL,lease_until=NULL,failure_code=CASE WHEN $2='failed' THEN 'interrupted_requires_review' ELSE failure_code END WHERE id=$1").bind(id).bind(next.name()).execute(&mut *tx).await.map_err(db)?;
        let saved = job(&mut tx, id).await?;
        audit(
            &mut tx,
            &saved,
            actor,
            &c.operation,
            json!({"from":j.state,"to":next}),
        )
        .await?;
        sqlx::query("INSERT INTO economic_commands(actor,request_id,job_id,request,result) VALUES($1,$2,$3,$4,$5)").bind(actor).bind(c.request_id).bind(id).bind(json!(c)).bind(json!(saved)).execute(&mut *tx).await.map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(saved)
    }
    pub async fn simulated_job_payment(
        &self,
        id: Uuid,
        actor: Uuid,
        p: SimulatedReceipt,
    ) -> Result<EconomicJob> {
        if !p.authorize_simulated || p.request_id.is_nil() {
            return Err(Error::Denied);
        }
        let j = self.economic_job(id, actor).await?;
        if j.network != EconomicNetwork::Simulated || j.customer_id != actor {
            return Err(Error::Denied);
        }
        self.economic_receipt(
            id,
            actor,
            format!("simulation:{}", p.request_id),
            p.amount_atoms,
            "simulated",
            json!({"request_id":p.request_id,"amount_atoms":p.amount_atoms}),
        )
        .await
    }
    pub async fn verified_job_payment(
        &self,
        id: Uuid,
        actor: Uuid,
        p: &VerifiedPayment,
    ) -> Result<EconomicJob> {
        let j = self.economic_job(id, actor).await?;
        let w = self.wallet(j.agent_id).await?;
        if j.network != EconomicNetwork::Devnet
            || p.network() != SolanaNetwork::Devnet
            || p.wallet_address() != w.address
            || p.mint() != SolanaNetwork::Devnet.usdc_mint()
        {
            return Err(Error::InvalidPayment);
        }
        self.economic_receipt(
            id,
            actor,
            p.signature().into(),
            p.amount_atoms(),
            p.evidence_source().name(),
            json!(p),
        )
        .await
    }
    async fn economic_receipt(
        &self,
        id: Uuid,
        actor: Uuid,
        signature: String,
        amount: i64,
        evidence: &str,
        proof: Value,
    ) -> Result<EconomicJob> {
        self.economic_writable().await?;
        if !(1..=1_000_000_000).contains(&amount) {
            return Err(Error::InvalidPayment);
        }
        let before = self.economic_job(id, actor).await?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, before.agent_id).await?;
        let j = job(&mut tx, id).await?;
        let old:Option<Value>=sqlx::query_scalar("SELECT jsonb_build_object('job_id',job_id,'proof',proof) FROM economic_receipts WHERE network=$1 AND signature=$2").bind(j.network.name()).bind(&signature).fetch_optional(&mut *tx).await.map_err(db)?;
        if let Some(v) = old {
            if v["job_id"] != json!(id) || v["proof"] != proof {
                return Err(Error::InvalidPayment);
            }
            return Ok(j);
        }
        let mixed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM economic_receipts r JOIN economic_jobs j ON j.id=r.job_id WHERE j.agent_id=$1 AND r.network=$2 AND r.evidence<>$3)").bind(j.agent_id).bind(j.network.name()).bind(evidence).fetch_one(&mut *tx).await.map_err(db)?;
        if mixed {
            return Err(Error::InvalidPayment);
        }
        sqlx::query("INSERT INTO economic_receipts(network,signature,job_id,amount_atoms,evidence,proof) VALUES($1,$2,$3,$4,$5,$6)").bind(j.network.name()).bind(&signature).bind(id).bind(amount).bind(evidence).bind(proof).execute(&mut *tx).await.map_err(db)?;
        posting(
            &mut tx,
            j.agent_id,
            Some(id),
            j.network,
            &format!("payment:{signature}"),
            "cash",
            "liability",
            amount,
        )
        .await?;
        let received = j.received_atoms.checked_add(amount).ok_or(Error::Invalid)?;
        let state = if j.expires_at <= Utc::now()
            && matches!(j.state, JobState::AwaitingPayment | JobState::Created)
        {
            JobState::Expired
        } else if j.state == JobState::AwaitingPayment && received >= j.spec.price_atoms {
            JobState::Funded
        } else {
            j.state
        };
        sqlx::query(
            "UPDATE economic_jobs SET received_atoms=$2,state=$3,revision=revision+1 WHERE id=$1",
        )
        .bind(id)
        .bind(received)
        .bind(state.name())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let saved = job(&mut tx, id).await?;
        audit(&mut tx,&saved,actor,"payment_recorded",json!({"amount_atoms":amount,"evidence":evidence,"underpayment":received<j.spec.price_atoms,"overpayment":received>j.spec.price_atoms})).await?;
        tx.commit().await.map_err(db)?;
        Ok(saved)
    }
    pub async fn claim_economic_job(
        &self,
        id: Uuid,
        actor: Uuid,
        c: JobCommand,
        assessment: Value,
    ) -> Result<EconomicJob> {
        self.economic_writable().await?;
        let before = self.economic_job(id, actor).await?;
        self.require_access(before.agent_id, actor, Role::Operator)
            .await?;
        if !c.authorize || c.operation != "execute" || c.request_id.is_nil() {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, before.agent_id).await?;
        let j = job(&mut tx, id).await?;
        let used: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM economic_commands WHERE actor=$1 AND request_id=$2)",
        )
        .bind(actor)
        .bind(c.request_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if used {
            return Err(Error::Stale);
        }
        if j.revision != c.expected_revision
            || j.state != JobState::Accepted
            || j.attempts >= i32::from(j.spec.max_attempts)
            || j.expires_at <= Utc::now()
        {
            return Err(Error::Stale);
        }
        let active:bool=sqlx::query_scalar("SELECT data->>'lifecycle'='active' AND data->'capabilities' ? 'evaluate_goal' FROM agents WHERE id=$1").bind(j.agent_id).fetch_one(&mut *tx).await.map_err(db)?;
        if !active {
            return Err(Error::Denied);
        }
        sqlx::query("UPDATE economic_jobs SET state='executing',revision=revision+1,attempts=attempts+1,execution_token=$2,lease_until=now()+interval '30 seconds',assessment=$3 WHERE id=$1").bind(id).bind(c.request_id).bind(assessment).execute(&mut *tx).await.map_err(db)?;
        let saved = job(&mut tx, id).await?;
        sqlx::query("INSERT INTO economic_commands(actor,request_id,job_id,request,result) VALUES($1,$2,$3,$4,$5)").bind(actor).bind(c.request_id).bind(id).bind(json!(c)).bind(json!(saved)).execute(&mut *tx).await.map_err(db)?;
        audit(
            &mut tx,
            &saved,
            actor,
            "executing",
            json!({"attempt":saved.attempts,"request_id":c.request_id}),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(saved)
    }
    pub async fn finish_economic_job(
        &self,
        id: Uuid,
        actor: Uuid,
        token: Uuid,
        output: Option<Value>,
        failure: Option<&str>,
    ) -> Result<EconomicJob> {
        self.economic_writable().await?;
        let before = self.economic_job(id, actor).await?;
        self.require_access(before.agent_id, actor, Role::Operator)
            .await?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let agent = locked(&mut tx, before.agent_id).await?;
        if output.is_some()
            && (agent.lifecycle != Lifecycle::Active
                || !agent.capabilities.contains(&Capability::EvaluateGoal))
        {
            return Err(Error::Denied);
        }
        let j = job(&mut tx, id).await?;
        if j.state != JobState::Executing
            || j.execution_token != Some(token)
            || j.lease_until.is_none_or(|t| t <= Utc::now())
        {
            return Err(Error::Stale);
        }
        if output.as_ref().is_some_and(|v| v.to_string().len() > 16000) {
            return Err(Error::Invalid);
        }
        let delivered = output
            .as_ref()
            .map(work_delivered)
            .transpose()?
            .unwrap_or(false);
        if delivered {
            posting(
                &mut tx,
                j.agent_id,
                Some(id),
                j.network,
                &format!("earned:{id}"),
                "liability",
                "revenue",
                j.spec.price_atoms,
            )
            .await?;
            if j.network == EconomicNetwork::Simulated {
                posting(
                    &mut tx,
                    j.agent_id,
                    Some(id),
                    j.network,
                    &format!("inference:{id}"),
                    "inference_expense",
                    "cash",
                    j.spec.estimated_inference_atoms,
                )
                .await?;
                posting(
                    &mut tx,
                    j.agent_id,
                    Some(id),
                    j.network,
                    &format!("infrastructure:{id}"),
                    "infrastructure_expense",
                    "cash",
                    j.spec.estimated_infrastructure_atoms,
                )
                .await?;
            }
        }
        let next = if delivered {
            JobState::Completed
        } else {
            JobState::Failed
        };
        sqlx::query("UPDATE economic_jobs SET state=$2,revision=revision+1,execution_token=NULL,lease_until=NULL,output=$3,output_sha256=CASE WHEN $3::jsonb IS NULL THEN NULL ELSE encode(sha256(convert_to(($3::jsonb)::text,'UTF8')),'hex') END,earned_atoms=CASE WHEN $2='completed' THEN (spec->>'price_atoms')::bigint ELSE 0 END,failure_code=$4 WHERE id=$1")
            .bind(id).bind(next.name()).bind(output).bind(failure).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("UPDATE economic_holds SET held=false WHERE job_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let saved = job(&mut tx, id).await?;
        audit(
            &mut tx,
            &saved,
            actor,
            next.name(),
            json!({"output_sha256":saved.output_sha256,"failure_code":failure}),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(saved)
    }
    pub async fn economic_capital(
        &self,
        agent: Uuid,
        actor: Uuid,
        key: Uuid,
        amount: i64,
    ) -> Result<()> {
        self.economic_writable().await?;
        self.require_access(agent, actor, Role::Owner).await?;
        if key.is_nil() || !(1..=1_000_000).contains(&amount) {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        posting(
            &mut tx,
            agent,
            None,
            EconomicNetwork::Simulated,
            &format!("capital:{actor}:{key}"),
            "cash",
            "capital",
            amount,
        )
        .await?;
        tx.commit().await.map_err(db)
    }
    pub async fn economic_policy(
        &self,
        agent: Uuid,
        actor: Uuid,
        ceiling: i64,
        reserve: i64,
        destinations: Vec<String>,
    ) -> Result<()> {
        self.economic_writable().await?;
        self.require_access(agent, actor, Role::Owner).await?;
        if !(0..=1_000_000).contains(&ceiling)
            || !(0..=1_000_000).contains(&reserve)
            || destinations.len() > 2
            || destinations
                .iter()
                .any(|v| !matches!(v.as_str(), "inference" | "infrastructure"))
        {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        let held:i64=sqlx::query_scalar("SELECT coalesce(sum(amount_atoms) FILTER(WHERE held),0)::bigint FROM economic_holds WHERE agent_id=$1 AND network='simulated'").bind(agent).fetch_one(&mut *tx).await.map_err(db)?;
        let spent = balance(
            &mut tx,
            agent,
            EconomicNetwork::Simulated,
            "inference_expense",
        )
        .await?
            + balance(
                &mut tx,
                agent,
                EconomicNetwork::Simulated,
                "infrastructure_expense",
            )
            .await?;
        let cash = balance(&mut tx, agent, EconomicNetwork::Simulated, "cash").await?;
        let liability = -balance(&mut tx, agent, EconomicNetwork::Simulated, "liability").await?;
        if spent + held > ceiling
            || cash - liability - held < reserve
            || (held > 0
                && !["inference", "infrastructure"]
                    .iter()
                    .all(|v| destinations.iter().any(|d| d == v)))
        {
            return Err(Error::ResourceBudget);
        }
        sqlx::query("INSERT INTO economic_spending_policies(agent_id,destinations,ceiling_atoms,reserve_atoms,approved_by) VALUES($1,$2,$3,$4,$5) ON CONFLICT(agent_id) DO UPDATE SET destinations=excluded.destinations,ceiling_atoms=excluded.ceiling_atoms,reserve_atoms=excluded.reserve_atoms,approved_by=excluded.approved_by").bind(agent).bind(&destinations).bind(ceiling).bind(reserve).bind(actor).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("INSERT INTO economic_audit(agent_id,actor,operation,data) VALUES($1,$2,'simulated_spending_policy',$3)").bind(agent).bind(actor).bind(json!({"ceiling_atoms":ceiling,"reserve_atoms":reserve,"destinations":destinations})).execute(&mut *tx).await.map_err(db)?;
        tx.commit().await.map_err(db)
    }
    pub async fn economic_spend(
        &self,
        agent: Uuid,
        actor: Uuid,
        key: Uuid,
        amount: i64,
        destination: &str,
    ) -> Result<()> {
        self.economic_writable().await?;
        self.require_access(agent, actor, Role::Owner).await?;
        if key.is_nil()
            || !(1..=1000000).contains(&amount)
            || !matches!(destination, "inference" | "infrastructure")
        {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        let event = format!("spend:{actor}:{key}");
        let previous:Option<(String,i64)>=sqlx::query_as("SELECT debit,amount_atoms FROM economic_entries WHERE agent_id=$1 AND network='simulated' AND event_key=$2").bind(agent).bind(&event).fetch_optional(&mut *tx).await.map_err(db)?;
        let debit = format!("{destination}_expense");
        if let Some(p) = previous {
            return if p == (debit, amount) {
                Ok(())
            } else {
                Err(Error::Stale)
            };
        }
        spending_allowed(&mut tx, agent, amount, &[destination]).await?;
        posting(
            &mut tx,
            agent,
            None,
            EconomicNetwork::Simulated,
            &event,
            &debit,
            "cash",
            amount,
        )
        .await?;
        tx.commit().await.map_err(db)
    }
    pub async fn economic_accounts(&self, agent: Uuid, actor: Uuid) -> Result<Value> {
        self.require_access(agent, actor, Role::Viewer).await?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        let mut networks = vec![];
        for network in [EconomicNetwork::Simulated, EconomicNetwork::Devnet] {
            let cash = balance(&mut tx, agent, network, "cash").await?;
            let liability = -balance(&mut tx, agent, network, "liability").await?;
            let revenue = -balance(&mut tx, agent, network, "revenue").await?;
            let inference = balance(&mut tx, agent, network, "inference_expense").await?;
            let infrastructure = balance(&mut tx, agent, network, "infrastructure_expense").await?;
            networks.push(json!({"network":network,"cash_atoms":cash,"liability_atoms":liability,"earned_atoms":revenue,"inference_atoms":inference,"infrastructure_atoms":infrastructure,"available_atoms":available(&mut tx,agent,network).await?,"net_result_atoms":revenue-inference-infrastructure,"runway_days":null,"runway_reason":"Recurring real costs are not established; no revenue or runway is fabricated."}));
        }
        let entries:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(e) FROM economic_entries e WHERE agent_id=$1 ORDER BY id DESC LIMIT 200").bind(agent).fetch_all(&mut *tx).await.map_err(db)?;
        let audit: Vec<Value> = sqlx::query_scalar(
            "SELECT to_jsonb(e) FROM economic_audit e WHERE agent_id=$1 ORDER BY id DESC LIMIT 200",
        )
        .bind(agent)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let policy: Option<Value> = sqlx::query_scalar(
            "SELECT to_jsonb(p) FROM economic_spending_policies p WHERE agent_id=$1",
        )
        .bind(agent)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        Ok(
            json!({"agent_id":agent,"networks":networks,"entries":entries,"audit":audit,"policy":policy,"mainnet_enabled":false,"signing_enabled":false}),
        )
    }
}
