use chrono::{DateTime, Duration, Utc};
use std::collections::BTreeSet;
use supa_domain::agent_identity::*;
use supa_domain::ecosystem_social::*;
use supa_domain::{
    Personality, PostStyle, PreviousDraft, ServiceKind, SocialChoice, SocialClaim, SocialDecision,
    SocialReason, SolanaNetwork,
};
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}
fn kinds() -> BTreeSet<EcosystemEventKind> {
    [
        EcosystemEventKind::AgentCreated,
        EcosystemEventKind::AgentQualified,
        EcosystemEventKind::TaskCompleted,
        EcosystemEventKind::TaskFailed,
        EcosystemEventKind::Achievement,
        EcosystemEventKind::TokenActivity,
    ]
    .into_iter()
    .collect()
}
fn created() -> PlatformEventData {
    PlatformEventData::AgentCreated {
        purpose_revision_id: id(3),
    }
}
fn completed() -> PlatformEventData {
    PlatformEventData::TaskCompleted {
        result_id: id(70),
        state: CompletionState::Completed,
        result_verified: true,
        semantic_review: SemanticReviewState::Accepted,
    }
}
struct Fixture {
    records: Vec<PlatformSourceRecord>,
    evidence: Vec<VerifiedPlatformEvidence>,
    visibility: Vec<CreatorPublicVisibility>,
    associations: AgentTokenAssociations,
    previous: Vec<PreviousDraft>,
    policy: EcosystemProjectionPolicy,
}
impl Fixture {
    fn new(data: Vec<PlatformEventData>) -> Self {
        let now: DateTime<Utc> = "2026-11-05T12:00:00Z".parse().unwrap();
        let purpose = PurposeRevision {
            id: id(3),
            agent_id: id(1),
            creator_id: id(2),
            revision: 1,
            created_at: now - Duration::hours(1),
            objective: "private goal sentinel".into(),
            permitted_tasks: vec![ServiceKind::Summary],
            success_criteria: vec!["private criteria sentinel".into()],
            excluded_actions: vec!["No execution".into()],
            public_summary: None,
        };
        let authority = AgentPurposeAuthority {
            agent_id: id(1),
            creator_id: id(2),
            current_purpose_revision_id: id(3),
            current_review_id: id(4),
        };
        let policy = AgentQualificationPolicy {
            id: id(5),
            version: 1,
            authorized_reviewers: BTreeSet::from([id(6)]),
            max_validity_seconds: 3600,
        };
        let review = QualificationReview {
            id: id(4),
            purpose: purpose.clone(),
            policy_id: id(5),
            policy_version: 1,
            reviewer_id: id(6),
            decision: AgentQualificationDecision::Qualified,
            evidence_ids: vec![id(7)],
            reviewed_at: now - Duration::minutes(5),
            expires_at: now + Duration::minutes(5),
        };
        let evidence = AgentTokenEvidence {
            id: id(8),
            state: AgentTokenEvidenceState::Confirmed,
            network: SolanaNetwork::Devnet,
            mint: bs58::encode([11u8; 32]).into_string(),
            transaction: bs58::encode([12u8; 64]).into_string(),
            observed_at: now,
        };
        let association = AgentTokenAssociation {
            id: id(9),
            agent_id: id(1),
            creator_id: id(2),
            purpose_revision_id: id(3),
            qualification_review_id: id(4),
            network: evidence.network,
            mint: evidence.mint.clone(),
            transaction: evidence.transaction.clone(),
            evidence_id: evidence.id,
        };
        let mut associations = AgentTokenAssociations::default();
        associations
            .try_associate(
                association,
                &AgentQualificationContext {
                    authority: &authority,
                    purpose: &purpose,
                    policy: &policy,
                    review: &review,
                    now,
                },
                &evidence,
            )
            .unwrap();
        let records = data
            .into_iter()
            .enumerate()
            .map(|(i, data)| PlatformSourceRecord {
                simulated: true,
                source_record_id: id(100 + i as u128),
                agent_id: id(1),
                creator_id: id(2),
                occurred_at: now - Duration::seconds(10),
                data,
            })
            .collect();
        let mut f = Self {
            records,
            evidence: vec![],
            visibility: vec![CreatorPublicVisibility {
                agent_id: id(1),
                creator_id: id(2),
                revision: 1,
                public: true,
                revoked: false,
                valid_from: now - Duration::minutes(30),
                expires_at: now + Duration::minutes(30),
                categories: kinds(),
            }],
            associations,
            previous: vec![],
            policy: EcosystemProjectionPolicy {
                authorized_agents: BTreeSet::from([id(1)]),
                authorized_verifiers: BTreeSet::from([id(6)]),
                approved_market_sources: BTreeSet::from([id(300)]),
                verifier_version: 1,
                now,
                max_event_age_seconds: 3600,
                max_evidence_age_seconds: 600,
                market_max_age_seconds: Some(300),
                market_max_window_seconds: Some(3600),
            },
        };
        f.refresh_evidence();
        f
    }
    fn refresh_evidence(&mut self) {
        self.evidence = self
            .records
            .iter()
            .enumerate()
            .map(|(i, r)| VerifiedPlatformEvidence {
                id: id(200 + i as u128),
                verifier_id: id(6),
                origin: if r.data.kind() == EcosystemEventKind::TokenActivity {
                    EvidenceOrigin::IndependentPublicMarket
                } else {
                    EvidenceOrigin::ApplicationRecord
                },
                record: r.clone(),
                verified_at: self.policy.now,
            })
            .collect();
    }
    fn token() -> Self {
        let mut f = Self::new(vec![]);
        let token = PublicTokenObservation {
            association_id: id(9),
            network: SolanaNetwork::Devnet,
            mint: f.associations.entries()[0].mint.clone(),
            source_id: id(300),
            venue_id: id(301),
            quote: QuoteUnit::USDC,
            decimals: 6,
            window_start: f.policy.now - Duration::minutes(1),
            window_end: f.policy.now - Duration::seconds(10),
            metric: PublicTokenMetric::PriceChange {
                before_atoms: 2_000_000,
                after_atoms: 3_000_000,
            },
        };
        f.records.push(PlatformSourceRecord {
            simulated: true,
            source_record_id: id(100),
            agent_id: id(1),
            creator_id: id(2),
            occurred_at: token.window_end,
            data: PlatformEventData::TokenActivity(token),
        });
        f.refresh_evidence();
        f
    }
    fn input(&self) -> EcosystemProjectionInput<'_> {
        EcosystemProjectionInput {
            records: &self.records,
            evidence: &self.evidence,
            visibility: &self.visibility,
            associations: &self.associations,
            previous_drafts: &self.previous,
        }
    }
    fn project(&self) -> Result<PublicEcosystemContext, EcosystemSocialRejection> {
        project_public_events(self.input(), &self.policy)
    }
    fn validate(&self, d: &SocialDecision) -> Result<(), EcosystemSocialRejection> {
        validate_current_ecosystem_decision(d, self.input(), &self.policy)
    }
    fn mutate_token(&mut self, change: impl FnOnce(&mut PublicTokenObservation)) {
        if let PlatformEventData::TokenActivity(t) = &mut self.records[0].data {
            change(t)
        } else {
            panic!("token fixture required")
        }
        self.refresh_evidence();
    }
}
fn decision(c: &PublicEcosystemContext, commentary: &str) -> SocialDecision {
    let e = &c.events()[0];
    SocialDecision {
        choice: SocialChoice::Draft,
        reason: SocialReason::WorthSharing,
        style: Some(PostStyle::Observation),
        text: format!("{} {commentary}", e.public_fact()).trim().into(),
        event_ids: vec![e.id()],
        memory_ids: vec![],
        claims: vec![SocialClaim {
            event_id: e.id(),
            fact: e.public_fact().into(),
            wording: e.public_fact().into(),
        }],
    }
}
fn assert_rejection(f: &Fixture, expected: EcosystemSocialRejection) {
    match f.project() {
        Ok(_) => panic!("projection should reject"),
        Err(e) => assert_eq!(e, expected),
    }
}

