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

struct SuccessorFixture {
    registry: AgentTokenAssociations,
    agent: Fixture,
    candidate: AgentTokenSuccessor,
    policy: AgentTokenSuccessorPolicy,
    authorization: AgentTokenSuccessorAuthorization,
    disclosure: AgentTokenSuccessorDisclosure,
}
impl SuccessorFixture {
    fn new() -> Self {
        let mut agent = Fixture::new();
        let original = agent.candidate();
        let mut registry = AgentTokenAssociations::default();
        registry
            .try_associate(original.clone(), &agent.context(), &agent.evidence)
            .unwrap();
        agent.now += Duration::minutes(1);
        agent.evidence.id = id(108);
        agent.evidence.mint = bs58::encode([31u8; 32]).into_string();
        agent.evidence.transaction = bs58::encode([32u8; 64]).into_string();
        agent.evidence.observed_at = agent.now - Duration::seconds(20);
        let mut successor = agent.candidate();
        successor.id = id(109);
        let candidate = AgentTokenSuccessor {
            original,
            successor,
            authorization_id: id(113),
            disclosure_id: id(114),
        };
        let policy = AgentTokenSuccessorPolicy {
            id: id(110),
            version: 1,
            authorized_approvers: BTreeSet::from([id(111)]),
            authorized_disclosure_verifiers: BTreeSet::from([id(112)]),
            max_authorization_validity_seconds: 300,
            max_disclosure_age_seconds: 120,
        };
        let authorization = AgentTokenSuccessorAuthorization {
            id: candidate.authorization_id,
            policy_id: policy.id,
            policy_version: 1,
            approver_id: id(111),
            approved: true,
            revoked: false,
            candidate: candidate.clone(),
            approved_at: agent.now - Duration::seconds(40),
            expires_at: agent.now + Duration::seconds(60),
        };
        let disclosure = AgentTokenSuccessorDisclosure {
            id: candidate.disclosure_id,
            source_id: id(115),
            verifier_id: id(112),
            state: AgentTokenEvidenceState::Confirmed,
            publicly_accessible: true,
            candidate: candidate.clone(),
            published_at: agent.now - Duration::seconds(10),
            verified_at: agent.now - Duration::seconds(5),
        };
        Self {
            registry,
            agent,
            candidate,
            policy,
            authorization,
            disclosure,
        }
    }
    fn attempt(&mut self) -> Result<(), AgentIdentityRejection> {
        self.registry.try_record_successor(
            self.candidate.clone(),
            &self.agent.context(),
            &self.agent.evidence,
            &self.policy,
            &self.authorization,
            &self.disclosure,
        )
    }
    fn history(&self) -> serde_json::Value {
        serde_json::json!({"originals":self.registry.original_history(),"successors":self.registry.successor_history(),"entries":self.registry.entries()})
    }
    fn sync_receipts(&mut self) {
        self.authorization.id = self.candidate.authorization_id;
        self.authorization.candidate = self.candidate.clone();
        self.disclosure.id = self.candidate.disclosure_id;
        self.disclosure.candidate = self.candidate.clone();
    }
    fn next_exception(&mut self) {
        self.agent.now += Duration::minutes(1);
        self.agent.evidence.id = id(208);
        self.agent.evidence.mint = bs58::encode([41u8; 32]).into_string();
        self.agent.evidence.transaction = bs58::encode([42u8; 64]).into_string();
        self.agent.evidence.observed_at = self.agent.now - Duration::seconds(20);
        self.candidate.successor = self.agent.candidate();
        self.candidate.successor.id = id(209);
        self.candidate.authorization_id = id(213);
        self.candidate.disclosure_id = id(214);
        self.sync_receipts();
        self.authorization.approved_at = self.agent.now - Duration::seconds(40);
        self.authorization.expires_at = self.agent.now + Duration::seconds(60);
        self.disclosure.source_id = id(215);
        self.disclosure.published_at = self.agent.now - Duration::seconds(10);
        self.disclosure.verified_at = self.agent.now - Duration::seconds(5);
    }
}
#[test]
fn original_binding_and_confirmation_history_remain_permanent_after_successor() {
    let mut f = SuccessorFixture::new();
    let original = f.candidate.original.clone();
    let before = serde_json::to_value(f.registry.original_history()).unwrap();
    f.attempt().unwrap();
    assert!(f.registry.original_token(original.agent_id) == Some(&original));
    assert!(f.registry.entries() == [original.clone()]);
    assert_eq!(
        before,
        serde_json::to_value(f.registry.original_history()).unwrap()
    );
    let record = &f.registry.successor_history()[0];
    assert!(record.candidate == f.candidate);
    assert_eq!(record.approved_by, id(111));
    assert_eq!(record.disclosure_verified_by, id(112));
    assert_eq!(record.confirmed_at, f.agent.evidence.observed_at);
    let mut detached = f
        .registry
        .original_token(original.agent_id)
        .unwrap()
        .clone();
    detached.mint = f.candidate.successor.mint.clone();
    assert!(detached != original);
    assert!(f.registry.original_token(original.agent_id) == Some(&original));
    // Expiry/termination policy cannot erase an already-confirmed mint identity.
    f.agent.now += Duration::days(1);
    f.agent.review.decision = AgentQualificationDecision::Revoked;
    assert!(f.registry.original_token(original.agent_id) == Some(&original));
}
#[test]
fn creator_or_administrator_cannot_replace_original_or_self_authorize_exception() {
    for approver in [id(2), id(999)] {
        let mut f = SuccessorFixture::new();
        let before = f.history();
        assert_eq!(
            f.registry.try_associate(
                f.candidate.successor.clone(),
                &f.agent.context(),
                &f.agent.evidence
            ),
            Err(AgentIdentityRejection::DuplicateAgent)
        );
        f.authorization.approver_id = approver;
        assert_eq!(
            f.attempt(),
            Err(AgentIdentityRejection::SuccessorAuthorizationRequired)
        );
        assert_eq!(before, f.history());
    }
}
#[test]
fn original_full_snapshot_cannot_be_rewritten_even_with_matching_exception_receipts() {
    for field in 0..9 {
        let mut f = SuccessorFixture::new();
        let before = f.history();
        let o = &mut f.candidate.original;
        match field {
            0 => o.mint = bs58::encode([99u8; 32]).into_string(),
            1 => o.network = SolanaNetwork::Mainnet,
            2 => o.creator_id = id(900),
            3 => o.agent_id = id(900),
            4 => o.id = id(900),
            5 => o.purpose_revision_id = id(900),
            6 => o.qualification_review_id = id(900),
            7 => o.evidence_id = id(900),
            _ => o.transaction = bs58::encode([99u8; 64]).into_string(),
        }
        f.sync_receipts();
        assert_eq!(
            f.attempt(),
            Err(AgentIdentityRejection::OriginalTokenMismatch)
        );
        assert_eq!(before, f.history());
    }
}
#[test]
fn successor_policy_is_explicit_bounded_and_has_no_role_based_fallback() {
    for field in 0..8 {
        let mut f = SuccessorFixture::new();
        let before = f.history();
        match field {
            0 => f.policy.id = Uuid::nil(),
            1 => f.policy.version = 0,
            2 => f.policy.authorized_approvers.clear(),
            3 => {
                f.policy.authorized_approvers.insert(Uuid::nil());
            }
            4 => f.policy.authorized_disclosure_verifiers.clear(),
            5 => f.policy.max_authorization_validity_seconds = 0,
            6 => f.policy.max_authorization_validity_seconds = 86401,
            _ => f.policy.max_disclosure_age_seconds = 0,
        }
        assert_eq!(
            f.attempt(),
            Err(AgentIdentityRejection::InvalidSuccessorPolicy)
        );
        assert_eq!(before, f.history());
    }
}
#[test]
fn successor_approval_requires_exact_current_expiring_nonrevoked_scope() {
    for field in 0..12 {
        let mut f = SuccessorFixture::new();
        let before = f.history();
        match field {
            0 => f.authorization.id = id(900),
            1 => f.authorization.policy_id = id(900),
            2 => f.authorization.policy_version += 1,
            3 => f.authorization.approver_id = id(900),
            4 => f.authorization.approved = false,
            5 => f.authorization.revoked = true,
            6 => f.authorization.candidate.successor.creator_id = id(900),
            7 => f.authorization.approved_at = f.agent.now - Duration::hours(1),
            8 => f.authorization.approved_at = f.agent.evidence.observed_at + Duration::seconds(1),
            9 => f.authorization.expires_at = f.agent.now,
            10 => f.authorization.expires_at = f.authorization.approved_at,
            _ => f.authorization.expires_at = f.authorization.approved_at + Duration::seconds(301),
        }
        assert_eq!(
            f.attempt(),
            Err(AgentIdentityRejection::SuccessorAuthorizationRequired)
        );
        assert_eq!(before, f.history());
    }
}
#[test]
fn public_disclosure_requires_verified_exact_original_and_successor_links() {
    for field in 0..11 {
        let mut f = SuccessorFixture::new();
        let before = f.history();
        match field {
            0 => f.disclosure.id = id(900),
            1 => f.disclosure.source_id = Uuid::nil(),
            2 => f.disclosure.verifier_id = id(900),
            3 => f.disclosure.publicly_accessible = false,
            4 => f.disclosure.candidate.original.mint = "SUPA".into(),
            5 => f.disclosure.candidate.successor.mint = "SUPA".into(),
            6 => f.disclosure.published_at = f.agent.evidence.observed_at - Duration::seconds(1),
            7 => f.disclosure.verified_at = f.disclosure.published_at - Duration::seconds(1),
            8 => f.disclosure.verified_at = f.agent.now + Duration::seconds(1),
            9 => f.policy.max_disclosure_age_seconds = 5,
            _ => f.disclosure.candidate.authorization_id = id(900),
        }
        assert_eq!(
            f.attempt(),
            Err(AgentIdentityRejection::SuccessorDisclosureRequired)
        );
        assert_eq!(before, f.history());
    }
    for state in [
        AgentTokenEvidenceState::Pending,
        AgentTokenEvidenceState::Failed,
        AgentTokenEvidenceState::Unknown,
    ] {
        let mut f = SuccessorFixture::new();
        f.disclosure.state = state;
        assert_eq!(
            f.attempt(),
            Err(AgentIdentityRejection::SuccessorDisclosureRequired)
        );
        assert!(f.registry.successor_history().is_empty());
    }
}
#[test]
fn failed_pending_unknown_launches_never_consume_an_original_or_successor_binding() {
    for state in [
        AgentTokenEvidenceState::Pending,
        AgentTokenEvidenceState::Failed,
        AgentTokenEvidenceState::Unknown,
    ] {
        let mut original = Fixture::new();
        original.evidence.state = state;
        let mut registry = AgentTokenAssociations::default();
        assert_eq!(
            registry.try_associate(
                original.candidate(),
                &original.context(),
                &original.evidence
            ),
            Err(AgentIdentityRejection::UnconfirmedEvidence)
        );
        assert!(
            registry.original_history().is_empty()
                && registry.original_token(original.purpose.agent_id).is_none()
        );
        let mut f = SuccessorFixture::new();
        let before = f.history();
        f.agent.evidence.state = state;
        assert_eq!(
            f.attempt(),
            Err(AgentIdentityRejection::UnconfirmedEvidence)
        );
        assert_eq!(before, f.history());
        f.agent.evidence.state = AgentTokenEvidenceState::Confirmed;
        f.attempt().unwrap();
        assert_eq!(f.registry.successor_history().len(), 1);
    }
}
#[test]
fn multiple_exception_records_link_to_original_and_preserve_complete_prior_history() {
    let mut f = SuccessorFixture::new();
    f.attempt().unwrap();
    let first = f.registry.successor_history()[0].clone();
    let original = f.candidate.original.clone();
    f.next_exception();
    f.attempt().unwrap();
    assert_eq!(f.registry.successor_history().len(), 2);
    assert!(f.registry.successor_history()[0] == first);
    assert!(
        f.registry
            .successor_history()
            .iter()
            .all(|r| r.candidate.original == original)
    );
    assert!(f.registry.original_token(original.agent_id) == Some(&original));
    let before = f.history();
    f.candidate.original = first.candidate.successor;
    f.sync_receipts();
    assert_eq!(
        f.attempt(),
        Err(AgentIdentityRejection::OriginalTokenMismatch)
    );
    assert_eq!(before, f.history());
}
#[test]
fn successor_ids_approval_receipts_and_disclosure_receipts_are_one_use() {
    for field in 0..4 {
        let mut f = SuccessorFixture::new();
        f.attempt().unwrap();
        let first = f.candidate.clone();
        f.next_exception();
        match field {
            0 => f.candidate.successor.id = first.successor.id,
            1 => f.candidate.authorization_id = first.authorization_id,
            2 => f.candidate.disclosure_id = first.disclosure_id,
            _ => {
                f.candidate.successor.evidence_id = first.successor.evidence_id;
                f.agent.evidence.id = first.successor.evidence_id;
            }
        }
        f.sync_receipts();
        let before = f.history();
        assert_eq!(f.attempt(), Err(AgentIdentityRejection::DuplicateRecord));
        assert_eq!(before, f.history());
    }
}
#[test]
fn duplicate_mints_cannot_be_reclaimed_from_other_originals_or_successor_history() {
    let mut replay = SuccessorFixture::new();
    replay.attempt().unwrap();
    let prior_mint = replay.candidate.successor.mint.clone();
    replay.next_exception();
    replay.candidate.successor.mint = prior_mint.clone();
    replay.agent.evidence.mint = prior_mint;
    replay.sync_receipts();
    let before = replay.history();
    assert_eq!(replay.attempt(), Err(AgentIdentityRejection::DuplicateMint));
    assert_eq!(before, replay.history());
    let mut f = SuccessorFixture::new();
    let mut other = Fixture::new();
    other.authority.agent_id = id(203);
    other.purpose.agent_id = id(203);
    other.review.purpose = other.purpose.clone();
    other.evidence.id = id(208);
    other.evidence.mint = bs58::encode([51u8; 32]).into_string();
    let mut c = other.candidate();
    c.id = id(209);
    f.registry
        .try_associate(c, &other.context(), &other.evidence)
        .unwrap();
    f.candidate.successor.mint = other.evidence.mint.clone();
    f.agent.evidence.mint = other.evidence.mint;
    f.sync_receipts();
    let before = f.history();
    assert_eq!(f.attempt(), Err(AgentIdentityRejection::DuplicateMint));
    assert_eq!(before, f.history());
    let mut f = SuccessorFixture::new();
    f.attempt().unwrap();
    let before = f.history();
    let mut other = Fixture::new();
    other.authority.agent_id = id(303);
    other.purpose.agent_id = id(303);
    other.review.purpose = other.purpose.clone();
    other.evidence.id = id(308);
    other.evidence.mint = f.candidate.successor.mint.clone();
    let mut c = other.candidate();
    c.id = id(309);
    assert_eq!(
        f.registry
            .try_associate(c, &other.context(), &other.evidence),
        Err(AgentIdentityRejection::DuplicateMint)
    );
    assert_eq!(before, f.history());
}
#[test]
fn successor_never_bypasses_qualification_or_silently_crosses_networks() {
    let mut f = SuccessorFixture::new();
    f.agent.review.decision = AgentQualificationDecision::Revoked;
    assert_eq!(f.attempt(), Err(AgentIdentityRejection::NotQualified));
    assert!(f.registry.successor_history().is_empty());
    let mut f = SuccessorFixture::new();
    let before = f.history();
    f.candidate.successor.network = SolanaNetwork::Mainnet;
    f.agent.evidence.network = SolanaNetwork::Mainnet;
    f.sync_receipts();
    assert_eq!(f.attempt(), Err(AgentIdentityRejection::InvalidSuccessor));
    assert_eq!(before, f.history());
}
#[test]
fn successor_payloads_cannot_request_balance_rights_or_permission_migration() {
    let f = SuccessorFixture::new();
    for field in [
        "transfer_holder_balances",
        "ownership_rights",
        "financial_claims",
        "wallet_control",
        "administrator_override",
        "replace_original",
    ] {
        let mut input = serde_json::to_value(&f.candidate).unwrap();
        input[field] = serde_json::json!(true);
        assert!(serde_json::from_value::<AgentTokenSuccessor>(input).is_err());
    }
}
