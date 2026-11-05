//! Explicit owner reviews. No evaluation-review delegation, wallet or token grants.
use super::*;
use crate::ecosystem_social::{scope, terminal_sources};
use chrono::{DateTime, Duration};
use serde::{Deserialize, Serialize};
use supa_domain::agent_identity::*;
use supa_domain::ecosystem_social::*;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcosystemAchievementRequest {
    pub id: Uuid,
    pub result_id: Uuid,
    pub result_sha256: String,
    /// "accept" or "revoke"; acceptance never completes a job or pays revenue.
    pub decision: String,
    pub confirm_semantic_review: bool,
}
async fn current_purpose(tx: &mut Tx<'_>, agent: Uuid, owner: Uuid) -> Result<PurposeRevision> {
    let body:Value=sqlx::query_scalar("SELECT body FROM ecosystem_social.purposes WHERE agent_id=$1 AND creator_id=$2 ORDER BY revision DESC LIMIT 1")
        .bind(agent).bind(owner).fetch_optional(&mut **tx).await.map_err(db)?.ok_or(Error::Denied)?;
    serde_json::from_value(body).map_err(|_| Error::Database)
}
async fn qualification_valid(
    tx: &mut Tx<'_>,
    agent: Uuid,
    owner: Uuid,
    r: &QualificationReview,
    now: DateTime<Utc>,
) -> Result<bool> {
    let purpose = current_purpose(tx, agent, owner).await?;
    let row=sqlx::query("SELECT version,enabled,reviewers,max_validity_seconds FROM ecosystem_social.qualification_policies WHERE id=$1 ORDER BY version DESC LIMIT 1")
        .bind(r.policy_id).fetch_optional(&mut **tx).await.map_err(db)?.ok_or(Error::Denied)?;
    if !row.get::<bool, _>("enabled") || r.reviewer_id != owner {
        return Err(Error::Denied);
    }
    let policy = AgentQualificationPolicy {
        id: r.policy_id,
        version: u32::try_from(row.get::<i32, _>("version")).map_err(|_| Error::Database)?,
        authorized_reviewers: row.get::<Vec<Uuid>, _>("reviewers").into_iter().collect(),
        max_validity_seconds: u32::try_from(row.get::<i32, _>("max_validity_seconds"))
            .map_err(|_| Error::Database)?,
    };
    let authority = AgentPurposeAuthority {
        agent_id: agent,
        creator_id: owner,
        current_purpose_revision_id: purpose.id,
        current_review_id: r.id,
    };
    match (AgentQualificationContext {
        authority: &authority,
        purpose: &purpose,
        policy: &policy,
        review: r,
        now,
    })
    .validate()
    {
        Ok(()) => {}
        Err(AgentIdentityRejection::NotQualified) => {}
        Err(_) => return Err(Error::Denied),
    }
    // Evidence is not caller-asserted: only actual creation receipts for this agent
    // qualify for this purpose-review slice; arbitrary activity and IDs fail closed.
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM public.activity WHERE agent_id=$1 AND id=ANY($2) AND kind='lifecycle' AND code='created' AND created_at<=$3")
        .bind(agent).bind(&r.evidence_ids).bind(r.reviewed_at).fetch_one(&mut **tx).await.map_err(db)?;
    if count != r.evidence_ids.len() as i64 {
        return Err(Error::Denied);
    }
    Ok(r.decision == AgentQualificationDecision::Qualified)
}
async fn append(
    tx: &mut Tx<'_>,
    id: Uuid,
    agent: Uuid,
    owner: Uuid,
    kind: &str,
    target: Uuid,
    body: Value,
) -> Result<()> {
    if id.is_nil() || target.is_nil() {
        return Err(Error::Invalid);
    }
    if let Some(row)=sqlx::query("SELECT agent_id,creator_id,kind,target_id,body FROM ecosystem_social.owner_reviews WHERE id=$1").bind(id).fetch_optional(&mut **tx).await.map_err(db)? {
        if row.get::<Uuid,_>("agent_id")!=agent || row.get::<Uuid,_>("creator_id")!=owner || row.get::<String,_>("kind")!=kind || row.get::<Uuid,_>("target_id")!=target || row.get::<Value,_>("body")!=body {return Err(Error::Stale);}
        return Ok(());
    }
    sqlx::query("INSERT INTO ecosystem_social.owner_reviews(id,agent_id,creator_id,kind,target_id,body) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(id).bind(agent).bind(owner).bind(kind).bind(target).bind(body).execute(&mut **tx).await.map_err(db)?;
    Ok(())
}
fn internal_grant(created: DateTime<Utc>, now: DateTime<Utc>) -> EcosystemConsentRequest {
    EcosystemConsentRequest {
        request_id: Uuid::nil(),
        expected_revision: 0,
        public: false,
        revoked: true,
        valid_from: created,
        expires_at: now,
        categories: vec!["task_completed".into()],
    }
}
impl Store {
    /// Requires both agent ownership and administrator-provisioned reviewer policy.
    /// A missing/disabled policy never falls back to owner self-qualification.
    pub async fn record_ecosystem_qualification(
        &self,
        agent: Uuid,
        actor: Uuid,
        r: &QualificationReview,
    ) -> Result<()> {
        if self.operational.is_some() {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        let (owner, _, now) = scope(&mut tx, agent, actor, true).await?;
        if r.reviewer_id != actor || r.reviewed_at < now - Duration::minutes(5) {
            return Err(Error::Denied);
        }
        qualification_valid(&mut tx, agent, owner, r, now).await?;
        append(
            &mut tx,
            r.id,
            agent,
            owner,
            "qualification",
            r.purpose.id,
            serde_json::to_value(r).map_err(|_| Error::Invalid)?,
        )
        .await?;
        tx.commit().await.map_err(db)
    }
    /// Explicit semantic acceptance of a deterministic, already-completed offline
    /// result. An evaluation-only session cannot qualify as a delivered result.
    pub async fn record_ecosystem_achievement(
        &self,
        agent: Uuid,
        actor: Uuid,
        r: &EcosystemAchievementRequest,
    ) -> Result<()> {
        if self.operational.is_some()
            || !r.confirm_semantic_review
            || !matches!(r.decision.as_str(), "accept" | "revoke")
        {
            return Err(Error::Denied);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        let (owner, created, now) = scope(&mut tx, agent, actor, true).await?;
        let candidates =
            terminal_sources(&mut tx, agent, owner, &internal_grant(created, now)).await?;
        if !candidates.iter().any(|(record, _, snapshot)| {
            record.source_record_id == r.result_id && snapshot["output_sha256"] == r.result_sha256
        }) {
            return Err(Error::Denied);
        }
        append(
            &mut tx,
            r.id,
            agent,
            owner,
            "achievement",
            r.result_id,
            serde_json::to_value(r).map_err(|_| Error::Invalid)?,
        )
        .await?;
        tx.commit().await.map_err(db)
    }
}
/// Latest reviews are revalidated at preparation/replay, including revocation,
/// current purpose, current policy version and the original result's hash.
pub(super) async fn review_sources(
    tx: &mut Tx<'_>,
    agent: Uuid,
    owner: Uuid,
    grant: &EcosystemConsentRequest,
    now: DateTime<Utc>,
) -> Result<Vec<(PlatformSourceRecord, &'static str, Value)>> {
    let wants_qualification = grant.categories.iter().any(|s| s == "agent_qualified");
    let wants_achievement = grant.categories.iter().any(|s| s == "achievement");
    if !wants_qualification && !wants_achievement {
        return Ok(vec![]);
    }
    let rows=sqlx::query("SELECT * FROM (SELECT DISTINCT ON (kind,CASE WHEN kind='qualification' THEN agent_id ELSE target_id END) id,kind,target_id,body,recorded_at FROM ecosystem_social.owner_reviews WHERE agent_id=$1 AND creator_id=$2 ORDER BY kind,CASE WHEN kind='qualification' THEN agent_id ELSE target_id END,recorded_at DESC,id DESC) latest WHERE recorded_at>=$3 ORDER BY recorded_at DESC,id LIMIT 64")
        .bind(agent).bind(owner).bind(grant.valid_from).fetch_all(&mut **tx).await.map_err(db)?;
    let terminal = if wants_achievement {
        let created: DateTime<Utc> = sqlx::query_scalar(
            "SELECT (data->>'created_at')::timestamptz FROM public.agents WHERE id=$1",
        )
        .bind(agent)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        terminal_sources(tx, agent, owner, &internal_grant(created, now)).await?
    } else {
        vec![]
    };
    let mut sources = vec![];
    for row in rows {
        let id: Uuid = row.get("id");
        let body: Value = row.get("body");
        let occurred: DateTime<Utc> = row.get("recorded_at");
        let (kind, data, simulated) = match row.get::<String, _>("kind").as_str() {
            "qualification" if wants_qualification => {
                let r: QualificationReview =
                    serde_json::from_value(body.clone()).map_err(|_| Error::Database)?;
                match qualification_valid(tx, agent, owner, &r, now).await {
                    Ok(true) => {}
                    Ok(false) | Err(Error::Denied) => continue,
                    Err(e) => return Err(e),
                };
                let simulated:bool=sqlx::query_scalar("SELECT coalesce(bool_or(simulated),false) FROM public.activity WHERE id=ANY($1) AND agent_id=$2").bind(&r.evidence_ids).bind(agent).fetch_one(&mut **tx).await.map_err(db)?;
                (
                    "agent_qualified",
                    PlatformEventData::AgentQualified {
                        purpose_revision_id: r.purpose.id,
                        review_id: id,
                        decision: r.decision,
                    },
                    simulated,
                )
            }
            "achievement" if wants_achievement => {
                let r: EcosystemAchievementRequest =
                    serde_json::from_value(body.clone()).map_err(|_| Error::Database)?;
                if r.decision != "accept" || !r.confirm_semantic_review {
                    continue;
                }
                let Some((record, _, _snapshot)) = terminal.iter().find(|(record, _, snapshot)| {
                    record.source_record_id == r.result_id
                        && snapshot["output_sha256"] == r.result_sha256
                }) else {
                    continue;
                };
                (
                    "achievement",
                    PlatformEventData::Achievement {
                        result_id: r.result_id,
                        semantic_review: SemanticReviewState::Accepted,
                    },
                    record.simulated,
                )
            }
            _ => continue,
        };
        let digest:String=sqlx::query_scalar("SELECT encode(sha256(convert_to(public.work_review_canonical($1::jsonb),'UTF8')),'hex')").bind(body).fetch_one(&mut **tx).await.map_err(db)?;
        let snapshot = json!({"agent_id":agent,"creator_id":owner,"kind":kind,"occurred_at":occurred,"simulated":simulated,"review_sha256":digest});
        sources.push((
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
    Ok(sources)
}
