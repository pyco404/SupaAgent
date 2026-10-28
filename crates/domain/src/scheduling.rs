use crate::*;

/// One non-renewable authorization. Resume never replenishes counters or extends expiry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulePolicy {
    pub goal_ids: Vec<Uuid>,
    pub interval_seconds: u64,
    pub max_cycles: i64,
    pub simulated_budget_microusd: i64,
    pub valid_for_seconds: i64,
}
impl SchedulePolicy {
    pub fn validate(&self) -> Result<()> {
        if self.goal_ids.is_empty()
            || self.goal_ids.len() > 10
            || self.goal_ids.iter().any(Uuid::is_nil)
            || self.goal_ids.iter().collect::<BTreeSet<_>>().len() != self.goal_ids.len()
            || !(5..=86400).contains(&self.interval_seconds)
            || !(1..=100).contains(&self.max_cycles)
            || !(1100..=110000).contains(&self.simulated_budget_microusd)
            || !(60..=86400).contains(&self.valid_for_seconds)
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleOperation {
    Start,
    Pause,
    Resume,
    Stop,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleCommand {
    pub request_id: Uuid,
    pub expected_revision: i64,
    pub operation: ScheduleOperation,
    pub authorize_offline: bool,
    pub reason: String,
    pub policy: Option<SchedulePolicy>,
}
impl ScheduleCommand {
    pub fn validate(&self) -> Result<()> {
        if !self.authorize_offline {
            return Err(Error::Denied);
        }
        if self.request_id.is_nil() || self.expected_revision < 0 {
            return Err(Error::Invalid);
        }
        clean_text(&self.reason, 500)?;
        match (&self.operation, &self.policy) {
            (ScheduleOperation::Start, Some(policy)) => policy.validate(),
            (ScheduleOperation::Start, None) | (_, Some(_)) => Err(Error::Invalid),
            _ => Ok(()),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleStatus {
    pub agent_id: Uuid,
    pub lifecycle: Lifecycle,
    pub revision: i64,
    pub enabled: bool,
    pub policy: Option<SchedulePolicy>,
    pub cycles_started: i64,
    /// Conservative authorization consumption, not a real or simulated ledger transaction.
    pub reserved_microusd: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub next_cycle_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub mode: String,
    pub simulated: bool,
    pub paid_requests_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scheduler_authorization_has_strict_bounds_and_no_live_options() {
        assert!(!CycleConfig::default().scheduler_enabled);
        let good = SchedulePolicy {
            goal_ids: vec![Uuid::new_v4()],
            interval_seconds: 5,
            max_cycles: 3,
            simulated_budget_microusd: 3300,
            valid_for_seconds: 3600,
        };
        good.validate().unwrap();
        for bad in [
            SchedulePolicy {
                interval_seconds: 0,
                ..good.clone()
            },
            SchedulePolicy {
                max_cycles: 101,
                ..good.clone()
            },
            SchedulePolicy {
                simulated_budget_microusd: 1099,
                ..good.clone()
            },
            SchedulePolicy {
                valid_for_seconds: 86401,
                ..good.clone()
            },
            SchedulePolicy {
                goal_ids: vec![good.goal_ids[0]; 2],
                ..good.clone()
            },
        ] {
            assert!(bad.validate().is_err());
        }
        assert!(serde_json::from_value::<SchedulePolicy>(serde_json::json!({"goal_ids":good.goal_ids,"interval_seconds":5,"max_cycles":3,"simulated_budget_microusd":3300,"valid_for_seconds":3600,"provider":"openai"})).is_err());
    }
}
