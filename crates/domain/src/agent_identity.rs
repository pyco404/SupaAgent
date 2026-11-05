//! Review-only domain rules: no persistence, network, launch, signing or execution.
//!
//! Authority and evidence contexts must come from authenticated application state,
//! never a request/model. Successful validation grants no capability or payment.
use crate::{ServiceKind, SolanaNetwork, validate_solana_address, validate_solana_signature};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AgentIdentityRejection {
    #[error("invalid_purpose")]
    InvalidPurpose,
    #[error("unauthorized_creator")]
    UnauthorizedCreator,
    #[error("purpose_revision_mismatch")]
    PurposeMismatch,
    #[error("invalid_qualification_policy")]
    InvalidPolicy,
    #[error("invalid_qualification_review")]
    InvalidReview,
    #[error("unauthorized_qualification_reviewer")]
    UnauthorizedReviewer,
    #[error("qualification_review_mismatch")]
    ReviewMismatch,
    #[error("stale_qualification_review")]
    StaleReview,
    #[error("agent_not_qualified")]
    NotQualified,
    #[error("invalid_token_association")]
    InvalidAssociation,
    #[error("token_evidence_unconfirmed")]
    UnconfirmedEvidence,
    #[error("token_evidence_mismatch")]
    EvidenceMismatch,
    #[error("duplicate_agent_token_association")]
    DuplicateAgent,
    #[error("duplicate_token_mint_association")]
    DuplicateMint,
    #[error("duplicate_token_association_record")]
    DuplicateRecord,
    #[error("original_token_binding_mismatch")]
    OriginalTokenMismatch,
    #[error("invalid_successor_policy")]
    InvalidSuccessorPolicy,
    #[error("successor_authorization_required")]
    SuccessorAuthorizationRequired,
    #[error("successor_public_disclosure_required")]
    SuccessorDisclosureRequired,
    #[error("invalid_token_successor")]
    InvalidSuccessor,
}
type IdentityResult<T> = std::result::Result<T, AgentIdentityRejection>;

/// Private purpose data is not automatically eligible for public/social disclosure.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurposeRevision {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub creator_id: Uuid,
    pub revision: u32,
    pub created_at: DateTime<Utc>,
    pub objective: String,
    pub permitted_tasks: Vec<ServiceKind>,
    pub success_criteria: Vec<String>,
    pub excluded_actions: Vec<String>,
    pub public_summary: Option<String>,
}

/// Trusted current database/authentication facts; intentionally not deserializable.
pub struct AgentPurposeAuthority {
    pub agent_id: Uuid,
    pub creator_id: Uuid,
    pub current_purpose_revision_id: Uuid,
    pub current_review_id: Uuid,
}

fn text_valid(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}
fn list_valid(values: &[String]) -> bool {
    !values.is_empty()
        && values.len() <= 8
        && values.iter().all(|v| text_valid(v, 240))
        && values
            .iter()
            .map(|v| v.trim())
            .collect::<BTreeSet<_>>()
            .len()
            == values.len()
}
impl PurposeRevision {
    pub fn validate_for(
        &self,
        authority: &AgentPurposeAuthority,
        now: DateTime<Utc>,
    ) -> IdentityResult<()> {
        use AgentIdentityRejection::*;
        if self.agent_id.is_nil()
            || self.creator_id.is_nil()
            || self.agent_id != authority.agent_id
            || self.creator_id != authority.creator_id
        {
            return Err(UnauthorizedCreator);
        }
        if self.id.is_nil() || self.id != authority.current_purpose_revision_id {
            return Err(PurposeMismatch);
        }
        if self.revision == 0
            || self.created_at > now
            || !text_valid(&self.objective, 600)
            || self.permitted_tasks.is_empty()
            || self.permitted_tasks.len() > 5
            || self
                .permitted_tasks
                .iter()
                .enumerate()
                .any(|(i, k)| self.permitted_tasks[..i].contains(k))
            || !list_valid(&self.success_criteria)
            || !list_valid(&self.excluded_actions)
            || self
                .public_summary
                .as_ref()
                .is_some_and(|v| !text_valid(v, 280))
        {
            return Err(InvalidPurpose);
        }
        Ok(())
    }
}

