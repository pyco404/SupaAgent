//! Contractual job economics. Deposits are liabilities until verifiable delivery.
use crate::*;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EconomicNetwork {
    Simulated,
    Devnet,
    Mainnet,
}
impl EconomicNetwork {
    pub fn name(self) -> &'static str {
        match self {
            Self::Simulated => "simulated",
            Self::Devnet => "devnet",
            Self::Mainnet => "mainnet",
        }
    }
    pub fn enabled(self) -> Result<()> {
        if self == Self::Mainnet {
            Err(Error::FinancialPolicy)
        } else {
            Ok(())
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    Summary,
    ResearchReport,
    CodeAnalysis,
    ApiReport,
    DelegationSimulation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceSpec {
    pub name: String,
    pub kind: ServiceKind,
    pub price_atoms: i64,
    pub estimated_inference_atoms: i64,
    pub estimated_infrastructure_atoms: i64,
    pub max_input_bytes: usize,
    pub max_attempts: u8,
}
impl ServiceSpec {
    pub fn validate(&self) -> Result<()> {
        clean_text(&self.name, 80)?;
        if !(2000..=1_000_000_000).contains(&self.price_atoms)
            || self.estimated_inference_atoms != 1000
            || self.estimated_infrastructure_atoms != 100
            || !(1..=8000).contains(&self.max_input_bytes)
            || !(1..=2).contains(&self.max_attempts)
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    pub fn cost(&self, network: EconomicNetwork) -> i64 {
        if network == EconomicNetwork::Simulated {
            1100
        } else {
            0
        }
    }
    pub fn input(&self, text: &str) -> Result<()> {
        self.validate()?;
        clean_text(text, self.max_input_bytes)?;
        if self.kind == ServiceKind::ApiReport
            && !serde_json::from_str::<Value>(text).is_ok_and(|v| v.is_object())
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    pub fn output_contract(&self) -> &'static str {
        "structured_report_v1; supplied-input analysis only; no external mutations or hidden reasoning"
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicService {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub spec: ServiceSpec,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Created,
    AwaitingPayment,
    Funded,
    Accepted,
    Executing,
    Completed,
    Failed,
    Cancelled,
    Expired,
    Disputed,
}
impl JobState {
    pub fn name(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::AwaitingPayment => "awaiting_payment",
            Self::Funded => "funded",
            Self::Accepted => "accepted",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
            Self::Disputed => "disputed",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicJob {
    pub id: Uuid,
    pub service_id: Uuid,
    pub agent_id: Uuid,
    pub customer_id: Uuid,
    pub network: EconomicNetwork,
    pub spec: ServiceSpec,
    pub input: String,
    pub state: JobState,
    pub revision: i64,
    pub attempts: i32,
    pub received_atoms: i64,
    pub earned_atoms: i64,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub lease_until: Option<DateTime<Utc>>,
    pub execution_token: Option<Uuid>,
    pub output: Option<Value>,
    pub output_sha256: Option<String>,
    pub failure_code: Option<String>,
    pub assessment: Option<Value>,
}
impl EconomicJob {
    pub fn refund_eligible_atoms(&self) -> i64 {
        if matches!(
            self.state,
            JobState::Failed
                | JobState::Cancelled
                | JobState::Expired
                | JobState::Disputed
                | JobState::Completed
        ) {
            self.received_atoms - self.earned_atoms
        } else {
            0
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CreateJob {
    pub request_id: Uuid,
    pub service_id: Uuid,
    pub network: EconomicNetwork,
    pub input: String,
    pub valid_for_seconds: i64,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobCommand {
    pub request_id: Uuid,
    pub expected_revision: i64,
    pub operation: String,
    pub authorize: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulatedReceipt {
    pub request_id: Uuid,
    pub amount_atoms: i64,
    pub authorize_simulated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn financial_and_input_limits_fail_closed() {
        assert!(EconomicNetwork::Mainnet.enabled().is_err());
        let mut s = ServiceSpec {
            name: "Summary".into(),
            kind: ServiceKind::Summary,
            price_atoms: 5000,
            estimated_inference_atoms: 1000,
            estimated_infrastructure_atoms: 100,
            max_input_bytes: 8000,
            max_attempts: 2,
        };
        assert!(s.input("Analyze supplied text").is_ok());
        assert!(s.input("sk-private-credential").is_err());
        assert_eq!(s.cost(EconomicNetwork::Devnet), 0);
        s.price_atoms = 1;
        assert!(s.validate().is_err());
    }
}