#[test]
fn five_application_event_kinds_have_only_verified_fixed_facts_and_provenance() {
    let f = Fixture::new(vec![
        created(),
        PlatformEventData::AgentQualified {
            purpose_revision_id: id(3),
            review_id: id(4),
            decision: AgentQualificationDecision::Qualified,
        },
        completed(),
        PlatformEventData::TaskFailed {
            reason: PublicFailureCode::PermissionDenied,
        },
        PlatformEventData::Achievement {
            result_id: id(71),
            semantic_review: SemanticReviewState::Accepted,
        },
    ]);
    let c = f.project().unwrap();
    assert_eq!(c.events().len(), 5);
    let payload = c.model_input(f.policy.now).unwrap();
    for event in payload["events"].as_array().unwrap() {
        assert_eq!(event["consent_revision"], 1);
        assert_eq!(event["verifier_version"], 1);
        for field in [
            "id",
            "agent_id",
            "evidence_id",
            "verifier_id",
            "occurred_at",
            "verified_at",
        ] {
            assert!(!event[field].is_null());
        }
        assert!(
            event["public_fact"]
                .as_str()
                .unwrap()
                .contains("publicly listed agent")
        );
    }
    assert!(payload["guidance"].as_str().unwrap().contains("dry wit"));
}

#[test]
fn public_projection_excludes_private_goals_memories_creator_identity_and_raw_history() {
    let mut f = Fixture::new(vec![created()]);
    f.previous.push(PreviousDraft {
        text:
            "private memory sentinel; private credentials sentinel; unpublished financial sentinel"
                .into(),
        event_ids: vec![id(999)],
        topics: vec![],
    });
    let json = f
        .project()
        .unwrap()
        .model_input(f.policy.now)
        .unwrap()
        .to_string();
    for forbidden in [
        "private goal sentinel",
        "private criteria sentinel",
        "private memory sentinel",
        "private credentials sentinel",
        "unpublished financial sentinel",
        "creator_id",
        &id(999).to_string(),
        "objective",
        "success_criteria",
        "transaction",
        "wallet",
        "private_key",
        &id(2).to_string(),
    ] {
        assert!(!json.contains(forbidden), "unexpected private field");
    }
}

