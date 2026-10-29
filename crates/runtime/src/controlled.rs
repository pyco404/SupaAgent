//! Operator-only single-shot inference. Never called by tick(), workers or offline cycles.
use super::*;
use supa_providers::controlled::ControlledProvider;
#[derive(Clone)]
pub struct ControlledInference {
    pub store: Store,
    pub provider: Arc<dyn ControlledProvider>,
}
impl ControlledInference {
    pub async fn run(&self, agent: Uuid, actor: Uuid, consent: LiveConsent) -> Result<LiveSession> {
        let policy = self.provider.policy();
        if self.provider.project().is_empty() {
            return Err(Error::Denied);
        }
        let (session, memory) = self
            .store
            .reserve_live_session(agent, actor, &consent, &policy)
            .await?;
        self.store
            .live_request_stage(agent, session.id, false)
            .await?;
        let verified = tokio::time::timeout(Duration::from_secs(26), self.provider.verify())
            .await
            .unwrap_or(Err(Error::Timeout));
        if let Err(e) = verified {
            return self
                .store
                .settle_live_session(agent, session.id, None, None, Some(&e), false)
                .await;
        }
        // Fresh consent is checked again after account verification, before generation.
        if consent.expires_at <= chrono::Utc::now() {
            return self
                .store
                .settle_live_session(agent, session.id, None, None, Some(&Error::Stale), false)
                .await;
        }
        self.store
            .live_request_stage(agent, session.id, true)
            .await?;
        let invocation = tokio::time::timeout(
            Duration::from_secs(26),
            self.provider.reason(&consent.goal, &memory),
        )
        .await
        .unwrap_or(Invocation {
            result: Err(Error::Timeout),
            usage: None,
        });
        let outcome = invocation.result.and_then(|proposal| {
            proposal.validate(&memory)?;
            let a = proposal
                .actions
                .iter()
                .map(|p| p.tool.as_str())
                .collect::<Vec<_>>();
            if a.iter().any(|name| *name != "read_memory") {
                return Err(Error::Denied);
            }
            Ok(proposal)
        });
        self.store
            .settle_live_session(
                agent,
                session.id,
                outcome.as_ref().ok().cloned(),
                invocation.usage,
                outcome.as_ref().err(),
                true,
            )
            .await
    }
    pub async fn approve(
        &self,
        agent: Uuid,
        id: Uuid,
        actor: Uuid,
        approve: bool,
    ) -> Result<LiveSession> {
        let a = self.store.get(agent).await?;
        let s = self
            .store
            .live_sessions(agent)
            .await?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or(Error::NotFound)?;
        if approve {
            for action in &s.proposal.as_ref().ok_or(Error::Invalid)?.actions {
                authorize(&a, action)?;
            }
        }
        self.store
            .approve_live_tools(agent, id, actor, approve)
            .await
    }
}
