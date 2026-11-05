use chrono::{DateTime, Duration, Utc};
use std::collections::BTreeSet;
use supa_domain::agent_identity::*;
use supa_domain::{ServiceKind, SolanaNetwork};
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}
struct Fixture {
    now: DateTime<Utc>,
    authority: AgentPurposeAuthority,
    purpose: PurposeRevision,
    policy: AgentQualificationPolicy,
    review: QualificationReview,
    evidence: AgentTokenEvidence,
}
impl Fixture {
    fn new() -> Self {
        let now: DateTime<Utc> = "2026-11-05T12:00:00Z".parse().unwrap();
        let purpose = PurposeRevision {
            id: id(3),
            agent_id: id(1),
            creator_id: id(2),
            revision: 1,
            created_at: now - Duration::hours(1),
            objective: "Summarize explicitly supplied synthetic documents".into(),
            permitted_tasks: vec![ServiceKind::Summary],
            success_criteria: vec!["Every factual claim cites supplied evidence".into()],
            excluded_actions: vec!["No network retrieval or financial execution".into()],
            public_summary: Some("Supplied-document research assistant".into()),
        };
        Self {
            now,
            authority: AgentPurposeAuthority {
                agent_id: id(1),
                creator_id: id(2),
                current_purpose_revision_id: id(3),
                current_review_id: id(4),
            },
            policy: AgentQualificationPolicy {
                id: id(5),
                version: 1,
                authorized_reviewers: BTreeSet::from([id(6)]),
                max_validity_seconds: 3600,
            },
            review: QualificationReview {
                id: id(4),
                purpose: purpose.clone(),
                policy_id: id(5),
                policy_version: 1,
                reviewer_id: id(6),
                decision: AgentQualificationDecision::Qualified,
                evidence_ids: vec![id(7)],
                reviewed_at: now - Duration::minutes(5),
                expires_at: now + Duration::minutes(5),
            },
            evidence: AgentTokenEvidence {
                id: id(8),
                state: AgentTokenEvidenceState::Confirmed,
                network: SolanaNetwork::Devnet,
                mint: bs58::encode([11u8; 32]).into_string(),
                transaction: bs58::encode([12u8; 64]).into_string(),
                observed_at: now,
            },
            purpose,
        }
    }
    fn context(&self) -> AgentQualificationContext<'_> {
        AgentQualificationContext {
            authority: &self.authority,
            purpose: &self.purpose,
            policy: &self.policy,
            review: &self.review,
            now: self.now,
        }
    }
    fn candidate(&self) -> AgentTokenAssociation {
        AgentTokenAssociation {
            id: id(9),
            agent_id: self.purpose.agent_id,
            creator_id: self.purpose.creator_id,
            purpose_revision_id: self.purpose.id,
            qualification_review_id: self.review.id,
            network: self.evidence.network,
            mint: self.evidence.mint.clone(),
            transaction: self.evidence.transaction.clone(),
            evidence_id: self.evidence.id,
        }
    }
}
fn rejected(f: &Fixture, expected: AgentIdentityRejection) {
    let mut registry = AgentTokenAssociations::default();
    assert_eq!(
        registry.try_associate(f.candidate(), &f.context(), &f.evidence),
        Err(expected)
    );
    assert!(registry.entries().is_empty());
}

#[test]
fn purposeful_agent_qualifies_only_for_explicit_current_policy() {
    let f = Fixture::new();
    f.purpose.validate_for(&f.authority, f.now).unwrap();
    f.context().validate().unwrap();
    let mut registry = AgentTokenAssociations::default();
    registry
        .try_associate(f.candidate(), &f.context(), &f.evidence)
        .unwrap();
    assert_eq!(registry.entries().len(), 1);
    assert_eq!(registry.entries()[0].mint, f.evidence.mint);
    // Qualification and association are data rules, not lifecycle or financial operations.
    assert_eq!(f.purpose.permitted_tasks, vec![ServiceKind::Summary]);
}