#[test]
fn missing_private_revoked_expired_future_or_category_absent_consent_produces_abstention() {
    for mode in 0..8 {
        let mut f = Fixture::new(vec![created()]);
        match mode {
            0 => f.visibility.clear(),
            1 => f.visibility[0].public = false,
            2 => f.visibility[0].revoked = true,
            3 => f.visibility[0].expires_at = f.policy.now,
            4 => f.visibility[0].valid_from = f.policy.now + Duration::seconds(1),
            5 => f.visibility[0].categories.clear(),
            6 => f.visibility[0].revision = 0,
            _ => f.visibility[0].valid_from = f.records[0].occurred_at + Duration::seconds(1),
        }
        let c = f.project().unwrap();
        assert!(c.events().is_empty());
        f.validate(&c.abstention()).unwrap();
        assert!(
            c.model_input(f.policy.now).unwrap()["events"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn unauthorized_agent_owner_and_ambiguous_creator_consent_fail_closed() {
    let mut f = Fixture::new(vec![created()]);
    f.policy.authorized_agents.clear();
    assert_rejection(&f, EcosystemSocialRejection::ScopeDenied);
    let mut f = Fixture::new(vec![created()]);
    f.records[0].creator_id = id(99);
    assert_rejection(&f, EcosystemSocialRejection::ScopeDenied);
    let mut f = Fixture::new(vec![created()]);
    f.visibility[0].creator_id = Uuid::nil();
    assert_rejection(&f, EcosystemSocialRejection::ScopeDenied);
    let mut f = Fixture::new(vec![created()]);
    f.visibility.push(CreatorPublicVisibility {
        agent_id: id(1),
        creator_id: id(2),
        revision: 2,
        public: true,
        revoked: false,
        valid_from: f.policy.now - Duration::minutes(2),
        expires_at: f.policy.now + Duration::minutes(2),
        categories: kinds(),
    });
    assert_rejection(&f, EcosystemSocialRejection::AmbiguousVisibility);
}

#[test]
fn privacy_is_checked_before_requiring_private_source_evidence() {
    let mut f = Fixture::new(vec![created()]);
    f.visibility[0].public = false;
    f.evidence.clear();
    assert!(f.project().unwrap().events().is_empty());
}

#[test]
fn evidence_is_required_exact_independently_authorized_and_time_bound() {
    let mut f = Fixture::new(vec![created()]);
    f.evidence.clear();
    assert_rejection(&f, EcosystemSocialRejection::EvidenceRequired);
    for mode in 0..4 {
        let mut f = Fixture::new(vec![created()]);
        match mode {
            0 => f.evidence[0].id = Uuid::nil(),
            1 => f.evidence[0].verifier_id = id(99),
            2 => f.evidence[0].record.creator_id = id(99),
            _ => f.evidence[0].origin = EvidenceOrigin::IndependentPublicMarket,
        }
        assert_rejection(&f, EcosystemSocialRejection::ConflictingEvidence);
    }
    for mode in 0..4 {
        let mut f = Fixture::new(vec![created()]);
        match mode {
            0 => f.evidence[0].verified_at = f.records[0].occurred_at - Duration::seconds(1),
            1 => f.evidence[0].verified_at = f.policy.now + Duration::seconds(1),
            2 => {
                f.records[0].occurred_at = f.policy.now + Duration::seconds(1);
                f.refresh_evidence();
            }
            _ => {
                f.records[0].occurred_at = f.policy.now - Duration::seconds(601);
                f.refresh_evidence();
                f.evidence[0].verified_at = f.records[0].occurred_at;
            }
        }
        assert_rejection(&f, EcosystemSocialRejection::StaleEvidence);
    }
}

#[test]
fn empty_or_nil_policy_authority_and_oversized_batches_never_default_to_public() {
    for mode in 0..7 {
        let mut f = Fixture::new(vec![created()]);
        match mode {
            0 => f.policy.verifier_version = 0,
            1 => f.policy.max_event_age_seconds = 0,
            2 => f.policy.max_evidence_age_seconds = 0,
            3 => f.policy.authorized_verifiers.clear(),
            4 => {
                f.policy.authorized_verifiers.insert(Uuid::nil());
            }
            5 => {
                f.policy.approved_market_sources.insert(Uuid::nil());
            }
            _ => f.records = vec![f.records[0].clone(); 65],
        }
        assert_rejection(&f, EcosystemSocialRejection::InvalidPolicy);
    }
}

#[test]
fn repeated_source_records_are_deduplicated_but_conflicts_and_duplicate_receipts_reject() {
    let mut f = Fixture::new(vec![created()]);
    f.records.push(f.records[0].clone());
    assert_eq!(f.project().unwrap().events().len(), 1);
    f.records[1].data = PlatformEventData::TaskFailed {
        reason: PublicFailureCode::InvalidResult,
    };
    assert_rejection(&f, EcosystemSocialRejection::ConflictingEvidence);
    let mut f = Fixture::new(vec![created(), completed()]);
    f.evidence[1].id = f.evidence[0].id;
    assert_rejection(&f, EcosystemSocialRejection::ConflictingEvidence);
    let mut f = Fixture::new(vec![created(), completed()]);
    f.evidence[1].record = f.evidence[0].record.clone();
    assert_rejection(&f, EcosystemSocialRejection::ConflictingEvidence);
}

#[test]
fn evaluation_only_incomplete_unreviewed_and_invalid_results_cannot_claim_completion() {
    for (state, verified, review) in [
        (
            CompletionState::EvaluationOnly,
            true,
            SemanticReviewState::Accepted,
        ),
        (
            CompletionState::Incomplete,
            true,
            SemanticReviewState::Accepted,
        ),
        (
            CompletionState::Completed,
            false,
            SemanticReviewState::Accepted,
        ),
        (
            CompletionState::Completed,
            true,
            SemanticReviewState::Pending,
        ),
        (
            CompletionState::Completed,
            true,
            SemanticReviewState::Rejected,
        ),
    ] {
        let f = Fixture::new(vec![PlatformEventData::TaskCompleted {
            result_id: id(70),
            state,
            result_verified: verified,
            semantic_review: review,
        }]);
        assert_rejection(&f, EcosystemSocialRejection::UnverifiedOutcome);
    }
    let f = Fixture::new(vec![PlatformEventData::TaskCompleted {
        result_id: id(70),
        state: CompletionState::Completed,
        result_verified: true,
        semantic_review: SemanticReviewState::NotRequired,
    }]);
    assert_eq!(f.project().unwrap().events().len(), 1);
}

#[test]
fn qualification_and_achievement_need_specific_accepted_evidence() {
    for decision in [
        AgentQualificationDecision::Rejected,
        AgentQualificationDecision::Revoked,
    ] {
        assert_rejection(
            &Fixture::new(vec![PlatformEventData::AgentQualified {
                purpose_revision_id: id(3),
                review_id: id(4),
                decision,
            }]),
            EcosystemSocialRejection::UnverifiedOutcome,
        );
    }
    for review in [
        SemanticReviewState::Pending,
        SemanticReviewState::Rejected,
        SemanticReviewState::NotRequired,
    ] {
        assert_rejection(
            &Fixture::new(vec![PlatformEventData::Achievement {
                result_id: id(71),
                semantic_review: review,
            }]),
            EcosystemSocialRejection::UnverifiedOutcome,
        );
    }
    for data in [
        PlatformEventData::AgentCreated {
            purpose_revision_id: Uuid::nil(),
        },
        PlatformEventData::AgentQualified {
            purpose_revision_id: id(3),
            review_id: Uuid::nil(),
            decision: AgentQualificationDecision::Qualified,
        },
        PlatformEventData::Achievement {
            result_id: Uuid::nil(),
            semantic_review: SemanticReviewState::Accepted,
        },
    ] {
        assert_rejection(
            &Fixture::new(vec![data]),
            EcosystemSocialRejection::UnverifiedOutcome,
        );
    }
}

#[test]
fn failure_codes_are_closed_public_facts_without_raw_error_text() {
    for reason in [
        PublicFailureCode::InsufficientEvidence,
        PublicFailureCode::PermissionDenied,
        PublicFailureCode::ProviderUnavailable,
        PublicFailureCode::InvalidResult,
    ] {
        let f = Fixture::new(vec![PlatformEventData::TaskFailed { reason }]);
        let c = f.project().unwrap();
        let mut d = decision(&c, "Apparently friction insists on attending.");
        d.style = Some(PostStyle::Failure);
        f.validate(&d).unwrap();
    }
}

#[test]
fn private_memories_are_not_claim_sources_and_unsupported_embellishments_are_rejected() {
    let f = Fixture::new(vec![created()]);
    let c = f.project().unwrap();
    let base = decision(&c, "Apparently boredom lost another round.");
    f.validate(&base).unwrap();
    for extra in [
        " We earned real revenue.",
        " We launched a token.",
        " Guaranteed returns.",
        " We can trade now.",
        " Another 42 payments.",
        " requires review",
        " sk-synthetic",
        " https://example.invalid",
    ] {
        let mut d = base.clone();
        d.text.push_str(extra);
        assert!(f.validate(&d).is_err());
    }
    let mut d = base.clone();
    d.memory_ids.push(id(500));
    assert_eq!(f.validate(&d), Err(EcosystemSocialRejection::InvalidDraft));
    let mut d = base.clone();
    d.claims[0].fact = "A token was launched.".into();
    assert_eq!(
        f.validate(&d),
        Err(EcosystemSocialRejection::UnsupportedClaim)
    );
    let mut d = base.clone();
    d.claims[0].wording = "An unsupported achievement".into();
    assert!(f.validate(&d).is_err());
    let mut d = base.clone();
    d.event_ids[0] = id(999);
    d.claims[0].event_id = id(999);
    assert!(f.validate(&d).is_err());
    let mut d = base.clone();
    d.style = Some(PostStyle::Achievement);
    assert!(f.validate(&d).is_err());
    let mut d = base;
    d.style = Some(PostStyle::Failure);
    assert!(f.validate(&d).is_err());
}

#[test]
fn revocation_and_category_changes_after_generation_are_rechecked_at_acceptance() {
    for mode in 0..4 {
        let mut f = Fixture::new(vec![created()]);
        let d = decision(
            &f.project().unwrap(),
            "Apparently boredom lost another round.",
        );
        match mode {
            0 => f.visibility[0].revoked = true,
            1 => f.visibility[0].categories.clear(),
            2 => f.visibility[0].public = false,
            _ => f.visibility[0].expires_at = f.policy.now,
        }
        assert_eq!(
            f.validate(&d),
            Err(EcosystemSocialRejection::UnsupportedClaim)
        );
    }
}

#[test]
fn empty_inputs_allow_abstention_but_never_fabricated_work() {
    let f = Fixture::new(vec![]);
    let c = f.project().unwrap();
    assert!(c.events().is_empty());
    f.validate(&c.abstention()).unwrap();
    let mut d = c.abstention();
    d.text = "Invented progress".into();
    assert_eq!(f.validate(&d), Err(EcosystemSocialRejection::InvalidDraft));
}

#[test]
fn event_reuse_topic_cooldown_and_recycled_prose_are_enforced() {
    let mut f = Fixture::new(vec![created()]);
    let c = f.project().unwrap();
    let d = decision(&c, "Apparently boredom lost another round.");
    f.previous.push(PreviousDraft {
        text: "Other prose".into(),
        event_ids: vec![c.events()[0].id()],
        topics: vec![],
    });
    assert!(f.project().unwrap().events().is_empty());
    assert!(f.validate(&d).is_err());
    f.previous[0].event_ids.clear();
    f.previous[0].topics.push(c.events()[0].topic());
    assert!(f.project().unwrap().events().is_empty());
    f.previous[0].topics.clear();
    f.previous[0].text = d.text.clone();
    assert_eq!(f.validate(&d), Err(EcosystemSocialRejection::Repetition));
}

#[test]
fn deterministic_selection_prefers_recent_distinct_topics_without_duplicate_events() {
    let mut f = Fixture::new(vec![created(), created(), completed()]);
    f.records[0].occurred_at -= Duration::seconds(1);
    f.refresh_evidence();
    let c = f.project().unwrap();
    assert_eq!(c.events().len(), 2);
    assert!(c.events().iter().any(|e| e.id() == id(101)));
    assert!(!c.events().iter().any(|e| e.id() == id(100)));
    assert_eq!(
        c.model_input(f.policy.now).unwrap(),
        f.project().unwrap().model_input(f.policy.now).unwrap()
    );
}

#[test]
fn multiple_supported_claims_and_natural_commentary_remain_separate_from_evidence() {
    let f = Fixture::new(vec![created(), completed()]);
    let c = f.project().unwrap();
    let mut d = decision(&c, "");
    let e = &c.events()[1];
    d.event_ids.push(e.id());
    d.claims.push(SocialClaim {
        event_id: e.id(),
        fact: e.public_fact().into(),
        wording: e.public_fact().into(),
    });
    d.text.push(' ');
    d.text.push_str(e.public_fact());
    d.text.push_str(" Boredom had a terrible afternoon.");
    f.validate(&d).unwrap();
    let mut duplicate = d.clone();
    duplicate.claims[1] = duplicate.claims[0].clone();
    assert!(f.validate(&duplicate).is_err());
    let mut missing = d.clone();
    missing.text = missing.text.replace(e.public_fact(), "");
    assert!(f.validate(&missing).is_err());
    let mut repeated = d;
    repeated.text.push_str(e.public_fact());
    assert!(f.validate(&repeated).is_err());
}

#[test]
fn token_price_volume_and_liquidity_require_public_independent_receipts_and_exact_integer_units() {
    for metric in [
        PublicTokenMetric::PriceChange {
            before_atoms: 2_000_000,
            after_atoms: 3_000_000,
        },
        PublicTokenMetric::Volume { atoms: 0 },
        PublicTokenMetric::Liquidity { atoms: u64::MAX },
    ] {
        let mut f = Fixture::token();
        f.mutate_token(|t| t.metric = metric);
        let c = f.project().unwrap();
        assert_eq!(c.events()[0].kind(), EcosystemEventKind::TokenActivity);
        assert!(c.events()[0].public_fact().contains("In a simulation"));
        assert!(c.events()[0].public_fact().contains("Devnet test"));
        let d = decision(&c, "Apparently the chart refuses to nap.");
        f.validate(&d).unwrap();
        let data = c.model_input(f.policy.now).unwrap();
        assert!(data["events"][0]["token_observation"]["mint"].is_string());
        assert!(!d.text.contains("earn"));
        assert!(!d.text.contains("profit"));
    }
}

#[test]
fn token_data_requires_approved_source_explicit_freshness_policy_and_confirmed_association() {
    for mode in 0..8 {
        let mut f = Fixture::token();
        match mode {
            0 => f.policy.approved_market_sources.clear(),
            1 => f.policy.market_max_age_seconds = None,
            2 => f.policy.market_max_window_seconds = None,
            3 => f.policy.market_max_age_seconds = Some(0),
            4 => f.policy.market_max_window_seconds = Some(1),
            5 => f.policy.market_max_age_seconds = Some(1),
            6 => f.associations = AgentTokenAssociations::default(),
            _ => f.evidence[0].origin = EvidenceOrigin::ApplicationRecord,
        }
        assert_rejection(&f, EcosystemSocialRejection::InvalidTokenEvidence);
    }
}

#[test]
fn token_mint_cluster_units_timestamps_and_noncomparable_values_fail_closed() {
    for mode in 0..10 {
        let mut f = Fixture::token();
        let now = f.policy.now;
        f.mutate_token(|t| match mode {
            0 => t.network = SolanaNetwork::Mainnet,
            1 => t.mint = bs58::encode([99u8; 32]).into_string(),
            2 => t.association_id = id(999),
            3 => t.venue_id = Uuid::nil(),
            4 => t.decimals = 10,
            5 => t.window_end = now,
            6 => t.window_start = t.window_end,
            7 => t.window_start = t.window_end + Duration::seconds(1),
            8 => {
                t.metric = PublicTokenMetric::PriceChange {
                    before_atoms: 0,
                    after_atoms: 100,
                }
            }
            _ => {
                t.metric = PublicTokenMetric::PriceChange {
                    before_atoms: 100,
                    after_atoms: 0,
                }
            }
        });
        assert_rejection(&f, EcosystemSocialRejection::InvalidTokenEvidence);
    }
}

#[test]
fn conflicting_market_observations_cannot_be_hidden_by_new_ids_or_topic_selection() {
    for mode in 0..3 {
        let mut f = Fixture::token();
        let mut other = f.records[0].clone();
        other.source_record_id = id(999);
        if let PlatformEventData::TokenActivity(t) = &mut other.data {
            match mode {
                0 => {
                    t.metric = PublicTokenMetric::PriceChange {
                        before_atoms: 2_000_000,
                        after_atoms: 4_000_000,
                    }
                }
                1 => t.quote = QuoteUnit::SOL,
                _ => t.decimals = 5,
            }
        }
        f.records.push(other);
        f.refresh_evidence();
        assert_rejection(&f, EcosystemSocialRejection::ConflictingEvidence);
    }
}

#[test]
fn token_disclosure_is_opt_in_and_does_not_follow_agent_visibility_alone() {
    let mut f = Fixture::token();
    f.visibility[0]
        .categories
        .remove(&EcosystemEventKind::TokenActivity);
    assert!(f.project().unwrap().events().is_empty());
}

#[test]
fn projected_payload_expires_and_future_clock_replays_are_rejected() {
    let mut f = Fixture::new(vec![created()]);
    f.visibility[0].expires_at = f.policy.now + Duration::seconds(1);
    let c = f.project().unwrap();
    assert!(c.model_input(f.policy.now).is_ok());
    assert_eq!(
        c.model_input(f.policy.now + Duration::seconds(1))
            .unwrap_err(),
        EcosystemSocialRejection::ContextExpired
    );
    assert_eq!(
        c.model_input(f.policy.now - Duration::seconds(1))
            .unwrap_err(),
        EcosystemSocialRejection::ContextExpired
    );
}

#[test]
fn legacy_social_runtime_still_refuses_arbitrary_ecosystem_events() {
    let e = supa_domain::Event {
        id: id(777),
        cycle_id: None,
        kind: "platform".into(),
        code: "agent_created".into(),
        simulated: false,
        created_at: "2026-11-05T12:00:00Z".parse().unwrap(),
    };
    assert!(supa_domain::verified_fact(&e).is_none());
    assert!(
        supa_domain::social_context(Personality::default(), &[e], &[], &[])
            .candidates
            .is_empty()
    );
}

#[test]
fn unchanged_prices_never_claim_movement_and_simulation_labels_cannot_be_removed() {
    let mut f = Fixture::token();
    f.mutate_token(|t| {
        t.metric = PublicTokenMetric::PriceChange {
            before_atoms: 3_000_000,
            after_atoms: 3_000_000,
        }
    });
    let c = f.project().unwrap();
    assert!(c.events()[0].public_fact().contains("unchanged at 3 USDC"));
    assert!(!c.events()[0].public_fact().contains("moved"));
    let mut d = decision(&c, "");
    f.validate(&d).unwrap();
    d.text = d.text.replace("In a simulation, ", "");
    assert_eq!(
        f.validate(&d),
        Err(EcosystemSocialRejection::UnsupportedClaim)
    );
}

#[test]
fn verified_fact_boilerplate_does_not_make_original_commentary_repetitive() {
    let mut f = Fixture::new(vec![created()]);
    let c = f.project().unwrap();
    let d = decision(&c, "Apparently boredom lost another round.");
    f.previous.push(PreviousDraft {
        text: format!(
            "{} Paperwork put on its best hat.",
            c.events()[0].public_fact()
        ),
        event_ids: vec![id(999)],
        topics: vec![],
    });
    f.validate(&d).unwrap();
    f.previous[0].text = d.text.clone();
    assert_eq!(f.validate(&d), Err(EcosystemSocialRejection::Repetition));
}
