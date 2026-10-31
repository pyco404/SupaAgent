//! Caller-scoped read-only metadata. Never returns capability hashes, secrets or private plans.
use super::*;
impl Store {
    pub async fn operator_status(&self, principal: Uuid) -> Result<Value> {
        if principal.is_nil()
            || self
                .operational_context()
                .is_some_and(|c| c.principal_id != principal)
        {
            return Err(Error::Denied);
        }
        let reader = Store {
            pool: self.status_pool.as_ref().unwrap_or(&self.pool).clone(),
            operational: None,
            status_pool: None,
        };
        let epoch = reader.application_epoch().await?;
        let mut tx = reader.pool.begin().await.map_err(db)?;
        sqlx::query("SET TRANSACTION READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let (database,readonly,now):(String,bool,chrono::DateTime<Utc>)=sqlx::query_as("SELECT current_database(),current_setting('default_transaction_read_only')='on',clock_timestamp()")
            .fetch_one(&mut *tx).await.map_err(db)?;
        let schema: i64 = sqlx::query_scalar(
            "SELECT coalesce(max(version),0) FROM _sqlx_migrations WHERE success",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let mut result = json!({"epoch":epoch,"database":database,"schema_version":schema,"database_read_only_default":readonly,"checked_at":now,
          "credential":{"status":"unverified","expires_at":null},
          "authorization":{"status":"inactive","scope":null,"agent_id":null,"expires_at":null,"max_cycles":null,"budget_microusd":null},
          "worker":{"status":"unavailable","execution_running":false,"heartbeat_at":null,"lease_until":null},
          "eligibility":{"create_agent":false,"grant_owner":false,"activate_offline":false,"execute_offline":false,"paid_inference":false,"financial_operations":false,"reason":"schema_12_required"}});
        if schema < 12 {
            return Ok(result);
        }
        // Explicit column-only SELECT grants are a provisioning requirement, never installed here.
        let binding=sqlx::query("SELECT p.credential_id,p.expires_at,p.verified_at<=clock_timestamp() AND EXISTS(SELECT 1 FROM epoch_guard.authorities a WHERE a.principal_id=p.verified_by) AND EXISTS(SELECT 1 FROM epoch_guard.executors e WHERE e.login=p.login AND e.principal_id=p.principal_id AND e.role='owner') AS verified, EXISTS(SELECT 1 FROM epoch_guard.credential_revocations r WHERE r.credential_id=p.credential_id) AS revoked FROM epoch_guard.operational_principals p WHERE p.principal_id=$1 AND p.login=session_user")
            .bind(principal).fetch_optional(&mut *tx).await;
        let binding = match binding {
            Ok(value) => value,
            Err(_) => {
                result["credential"]["status"] = json!("unavailable");
                result["eligibility"]["reason"] = json!("status_metadata_access_required");
                return Ok(result);
            }
        };
        result["eligibility"]["reason"] = json!("verified_owner_required");
        let Some(binding) = binding else {
            return Ok(result);
        };
        let expiry: chrono::DateTime<Utc> = binding.get("expires_at");
        let credential = if binding.get::<bool, _>("revoked") {
            "revoked"
        } else if expiry <= now {
            "expired"
        } else if binding.get::<bool, _>("verified") {
            "valid"
        } else {
            "unverified"
        };
        result["credential"] = json!({"status":credential,"expires_at":expiry});
        result["eligibility"]["reason"] = json!("scoped_approval_required");
        let Some(context) = self.operational_context() else {
            return Ok(result);
        };
        let consent=sqlx::query("SELECT a.plan,EXISTS(SELECT 1 FROM epoch_guard.operational_revocations r WHERE r.approval_id=a.id) AS revoked,c.server_start=pg_postmaster_start_time() AS incarnation,epoch_guard.operation_live(a.id) AS live FROM epoch_guard.operational_approvals a JOIN epoch_guard.operational_claims c ON c.approval_id=a.id WHERE a.id=$1 AND a.plan->>'principal_id'=$2 AND a.plan->>'credential_id'=$3 AND a.plan->>'database'=current_database() AND c.login=session_user")
            .bind(context.approval_id).bind(principal.to_string()).bind(binding.get::<Uuid,_>("credential_id").to_string()).fetch_optional(&mut *tx).await.map_err(db)?;
        let Some(consent) = consent else {
            result["authorization"]["status"] = json!("unavailable");
            return Ok(result);
        };
        let p: Value = consent.get("plan");
        let until =
            chrono::DateTime::parse_from_rfc3339(p["expires_at"].as_str().ok_or(Error::Database)?)
                .map_err(|_| Error::Database)?
                .with_timezone(&Utc);
        let status = if credential != "valid" {
            "credential_invalid"
        } else if consent.get::<bool, _>("revoked") {
            "revoked"
        } else if until <= now {
            "expired"
        } else if !consent.get::<bool, _>("incarnation") {
            "restart_required"
        } else if consent.get::<bool, _>("live") {
            "active"
        } else {
            "unavailable"
        };
        result["authorization"] = json!({"status":status,"scope":context.scope,"agent_id":context.agent_id,"expires_at":until,"max_cycles":p["max_cycles"],"budget_microusd":p["budget_microusd"]});
        let active = status == "active";
        let (exists,owned,goals):(bool,bool,bool)=sqlx::query_as("SELECT EXISTS(SELECT 1 FROM agents WHERE id=$1),EXISTS(SELECT 1 FROM agent_access WHERE agent_id=$1 AND principal_id=$2 AND role='owner'),EXISTS(SELECT 1 FROM agents WHERE id=$1 AND jsonb_array_length(coalesce(data->'goals','[]'::jsonb))>0)")
            .bind(context.agent_id).bind(principal).fetch_one(&mut *tx).await.map_err(db)?;

        result["eligibility"] = json!({"create_agent":active&&!exists&&context.scope=="agent_creation","grant_owner":active&&exists&&context.scope=="ownership_grant","activate_offline":active&&owned&&goals&&context.scope=="offline_control","execute_offline":active&&owned&&goals&&context.scope=="offline_execution","paid_inference":false,"financial_operations":false,"reason":if active {"scoped_permission_only_finite_consent_still_required"} else {"permit_inactive"}});
        let worker=sqlx::query("SELECT c.status='running' AND c.lease_until>clock_timestamp() AND epoch_guard.operation_live(c.epoch_permit_id) AND w.enabled AND w.stopped_at IS NULL AND w.heartbeat_at>clock_timestamp()-interval '15 seconds' AND a.data->>'lifecycle'='active' AND EXISTS(SELECT 1 FROM agent_schedules s WHERE s.agent_id=a.id AND s.enabled AND s.expires_at>clock_timestamp()) AS running,(a.data->>'lifecycle'='suspended' OR c.status='running' AND (c.lease_until<=clock_timestamp() OR NOT epoch_guard.operation_live(c.epoch_permit_id))) AS recovery,w.enabled AND w.stopped_at IS NULL AND w.heartbeat_at>clock_timestamp()-interval '15 seconds' AS fresh,(w.last_error IS NOT NULL OR w.tick_finished_at IS NULL OR w.tick_finished_at<clock_timestamp()-make_interval(secs=>w.poll_seconds+15) OR w.tick_started_at>w.tick_finished_at AND w.tick_started_at<clock_timestamp()-interval '30 seconds') AS delayed,w.heartbeat_at,c.lease_until FROM cycles c JOIN agents a ON a.id=c.agent_id LEFT JOIN worker_health w ON w.id=c.worker_id WHERE c.agent_id=$1 ORDER BY c.started_at DESC LIMIT 1")
            .bind(context.agent_id).fetch_optional(&mut *tx).await.map_err(db)?;
        if let Some(w) = worker {
            let running = w.get::<Option<bool>, _>("running") == Some(true);
            let state = if w.get::<Option<bool>, _>("recovery") == Some(true) {
                "recovery_required"
            } else if w.get::<Option<bool>, _>("fresh") != Some(true) {
                "unavailable"
            } else if w.get::<Option<bool>, _>("delayed") == Some(true) {
                "delayed"
            } else {
                "healthy"
            };
            result["worker"] = json!({"status":state,"execution_running":running,"heartbeat_at":w.get::<Option<chrono::DateTime<Utc>>,_>("heartbeat_at"),"lease_until":w.get::<chrono::DateTime<Utc>,_>("lease_until")});
        }
        tx.commit().await.map_err(db)?;
        Ok(result)
    }
}