#[test]
fn purpose_rejects_blank_unbounded_duplicate_and_control_character_fields() {
    let mut cases = vec![];
    let p = Fixture::new().purpose;
    let mut c = p.clone();
    c.objective = " ".into();
    cases.push(c);
    let mut c = p.clone();
    c.objective = "x".repeat(601);
    cases.push(c);
    let mut c = p.clone();
    c.objective = "hidden\ncontrol".into();
    cases.push(c);
    let mut c = p.clone();
    c.revision = 0;
    cases.push(c);
    let mut c = p.clone();
    c.permitted_tasks.clear();
    cases.push(c);
    let mut c = p.clone();
    c.permitted_tasks.push(ServiceKind::Summary);
    cases.push(c);
    let mut c = p.clone();
    c.success_criteria.clear();
    cases.push(c);
    let mut c = p.clone();
    c.success_criteria = vec!["same".into(), " same ".into()];
    cases.push(c);
    let mut c = p.clone();
    c.excluded_actions.clear();
    cases.push(c);
    let mut c = p.clone();
    c.public_summary = Some("x".repeat(281));
    cases.push(c);
    let mut c = p;
    c.success_criteria = vec!["x".into(); 9];
    cases.push(c);
    for p in cases {
        let f = Fixture::new();
        assert_eq!(
            p.validate_for(&f.authority, f.now),
            Err(AgentIdentityRejection::InvalidPurpose)
        );
    }
}

#[test]
fn private_purpose_does_not_require_or_imply_public_disclosure() {
    let mut f = Fixture::new();
    f.purpose.public_summary = None;
    f.review.purpose = f.purpose.clone();
    f.context().validate().unwrap();
    assert!(f.purpose.public_summary.is_none());
}

#[test]
fn creator_and_agent_authority_cannot_be_inferred_from_token_ownership() {
    for field in [0, 1, 2, 3] {
        let mut f = Fixture::new();
        match field {
            0 => f.authority.creator_id = id(90),
            1 => f.authority.agent_id = id(90),
            2 => f.purpose.creator_id = Uuid::nil(),
            _ => f.purpose.agent_id = Uuid::nil(),
        }
        rejected(&f, AgentIdentityRejection::UnauthorizedCreator);
    }
}

#[test]
fn reused_revision_uuid_cannot_hide_changes_to_any_reviewed_purpose_field() {
    for field in 0..8 {
        let mut f = Fixture::new();
        match field {
            0 => f.purpose.objective = "Changed objective".into(),
            1 => f.purpose.success_criteria = vec!["Different outcome".into()],
            2 => f.purpose.excluded_actions = vec!["Different exclusions".into()],
            3 => f.purpose.permitted_tasks = vec![ServiceKind::CodeAnalysis],
            4 => f.purpose.public_summary = Some("Changed public summary".into()),
            5 => f.purpose.revision = 2,
            6 => f.purpose.created_at -= Duration::seconds(1),
            _ => f.authority.current_purpose_revision_id = id(90),
        }
        rejected(&f, AgentIdentityRejection::PurposeMismatch);
    }
}

#[test]
fn default_empty_or_invalid_qualification_policy_never_qualifies() {
    for field in 0..5 {
        let mut f = Fixture::new();
        match field {
            0 => f.policy.id = Uuid::nil(),
            1 => f.policy.version = 0,
            2 => f.policy.max_validity_seconds = 0,
            3 => f.policy.authorized_reviewers.clear(),
            _ => {
                f.policy.authorized_reviewers.insert(Uuid::nil());
            }
        }
        rejected(&f, AgentIdentityRejection::InvalidPolicy);
    }
}

#[test]
fn non_allowlisted_reviewer_is_rejected() {
    let mut f = Fixture::new();
    f.review.reviewer_id = f.purpose.creator_id;
    rejected(&f, AgentIdentityRejection::UnauthorizedReviewer);
}

