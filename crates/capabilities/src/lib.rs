use supa_domain::{Agent, Capability, Error, ProposedAction, Result, clean_text};

pub struct Tool {
    pub name: &'static str,
    pub capability: Capability,
}
pub const REGISTRY: &[Tool] = &[
    Tool {
        name: "consider_tweet",
        capability: Capability::DraftTweet,
    },
    Tool {
        name: "evaluate_goal",
        capability: Capability::EvaluateGoal,
    },
    Tool {
        name: "conserve",
        capability: Capability::Conserve,
    },
    Tool {
        name: "read_memory",
        capability: Capability::ReadMemory,
    },
    Tool {
        name: "remember",
        capability: Capability::Remember,
    },
    Tool {
        name: "draft_tweet",
        capability: Capability::DraftTweet,
    },
];
pub fn authorize(agent: &Agent, action: &ProposedAction) -> Result<Capability> {
    let tool = REGISTRY
        .iter()
        .find(|t| t.name == action.tool)
        .ok_or(Error::Denied)?;
    if !agent.capabilities.contains(&tool.capability) {
        return Err(Error::Denied);
    }
    if action.event_ids.len() > 8 {
        return Err(Error::Invalid);
    }
    match tool.capability {
        Capability::Remember => clean_text(action.lesson.as_deref().ok_or(Error::Invalid)?, 1000)?,
        Capability::DraftTweet if action.tool == "draft_tweet" && action.event_ids.is_empty() => {
            return Err(Error::Invalid);
        }
        Capability::EvaluateGoal => {
            let e = action.evaluation.as_ref().ok_or(Error::Invalid)?;
            if e.progress_bps > 10000
                || action.event_ids.is_empty()
                || !agent
                    .goals
                    .iter()
                    .any(|g| g.id == e.goal_id && !g.completed)
            {
                return Err(Error::Invalid);
            }
            clean_text(&e.assessment, 500)?;
        }
        _ => {}
    }
    Ok(tool.capability)
}
/// Audit tool names only from the fixed registry, never arbitrary model strings.
pub fn audit_name(name: &str) -> &'static str {
    REGISTRY
        .iter()
        .find(|t| t.name == name)
        .map(|t| t.name)
        .unwrap_or("unknown")
}
#[cfg(test)]
mod tests {
    use super::*;
    use supa_domain::{CycleConfig, Personality};
    #[test]
    fn tools_cannot_grant_permissions_or_execute_shell() {
        let mut a = Agent::new(
            "Supa".into(),
            Personality::default(),
            CycleConfig::default(),
            Default::default(),
        )
        .unwrap();
        let mut p = ProposedAction {
            evaluation: None,
            tool: "remember".into(),
            event_ids: vec![],
            lesson: Some("I should check my assumptions.".into()),
        };
        assert!(authorize(&a, &p).is_err());
        a.capabilities.insert(Capability::Remember);
        assert!(authorize(&a, &p).is_ok());
        for tool in [
            "shell",
            "trade",
            "publish",
            "create_agent",
            "transfer",
            "set_permissions",
            "simulate_revenue",
        ] {
            p.tool = tool.into();
            assert!(authorize(&a, &p).is_err());
        }
    }
}
