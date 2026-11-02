//! Human evaluation review is neither paid consent nor commercial acceptance.
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkReviewDecision {
    AcceptEvaluationOnly,
    RejectEvaluationOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkReviewRequest {
    pub request_id: Uuid,
    pub decision: WorkReviewDecision,
    pub report_sha256: String,
    pub confirm_review: bool,
    pub consented_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
impl WorkReviewRequest {
    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<()> {
        if !self.confirm_review {
            return Err(Error::Denied);
        }
        if self.request_id.is_nil()
            || self.report_sha256.len() != 64
            || !self
                .report_sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(Error::Invalid);
        }
        if self.consented_at > now
            || self.expires_at <= now
            || self.expires_at <= self.consented_at
            || self.expires_at - self.consented_at > chrono::Duration::minutes(5)
        {
            return Err(Error::Stale);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkEvaluationReview {
    pub request_id: Uuid,
    pub session_id: Uuid,
    pub agent_id: Uuid,
    pub reviewer_principal_id: Uuid,
    pub decision: WorkReviewDecision,
    pub report_sha256: String,
    pub reviewed_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(now: DateTime<Utc>) -> WorkReviewRequest {
        WorkReviewRequest {
            request_id: Uuid::new_v4(),
            decision: WorkReviewDecision::AcceptEvaluationOnly,
            report_sha256: "0".repeat(64),
            confirm_review: true,
            consented_at: now,
            expires_at: now + chrono::Duration::minutes(5),
        }
    }
    #[test]
    fn review_requires_explicit_fresh_finite_confirmation() {
        let now = Utc::now();
        let valid = request(now);
        valid.validate_at(now).unwrap();
        let mut r = valid.clone();
        r.confirm_review = false;
        assert!(matches!(r.validate_at(now), Err(Error::Denied)));
        let mut r = valid.clone();
        r.expires_at = now;
        assert!(matches!(r.validate_at(now), Err(Error::Stale)));
        let mut r = valid.clone();
        r.consented_at = now + chrono::Duration::seconds(1);
        assert!(r.validate_at(now).is_err());
        let mut r = valid.clone();
        r.expires_at += chrono::Duration::seconds(1);
        assert!(r.validate_at(now).is_err());
        let mut r = valid.clone();
        r.request_id = Uuid::nil();
        assert!(r.validate_at(now).is_err());
        let mut r = valid;
        r.report_sha256 = "X".repeat(64);
        assert!(r.validate_at(now).is_err());
    }
    #[test]
    fn identity_money_and_unknown_decisions_are_not_client_fields() {
        let value = serde_json::to_value(request(Utc::now())).unwrap();
        for field in [
            "reviewer_principal_id",
            "owner_id",
            "earned_revenue",
            "approve_tools",
        ] {
            let mut v = value.clone();
            v[field] = serde_json::json!("injected");
            assert!(serde_json::from_value::<WorkReviewRequest>(v).is_err());
        }
        let mut v = value;
        v["decision"] = serde_json::json!("complete_job");
        assert!(serde_json::from_value::<WorkReviewRequest>(v).is_err());
    }
}
