//! Explicit offline draft preparation. No model calls, publication or accounting.
//! Actor IDs must come from authenticated server identity, never a request body.
//! Extension SQL is deliberately outside automatic migrations; no runtime grants.
use super::*;
use chrono::{DateTime, Duration};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use supa_domain::agent_identity::{AgentPurposeAuthority, AgentTokenAssociations, PurposeRevision};
use supa_domain::ecosystem_social::*;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcosystemConsentRequest {
    pub request_id: Uuid,
    pub expected_revision: u32,
    pub public: bool,
    pub revoked: bool,
    pub valid_from: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub categories: Vec<String>,
}
fn category(value: &str) -> Result<EcosystemEventKind> {
    Ok(match value {
        "agent_created" => EcosystemEventKind::AgentCreated,
        "agent_qualified" => EcosystemEventKind::AgentQualified,
        "task_completed" => EcosystemEventKind::TaskCompleted,
        "task_failed" => EcosystemEventKind::TaskFailed,
        "achievement" => EcosystemEventKind::Achievement,
        "token_activity" => EcosystemEventKind::TokenActivity,
        _ => return Err(Error::Invalid),
    })
}
/// One agent lock serializes consent, ACL changes and event reservation. No wallet lock.
pub(super) async fn scope(
    tx: &mut Tx<'_>,
    agent: Uuid,
    actor: Uuid,
    owner_only: bool,
) -> Result<(Uuid, DateTime<Utc>, DateTime<Utc>)> {
    if agent.is_nil() || actor.is_nil() {
        return Err(Error::Denied);
    }
    let epoch: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public.application_epoch)")
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
    if epoch {
        return Err(Error::Denied);
    } // No new operational-epoch scope is authorized.
    let allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public.agent_access WHERE agent_id=$1 AND principal_id=$2 AND (role='owner' OR (NOT $3 AND role='operator')))")
        .bind(agent).bind(actor).bind(owner_only).fetch_one(&mut **tx).await.map_err(db)?;
    if !allowed {
        return Err(Error::Denied);
    }
    sqlx::query("SELECT ecosystem_social.lock_agent($1)")
        .bind(agent)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    let created: String =
        sqlx::query_scalar("SELECT data->>'created_at' FROM public.agents WHERE id=$1")
            .bind(agent)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db)?
            .ok_or(Error::NotFound)?;
    let rows = sqlx::query("SELECT principal_id,role FROM public.agent_access WHERE agent_id=$1")
        .bind(agent)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
    let owners: Vec<Uuid> = rows
        .iter()
        .filter(|r| r.get::<String, _>("role") == "owner")
        .map(|r| r.get("principal_id"))
        .collect();
    if owners.len() != 1 {
        return Err(Error::Denied);
    }
    let role = rows
        .iter()
        .find(|r| r.get::<Uuid, _>("principal_id") == actor)
        .map(|r| r.get::<String, _>("role"));
    if (owner_only && actor != owners[0]) || !matches!(role.as_deref(), Some("owner" | "operator"))
    {
        return Err(Error::Denied);
    }
    let now = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
    let created: DateTime<Utc> = created.parse().map_err(|_| Error::Database)?;
    Ok((owners[0], created, now))
}
fn value<T: Serialize>(v: &T) -> Result<Value> {
    serde_json::to_value(v).map_err(|_| Error::Invalid)
}
async fn consent(tx: &mut Tx<'_>, agent: Uuid) -> Result<(Uuid, i64, EcosystemConsentRequest)> {
    let row=sqlx::query("SELECT creator_id,revision,body FROM ecosystem_social.consents WHERE agent_id=$1 ORDER BY revision DESC LIMIT 1")
        .bind(agent).fetch_optional(&mut **tx).await.map_err(db)?.ok_or(Error::Denied)?;
    Ok((
        row.get("creator_id"),
        row.get("revision"),
        serde_json::from_value(row.get("body")).map_err(|_| Error::Database)?,
    ))
}
impl Store {
    pub async fn record_ecosystem_purpose(
        &self,
        actor: Uuid,
        purpose: &PurposeRevision,
    ) -> Result<()> {
        if self.operational.is_some() {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        let (owner, _, now) = scope(&mut tx, purpose.agent_id, actor, true).await?;
        purpose
            .validate_for(
                &AgentPurposeAuthority {
                    agent_id: purpose.agent_id,
                    creator_id: owner,
                    current_purpose_revision_id: purpose.id,
                    current_review_id: Uuid::nil(),
                },
                now,
            )
            .map_err(|_| Error::Invalid)?;
        let body = value(purpose)?;
        if let Some(old) =
            sqlx::query_scalar::<_, Value>("SELECT body FROM ecosystem_social.purposes WHERE id=$1")
                .bind(purpose.id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
        {
            if old != body {
                return Err(Error::Stale);
            }
            return Ok(());
        }
        let revision: i64 = sqlx::query_scalar(
            "SELECT coalesce(max(revision),0) FROM ecosystem_social.purposes WHERE agent_id=$1",
        )
        .bind(purpose.agent_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if i64::from(purpose.revision) != revision + 1 {
            return Err(Error::Stale);
        }
        sqlx::query("INSERT INTO ecosystem_social.purposes(id,agent_id,creator_id,revision,body) VALUES($1,$2,$3,$4,$5)")
            .bind(purpose.id).bind(purpose.agent_id).bind(owner).bind(i64::from(purpose.revision)).bind(body).execute(&mut *tx).await.map_err(db)?;
        tx.commit().await.map_err(db)
    }
    pub async fn record_ecosystem_consent(
        &self,
        agent: Uuid,
        actor: Uuid,
        r: &EcosystemConsentRequest,
    ) -> Result<i64> {
        if self.operational.is_some() {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        let (owner, created, now) = scope(&mut tx, agent, actor, true).await?;
        let categories: BTreeSet<_> = r
            .categories
            .iter()
            .map(|s| category(s))
            .collect::<Result<_>>()?;
        if r.request_id.is_nil()
            || r.categories.len() > 6
            || categories.len() != r.categories.len()
            || r.valid_from < created
            || r.valid_from > now
            || r.expires_at > now + Duration::days(7)
            || (r.public && !r.revoked && r.expires_at <= now)
        {
            return Err(Error::Invalid);
        }
        let body = value(r)?;
        if let Some(old)=sqlx::query("SELECT agent_id,creator_id,revision,body FROM ecosystem_social.consents WHERE request_id=$1")
            .bind(r.request_id).fetch_optional(&mut *tx).await.map_err(db)? {
            if old.get::<Uuid,_>("agent_id")!=agent || old.get::<Uuid,_>("creator_id")!=owner || old.get::<Value,_>("body")!=body {return Err(Error::Stale);}
            return Ok(old.get("revision"));
        }
        let revision: i64 = sqlx::query_scalar(
            "SELECT coalesce(max(revision),0) FROM ecosystem_social.consents WHERE agent_id=$1",
        )
        .bind(agent)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if revision != i64::from(r.expected_revision) {
            return Err(Error::Stale);
        }
        sqlx::query("INSERT INTO ecosystem_social.consents(request_id,agent_id,creator_id,revision,body) VALUES($1,$2,$3,$4,$5)")
            .bind(r.request_id).bind(agent).bind(owner).bind(revision+1).bind(body).execute(&mut *tx).await.map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(revision + 1)
    }
    /// Persist only a model-input envelope. No generation or draft acceptance occurs.
    /// Restart-safe request replay rechecks current ACL, consent and original sources.
    pub async fn prepare_ecosystem_social(
        &self,
        agent: Uuid,
        actor: Uuid,
        request: Uuid,
    ) -> Result<Value> {
        if self.operational.is_some() || request.is_nil() {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        let (owner, _, now) = scope(&mut tx, agent, actor, false).await?;
        let (consent_owner, revision, grant) = consent(&mut tx, agent).await?;
        if consent_owner != owner
            || !grant.public
            || grant.revoked
            || grant.valid_from > now
            || grant.expires_at <= now
        {
            return Err(Error::Denied);
        }
        let old=sqlx::query("SELECT agent_id,actor,consent_revision,payload,expires_at FROM ecosystem_social.preparations WHERE request_id=$1")
            .bind(request).fetch_optional(&mut *tx).await.map_err(db)?;
        if let Some(old) = &old
            && (old.get::<Uuid, _>("agent_id") != agent
                || old.get::<Uuid, _>("actor") != actor
                || old.get::<i64, _>("consent_revision") != revision
                || old.get::<DateTime<Utc>, _>("expires_at") <= now)
        {
            return Err(Error::Stale);
        }
        // Authoritative creation and terminal offline-job records only.
        // Qualification and achievements require recorded human reviews; markets stay unavailable.
        let rows=sqlx::query("SELECT a.id,a.created_at,a.simulated,p.id AS purpose_id FROM public.activity a JOIN LATERAL (SELECT id FROM ecosystem_social.purposes WHERE agent_id=a.agent_id AND creator_id=$2 ORDER BY revision LIMIT 1) p ON true WHERE $3 AND a.created_at>=$4 AND a.agent_id=$1 AND a.kind='lifecycle' AND a.code='created' ORDER BY a.created_at DESC,a.id LIMIT 64")
            .bind(agent).bind(owner).bind(grant.categories.iter().any(|s|s=="agent_created")).bind(grant.valid_from).fetch_all(&mut *tx).await.map_err(db)?;
        let old_ids: Vec<Uuid> = if old.is_some() {
            sqlx::query_scalar("SELECT source_id FROM ecosystem_social.prepared_events WHERE request_id=$1 ORDER BY source_id")
            .bind(request).fetch_all(&mut *tx).await.map_err(db)?
        } else {
            vec![]
        };
        let mut candidates = vec![];
        for row in rows {
            let id: Uuid = row.get("id");
            let occurred = row.get("created_at");
            let simulated = row.get("simulated");
            let purpose = row.get("purpose_id");
            let record = PlatformSourceRecord {
                simulated,
                source_record_id: id,
                agent_id: agent,
                creator_id: owner,
                occurred_at: occurred,
                data: PlatformEventData::AgentCreated {
                    purpose_revision_id: purpose,
                },
            };
            let snapshot = json!({"agent_id":agent,"creator_id":owner,"kind":"agent_created","occurred_at":occurred,"simulated":simulated,"purpose_revision_id":purpose});
            candidates.push((record, "agent_created", snapshot));
        }
        candidates.extend(terminal_sources(&mut tx, agent, owner, &grant).await?);
        candidates.extend(
            crate::ecosystem_reviews::review_sources(&mut tx, agent, owner, &grant, now).await?,
        );
        candidates.sort_by_key(|(r, _, _)| std::cmp::Reverse((r.occurred_at, r.source_record_id)));
        candidates.truncate(64);
        let mut records = vec![];
        for (record, kind, snapshot) in candidates {
            let id = record.source_record_id;
            sqlx::query("INSERT INTO ecosystem_social.sources(source_id,agent_id,kind,snapshot) VALUES($1,$2,$3,$4) ON CONFLICT(source_id) DO NOTHING")
                .bind(id).bind(agent).bind(kind).bind(&snapshot).execute(&mut *tx).await.map_err(db)?;
            let saved: Value = sqlx::query_scalar(
                "SELECT snapshot FROM ecosystem_social.sources WHERE source_id=$1",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if saved != snapshot {
                return Err(Error::Stale);
            }
            let seen: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM ecosystem_social.prepared_events WHERE source_id=$1)",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if (old.is_some() && !old_ids.contains(&id)) || (old.is_none() && seen) {
                continue;
            }
            records.push(record);
        }
        let evidence: Vec<_> = records
            .iter()
            .map(|r| VerifiedPlatformEvidence {
                id: r.source_record_id,
                verifier_id: actor,
                origin: EvidenceOrigin::ApplicationRecord,
                record: r.clone(),
                verified_at: now,
            })
            .collect();
        let visibility = [CreatorPublicVisibility {
            agent_id: agent,
            creator_id: owner,
            revision: u32::try_from(revision).map_err(|_| Error::Invalid)?,
            public: grant.public,
            revoked: grant.revoked,
            valid_from: grant.valid_from,
            expires_at: grant.expires_at,
            categories: grant
                .categories
                .iter()
                .map(|s| category(s))
                .collect::<Result<_>>()?,
        }];
        let associations = AgentTokenAssociations::default(); // No market ingestion or token registry authority.
        let input = EcosystemProjectionInput {
            records: &records,
            evidence: &evidence,
            visibility: &visibility,
            associations: &associations,
            previous_drafts: &[],
        };
        let policy = EcosystemProjectionPolicy {
            authorized_agents: BTreeSet::from([agent]),
            authorized_verifiers: BTreeSet::from([actor]),
            approved_market_sources: BTreeSet::new(),
            verifier_version: 1,
            now,
            max_event_age_seconds: 86400,
            max_evidence_age_seconds: 60,
            market_max_age_seconds: None,
            market_max_window_seconds: None,
        };
        let context = project_public_events(input, &policy).map_err(|_| Error::Invalid)?;
        let ids: Vec<_> = context
            .events()
            .iter()
            .map(PublicPlatformEvent::id)
            .collect();
        if let Some(old) = old {
            if old_ids.len() != ids.len() || !old_ids.iter().all(|id| ids.contains(id)) {
                return Err(Error::Stale);
            }
            let payload: Value = old.get("payload");
            let fresh = context.model_input(now).map_err(|_| Error::Invalid)?;
            let normalize = |mut value: Value| {
                if let Some(events) = value.get_mut("events").and_then(Value::as_array_mut) {
                    for event in events {
                        if let Some(object) = event.as_object_mut() {
                            object.remove("verified_at");
                        }
                    }
                }
                value
            };
            // Verification time is refreshed by the loader; every other envelope field
            // must still match current policy, consent, source facts and schema.
            if normalize(payload.clone()) != normalize(fresh) {
                return Err(Error::Stale);
            }
            tx.commit().await.map_err(db)?;
            return Ok(payload);
        }
        let payload = context.model_input(now).map_err(|_| Error::Invalid)?;
        let expiry = grant.expires_at.min(now + Duration::seconds(30));
        sqlx::query("INSERT INTO ecosystem_social.preparations(request_id,agent_id,actor,consent_revision,payload,payload_sha256,expires_at) VALUES($1,$2,$3,$4,$5,encode(sha256(convert_to(public.work_review_canonical($5::jsonb),'UTF8')),'hex'),$6)")
            .bind(request).bind(agent).bind(actor).bind(revision).bind(&payload).bind(expiry).execute(&mut *tx).await.map_err(db)?;
        for id in ids {
            sqlx::query(
                "INSERT INTO ecosystem_social.prepared_events(source_id,request_id) VALUES($1,$2)",
            )
            .bind(id)
            .bind(request)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(payload)
    }
}

/// Derive outcomes from terminal jobs joined to their immutable terminal audit receipt.
/// No customer prose leaves this loader. Evaluation sessions and generic activity
/// cannot qualify. Completion must match the application's deterministic analyzer.
pub(super) async fn terminal_sources(
    tx: &mut Tx<'_>,
    agent: Uuid,
    owner: Uuid,
    grant: &EcosystemConsentRequest,
) -> Result<Vec<(PlatformSourceRecord, &'static str, Value)>> {
    if !grant
        .categories
        .iter()
        .any(|s| matches!(s.as_str(), "task_completed" | "task_failed"))
    {
        return Ok(vec![]);
    }
    let rows=sqlx::query("SELECT j.id,j.state,j.network,j.spec,j.input,j.output,j.output_sha256,j.failure_code,a.id AS audit_id,a.created_at,a.data FROM public.economic_jobs j JOIN LATERAL (SELECT id,created_at,data,operation FROM public.economic_audit WHERE job_id=j.id AND agent_id=j.agent_id ORDER BY id DESC LIMIT 1) a ON a.operation=j.state WHERE j.agent_id=$1 AND j.state IN ('completed','failed') AND j.execution_token IS NULL AND j.lease_until IS NULL AND a.created_at>=$2 AND j.output_sha256=encode(sha256(convert_to(j.output::text,'UTF8')),'hex') AND a.data->>'output_sha256'=j.output_sha256 ORDER BY a.created_at DESC,j.id LIMIT 64")
        .bind(agent).bind(grant.valid_from).fetch_all(&mut **tx).await.map_err(db)?;
    let mut result = vec![];
    for row in rows {
        let id: Uuid = row.get("id");
        let output: Value = row.get("output");
        if output["contract"] != "structured_report_v1"
            || output["live_inference"] != false
            || output["job_id"] != json!(id)
        {
            continue;
        }
        let state: String = row.get("state");
        let failure: Option<String> = row.get("failure_code");
        let audit: Value = row.get("data");
        if audit.get("failure_code") != Some(&json!(failure)) {
            continue;
        }
        let (kind, data) = if state == "completed"
            && grant.categories.iter().any(|s| s == "task_completed")
        {
            let spec: ServiceSpec = match serde_json::from_value(row.get("spec")) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let input: String = row.get("input");
            if spec.input(&input).is_err() || failure.is_some() {
                continue;
            }
            let expected =
                WorkRequest::parse(&input, spec.kind).and_then(|r| analyze_work(&r, spec.kind));
            let Ok(expected) = expected else {
                continue;
            };
            if expected["status"] != "completed"
                || expected.as_object().is_none_or(|fields| {
                    fields.iter().any(|(k, v)| output["work"].get(k) != Some(v))
                })
            {
                continue;
            }
            (
                "task_completed",
                PlatformEventData::TaskCompleted {
                    result_id: id,
                    state: CompletionState::Completed,
                    result_verified: true,
                    semantic_review: SemanticReviewState::NotRequired,
                },
            )
        } else if state == "failed" && grant.categories.iter().any(|s| s == "task_failed") {
            let reason = match failure.as_deref() {
                Some("incomplete_or_unsupported_work") => PublicFailureCode::InsufficientEvidence,
                Some("bounded_work_permission_denied") => PublicFailureCode::PermissionDenied,
                _ => continue,
            };
            if !matches!(
                output["work"]["status"].as_str(),
                Some("abstained" | "incomplete")
            ) {
                continue;
            }
            ("task_failed", PlatformEventData::TaskFailed { reason })
        } else {
            continue;
        };
        let occurred: DateTime<Utc> = row.get("created_at");
        let simulated = row.get::<String, _>("network") == "simulated";
        let snapshot = json!({"agent_id":agent,"creator_id":owner,"kind":kind,"occurred_at":occurred,"simulated":simulated,"audit_id":row.get::<i64,_>("audit_id"),"output_sha256":row.get::<String,_>("output_sha256"),"failure_code":failure});
        result.push((
            PlatformSourceRecord {
                simulated,
                source_record_id: id,
                agent_id: agent,
                creator_id: owner,
                occurred_at: occurred,
                data,
            },
            kind,
            snapshot,
        ));
    }
    Ok(result)
}