/// No default qualification policy: reviewer authority and validity must be supplied.
/// Labels describing purpose never grant tools or spending permissions.
pub struct AgentQualificationPolicy {
    pub id: Uuid,
    pub version: u32,
    pub authorized_reviewers: BTreeSet<Uuid>,
    pub max_validity_seconds: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentQualificationDecision {
    Qualified,
    Rejected,
    Revoked,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationReview {
    pub id: Uuid,
    /// Full immutable snapshot binds every field, not just a caller-reused UUID.
    pub purpose: PurposeRevision,
    pub policy_id: Uuid,
    pub policy_version: u32,
    pub reviewer_id: Uuid,
    pub decision: AgentQualificationDecision,
    pub evidence_ids: Vec<Uuid>,
    pub reviewed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub struct AgentQualificationContext<'a> {
    pub authority: &'a AgentPurposeAuthority,
    pub purpose: &'a PurposeRevision,
    pub policy: &'a AgentQualificationPolicy,
    pub review: &'a QualificationReview,
    pub now: DateTime<Utc>,
}
impl AgentQualificationContext<'_> {
    pub fn validate(&self) -> IdentityResult<()> {
        use AgentIdentityRejection::*;
        self.purpose.validate_for(self.authority, self.now)?;
        let p = self.policy;
        let r = self.review;
        if p.id.is_nil()
            || p.version == 0
            || p.max_validity_seconds == 0
            || p.authorized_reviewers.is_empty()
            || p.authorized_reviewers.contains(&Uuid::nil())
        {
            return Err(InvalidPolicy);
        }
        if r.id.is_nil()
            || r.id != self.authority.current_review_id
            || r.policy_id != p.id
            || r.policy_version != p.version
        {
            return Err(ReviewMismatch);
        }
        if !p.authorized_reviewers.contains(&r.reviewer_id) {
            return Err(UnauthorizedReviewer);
        }
        if r.purpose != *self.purpose {
            return Err(PurposeMismatch);
        }
        if r.evidence_ids.is_empty()
            || r.evidence_ids.len() > 16
            || r.evidence_ids.contains(&Uuid::nil())
            || r.evidence_ids.iter().collect::<BTreeSet<_>>().len() != r.evidence_ids.len()
        {
            return Err(InvalidReview);
        }
        if r.reviewed_at < self.purpose.created_at
            || r.reviewed_at > self.now
            || r.expires_at <= self.now
            || r.expires_at <= r.reviewed_at
            || r.expires_at.signed_duration_since(r.reviewed_at)
                > Duration::seconds(i64::from(p.max_validity_seconds))
        {
            return Err(StaleReview);
        }
        if r.decision != AgentQualificationDecision::Qualified {
            return Err(NotQualified);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentTokenEvidenceState {
    Confirmed,
    Pending,
    Failed,
    Unknown,
}
/// Trusted verification projection, not a chain-verification implementation.
/// Upstream must independently verify cluster, mint and transaction semantics.
/// No HTTP/API/model payload may be deserialized into this authority context.
pub struct AgentTokenEvidence {
    pub id: Uuid,
    pub state: AgentTokenEvidenceState,
    pub network: SolanaNetwork,
    pub mint: String,
    pub transaction: String,
    pub observed_at: DateTime<Utc>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTokenAssociation {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub creator_id: Uuid,
    pub purpose_revision_id: Uuid,
    pub qualification_review_id: Uuid,
    pub network: SolanaNetwork,
    pub mint: String,
    pub transaction: String,
    pub evidence_id: Uuid,
}

/// In-memory domain aggregate only. Durable uniqueness needs separately reviewed
/// database constraints/locking. It grants no launch, wallet, role or revenue rights.
#[derive(Default)]
pub struct AgentTokenAssociations {
    entries: Vec<AgentTokenAssociation>,
    originals: Vec<AgentTokenOriginalRecord>,
    successors: Vec<AgentTokenSuccessorRecord>,
}
impl AgentTokenAssociations {
    pub fn entries(&self) -> &[AgentTokenAssociation] {
        &self.entries
    }
    pub fn try_associate(
        &mut self,
        candidate: AgentTokenAssociation,
        qualification: &AgentQualificationContext<'_>,
        evidence: &AgentTokenEvidence,
    ) -> IdentityResult<()> {
        use AgentIdentityRejection::*;
        validate_token_association(&candidate, qualification, evidence)?;
        let c = &candidate;
        if self.record_id_used(c.id) {
            return Err(DuplicateRecord);
        }
        // One canonical token per agent across all supported clusters.
        if self.entries.iter().any(|e| e.agent_id == c.agent_id) {
            return Err(DuplicateAgent);
        }
        if self.mint_used(c.network, &c.mint) {
            return Err(DuplicateMint);
        }
        if self.evidence_used(c.evidence_id) {
            return Err(DuplicateRecord);
        }
        self.originals.push(AgentTokenOriginalRecord {
            association: candidate.clone(),
            confirmed_at: evidence.observed_at,
            recorded_at: qualification.now,
        });
        self.entries.push(candidate);
        Ok(())
    }
}

/// Confirmed originals are retained permanently; no deletion or reassignment API.
/// Serialize-only audit snapshots do not provide a client-controlled restore path.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct AgentTokenOriginalRecord {
    pub association: AgentTokenAssociation,
    pub confirmed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
}
/// A separately approved exception, never a replacement of the original binding.
/// No holder balances, ownership, wallet permissions or financial claims are modeled.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTokenSuccessor {
    pub original: AgentTokenAssociation,
    pub successor: AgentTokenAssociation,
    pub authorization_id: Uuid,
    pub disclosure_id: Uuid,
}
/// Server-loaded current policy. Roles alone do not confer this authority.
/// No defaults or Deserialize implementation: never accept this from HTTP/model JSON.
pub struct AgentTokenSuccessorPolicy {
    pub id: Uuid,
    pub version: u32,
    pub authorized_approvers: BTreeSet<Uuid>,
    pub authorized_disclosure_verifiers: BTreeSet<Uuid>,
    pub max_authorization_validity_seconds: u32,
    pub max_disclosure_age_seconds: u32,
}
/// Explicit, current, expiring approval of the full original/successor snapshot.
/// This receipt authorizes recording an association only, not signing or launching.
pub struct AgentTokenSuccessorAuthorization {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub policy_version: u32,
    pub approver_id: Uuid,
    pub approved: bool,
    pub revoked: bool,
    pub candidate: AgentTokenSuccessor,
    pub approved_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
/// Trusted independent verification that both network/mint identities, their
/// canonical agent linkage and exceptional status were publicly disclosed.
/// Does not grant disclosure permission for private creator or customer information.
pub struct AgentTokenSuccessorDisclosure {
    pub id: Uuid,
    pub source_id: Uuid,
    pub verifier_id: Uuid,
    pub state: AgentTokenEvidenceState,
    pub publicly_accessible: bool,
    pub candidate: AgentTokenSuccessor,
    pub published_at: DateTime<Utc>,
    pub verified_at: DateTime<Utc>,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct AgentTokenSuccessorRecord {
    pub candidate: AgentTokenSuccessor,
    pub policy_id: Uuid,
    pub policy_version: u32,
    pub approved_by: Uuid,
    pub approved_at: DateTime<Utc>,
    pub authorization_expires_at: DateTime<Utc>,
    pub disclosure_source_id: Uuid,
    pub disclosure_verified_by: Uuid,
    pub disclosed_at: DateTime<Utc>,
    pub disclosure_verified_at: DateTime<Utc>,
    pub confirmed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
}
fn validate_token_association(
    candidate: &AgentTokenAssociation,
    qualification: &AgentQualificationContext<'_>,
    evidence: &AgentTokenEvidence,
) -> IdentityResult<()> {
    use AgentIdentityRejection::*;
    qualification.validate()?;
    let c = candidate;
    let purpose = qualification.purpose;
    if c.id.is_nil()
        || c.agent_id != purpose.agent_id
        || c.creator_id != purpose.creator_id
        || c.purpose_revision_id != purpose.id
        || c.qualification_review_id != qualification.review.id
        || c.evidence_id.is_nil()
        || validate_solana_address(&c.mint).is_err()
        || validate_solana_signature(&c.transaction).is_err()
    {
        return Err(InvalidAssociation);
    }
    if evidence.state != AgentTokenEvidenceState::Confirmed {
        return Err(UnconfirmedEvidence);
    }
    if evidence.id != c.evidence_id
        || evidence.network != c.network
        || evidence.mint != c.mint
        || evidence.transaction != c.transaction
        || evidence.observed_at < qualification.review.reviewed_at
        || evidence.observed_at > qualification.now
    {
        return Err(EvidenceMismatch);
    }
    Ok(())
}
impl AgentTokenAssociations {
    /// Always returns the first confirmed official binding, even after exceptions
    /// or expiry/revocation of qualification. No caller can mutate this reference.
    pub fn original_token(&self, agent: Uuid) -> Option<&AgentTokenAssociation> {
        self.entries.iter().find(|e| e.agent_id == agent)
    }
    pub fn original_history(&self) -> &[AgentTokenOriginalRecord] {
        &self.originals
    }
    pub fn successor_history(&self) -> &[AgentTokenSuccessorRecord] {
        &self.successors
    }
    fn record_id_used(&self, id: Uuid) -> bool {
        self.entries.iter().any(|e| e.id == id)
            || self
                .successors
                .iter()
                .any(|e| e.candidate.successor.id == id)
    }
    fn evidence_used(&self, id: Uuid) -> bool {
        self.entries.iter().any(|e| e.evidence_id == id)
            || self
                .successors
                .iter()
                .any(|e| e.candidate.successor.evidence_id == id)
    }
    fn mint_used(&self, network: SolanaNetwork, mint: &str) -> bool {
        self.entries
            .iter()
            .any(|e| e.network == network && e.mint == mint)
            || self.successors.iter().any(|e| {
                e.candidate.successor.network == network && e.candidate.successor.mint == mint
            })
    }
    pub fn try_record_successor(
        &mut self,
        candidate: AgentTokenSuccessor,
        qualification: &AgentQualificationContext<'_>,
        evidence: &AgentTokenEvidence,
        policy: &AgentTokenSuccessorPolicy,
        authorization: &AgentTokenSuccessorAuthorization,
        disclosure: &AgentTokenSuccessorDisclosure,
    ) -> IdentityResult<()> {
        use AgentIdentityRejection::*;
        let now = qualification.now;
        let original = self
            .originals
            .iter()
            .find(|r| r.association.agent_id == candidate.original.agent_id)
            .ok_or(OriginalTokenMismatch)?;
        if original.association != candidate.original {
            return Err(OriginalTokenMismatch);
        }
        let c = &candidate.successor;
        if c.agent_id != candidate.original.agent_id
            || c.network != candidate.original.network
            || c.mint == candidate.original.mint
            || candidate.authorization_id.is_nil()
            || candidate.disclosure_id.is_nil()
            || candidate.authorization_id == candidate.disclosure_id
        {
            return Err(InvalidSuccessor);
        }
        validate_token_association(c, qualification, evidence)?;
        let valid_set = |set: &BTreeSet<Uuid>| !set.is_empty() && !set.contains(&Uuid::nil());
        if policy.id.is_nil()
            || policy.version == 0
            || !valid_set(&policy.authorized_approvers)
            || !valid_set(&policy.authorized_disclosure_verifiers)
            || !(1..=86400).contains(&policy.max_authorization_validity_seconds)
            || !(1..=86400).contains(&policy.max_disclosure_age_seconds)
        {
            return Err(InvalidSuccessorPolicy);
        }
        if authorization.id != candidate.authorization_id
            || authorization.policy_id != policy.id
            || authorization.policy_version != policy.version
            || !policy
                .authorized_approvers
                .contains(&authorization.approver_id)
            || !authorization.approved
            || authorization.revoked
            || authorization.candidate != candidate
            || authorization.approved_at < original.recorded_at
            || authorization.approved_at > evidence.observed_at
            || authorization.expires_at <= now
            || authorization.expires_at <= authorization.approved_at
            || authorization.expires_at - authorization.approved_at
                > Duration::seconds(i64::from(policy.max_authorization_validity_seconds))
        {
            return Err(SuccessorAuthorizationRequired);
        }
        if disclosure.id != candidate.disclosure_id
            || disclosure.source_id.is_nil()
            || !policy
                .authorized_disclosure_verifiers
                .contains(&disclosure.verifier_id)
            || disclosure.state != AgentTokenEvidenceState::Confirmed
            || !disclosure.publicly_accessible
            || disclosure.candidate != candidate
            || disclosure.published_at < evidence.observed_at
            || disclosure.verified_at < disclosure.published_at
            || disclosure.verified_at > now
            || now - disclosure.published_at
                > Duration::seconds(i64::from(policy.max_disclosure_age_seconds))
        {
            return Err(SuccessorDisclosureRequired);
        }
        if self.record_id_used(c.id)
            || self.evidence_used(c.evidence_id)
            || self.successors.iter().any(|r| {
                r.candidate.authorization_id == candidate.authorization_id
                    || r.candidate.disclosure_id == candidate.disclosure_id
            })
        {
            return Err(DuplicateRecord);
        }
        if self.mint_used(c.network, &c.mint) {
            return Err(DuplicateMint);
        }
        self.successors.push(AgentTokenSuccessorRecord {
            candidate,
            policy_id: policy.id,
            policy_version: policy.version,
            approved_by: authorization.approver_id,
            approved_at: authorization.approved_at,
            authorization_expires_at: authorization.expires_at,
            disclosure_source_id: disclosure.source_id,
            disclosure_verified_by: disclosure.verifier_id,
            disclosed_at: disclosure.published_at,
            disclosure_verified_at: disclosure.verified_at,
            confirmed_at: evidence.observed_at,
            recorded_at: now,
        });
        Ok(())
    }
}
