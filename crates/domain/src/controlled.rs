//! One paid request per explicit session. Summaries are outputs, never hidden reasoning.
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveConsent {
    pub request_id: Uuid,
    pub goal: String,
    pub model: String,
    pub authorize_paid: bool,
    pub consented_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub max_requests: u8,
    pub max_input_tokens: u64,
    pub max_output_tokens: u64,
    pub max_cost_microusd: i64,
}
impl LiveConsent {
    pub fn validate(&self) -> Result<()> {
        clean_text(&self.goal, 500)?;
        clean_text(&self.model, 100)?;
        let now = Utc::now();
        if !self.authorize_paid {
            return Err(Error::Denied);
        }
        if self.request_id.is_nil()
            || self.max_requests != 1
            || !(9000..=24192).contains(&self.max_input_tokens)
            || !(256..=4096).contains(&self.max_output_tokens)
            || !(1..=100000).contains(&self.max_cost_microusd)
            || (now - self.consented_at).num_seconds() > 120
            || (now - self.consented_at).num_seconds() < -5
            || self.expires_at <= now
            || (self.expires_at - self.consented_at).num_seconds() > 600
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveProposal {
    pub summary: String,
    pub memory_ids: Vec<Uuid>,
    pub actions: Vec<ProposedAction>,
}
impl LiveProposal {
    pub fn validate(&self, memories: &[Memory]) -> Result<()> {
        clean_text(&self.summary, 1000)?;
        if self.actions.len() > 1
            || self.memory_ids.len() > 12
            || self
                .memory_ids
                .iter()
                .any(|id| !memories.iter().any(|m| m.id == *id))
            || self.memory_ids.iter().collect::<BTreeSet<_>>().len() != self.memory_ids.len()
            || self.actions.iter().any(|a| {
                a.tool != "read_memory"
                    || !a.event_ids.is_empty()
                    || a.lesson.is_some()
                    || a.evaluation.is_some()
            })
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSession {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub approved_by: Uuid,
    pub consent: LiveConsent,
    pub policy: InferencePolicy,
    pub status: String,
    pub memory_ids: Vec<Uuid>,
    pub proposal: Option<LiveProposal>,
    pub usage: Option<TokenUsage>,
    pub reserved_microusd: i64,
    pub accounted_microusd: i64,
    pub failure_code: Option<String>,
    pub metadata_requests: u8,
    pub generation_requests: u8,
    pub project_verified: bool,
    pub tool_result: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consent_is_finite_fresh_explicit_and_rejects_extra_fields() {
        let c = LiveConsent {
            request_id: Uuid::new_v4(),
            goal: "Review memories".into(),
            model: "gpt-6-luna".into(),
            authorize_paid: true,
            consented_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::minutes(5),
            max_requests: 1,
            max_input_tokens: 24192,
            max_output_tokens: 1024,
            max_cost_microusd: 10000,
        };
        assert!(c.validate().is_ok());
        let mut stale = c.clone();
        stale.consented_at -= chrono::Duration::minutes(3);
        assert!(stale.validate().is_err());
        let mut expired = c.clone();
        expired.expires_at = Utc::now();
        assert!(expired.validate().is_err());
        let mut denied = c.clone();
        denied.authorize_paid = false;
        assert!(matches!(denied.validate(), Err(Error::Denied)));
        let mut many = c.clone();
        many.max_requests = 2;
        assert!(many.validate().is_err());
        let mut v = serde_json::to_value(c).unwrap();
        v["api_key"] = serde_json::json!("forbidden");
        assert!(serde_json::from_value::<LiveConsent>(v).is_err());
    }
    #[test]
    fn proposal_cannot_invent_memories_or_expand_tools() {
        let mut p = LiveProposal {
            summary: "No relevant memories supplied.".into(),
            memory_ids: vec![],
            actions: vec![],
        };
        assert!(p.validate(&[]).is_ok());
        p.memory_ids.push(Uuid::new_v4());
        assert!(p.validate(&[]).is_err());
        p.memory_ids.clear();
        p.actions.push(ProposedAction {
            tool: "read_memory".into(),
            event_ids: vec![],
            lesson: None,
            evaluation: None,
        });
        assert!(p.validate(&[]).is_ok());
        p.actions[0].tool = "transfer".into();
        assert!(p.validate(&[]).is_err());
    }
}