#[test]
fn stale_current_review_or_policy_binding_is_rejected() {
    for field in 0..4 {
        let mut f = Fixture::new();
        match field {
            0 => f.authority.current_review_id = id(90),
            1 => f.review.id = Uuid::nil(),
            2 => f.review.policy_id = id(90),
            _ => f.review.policy_version += 1,
        }
        rejected(&f, AgentIdentityRejection::ReviewMismatch);
    }
}

#[test]
fn qualification_expiry_future_and_overlong_validity_fail_closed() {
    for field in 0..6 {
        let mut f = Fixture::new();
        match field {
            0 => f.review.expires_at = f.now,
            1 => f.review.reviewed_at = f.now + Duration::seconds(1),
            2 => f.review.expires_at = f.review.reviewed_at,
            3 => f.review.reviewed_at = f.purpose.created_at - Duration::seconds(1),
            4 => f.review.expires_at = f.review.reviewed_at + Duration::seconds(3601),
            _ => {
                f.purpose.created_at = f.now + Duration::seconds(1);
                f.review.purpose = f.purpose.clone();
            }
        }
        rejected(
            &f,
            if field == 5 {
                AgentIdentityRejection::InvalidPurpose
            } else {
                AgentIdentityRejection::StaleReview
            },
        );
    }
}

#[test]
fn qualification_requires_unique_non_nil_evidence_references() {
    for evidence in [
        vec![],
        vec![Uuid::nil()],
        vec![id(7), id(7)],
        vec![id(7); 17],
    ] {
        let mut f = Fixture::new();
        f.review.evidence_ids = evidence;
        rejected(&f, AgentIdentityRejection::InvalidReview);
    }
}

#[test]
fn rejected_and_revoked_reviews_never_authorize_associations() {
    for decision in [
        AgentQualificationDecision::Rejected,
        AgentQualificationDecision::Revoked,
    ] {
        let mut f = Fixture::new();
        f.review.decision = decision;
        rejected(&f, AgentIdentityRejection::NotQualified);
    }
}

#[test]
fn pending_failed_and_unknown_chain_outcomes_never_form_associations() {
    for state in [
        AgentTokenEvidenceState::Pending,
        AgentTokenEvidenceState::Failed,
        AgentTokenEvidenceState::Unknown,
    ] {
        let mut f = Fixture::new();
        f.evidence.state = state;
        rejected(&f, AgentIdentityRejection::UnconfirmedEvidence);
    }
}

#[test]
fn canonical_address_and_signature_and_exact_association_bindings_are_required() {
    let f = Fixture::new();
    for field in 0..9 {
        let mut c = f.candidate();
        match field {
            0 => c.id = Uuid::nil(),
            1 => c.creator_id = id(90),
            2 => c.agent_id = id(90),
            3 => c.purpose_revision_id = id(90),
            4 => c.qualification_review_id = id(90),
            5 => c.mint = "SUPA".into(),
            6 => c.transaction = "not a signature".into(),
            7 => c.evidence_id = Uuid::nil(),
            _ => c.mint.push(' '),
        }
        let mut registry = AgentTokenAssociations::default();
        assert_eq!(
            registry.try_associate(c, &f.context(), &f.evidence),
            Err(AgentIdentityRejection::InvalidAssociation)
        );
        assert!(registry.entries().is_empty());
    }
}

