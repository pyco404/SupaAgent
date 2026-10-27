//! Event-grounded social decisions. Fact fragments are evidence labels, never joke templates.
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostStyle {
    Humor,
    Observation,
    Achievement,
    Failure,
    Question,
    Reflection,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialChoice {
    Draft,
    Abstain,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialReason {
    WorthSharing,
    NothingNew,
    TooRepetitive,
    Conserve,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialClaim {
    pub event_id: Uuid,
    pub fact: String,
    pub wording: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialDecision {
    pub choice: SocialChoice,
    pub reason: SocialReason,
    pub style: Option<PostStyle>,
    pub text: String,
    pub event_ids: Vec<Uuid>,
    pub memory_ids: Vec<Uuid>,
    pub claims: Vec<SocialClaim>,
}
impl SocialDecision {
    pub fn abstain(reason: SocialReason) -> Self {
        Self {
            choice: SocialChoice::Abstain,
            reason,
            style: None,
            text: String::new(),
            event_ids: vec![],
            memory_ids: vec![],
            claims: vec![],
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialCandidate {
    pub event: Event,
    pub topic: String,
    pub canonical_fact: String,
    pub public_fact: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousDraft {
    pub text: String,
    pub event_ids: Vec<Uuid>,
    pub topics: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialContext {
    pub personality: Personality,
    pub candidates: Vec<SocialCandidate>,
    pub memories: Vec<Memory>,
    pub previous_drafts: Vec<PreviousDraft>,
}
/// Safe noun phrases preserve factual meaning without forcing a log sentence into public text.
pub fn social_fact(event: &Event) -> Option<(&'static str, &'static str, u8)> {
    match (event.kind.as_str(), event.code.as_str()) {
        ("simulated_ledger", "seed") if event.simulated => {
            Some(("treasury", "a simulated starting treasury", 2))
        }
        ("simulated_ledger", "revenue") if event.simulated => {
            Some(("revenue", "simulated revenue", 4))
        }
        ("simulated_ledger", "inference") if event.simulated => {
            Some(("inference_cost", "simulated inference costs", 1))
        }
        ("simulated_ledger", "operating") if event.simulated => {
            Some(("operating_cost", "simulated operating costs", 1))
        }
        ("cycle", "completed") => Some(("execution", "a completed reasoning cycle", 3)),
        ("cycle", "interrupted") => Some(("failure", "an interrupted reasoning cycle", 5)),
        (
            "cycle",
            "provider_failed"
            | "provider_timeout"
            | "provider_transport_failed"
            | "invalid_input"
            | "insufficient_simulated_balance"
            | "live_budget_exhausted"
            | "resource_budget_exhausted"
            | "provider_http_400"
            | "provider_http_401"
            | "provider_http_403"
            | "provider_http_429"
            | "provider_http_500"
            | "provider_http_502"
            | "provider_http_503"
            | "provider_http_504",
        ) => Some(("failure", "a failed reasoning cycle", 5)),
        ("goal", "evaluated") => Some(("goal_review", "a goal progress review", 3)),
        ("goal", "added") => Some(("goal", "an operator-assigned goal", 2)),
        _ => None,
    }
}
pub fn social_context(
    personality: Personality,
    events: &[Event],
    memories: &[Memory],
    previous: &[PreviousDraft],
) -> SocialContext {
    let previous_drafts: Vec<_> = previous.iter().take(20).cloned().collect();
    let used: BTreeSet<_> = previous_drafts
        .iter()
        .flat_map(|p| p.event_ids.iter().copied())
        .collect();
    let cooling: BTreeSet<_> = previous_drafts
        .iter()
        .take(3)
        .flat_map(|p| p.topics.iter().cloned())
        .collect();
    let mut ranked: Vec<_> = events
        .iter()
        .filter_map(|e| {
            let (topic, wording, weight) = social_fact(e)?;
            let fact = verified_fact(e)?;
            if used.contains(&e.id) || cooling.contains(topic) {
                return None;
            }
            Some((
                weight,
                e.created_at,
                e.id,
                SocialCandidate {
                    event: e.clone(),
                    topic: topic.into(),
                    canonical_fact: fact.into(),
                    public_fact: wording.into(),
                },
            ))
        })
        .collect();
    ranked.sort_by_key(|(weight, time, id, _)| std::cmp::Reverse((*weight, *time, *id)));
    let mut topics = BTreeSet::new();
    let candidates: Vec<_> = ranked
        .into_iter()
        .filter_map(|(_, _, _, c)| topics.insert(c.topic.clone()).then_some(c))
        .take(8)
        .collect();
    let selected_ids: BTreeSet<_> = candidates.iter().map(|c| c.event.id).collect();
    let query: Vec<_> = candidates.iter().flat_map(|c| c.topic.split('_')).collect();
    // Verified memories may be old. Rank relevance before recency; model hypotheses never become facts.
    let mut relevant: Vec<_> = memories
        .iter()
        .filter(|m| {
            m.kind == MemoryKind::VerifiedObservation
                && !m.source_event_ids.is_empty()
                && m.source_event_ids.iter().all(|id| {
                    events
                        .iter()
                        .any(|e| e.id == *id && verified_fact(e).is_some())
                })
        })
        .map(|m| {
            let score = m
                .source_event_ids
                .iter()
                .filter(|id| selected_ids.contains(id))
                .count()
                * 10
                + query
                    .iter()
                    .filter(|t| m.lesson.to_lowercase().contains(**t))
                    .count();
            (score, m.created_at, m)
        })
        .filter(|(score, _, _)| *score > 0)
        .collect();
    relevant.sort_by_key(|(score, time, _)| std::cmp::Reverse((*score, *time)));
    SocialContext {
        personality,
        candidates,
        memories: relevant
            .into_iter()
            .take(6)
            .map(|(_, _, m)| m.clone())
            .collect(),
        previous_drafts,
    }
}
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect()
}
fn joke_words(text: &str) -> BTreeSet<String> {
    let mut s = text.to_lowercase();
    for phrase in [
        "a simulated starting treasury",
        "simulated revenue",
        "simulated inference costs",
        "simulated operating costs",
        "a completed reasoning cycle",
        "an interrupted reasoning cycle",
        "a failed reasoning cycle",
        "a goal progress review",
        "an operator-assigned goal",
    ] {
        s = s.replace(phrase, "");
    }
    words(&s).into_iter().filter(|w| w.len() > 2).collect()
}
pub fn repeated_social_text(text: &str, previous: &[PreviousDraft]) -> bool {
    let tokens = words(text);
    let joke = joke_words(text);
    previous.iter().any(|p| {
        let old = words(&p.text);
        let other = joke_words(&p.text);
        tokens == old || (tokens.len() >= 4 && old.len() >= 4 && tokens[..4] == old[..4]) || {
            let overlap = joke.intersection(&other).count();
            let union = joke.union(&other).count();
            (union > 0 && overlap * 100 >= union * 65)
                || (joke.len().min(other.len()) >= 5
                    && overlap * 100 >= joke.len().min(other.len()) * 85)
        }
    })
}
impl SocialDecision {
    pub fn validate(&self, context: &SocialContext) -> Result<()> {
        if self.choice == SocialChoice::Abstain {
            return if self.reason != SocialReason::WorthSharing
                && self.text.is_empty()
                && self.style.is_none()
                && self.event_ids.is_empty()
                && self.memory_ids.is_empty()
                && self.claims.is_empty()
            {
                Ok(())
            } else {
                Err(Error::Invalid)
            };
        }
        clean_text(&self.text, 1120)?;
        if self.reason != SocialReason::WorthSharing
            || self.style.is_none()
            || self.text.chars().count() > 280
            || self.text.chars().any(char::is_numeric)
            || self.event_ids.is_empty()
            || self.event_ids.len() > 2
            || self.claims.len() != self.event_ids.len()
            || self.memory_ids.len() > 3
            || self.event_ids.iter().collect::<BTreeSet<_>>().len() != self.event_ids.len()
        {
            return Err(Error::Invalid);
        }
        let lower = self.text.to_lowercase();
        if [
            "human review",
            "draft for",
            "requires review",
            "canonical_fact",
            "event_id",
            "a simulated inference expense was recorded",
            "a simulated operating expense was recorded",
        ]
        .iter()
        .any(|s| lower.contains(s))
        {
            return Err(Error::Invalid);
        }
        // Defense in depth, not a semantic proof: all drafts still require human review.
        let tokens = words(&self.text);
        if [
            "payments",
            "payment",
            "paid",
            "earned",
            "earnings",
            "payout",
            "trade",
            "created",
            "spawned",
            "cloned",
            "milestone",
            "milestones",
            "traded",
            "trades",
            "trading",
            "transferred",
            "transfer",
            "mainnet",
            "published",
            "deployed",
            "wallet",
            "profitable",
            "profit",
            "shipped",
            "unlocked",
            "launched",
        ]
        .iter()
        .any(|s| tokens.iter().any(|t| t == s))
        {
            return Err(Error::Invalid);
        }
        let normalized = format!(" {} ", tokens.join(" "));
        if [
            " i can ",
            " we can ",
            " supa can ",
            " i am able ",
            " i have built ",
            " i built ",
            " i now support ",
            " my new ability ",
            " real money ",
            " real revenue ",
            " real funds ",
            " real income ",
        ]
        .iter()
        .any(|phrase| normalized.contains(phrase))
        {
            return Err(Error::Invalid);
        }
        for id in &self.event_ids {
            let candidate = context
                .candidates
                .iter()
                .find(|c| c.event.id == *id)
                .ok_or(Error::Invalid)?;
            let claim = self
                .claims
                .iter()
                .find(|c| c.event_id == *id)
                .ok_or(Error::Invalid)?;
            if verified_fact(&candidate.event) != Some(claim.fact.as_str())
                || social_fact(&candidate.event).map(|(_, wording, _)| wording)
                    != Some(claim.wording.as_str())
                || !lower.contains(&claim.wording.to_lowercase())
                || (candidate.event.simulated && !lower.contains("simulated"))
            {
                return Err(Error::Invalid);
            }
            if self.style == Some(PostStyle::Achievement) && candidate.topic != "execution" {
                return Err(Error::Invalid);
            }
            if self.style == Some(PostStyle::Failure) && candidate.topic != "failure" {
                return Err(Error::Invalid);
            }
            if context
                .previous_drafts
                .iter()
                .take(3)
                .any(|p| p.topics.contains(&candidate.topic))
                || context
                    .previous_drafts
                    .iter()
                    .any(|p| p.event_ids.contains(id))
            {
                return Err(Error::SocialRepetition);
            }
        }
        for id in &self.memory_ids {
            if !context
                .memories
                .iter()
                .any(|m| m.id == *id && m.kind == MemoryKind::VerifiedObservation)
            {
                return Err(Error::Invalid);
            }
        }
        if repeated_social_text(&self.text, &context.previous_drafts) {
            return Err(Error::SocialRepetition);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(kind: &str, code: &str, simulated: bool) -> Event {
        Event {
            id: Uuid::new_v4(),
            cycle_id: None,
            kind: kind.into(),
            code: code.into(),
            simulated,
            created_at: Utc::now(),
        }
    }
    fn context(e: &Event) -> SocialContext {
        social_context(Personality::default(), std::slice::from_ref(e), &[], &[])
    }
    fn draft(c: &SocialContext, text: &str) -> SocialDecision {
        let e = &c.candidates[0];
        SocialDecision {
            choice: SocialChoice::Draft,
            reason: SocialReason::WorthSharing,
            style: Some(PostStyle::Humor),
            text: text.into(),
            event_ids: vec![e.event.id],
            memory_ids: vec![],
            claims: vec![SocialClaim {
                event_id: e.event.id,
                fact: e.canonical_fact.clone(),
                wording: e.public_fact.clone(),
            }],
        }
    }
    #[test]
    fn factual_fragments_allow_natural_openings_without_review_metadata() {
        let c = context(&event("simulated_ledger", "inference", true));
        let d = draft(
            &c,
            "Ambition has expensive taste: simulated inference costs. I would like to speak to ambition's manager.",
        );
        d.validate(&c).unwrap();
        assert!(!d.text.contains("was recorded"));
        for text in [
            "Draft for human review: simulated inference costs.",
            "A simulated inference expense was recorded.",
            "I paid for simulated inference costs.",
            "Simulated inference costs: I traded on Mainnet.",
            "Simulated inference costs exceeded 5 dollars.",
            "Simulated inference costs. I can create agents now.",
            "Simulated inference costs. I trade on Solana.",
            "Simulated inference costs. I earned real money.",
        ] {
            assert!(draft(&c, text).validate(&c).is_err());
        }
    }
    #[test]
    fn all_six_styles_are_supported_but_achievement_and_failure_need_evidence() {
        let c = context(&event("cycle", "completed", false));
        for style in [
            PostStyle::Humor,
            PostStyle::Observation,
            PostStyle::Achievement,
            PostStyle::Question,
            PostStyle::Reflection,
        ] {
            let mut d = draft(
                &c,
                "A completed reasoning cycle. Would restraint be an ambitious next step?",
            );
            d.style = Some(style);
            d.validate(&c).unwrap();
        }
        let mut d = draft(
            &c,
            "A completed reasoning cycle. Would restraint be an ambitious next step?",
        );
        d.style = Some(PostStyle::Failure);
        assert!(d.validate(&c).is_err());
        let c = context(&event("cycle", "provider_failed", false));
        let mut d = draft(
            &c,
            "A failed reasoning cycle. Confidence would like to revise its opening statement.",
        );
        d.style = Some(PostStyle::Failure);
        d.validate(&c).unwrap();
    }
    #[test]
    fn unknown_events_and_unlabeled_finances_cannot_become_facts() {
        assert!(
            context(&event("payment", "received", false))
                .candidates
                .is_empty()
        );
        assert!(
            context(&event("simulated_ledger", "revenue", false))
                .candidates
                .is_empty()
        );
        let c = context(&event("simulated_ledger", "revenue", true));
        let mut d = draft(&c, "Revenue. My imaginary empire is looking up.");
        assert!(d.validate(&c).is_err());
        d.text = "Simulated revenue. My imaginary empire is looking up.".into();
        d.validate(&c).unwrap();
        d.claims[0].fact = "A real payment arrived.".into();
        assert!(d.validate(&c).is_err());
    }
    #[test]
    fn same_text_openings_jokes_topics_and_event_reuse_are_rejected() {
        let e = event("simulated_ledger", "inference", true);
        let c = context(&e);
        let text = "Ambition has expensive taste: simulated inference costs. I would like to speak to ambition's manager.";
        for old in [
            text.to_uppercase(),
            "Ambition has expensive taste, but a completely different ending.".to_string(),
            "Simulated operating costs: expensive ambition has taste; I would like to speak to ambition's manager.".to_string(),
        ] {
            let mut c = c.clone();
            c.previous_drafts.push(PreviousDraft {
                text: old,
                event_ids: vec![],
                topics: vec![],
            });
            assert!(matches!(
                draft(&c, text).validate(&c),
                Err(Error::SocialRepetition)
            ));
        }
        for previous in [
            PreviousDraft {
                text: "Unrelated wording".into(),
                event_ids: vec![e.id],
                topics: vec![],
            },
            PreviousDraft {
                text: "Different joke".into(),
                event_ids: vec![],
                topics: vec!["inference_cost".into()],
            },
        ] {
            let mut c = c.clone();
            c.previous_drafts.push(previous);
            assert!(matches!(
                draft(&c, text).validate(&c),
                Err(Error::SocialRepetition)
            ));
        }
    }
    #[test]
    fn candidates_are_diverse_ranked_and_fully_cooled_topics_allow_silence() {
        let failure = event("cycle", "provider_timeout", false);
        let cost = event("simulated_ledger", "inference", true);
        let mut events = vec![failure.clone(), cost];
        for _ in 0..30 {
            events.push(event("simulated_ledger", "inference", true));
        }
        let c = social_context(Personality::default(), &events, &[], &[]);
        assert_eq!(c.candidates.len(), 2);
        assert_eq!(c.candidates[0].event.id, failure.id);
        let p = PreviousDraft {
            text: "past".into(),
            event_ids: vec![],
            topics: vec!["failure".into(), "inference_cost".into()],
        };
        assert!(
            social_context(Personality::default(), &events, &[], &[p])
                .candidates
                .is_empty()
        );
        SocialDecision::abstain(SocialReason::NothingNew)
            .validate(&c)
            .unwrap();
        let mut bad = SocialDecision::abstain(SocialReason::NothingNew);
        bad.text = "Sneak in a post".into();
        assert!(bad.validate(&c).is_err());
    }
    #[test]
    fn old_relevant_verified_memories_are_retrieved_hypotheses_and_unowned_sources_excluded() {
        let e = event("cycle", "completed", false);
        let old = Memory {
            id: Uuid::new_v4(),
            lesson: "Execution restraint matters.".into(),
            kind: MemoryKind::VerifiedObservation,
            source_event_ids: vec![e.id],
            cycle_id: Uuid::new_v4(),
            created_at: Utc::now() - chrono::Duration::days(365),
        };
        let mut hypothesis = old.clone();
        hypothesis.id = Uuid::new_v4();
        hypothesis.kind = MemoryKind::Hypothesis;
        let mut foreign = old.clone();
        foreign.source_event_ids = vec![Uuid::new_v4()];
        let c = social_context(
            Personality::default(),
            &[e],
            &[old.clone(), hypothesis.clone(), foreign],
            &[],
        );
        assert_eq!(c.memories.len(), 1);
        assert_eq!(c.memories[0].id, old.id);
        let mut d = draft(
            &c,
            "A completed reasoning cycle. Patience seems suspiciously useful.",
        );
        d.memory_ids = vec![hypothesis.id];
        assert!(d.validate(&c).is_err());
        d.memory_ids = vec![old.id];
        d.validate(&c).unwrap();
    }
}

#[cfg(test)]
mod comparison_tests {
    use super::*;
    #[test]
    fn authored_new_examples_cover_all_styles_without_log_openings_or_review_leakage() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/social-comparison.json")).unwrap();
        let mut history = vec![];
        for row in fixture["new_examples"].as_array().unwrap() {
            let e = Event {
                id: Uuid::new_v4(),
                cycle_id: None,
                kind: row["kind"].as_str().unwrap().into(),
                code: row["code"].as_str().unwrap().into(),
                simulated: row["simulated"].as_bool().unwrap(),
                created_at: Utc::now(),
            };
            let c = social_context(Personality::default(), &[e], &[], &history);
            assert_eq!(c.candidates.len(), 1);
            let candidate = &c.candidates[0];
            let d = SocialDecision {
                choice: SocialChoice::Draft,
                reason: SocialReason::WorthSharing,
                style: Some(serde_json::from_value(row["style"].clone()).unwrap()),
                text: row["text"].as_str().unwrap().into(),
                event_ids: vec![candidate.event.id],
                memory_ids: vec![],
                claims: vec![SocialClaim {
                    event_id: candidate.event.id,
                    fact: candidate.canonical_fact.clone(),
                    wording: candidate.public_fact.clone(),
                }],
            };
            d.validate(&c).unwrap();
            history.insert(
                0,
                PreviousDraft {
                    text: d.text,
                    event_ids: d.event_ids,
                    topics: vec![candidate.topic.clone()],
                },
            );
        }
        assert_eq!(history.len(), 6);
    }
}
