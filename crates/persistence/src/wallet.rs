use super::*;
fn value<T: serde::Serialize>(v: &T) -> Result<Value> {
    serde_json::to_value(v).map_err(|_| Error::Invalid)
}
fn from<T: serde::de::DeserializeOwned>(v: Value) -> Result<T> {
    serde_json::from_value(v).map_err(|_| Error::Database)
}
async fn identity(tx: &mut Tx<'_>, agent: Uuid) -> Result<WalletIdentity> {
    let row=sqlx::query("SELECT id,agent_id,network,address,owner_id,origin FROM wallet_identities WHERE agent_id=$1 FOR UPDATE").bind(agent).fetch_optional(&mut **tx).await.map_err(db)?.ok_or(Error::NotFound)?;
    Ok(WalletIdentity {
        id: row.get(0),
        agent_id: row.get(1),
        network: from(json!(row.get::<String, _>(2)))?,
        address: row.get(3),
        owner_id: row.get(4),
        origin: from(row.get(5))?,
    })
}
async fn audit(
    tx: &mut Tx<'_>,
    agent: Uuid,
    wallet: Option<Uuid>,
    actor: Uuid,
    op: &str,
    outcome: &str,
    key: Option<Uuid>,
) -> Result<()> {
    sqlx::query("INSERT INTO financial_audit(agent_id,wallet_id,actor_id,operation,outcome,request_id) VALUES($1,$2,$3,$4,$5,$6)").bind(agent).bind(wallet).bind(actor).bind(op).bind(outcome).bind(key).execute(&mut **tx).await.map_err(db)?;
    event(tx, agent, None, "wallet", outcome, false).await
}
async fn summary(tx: &mut Tx<'_>, w: &WalletIdentity) -> Result<TreasurySummary> {
    let row = sqlx::query("SELECT data,observed_at FROM wallet_balances WHERE wallet_id=$1")
        .bind(w.id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
    let (balance, at) = match row {
        Some(r) => (
            Some(from::<WalletBalance>(r.get(0))?),
            Some(r.get::<chrono::DateTime<Utc>, _>(1)),
        ),
        None => (None, None),
    };
    let receipts: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(amount_atoms),0)::bigint FROM verified_payments WHERE wallet_id=$1",
    )
    .bind(w.id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)?;
    let latest: i64 = sqlx::query_scalar(
        "SELECT COALESCE(max(slot),0) FROM verified_payments WHERE wallet_id=$1",
    )
    .bind(w.id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)?;
    let reserved:i64=sqlx::query_scalar("SELECT COALESCE(sum(amount_atoms),0)::bigint FROM operating_reservations WHERE wallet_id=$1 AND status='held'").bind(w.id).fetch_one(&mut **tx).await.map_err(db)?;
    let estimate: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(accounted_microusd),0)::bigint FROM inference_calls WHERE agent_id=$1",
    )
    .bind(w.agent_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)?;
    let observed = balance.as_ref().map(|b| b.usdc_atoms).unwrap_or(0);
    let fresh = at.is_some_and(|at| Utc::now().signed_duration_since(at).num_seconds() < 120);
    let reconciled = balance
        .as_ref()
        .is_some_and(|b| b.slot >= latest && observed == receipts);
    Ok(TreasurySummary {
        network: w.network,
        environment: match balance.as_ref().map(|b| b.evidence_source) {
            Some(ChainEvidence::MockRpc) => "mocked_devnet_test_tokens_no_monetary_value",
            _ => "devnet_test_tokens_no_monetary_value",
        }
        .into(),
        asset: "USDC".into(),
        decimals: 6,
        observed_sol_lamports: balance.as_ref().map(|b| b.sol_lamports).unwrap_or(0),
        observed_usdc_atoms: observed,
        verified_incoming_usdc_atoms: receipts,
        reserved_usdc_atoms: reserved,
        inference_expense_usdc_atoms: 0,
        infrastructure_expense_usdc_atoms: 0,
        available_operating_usdc_atoms: if reconciled && fresh {
            observed.min(receipts).saturating_sub(reserved).max(0)
        } else {
            0
        },
        api_inference_estimate_microusd: estimate,
        reconciled,
        fresh,
        observed_at: at,
        latest_payment_slot: latest,
    })
}
impl Store {
    pub async fn wallet(&self, agent: Uuid) -> Result<WalletIdentity> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let w = identity(&mut tx, agent).await?;
        tx.commit().await.map_err(db)?;
        Ok(w)
    }
    pub async fn register_wallet(&self, w: &WalletIdentity, actor: Uuid) -> Result<()> {
        validate_solana_address(&w.address)?;
        w.network.require_devnet()?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let a = locked(&mut tx, w.agent_id).await?;
        let allowed = actor == w.owner_id
            && matches!(w.origin, IdentityOrigin::HumanOperator)
            && a.lifecycle != Lifecycle::Terminated;
        if !allowed {
            audit(
                &mut tx,
                w.agent_id,
                None,
                actor,
                "create_wallet",
                "denied",
                None,
            )
            .await?;
            tx.commit().await.map_err(db)?;
            return Err(Error::Denied);
        }
        sqlx::query("INSERT INTO wallet_identities(id,agent_id,network,address,owner_id,origin) VALUES($1,$2,$3,$4,$5,$6)").bind(w.id).bind(w.agent_id).bind(w.network.name()).bind(&w.address).bind(w.owner_id).bind(value(&w.origin)?).execute(&mut *tx).await.map_err(db)?;
        audit(
            &mut tx,
            w.agent_id,
            Some(w.id),
            actor,
            "create_wallet",
            "created",
            None,
        )
        .await?;
        tx.commit().await.map_err(db)
    }
    pub async fn wallet_audit(
        &self,
        agent: Uuid,
        actor: Uuid,
        op: &str,
        outcome: &str,
    ) -> Result<()> {
        if !["recover_wallet", "sign", "sync", "verify_payment"].contains(&op)
            || ![
                "denied",
                "milestone_signing_disabled",
                "recovered",
                "rpc_failed",
                "verified",
                "payment_invalid",
                "payment_unconfirmed",
                "invalid_input",
                "duplicate",
                "completed",
                "key_vault_failed",
            ]
            .contains(&outcome)
        {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        let w = identity(&mut tx, agent).await?;
        audit(&mut tx, agent, Some(w.id), actor, op, outcome, None).await?;
        tx.commit().await.map_err(db)
    }
    pub async fn wallet_balance(&self, agent: Uuid) -> Result<Option<WalletBalance>> {
        let w = self.wallet(agent).await?;
        let v: Option<Value> =
            sqlx::query_scalar("SELECT data FROM wallet_balances WHERE wallet_id=$1")
                .bind(w.id)
                .fetch_optional(&self.pool)
                .await
                .map_err(db)?;
        v.map(from).transpose()
    }
    pub async fn observe_balance(&self, agent: Uuid, b: &WalletBalance) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let w = identity(&mut tx, agent).await?;
        if w.network != b.network
            || w.address != b.address
            || b.slot < 0
            || b.sol_lamports < 0
            || b.usdc_atoms < 0
        {
            return Err(Error::Invalid);
        }
        let incompatible:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM verified_payments WHERE wallet_id=$1 AND evidence_source<>$2) OR EXISTS(SELECT 1 FROM wallet_balances WHERE wallet_id=$1 AND data->>'evidence_source'<>$2)").bind(w.id).bind(b.evidence_source.name()).fetch_one(&mut *tx).await.map_err(db)?;
        if incompatible {
            return Err(Error::Invalid);
        }
        sqlx::query("INSERT INTO wallet_balances(wallet_id,data,slot) VALUES($1,$2,$3) ON CONFLICT(wallet_id) DO UPDATE SET data=EXCLUDED.data,slot=EXCLUDED.slot,observed_at=now() WHERE wallet_balances.slot<=EXCLUDED.slot").bind(w.id).bind(value(b)?).bind(b.slot).execute(&mut *tx).await.map_err(db)?;
        for token in &b.token_accounts {
            sqlx::query("INSERT INTO wallet_token_accounts(wallet_id,address) VALUES($1,$2) ON CONFLICT DO NOTHING").bind(w.id).bind(&token.address).execute(&mut *tx).await.map_err(db)?;
        }
        tx.commit().await.map_err(db)
    }
    pub async fn record_payment(&self, agent: Uuid, p: &VerifiedPayment) -> Result<bool> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        let w = identity(&mut tx, agent).await?;
        if p.network() != w.network
            || p.wallet_address() != w.address
            || p.mint() != w.network.usdc_mint()
        {
            return Err(Error::InvalidPayment);
        }
        let incompatible:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM wallet_balances WHERE wallet_id=$1 AND data->>'evidence_source'<>$2) OR EXISTS(SELECT 1 FROM verified_payments WHERE wallet_id=$1 AND evidence_source<>$2)").bind(w.id).bind(p.evidence_source().name()).fetch_one(&mut *tx).await.map_err(db)?;
        if incompatible {
            return Err(Error::InvalidPayment);
        }
        let n=sqlx::query("INSERT INTO verified_payments(id,wallet_id,signature,mint,amount_atoms,slot,evidence_source,data) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(wallet_id,signature,mint) DO NOTHING").bind(Uuid::new_v4()).bind(w.id).bind(p.signature()).bind(p.mint()).bind(p.amount_atoms()).bind(p.slot()).bind(p.evidence_source().name()).bind(value(p)?).execute(&mut *tx).await.map_err(db)?.rows_affected();
        if n == 0 {
            let prior:Value=sqlx::query_scalar("SELECT data FROM verified_payments WHERE wallet_id=$1 AND signature=$2 AND mint=$3").bind(w.id).bind(p.signature()).bind(p.mint()).fetch_one(&mut *tx).await.map_err(db)?;
            if prior != value(p)? {
                freeze_wallet(&mut tx, w.id).await?;
                audit(
                    &mut tx,
                    agent,
                    Some(w.id),
                    w.owner_id,
                    "verify_payment",
                    "payment_invalid",
                    None,
                )
                .await?;
                tx.commit().await.map_err(db)?;
                return Err(Error::InvalidPayment);
            }
        }
        sqlx::query("INSERT INTO payment_checks(wallet_id,signature,status) VALUES($1,$2,'verified') ON CONFLICT(wallet_id,signature) DO UPDATE SET status='verified',checked_at=now()").bind(w.id).bind(p.signature()).execute(&mut *tx).await.map_err(db)?;
        audit(
            &mut tx,
            agent,
            Some(w.id),
            w.owner_id,
            "verify_payment",
            if n == 1 { "verified" } else { "duplicate" },
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(n == 1)
    }
    pub async fn payment_failure(&self, agent: Uuid, sig: &str, error: &Error) -> Result<()> {
        validate_solana_signature(sig)?;
        let code = match error {
            Error::Unconfirmed => "payment_unconfirmed",
            Error::InvalidPayment => "payment_invalid",
            Error::Invalid => "invalid_input",
            _ => "rpc_failed",
        };
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        let w = identity(&mut tx, agent).await?;
        let known: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM verified_payments WHERE wallet_id=$1 AND signature=$2)",
        )
        .bind(w.id)
        .bind(sig)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("INSERT INTO payment_checks(wallet_id,signature,status) VALUES($1,$2,$3) ON CONFLICT(wallet_id,signature) DO UPDATE SET status=EXCLUDED.status,checked_at=now() WHERE payment_checks.status<>'verified'").bind(w.id).bind(sig).bind(code).execute(&mut *tx).await.map_err(db)?;
        if known && matches!(error, Error::Unconfirmed | Error::InvalidPayment) {
            // Retain historical evidence and all financial records; freeze new allocations for human review.
            freeze_wallet(&mut tx, w.id).await?;
            audit(
                &mut tx,
                agent,
                Some(w.id),
                w.owner_id,
                "verify_payment",
                "confirmation_regressed",
                None,
            )
            .await?;
        }
        audit(
            &mut tx,
            agent,
            Some(w.id),
            w.owner_id,
            "verify_payment",
            code,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)
    }
    pub async fn treasury(&self, agent: Uuid) -> Result<TreasurySummary> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let w = identity(&mut tx, agent).await?;
        let s = summary(&mut tx, &w).await?;
        tx.commit().await.map_err(db)?;
        Ok(s)
    }
    pub async fn spending_policy(&self, agent: Uuid) -> Result<SpendingPolicy> {
        let v: Value = sqlx::query_scalar("SELECT policy FROM wallet_identities WHERE agent_id=$1")
            .bind(agent)
            .fetch_optional(&self.pool)
            .await
            .map_err(db)?
            .ok_or(Error::NotFound)?;
        from(v)
    }
    pub async fn set_spending_policy(
        &self,
        agent: Uuid,
        actor: Uuid,
        p: &SpendingPolicy,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let a = locked(&mut tx, agent).await?;
        let w = identity(&mut tx, agent).await?;
        let result = if actor != w.owner_id || a.lifecycle == Lifecycle::Terminated {
            Err(Error::Denied)
        } else {
            p.validate()
        };
        audit(
            &mut tx,
            agent,
            Some(w.id),
            actor,
            "set_policy",
            if result.is_ok() { "updated" } else { "denied" },
            None,
        )
        .await?;
        if result.is_ok() {
            sqlx::query("UPDATE wallet_identities SET policy=$2 WHERE id=$1")
                .bind(w.id)
                .bind(value(p)?)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        result
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn reserve_operating(
        &self,
        agent: Uuid,
        actor: Uuid,
        key: Uuid,
        destination: &str,
        amount: i64,
        purpose: OperatingPurpose,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let a = locked(&mut tx, agent).await?;
        let w = identity(&mut tx, agent).await?;
        let policy: SpendingPolicy = from(
            sqlx::query_scalar("SELECT policy FROM wallet_identities WHERE id=$1")
                .bind(w.id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?,
        )?;
        let purpose = match purpose {
            OperatingPurpose::Inference => "inference",
            OperatingPurpose::Infrastructure => "infrastructure",
        };
        let prior=sqlx::query("SELECT wallet_id,destination,amount_atoms,purpose,status FROM operating_reservations WHERE id=$1").bind(key).fetch_optional(&mut *tx).await.map_err(db)?;
        let committed:i64=sqlx::query_scalar("SELECT COALESCE(sum(amount_atoms),0)::bigint FROM operating_reservations WHERE wallet_id=$1 AND (status='held' OR created_at >= date_trunc('day',now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC')").bind(w.id).fetch_one(&mut *tx).await.map_err(db)?;
        let available = summary(&mut tx, &w).await?.available_operating_usdc_atoms;
        let result = if actor != w.owner_id
            || a.lifecycle != Lifecycle::Active
            || policy.locked
            || !policy.reservations_enabled
        {
            Err(Error::FinancialPolicy)
        } else if let Some(r) = prior {
            if r.get::<Uuid, _>(0) == w.id
                && r.get::<String, _>(1) == destination
                && r.get::<i64, _>(2) == amount
                && r.get::<String, _>(3) == purpose
                && r.get::<String, _>(4) == "held"
            {
                Ok(false)
            } else {
                Err(Error::Invalid)
            }
        } else {
            policy
                .authorize(
                    w.network,
                    a.lifecycle,
                    true,
                    destination,
                    amount,
                    committed,
                    available,
                )
                .map(|_| true)
        };
        audit(
            &mut tx,
            agent,
            Some(w.id),
            actor,
            "reserve_operating",
            if result.is_ok() { "reserved" } else { "denied" },
            Some(key),
        )
        .await?;
        if let Ok(true) = result {
            sqlx::query("INSERT INTO operating_reservations(id,wallet_id,destination,amount_atoms,purpose) VALUES($1,$2,$3,$4,$5)").bind(key).bind(w.id).bind(destination).bind(amount).bind(purpose).execute(&mut *tx).await.map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        result.map(|_| ())
    }
    pub async fn release_operating(&self, agent: Uuid, actor: Uuid, key: Uuid) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        locked(&mut tx, agent).await?;
        let w = identity(&mut tx, agent).await?;
        let allowed = actor == w.owner_id;
        audit(
            &mut tx,
            agent,
            Some(w.id),
            actor,
            "release_operating",
            if allowed { "released" } else { "denied" },
            Some(key),
        )
        .await?;
        let n = if allowed {
            sqlx::query("UPDATE operating_reservations SET status='released',released_at=now() WHERE id=$1 AND wallet_id=$2").bind(key).bind(w.id).execute(&mut *tx).await.map_err(db)?.rows_affected()
        } else {
            0
        };
        tx.commit().await.map_err(db)?;
        if n == 1 { Ok(()) } else { Err(Error::Denied) }
    }
    pub async fn wallet_history(&self, agent: Uuid, kind: &str) -> Result<Vec<Value>> {
        self.get(agent).await?;
        let q = match kind {
            "payments" => {
                "SELECT to_jsonb(t) FROM (SELECT p.* FROM verified_payments p JOIN wallet_identities w ON w.id=p.wallet_id WHERE w.agent_id=$1 ORDER BY p.verified_at DESC LIMIT 100) t"
            }
            "checks" => {
                "SELECT to_jsonb(t) FROM (SELECT p.* FROM payment_checks p JOIN wallet_identities w ON w.id=p.wallet_id WHERE w.agent_id=$1 ORDER BY p.checked_at DESC LIMIT 100) t"
            }
            "reservations" => {
                "SELECT to_jsonb(t) FROM (SELECT p.* FROM operating_reservations p JOIN wallet_identities w ON w.id=p.wallet_id WHERE w.agent_id=$1 ORDER BY p.created_at DESC LIMIT 100) t"
            }
            "audit" => {
                "SELECT to_jsonb(t) FROM (SELECT * FROM financial_audit WHERE agent_id=$1 ORDER BY id DESC LIMIT 100) t"
            }
            _ => return Err(Error::Invalid),
        };
        sqlx::query_scalar(q)
            .bind(agent)
            .fetch_all(&self.pool)
            .await
            .map_err(db)
    }
    pub async fn monitored_addresses(&self, agent: Uuid) -> Result<Vec<String>> {
        let w = self.wallet(agent).await?;
        let mut v: Vec<String> = sqlx::query_scalar(
            "SELECT address FROM wallet_token_accounts WHERE wallet_id=$1 ORDER BY address",
        )
        .bind(w.id)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        v.insert(0, w.address);
        Ok(v)
    }
    pub async fn scan_cursor(&self, agent: Uuid, address: &str) -> Result<Value> {
        let w = self.wallet(agent).await?;
        let v:Option<Value>=sqlx::query_scalar("SELECT jsonb_build_object('watermark',watermark,'before',page_before,'head',pending_head) FROM wallet_cursors WHERE wallet_id=$1 AND address=$2").bind(w.id).bind(address).fetch_optional(&self.pool).await.map_err(db)?;
        Ok(v.unwrap_or(json!({})))
    }
    pub async fn save_cursor(
        &self,
        agent: Uuid,
        address: &str,
        watermark: Option<&str>,
        before: Option<&str>,
        head: Option<&str>,
    ) -> Result<()> {
        let w = self.wallet(agent).await?;
        sqlx::query("INSERT INTO wallet_cursors(wallet_id,address,watermark,page_before,pending_head) VALUES($1,$2,$3,$4,$5) ON CONFLICT(wallet_id,address) DO UPDATE SET watermark=$3,page_before=$4,pending_head=$5").bind(w.id).bind(address).bind(watermark).bind(before).bind(head).execute(&self.pool).await.map_err(db)?;
        Ok(())
    }
}

async fn freeze_wallet(tx: &mut Tx<'_>, id: Uuid) -> Result<()> {
    sqlx::query("UPDATE wallet_identities SET policy=jsonb_set(policy,'{locked}','true'::jsonb) WHERE id=$1").bind(id).execute(&mut **tx).await.map_err(db)?;
    sqlx::query("UPDATE wallet_balances SET observed_at=LEAST(observed_at,now()-interval '121 seconds') WHERE wallet_id=$1").bind(id).execute(&mut **tx).await.map_err(db)?;
    Ok(())
}
