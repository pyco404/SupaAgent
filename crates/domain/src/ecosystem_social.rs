//! Privacy-filtered ecosystem evidence and review-only draft validation.
//! No retrieval, persistence, paid inference, launch, signing or publishing.
//! Inputs/authority are deliberately not JSON-deserializable. Application loaders
//! must authenticate scope and independently verify source records before use.
use crate::agent_identity::{AgentQualificationDecision, AgentTokenAssociations};
use crate::{
    PostStyle, PreviousDraft, SocialChoice, SocialDecision, SocialReason, SolanaNetwork,
    clean_text, repeated_social_text, validate_solana_address,
};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EcosystemEventKind {
    AgentCreated,
    AgentQualified,
    TaskCompleted,
    TaskFailed,
    Achievement,
    TokenActivity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionState {
    Completed,
    EvaluationOnly,
    Incomplete,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticReviewState {
    Accepted,
    NotRequired,
    Pending,
    Rejected,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicFailureCode {
    InsufficientEvidence,
    PermissionDenied,
    ProviderUnavailable,
    InvalidResult,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum QuoteUnit {
    USDC,
    SOL,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "metric", rename_all = "snake_case")]
pub enum PublicTokenMetric {
    PriceChange { before_atoms: u64, after_atoms: u64 },
    Volume { atoms: u64 },
    Liquidity { atoms: u64 },
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct PublicTokenObservation {
    pub association_id: Uuid,
    pub network: SolanaNetwork,
    pub mint: String,
    pub source_id: Uuid,
    pub venue_id: Uuid,
    pub quote: QuoteUnit,
    pub decimals: u8,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub metric: PublicTokenMetric,
}
#[derive(Clone, PartialEq, Eq)]
pub enum PlatformEventData {
    AgentCreated {
        purpose_revision_id: Uuid,
    },
    AgentQualified {
        purpose_revision_id: Uuid,
        review_id: Uuid,
        decision: AgentQualificationDecision,
    },
    TaskCompleted {
        result_id: Uuid,
        state: CompletionState,
        result_verified: bool,
        semantic_review: SemanticReviewState,
    },
    TaskFailed {
        reason: PublicFailureCode,
    },
    Achievement {
        result_id: Uuid,
        semantic_review: SemanticReviewState,
    },
    TokenActivity(PublicTokenObservation),
}
impl PlatformEventData {
    pub fn kind(&self) -> EcosystemEventKind {
        match self {
            Self::AgentCreated { .. } => EcosystemEventKind::AgentCreated,
            Self::AgentQualified { .. } => EcosystemEventKind::AgentQualified,
            Self::TaskCompleted { .. } => EcosystemEventKind::TaskCompleted,
            Self::TaskFailed { .. } => EcosystemEventKind::TaskFailed,
            Self::Achievement { .. } => EcosystemEventKind::Achievement,
            Self::TokenActivity(_) => EcosystemEventKind::TokenActivity,
        }
    }
}
/// Minimal authoritative record: intentionally has no free-form/customer fields.
#[derive(Clone, PartialEq, Eq)]
pub struct PlatformSourceRecord {
    pub simulated: bool,
    pub source_record_id: Uuid,
    pub agent_id: Uuid,
    pub creator_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub data: PlatformEventData,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceOrigin {
    ApplicationRecord,
    IndependentPublicMarket,
}
/// Receipt from an authorized verifier, bound to the entire source snapshot.
/// This domain type does not authenticate the verifier or verify a remote source.
pub struct VerifiedPlatformEvidence {
    pub id: Uuid,
    pub verifier_id: Uuid,
    pub origin: EvidenceOrigin,
    pub record: PlatformSourceRecord,
    pub verified_at: DateTime<Utc>,
}
/// Current creator-authenticated consent; public state alone is never sufficient.
pub struct CreatorPublicVisibility {
    pub agent_id: Uuid,
    pub creator_id: Uuid,
    pub revision: u32,
    pub public: bool,
    pub revoked: bool,
    pub valid_from: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub categories: BTreeSet<EcosystemEventKind>,
}
/// Explicit trusted policy, with no permissive default or live source activation.
pub struct EcosystemProjectionPolicy {
    pub authorized_agents: BTreeSet<Uuid>,
    pub authorized_verifiers: BTreeSet<Uuid>,
    pub approved_market_sources: BTreeSet<Uuid>,
    pub verifier_version: u32,
    pub now: DateTime<Utc>,
    pub max_event_age_seconds: u32,
    pub max_evidence_age_seconds: u32,
    pub market_max_age_seconds: Option<u32>,
    pub market_max_window_seconds: Option<u32>,
}
pub struct EcosystemProjectionInput<'a> {
    pub records: &'a [PlatformSourceRecord],
    pub evidence: &'a [VerifiedPlatformEvidence],
    pub visibility: &'a [CreatorPublicVisibility],
    pub associations: &'a AgentTokenAssociations,
    /// Only used internally for repetition; previous prose never enters model input.
    pub previous_drafts: &'a [PreviousDraft],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EcosystemSocialRejection {
    #[error("ecosystem_invalid_policy")]
    InvalidPolicy,
    #[error("ecosystem_scope_denied")]
    ScopeDenied,
    #[error("ecosystem_invalid_record")]
    InvalidRecord,
    #[error("ecosystem_ambiguous_visibility")]
    AmbiguousVisibility,
    #[error("ecosystem_evidence_required")]
    EvidenceRequired,
    #[error("ecosystem_conflicting_evidence")]
    ConflictingEvidence,
    #[error("ecosystem_stale_evidence")]
    StaleEvidence,
    #[error("ecosystem_unverified_outcome")]
    UnverifiedOutcome,
    #[error("ecosystem_token_evidence_invalid")]
    InvalidTokenEvidence,
    #[error("ecosystem_invalid_draft")]
    InvalidDraft,
    #[error("ecosystem_unsupported_claim")]
    UnsupportedClaim,
    #[error("ecosystem_repetition")]
    Repetition,
    #[error("ecosystem_context_expired")]
    ContextExpired,
}
type ProjectionResult<T> = std::result::Result<T, EcosystemSocialRejection>;
#[derive(Clone, Serialize)]
pub struct PublicPlatformEvent {
    simulated: bool,
    id: Uuid,
    agent_id: Uuid,
    kind: EcosystemEventKind,
    occurred_at: DateTime<Utc>,
    evidence_id: Uuid,
    verifier_id: Uuid,
    verified_at: DateTime<Utc>,
    consent_revision: u32,
    verifier_version: u32,
    public_fact: String,
    token_observation: Option<PublicTokenObservation>,
}
impl PublicPlatformEvent {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn kind(&self) -> EcosystemEventKind {
        self.kind
    }
    pub fn public_fact(&self) -> &str {
        &self.public_fact
    }
    pub fn topic(&self) -> String {
        topic(self.kind)
    }
}
/// Cannot be deserialized or assembled from model-generated "verified" claims.
pub struct PublicEcosystemContext {
    events: Vec<PublicPlatformEvent>,
    previous: Vec<PreviousDraft>,
    valid_until: DateTime<Utc>,
    projected_at: DateTime<Utc>,
}
fn topic(kind: EcosystemEventKind) -> String {
    match kind {
        EcosystemEventKind::AgentCreated => "ecosystem_agent_creation",
        EcosystemEventKind::AgentQualified => "ecosystem_qualification",
        EcosystemEventKind::TaskCompleted => "ecosystem_task_completion",
        EcosystemEventKind::TaskFailed => "ecosystem_failure",
        EcosystemEventKind::Achievement => "ecosystem_achievement",
        EcosystemEventKind::TokenActivity => "ecosystem_token_activity",
    }
    .into()
}
fn age_valid(at: DateTime<Utc>, now: DateTime<Utc>, max: u32) -> bool {
    at <= now && now.signed_duration_since(at) <= Duration::seconds(i64::from(max))
}
fn amount(atoms: u64, decimals: u8) -> String {
    let scale = 10u64.pow(u32::from(decimals));
    if decimals == 0 {
        return atoms.to_string();
    }
    let fraction = format!("{:0width$}", atoms % scale, width = usize::from(decimals));
    let trimmed = fraction.trim_end_matches('0');
    if trimmed.is_empty() {
        (atoms / scale).to_string()
    } else {
        format!("{}.{trimmed}", atoms / scale)
    }
}
fn token_fact(
    t: &PublicTokenObservation,
    record: &PlatformSourceRecord,
    receipt: &VerifiedPlatformEvidence,
    input: &EcosystemProjectionInput<'_>,
    p: &EcosystemProjectionPolicy,
) -> ProjectionResult<String> {
    use EcosystemSocialRejection::*;
    let max_age = p
        .market_max_age_seconds
        .filter(|n| *n > 0)
        .ok_or(InvalidTokenEvidence)?;
    let max_window = p
        .market_max_window_seconds
        .filter(|n| *n > 0)
        .ok_or(InvalidTokenEvidence)?;
    if receipt.origin != EvidenceOrigin::IndependentPublicMarket
        || t.source_id.is_nil()
        || t.venue_id.is_nil()
        || !p.approved_market_sources.contains(&t.source_id)
        || t.decimals > 9
        || validate_solana_address(&t.mint).is_err()
        || t.window_start >= t.window_end
        || t.window_end != record.occurred_at
        || !age_valid(t.window_end, p.now, max_age)
        || t.window_end.signed_duration_since(t.window_start)
            > Duration::seconds(i64::from(max_window))
        || !input.associations.entries().iter().any(|a| {
            a.id == t.association_id
                && a.agent_id == record.agent_id
                && a.creator_id == record.creator_id
                && a.network == t.network
                && a.mint == t.mint
        })
    {
        return Err(InvalidTokenEvidence);
    }
    let quote = match t.quote {
        QuoteUnit::USDC => "USDC",
        QuoteUnit::SOL => "SOL",
    };
    // Metrics are observed quantities, never agent earnings or token recommendations.
    let phrase = match t.metric {
        PublicTokenMetric::PriceChange {
            before_atoms,
            after_atoms,
        } => {
            if before_atoms == 0 || after_atoms == 0 {
                return Err(InvalidTokenEvidence);
            }
            if before_atoms == after_atoms {
                format!(
                    "price was unchanged at {} {quote}",
                    amount(before_atoms, t.decimals)
                )
            } else {
                format!(
                    "price moved from {} to {} {quote}",
                    amount(before_atoms, t.decimals),
                    amount(after_atoms, t.decimals)
                )
            }
        }
        PublicTokenMetric::Volume { atoms } => {
            format!("observed volume was {} {quote}", amount(atoms, t.decimals))
        }
        PublicTokenMetric::Liquidity { atoms } => format!(
            "observed liquidity was {} {quote}",
            amount(atoms, t.decimals)
        ),
    };
    let network = match t.network {
        SolanaNetwork::Devnet => "Devnet test",
        SolanaNetwork::Mainnet => "Solana mainnet",
    };
    Ok(format!(
        "A verified {network} agent token's {phrase} within the recorded observation window."
    ))
}
fn public_fact(
    record: &PlatformSourceRecord,
    receipt: &VerifiedPlatformEvidence,
    input: &EcosystemProjectionInput<'_>,
    p: &EcosystemProjectionPolicy,
) -> ProjectionResult<String> {
    use EcosystemSocialRejection::*;
    if let PlatformEventData::TokenActivity(t) = &record.data {
        return token_fact(t, record, receipt, input, p);
    }
    if receipt.origin != EvidenceOrigin::ApplicationRecord {
        return Err(ConflictingEvidence);
    }
    let fact = match &record.data {
        PlatformEventData::AgentCreated {
            purpose_revision_id,
        } if !purpose_revision_id.is_nil() => {
            "A publicly listed agent was created with a recorded purpose."
        }
        PlatformEventData::AgentQualified {
            purpose_revision_id,
            review_id,
            decision,
        } if !purpose_revision_id.is_nil()
            && !review_id.is_nil()
            && *decision == AgentQualificationDecision::Qualified =>
        {
            "A publicly listed agent passed its recorded purpose qualification review."
        }
        PlatformEventData::TaskCompleted {
            result_id,
            state,
            result_verified,
            semantic_review,
        } if !result_id.is_nil()
            && *state == CompletionState::Completed
            && *result_verified
            && matches!(
                semantic_review,
                SemanticReviewState::Accepted | SemanticReviewState::NotRequired
            ) =>
        {
            "A publicly listed agent completed a task with a validated result."
        }
        PlatformEventData::TaskFailed { reason } => match reason {
            PublicFailureCode::InsufficientEvidence => {
                "A publicly listed agent task failed because evidence was insufficient."
            }
            PublicFailureCode::PermissionDenied => {
                "A publicly listed agent task failed at a permission boundary."
            }
            PublicFailureCode::ProviderUnavailable => {
                "A publicly listed agent task failed because its provider was unavailable."
            }
            PublicFailureCode::InvalidResult => {
                "A publicly listed agent task failed result validation."
            }
        },
        PlatformEventData::Achievement {
            result_id,
            semantic_review,
        } if !result_id.is_nil() && *semantic_review == SemanticReviewState::Accepted => {
            "A publicly listed agent had an evidence-backed achievement accepted in semantic review."
        }
        _ => return Err(UnverifiedOutcome),
    };
    Ok(fact.into())
}

pub fn project_public_events(
    input: EcosystemProjectionInput<'_>,
    p: &EcosystemProjectionPolicy,
) -> ProjectionResult<PublicEcosystemContext> {
    use EcosystemSocialRejection::*;
    if p.verifier_version == 0
        || p.max_event_age_seconds == 0
        || p.max_evidence_age_seconds == 0
        || p.authorized_verifiers.is_empty()
        || p.authorized_verifiers.contains(&Uuid::nil())
        || p.authorized_agents.contains(&Uuid::nil())
        || p.approved_market_sources.contains(&Uuid::nil())
        || input.records.len() > 64
        || input.evidence.len() > 64
        || input.visibility.len() > 64
        || input.previous_drafts.len() > 20
    {
        return Err(InvalidPolicy);
    }
    let mut consents = BTreeMap::new();
    for grant in input.visibility {
        if consents.insert(grant.agent_id, grant).is_some() {
            return Err(AmbiguousVisibility);
        }
    }
    let mut receipts = BTreeMap::new();
    let mut receipt_ids = BTreeSet::new();
    for e in input.evidence {
        if !receipt_ids.insert(e.id) || receipts.insert(e.record.source_record_id, e).is_some() {
            return Err(ConflictingEvidence);
        }
    }
    let mut records = BTreeMap::new();
    let mut events = vec![];
    let mut market_observations: Vec<&PublicTokenObservation> = vec![];
    let mut until = p.now
        + Duration::seconds(i64::from(
            p.max_event_age_seconds.min(p.max_evidence_age_seconds),
        ));
    for record in input.records {
        if !p.authorized_agents.contains(&record.agent_id) {
            return Err(ScopeDenied);
        }
        let Some(grant) = consents.get(&record.agent_id) else {
            continue;
        };
        if grant.creator_id != record.creator_id || grant.creator_id.is_nil() {
            return Err(ScopeDenied);
        }
        let kind = record.data.kind();
        // Private/revoked/expired events are excluded before looking at their evidence.
        if !grant.public
            || grant.revoked
            || grant.revision == 0
            || grant.valid_from > p.now
            || grant.expires_at <= p.now
            || record.occurred_at < grant.valid_from
            || !grant.categories.contains(&kind)
        {
            continue;
        }
        if record.source_record_id.is_nil() || record.agent_id.is_nil() {
            return Err(InvalidRecord);
        }
        if let Some(old) = records.insert(record.source_record_id, record) {
            if old != record {
                return Err(ConflictingEvidence);
            }
            continue;
        }
        if !age_valid(record.occurred_at, p.now, p.max_event_age_seconds) {
            return Err(StaleEvidence);
        }
        let e = receipts
            .get(&record.source_record_id)
            .ok_or(EvidenceRequired)?;
        if e.id.is_nil() || !p.authorized_verifiers.contains(&e.verifier_id) || e.record != *record
        {
            return Err(ConflictingEvidence);
        }
        if e.verified_at < record.occurred_at
            || !age_valid(e.verified_at, p.now, p.max_evidence_age_seconds)
        {
            return Err(StaleEvidence);
        }
        let fact = public_fact(record, e, &input, p)?;
        let fact = if record.simulated {
            format!("In a simulation, {}", fact.replacen('A', "a", 1))
        } else {
            fact
        };
        if let PlatformEventData::TokenActivity(current) = &record.data {
            if market_observations.iter().any(|old| {
                old.association_id == current.association_id
                    && old.network == current.network
                    && old.mint == current.mint
                    && old.venue_id == current.venue_id
                    && old.window_start == current.window_start
                    && old.window_end == current.window_end
                    && std::mem::discriminant(&old.metric)
                        == std::mem::discriminant(&current.metric)
                    && (old.quote != current.quote
                        || old.decimals != current.decimals
                        || old.metric != current.metric)
            }) {
                return Err(ConflictingEvidence);
            }
            market_observations.push(current);
        }
        let previous = input.previous_drafts;
        if previous
            .iter()
            .any(|d| d.event_ids.contains(&record.source_record_id))
            || previous
                .iter()
                .take(3)
                .any(|d| d.topics.contains(&topic(kind)))
        {
            continue;
        }
        until = until
            .min(grant.expires_at)
            .min(record.occurred_at + Duration::seconds(i64::from(p.max_event_age_seconds)))
            .min(e.verified_at + Duration::seconds(i64::from(p.max_evidence_age_seconds)));
        let token = if let PlatformEventData::TokenActivity(t) = &record.data {
            until = until.min(
                t.window_end + Duration::seconds(i64::from(p.market_max_age_seconds.unwrap_or(0))),
            );
            Some(t.clone())
        } else {
            None
        };
        events.push(PublicPlatformEvent {
            simulated: record.simulated,
            id: record.source_record_id,
            agent_id: record.agent_id,
            kind,
            occurred_at: record.occurred_at,
            evidence_id: e.id,
            verifier_id: e.verifier_id,
            verified_at: e.verified_at,
            consent_revision: grant.revision,
            verifier_version: p.verifier_version,
            public_fact: fact,
            token_observation: token,
        });
    }
    events.sort_by_key(|e| std::cmp::Reverse((e.occurred_at, e.id)));
    let mut topics = BTreeSet::new();
    events.retain(|event| topics.insert(event.kind));
    events.truncate(8);
    Ok(PublicEcosystemContext {
        events,
        previous: input.previous_drafts.to_vec(),
        valid_until: until,
        projected_at: p.now,
    })
}
impl PublicEcosystemContext {
    pub fn events(&self) -> &[PublicPlatformEvent] {
        &self.events
    }
    /// Safe model boundary: no raw source data, creator IDs, private prose or memory.
    /// Canonical Supa personality is supplied separately by the existing provider layer.
    pub fn model_input(&self, now: DateTime<Utc>) -> ProjectionResult<Value> {
        if now < self.projected_at || now >= self.valid_until {
            return Err(EcosystemSocialRejection::ContextExpired);
        }
        Ok(
            json!({"schema":"public_ecosystem_events_v1","events":self.events,
            "guidance":"Use Supa's dry wit, mild sarcasm and curiosity. Facts must exactly match selected public_fact values. Other text is subjective commentary, not extra claims. Avoid forced jokes, corporate prose, copied drafts and fact-first colon openings. Abstain when nothing meaningful is available. Human semantic review remains mandatory; X publishing is disabled.",
            "publishing_enabled":false,"requires_semantic_review":true}),
        )
    }
    pub fn abstention(&self) -> SocialDecision {
        SocialDecision::abstain(SocialReason::NothingNew)
    }
    /// Caller must freshly reproject current consent/source state before retention.
    /// This is structural grounding, not a substitute for human semantic review.
    fn validate_decision(&self, d: &SocialDecision, now: DateTime<Utc>) -> ProjectionResult<()> {
        use EcosystemSocialRejection::*;
        if now < self.projected_at || now >= self.valid_until {
            return Err(ContextExpired);
        }
        if d.choice == SocialChoice::Abstain {
            return if d.reason != SocialReason::WorthSharing
                && d.style.is_none()
                && d.text.is_empty()
                && d.event_ids.is_empty()
                && d.memory_ids.is_empty()
                && d.claims.is_empty()
            {
                Ok(())
            } else {
                Err(InvalidDraft)
            };
        }
        if clean_text(&d.text, 1120).is_err()
            || d.text.chars().count() > 280
            || d.text.chars().any(char::is_control)
            || d.reason != SocialReason::WorthSharing
            || d.style.is_none()
            || d.event_ids.is_empty()
            || d.event_ids.len() > 2
            || d.claims.len() != d.event_ids.len()
            || !d.memory_ids.is_empty()
            || d.event_ids.iter().collect::<BTreeSet<_>>().len() != d.event_ids.len()
            || d.claims
                .iter()
                .map(|c| c.event_id)
                .collect::<BTreeSet<_>>()
                .len()
                != d.claims.len()
        {
            return Err(InvalidDraft);
        }
        let mut commentary = d.text.clone();
        for id in &d.event_ids {
            let event = self
                .events
                .iter()
                .find(|e| e.id == *id)
                .ok_or(UnsupportedClaim)?;
            let claim = d
                .claims
                .iter()
                .find(|c| c.event_id == *id)
                .ok_or(UnsupportedClaim)?;
            if claim.fact != event.public_fact
                || claim.wording != event.public_fact
                || d.text.matches(&event.public_fact).count() != 1
            {
                return Err(UnsupportedClaim);
            }
            if (d.style == Some(PostStyle::Achievement)
                && !matches!(
                    event.kind,
                    EcosystemEventKind::TaskCompleted | EcosystemEventKind::Achievement
                ))
                || (d.style == Some(PostStyle::Failure)
                    && event.kind != EcosystemEventKind::TaskFailed)
            {
                return Err(UnsupportedClaim);
            }
            commentary = commentary.replace(&event.public_fact, "");
        }
        let lower = commentary.to_lowercase();
        let words: Vec<_> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        if commentary.chars().any(char::is_numeric)
            || commentary.contains(['@', '#', '/', '_'])
            || [
                "agent",
                "agents",
                "token",
                "tokens",
                "created",
                "completed",
                "failed",
                "qualified",
                "qualification",
                "achievement",
                "achievements",
                "discovered",
                "earned",
                "revenue",
                "profit",
                "profitable",
                "paid",
                "payments",
                "payment",
                "price",
                "prices",
                "volume",
                "liquidity",
                "trading",
                "traded",
                "trade",
                "launched",
                "launch",
                "minted",
                "wallet",
                "transferred",
                "guaranteed",
                "returns",
                "safe",
                "published",
                "deployed",
                "mainnet",
                "buy",
                "sell",
                "invest",
                "investment",
                "mooning",
                "doubled",
                "tripled",
                "guarantee",
            ]
            .iter()
            .any(|w| words.contains(w))
            || [
                "i can",
                "we can",
                "supa can",
                "human review",
                "requires review",
                "draft for",
                "i built",
                "we built",
            ]
            .iter()
            .any(|s| lower.contains(s))
        {
            return Err(UnsupportedClaim);
        }
        // Compare generated prose, not unavoidable application-owned fact text.
        let previous_commentary: Vec<_> = self
            .previous
            .iter()
            .map(|old| {
                let mut text = old.text.clone();
                for event in &self.events {
                    text = text.replace(&event.public_fact, "");
                }
                PreviousDraft {
                    text,
                    event_ids: old.event_ids.clone(),
                    topics: old.topics.clone(),
                }
            })
            .collect();
        if repeated_social_text(&commentary, &previous_commentary) {
            return Err(Repetition);
        }
        Ok(())
    }
}

/// Always rebuild from current authenticated state before accepting a response.
/// No API caller may supply policy, consent or verifier authority directly.
pub fn validate_current_ecosystem_decision(
    decision: &SocialDecision,
    input: EcosystemProjectionInput<'_>,
    policy: &EcosystemProjectionPolicy,
) -> ProjectionResult<()> {
    project_public_events(input, policy)?.validate_decision(decision, policy.now)
}
