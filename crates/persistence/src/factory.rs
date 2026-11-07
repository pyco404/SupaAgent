//! Atomic, owner-bound, private synthetic creation. No execution or token launch.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use supa_domain::agent_identity::{AgentPurposeAuthority, PurposeRevision};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactoryRequest {
    pub request_id: Uuid,
    pub name: String,
    pub description: String,
    pub purpose: String,
    pub personality: Personality,
    pub goals: Vec<String>,
    pub requested_capabilities: BTreeSet<Capability>,
    pub autonomy: String,
    pub spending_limit_microusd: u64,
    pub private_consent: bool,
}
impl FactoryRequest {
    pub fn validate(&self) -> Result<()> {
        if self.request_id.is_nil()
            || self.autonomy != "disabled"
            || self.spending_limit_microusd != 0
            || !self.private_consent
            || self.goals.is_empty()
            || self.goals.len() > 8
            || self.goals.iter().collect::<BTreeSet<_>>().len() != self.goals.len()
        {
            return Err(Error::Invalid);
        }
        for (text, max) in [
            (&self.name, 80),
            (&self.description, 600),
            (&self.purpose, 600),
            (&self.personality.voice, 1000),
        ] {
            clean_text(text, max)?;
            if text.trim() != text || text.chars().any(char::is_control) {
                return Err(Error::Invalid);
            }
        }
        for trait_text in &self.personality.traits {
            if trait_text.trim() != trait_text || trait_text.chars().any(char::is_control) {
                return Err(Error::Invalid);
            }
        }
        for goal in &self.goals {
            clean_text(goal, 240)?;
            if goal.trim() != goal || goal.chars().any(char::is_control) {
                return Err(Error::Invalid);
            }
        }
        Agent::new(
            self.name.clone(),
            self.personality.clone(),
            CycleConfig::default(),
            BTreeSet::new(),
        )?;
        Ok(())
    }
}
fn view(agent: Uuid, configuration: Value) -> Value {
    json!({"agent_id":agent,"configuration":configuration,"status":"paused",
        "qualification":"pending_review","public":false,"enabled_capabilities":[],
        "execution_enabled":false,"token_mint":null})
}
impl Store {
    pub async fn factory_managed(&self, id: Uuid) -> Result<bool> {
        sqlx::query_scalar("SELECT agent_factory.managed($1)")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(db)
    }
    pub async fn factory_create(&self, actor: Uuid, r: &FactoryRequest) -> Result<Value> {
        r.validate()?;
        if actor.is_nil() || self.operational.is_some() {
            return Err(Error::Denied);
        }
        let body = serde_json::to_value(r).map_err(|_| Error::Invalid)?;
        if self.application_epoch().await?.is_some() {
            return Err(Error::Denied);
        }
        let mut a = Agent::new(
            r.name.clone(),
            r.personality.clone(),
            CycleConfig::default(),
            BTreeSet::new(),
        )?;
        a.lifecycle = Lifecycle::Paused;
        a.config.inference_cost_microusd = 0;
        a.config.operating_cost_microusd = 0;
        a.config.draft_cost_microusd = 0;
        a.goals = r
            .goals
            .iter()
            .map(|description| Goal {
                id: Uuid::new_v4(),
                description: description.clone(),
                completed: false,
                priority: 0,
                suggested_progress_bps: 0,
            })
            .collect();
        let purpose = PurposeRevision {
            id: Uuid::new_v4(),
            agent_id: a.id,
            creator_id: actor,
            revision: 1,
            created_at: a.created_at,
            objective: r.purpose.clone(),
            permitted_tasks: vec![ServiceKind::Summary],
            success_criteria: r.goals.clone(),
            excluded_actions: vec![
                "No execution, spending, signing, publishing or token launches".into(),
            ],
            public_summary: None,
        };
        purpose
            .validate_for(
                &AgentPurposeAuthority {
                    agent_id: a.id,
                    creator_id: actor,
                    current_purpose_revision_id: purpose.id,
                    current_review_id: Uuid::nil(),
                },
                Utc::now(),
            )
            .map_err(|_| Error::Invalid)?;
        let consent = EcosystemConsentRequest {
            request_id: Uuid::new_v4(),
            expected_revision: 0,
            public: false,
            revoked: true,
            valid_from: a.created_at,
            expires_at: a.created_at + chrono::Duration::days(1),
            categories: vec![],
        };
        let id: Uuid = sqlx::query_scalar("SELECT agent_factory.create_private($1,$2,$3,$4,$5,$6)")
            .bind(actor)
            .bind(r.request_id)
            .bind(&body)
            .bind(encode(&a)?)
            .bind(serde_json::to_value(purpose).map_err(|_| Error::Invalid)?)
            .bind(serde_json::to_value(consent).map_err(|_| Error::Invalid)?)
            .fetch_one(&self.pool)
            .await
            .map_err(
                |e| match e.as_database_error().and_then(|e| e.code()).as_deref() {
                    Some("40001") => Error::Stale,
                    Some("42501") => Error::Denied,
                    _ => db(e),
                },
            )?;
        Ok(view(id, body))
    }
    pub async fn factory_owned(&self, actor: Uuid) -> Result<Vec<Value>> {
        if actor.is_nil() || self.operational.is_some() {
            return Err(Error::Denied);
        }
        let rows = sqlx::query("SELECT * FROM agent_factory.owned($1)")
            .bind(actor)
            .fetch_all(&self.pool)
            .await
            .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| view(r.get("agent_id"), r.get("configuration")))
            .collect())
    }
}
