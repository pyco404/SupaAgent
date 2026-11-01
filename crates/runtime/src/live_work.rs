//! One owner-authorized paid request. Never invoked by economic execution, tick or worker.
use super::*;
use crate::controlled::ControlledInference;
use supa_persistence::Role;
impl ControlledInference {
    pub async fn run_work(
        &self,
        agent: Uuid,
        actor: Uuid,
        authorization: LiveWorkConsent,
    ) -> Result<LiveSession> {
        if self.provider.project().is_empty() {
            return Err(Error::Denied);
        }
        let policy = self.provider.policy();
        let (session, memories) = self
            .store
            .reserve_live_work_session(
                agent,
                actor,
                &authorization,
                &policy,
                self.provider.work_evidence_kind(),
            )
            .await?;
        let work = session.work.as_ref().ok_or(Error::Invalid)?;
        let input = LiveWorkInput {
            request: work.request.clone(),
            kind: work.kind,
            approved_tools: authorization.approved_tools.clone(),
            memories,
            deterministic_findings: work.baseline["findings"].clone(),
        };
        if let Err(error) = self
            .store
            .live_request_stage(agent, session.id, false)
            .await
        {
            return self
                .store
                .settle_live_work_session(agent, session.id, None, None, Some(&error), false)
                .await;
        }
        let verified = tokio::time::timeout(Duration::from_secs(26), self.provider.verify())
            .await
            .unwrap_or(Err(Error::Timeout));
        if let Err(error) = verified {
            return self
                .store
                .settle_live_work_session(agent, session.id, None, None, Some(&error), false)
                .await;
        }
        if let Err(error) = self.store.live_request_stage(agent, session.id, true).await {
            return self
                .store
                .settle_live_work_session(agent, session.id, None, None, Some(&error), false)
                .await;
        }
        let invocation = tokio::time::timeout(Duration::from_secs(26), self.provider.work(&input))
            .await
            .unwrap_or(Invocation {
                result: Err(Error::Timeout),
                usage: None,
            });
        // Accounting must settle even if permissions were revoked during the provider request.
        let report = async {
            let report = invocation.result?;
            report.validate(&input)?;
            self.store.require_access(agent, actor, Role::Owner).await?;
            let current = self.store.get(agent).await?;
            if matches!(
                current.lifecycle,
                Lifecycle::Suspended | Lifecycle::Terminated
            ) || !current.capabilities.contains(&Capability::EvaluateGoal)
                || !current.capabilities.contains(&Capability::ReadMemory)
                || authorization.consent.expires_at <= chrono::Utc::now()
            {
                return Err(Error::Denied);
            }
            // Pure application tools are explicitly authorized by the exact consent. No hosted effects.
            for tool in &report.tools {
                self.store.require_access(agent, actor, Role::Owner).await?;
                match tool.as_str() {
                    "inspect_input" => {
                        clean_text(&work.request.material, 4000)?;
                    }
                    "read_memory" => {
                        if !authorization.use_memory {
                            return Err(Error::Denied);
                        }
                    }
                    "extract_evidence" | "review_code" | "analyze_data" | "decompose_task"
                    | "check_requirements" => {
                        analyze_work(&work.request, work.kind)?;
                    }
                    _ => return Err(Error::Denied),
                }
            }
            Ok(report)
        }
        .await;
        self.store
            .settle_live_work_session(
                agent,
                session.id,
                report.as_ref().ok().cloned(),
                invocation.usage,
                report.as_ref().err(),
                true,
            )
            .await
    }
}