#[test]
fn confirmed_evidence_must_match_every_field_and_valid_observation_window() {
    let f = Fixture::new();
    for field in 0..6 {
        let mut c = f.candidate();
        match field {
            0 => c.evidence_id = id(90),
            1 => c.network = SolanaNetwork::Mainnet,
            2 => c.mint = bs58::encode([90u8; 32]).into_string(),
            3 => c.transaction = bs58::encode([90u8; 64]).into_string(),
            _ => {}
        }
        let observed_at = if field == 4 {
            f.now + Duration::seconds(1)
        } else if field == 5 {
            f.review.reviewed_at - Duration::seconds(1)
        } else {
            f.evidence.observed_at
        };
        let evidence = AgentTokenEvidence {
            id: f.evidence.id,
            state: f.evidence.state,
            network: f.evidence.network,
            mint: f.evidence.mint.clone(),
            transaction: f.evidence.transaction.clone(),
            observed_at,
        };
        let mut registry = AgentTokenAssociations::default();
        assert_eq!(
            registry.try_associate(c, &f.context(), &evidence),
            Err(AgentIdentityRejection::EvidenceMismatch)
        );
        assert!(registry.entries().is_empty());
    }
}

#[test]
fn duplicate_records_and_second_tokens_across_clusters_preserve_first_association() {
    let mut f = Fixture::new();
    let mut registry = AgentTokenAssociations::default();
    let original = f.candidate();
    registry
        .try_associate(original.clone(), &f.context(), &f.evidence)
        .unwrap();
    assert_eq!(
        registry.try_associate(original.clone(), &f.context(), &f.evidence),
        Err(AgentIdentityRejection::DuplicateRecord)
    );
    let mut second = f.candidate();
    second.id = id(91);
    second.mint = bs58::encode([91u8; 32]).into_string();
    f.evidence.mint = second.mint.clone();
    second.network = SolanaNetwork::Mainnet;
    f.evidence.network = second.network;
    assert_eq!(
        registry.try_associate(second, &f.context(), &f.evidence),
        Err(AgentIdentityRejection::DuplicateAgent)
    );
    assert!(registry.entries() == [original]);
}

#[test]
fn same_mint_and_cluster_cannot_be_claimed_by_another_qualified_agent() {
    let f = Fixture::new();
    let mut registry = AgentTokenAssociations::default();
    registry
        .try_associate(f.candidate(), &f.context(), &f.evidence)
        .unwrap();
    let mut other = Fixture::new();
    other.authority.agent_id = id(91);
    other.purpose.agent_id = id(91);
    other.review.purpose = other.purpose.clone();
    let mut candidate = other.candidate();
    candidate.id = id(92);
    assert_eq!(
        registry.try_associate(candidate, &other.context(), &other.evidence),
        Err(AgentIdentityRejection::DuplicateMint)
    );
    assert_eq!(registry.entries().len(), 1);
}

#[test]
fn client_shapes_reject_financial_role_launch_and_unknown_fields() {
    let f = Fixture::new();
    for field in [
        "wallet_signing",
        "earned_revenue",
        "token_holder_role",
        "launch_token",
        "private_key",
    ] {
        let mut p = serde_json::to_value(&f.purpose).unwrap();
        p[field] = serde_json::json!(true);
        assert!(serde_json::from_value::<PurposeRevision>(p).is_err());
        let mut r = serde_json::to_value(&f.review).unwrap();
        r[field] = serde_json::json!(true);
        assert!(serde_json::from_value::<QualificationReview>(r).is_err());
        let mut a = serde_json::to_value(f.candidate()).unwrap();
        a[field] = serde_json::json!(true);
        assert!(serde_json::from_value::<AgentTokenAssociation>(a).is_err());
    }
    let mut p = serde_json::to_value(&f.purpose).unwrap();
    p["permitted_tasks"] = serde_json::json!(["launch_token"]);
    assert!(serde_json::from_value::<PurposeRevision>(p).is_err());
    let mut r = serde_json::to_value(&f.review).unwrap();
    r["decision"] = serde_json::json!("pay_creator");
    assert!(serde_json::from_value::<QualificationReview>(r).is_err());
}

#[test]
fn rejections_contain_stable_codes_never_customer_prose() {
    let mut f = Fixture::new();
    f.purpose.objective = "private synthetic customer prose\n".into();
    let error = f.context().validate().unwrap_err();
    assert_eq!(error.to_string(), "invalid_purpose");
    assert!(!format!("{error:?}").contains("private synthetic"));
}
