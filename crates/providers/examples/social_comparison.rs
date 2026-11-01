//! Offline fixture assessment, not a model-quality benchmark. No provider or database calls.
//! Optionally pass an explicitly reviewed JSON file of saved decisions:
//! {"provenance":"reviewed saved outputs","cases":[{"fixture_index":0,"decision":{...}}]}.
use serde_json::{Value, json};
use std::collections::BTreeSet;
use supa_domain::*;

fn context(index: usize, row: &Value, personality: &Personality) -> SocialContext {
    let event: Event = serde_json::from_value(json!({
        "id":format!("00000000-0000-4000-8000-{:012}",index+1),
        "cycle_id":null,"kind":row["kind"],"code":row["code"],
        "simulated":row["simulated"],"created_at":"2026-10-27T00:00:00Z"
    }))
    .unwrap();
    let memory = Memory {
        id: uuid::Uuid::from_u128(100 + index as u128),
        lesson: "Verified experience informs perspective, not new achievements.".into(),
        kind: MemoryKind::VerifiedObservation,
        source_event_ids: vec![event.id],
        cycle_id: event.id,
        created_at: event.created_at,
    };
    social_context(personality.clone(), &[event], &[memory], &[])
}
fn authored(row: &Value, c: &SocialContext) -> SocialDecision {
    let e = &c.candidates[0];
    SocialDecision {
        choice: SocialChoice::Draft,
        reason: SocialReason::WorthSharing,
        style: Some(serde_json::from_value(row["style"].clone()).unwrap()),
        text: row["text"].as_str().unwrap().into(),
        event_ids: vec![e.event.id],
        memory_ids: vec![],
        claims: vec![SocialClaim {
            event_id: e.event.id,
            fact: e.canonical_fact.clone(),
            wording: e.public_fact.clone(),
        }],
    }
}
fn assess(saved: Option<Value>) -> Value {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/social-comparison.json"
    ))
    .unwrap();
    let config: Value = serde_json::from_str(include_str!("../../../config/supa.json")).unwrap();
    let personality: Personality = serde_json::from_value(config["personality"].clone()).unwrap();
    let mut history = vec![];
    let mut styles = BTreeSet::new();
    let mut openings = BTreeSet::new();
    let mut results = vec![];
    let cases = saved
        .as_ref()
        .map(|s| s["cases"].as_array().expect("cases array required"));
    let count = cases.map_or(6, Vec::len);
    for index in 0..count {
        let fixture_index = cases.map_or(index, |rows| {
            rows[index]["fixture_index"]
                .as_u64()
                .expect("fixture index") as usize
        });
        let row = &fixture["new_examples"].as_array().unwrap()[fixture_index];
        let mut c = context(fixture_index, row, &personality);
        let decision: SocialDecision = cases.map_or_else(
            || authored(row, &c),
            |rows| {
                serde_json::from_value(rows[index]["decision"].clone())
                    .expect("strict decision schema")
            },
        );
        let grounded = decision.validate(&c).is_ok();
        let memory_references = decision.memory_ids.len();
        c.previous_drafts = history.clone();
        let accepted_with_history = decision.validate(&c).is_ok();
        let opening = decision
            .text
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join(" ");
        let unique_opening = openings.insert(opening);
        if let Some(style) = decision.style {
            styles.insert(serde_json::to_string(&style).unwrap());
        }
        let mut bad = decision.clone();
        bad.memory_ids = vec![uuid::Uuid::nil()];
        let foreign_memory_rejected = bad.validate(&c).is_err();
        let mut with_memory = decision.clone();
        if decision.choice == SocialChoice::Draft {
            with_memory.memory_ids = vec![c.memories[0].id];
        }
        let verified_memory_reference_accepted = with_memory.validate(&c).is_ok();
        let empty = social_context(personality.clone(), &[], &[], &[]);
        let empty_abstention_valid = SocialDecision::abstain(SocialReason::NothingNew)
            .validate(&empty)
            .is_ok();
        let mut replay = c.clone();
        replay.previous_drafts.insert(
            0,
            PreviousDraft {
                text: decision.text.clone(),
                event_ids: decision.event_ids.clone(),
                topics: c.candidates.iter().map(|e| e.topic.clone()).collect(),
            },
        );
        let repeated_draft_rejected =
            decision.choice == SocialChoice::Abstain || decision.validate(&replay).is_err();
        let current_event = c.candidates[0].event.clone();
        let cooled = social_context(
            personality.clone(),
            &[current_event],
            &[],
            &replay.previous_drafts,
        );
        let topic_cooldown_enforced =
            decision.choice == SocialChoice::Abstain || cooled.candidates.is_empty();
        results.push(json!({"fixture_index":fixture_index,"event_grounding_and_schema_valid":grounded,
            "accepted_with_history":accepted_with_history,"memory_reference_count":memory_references,
            "foreign_memory_rejected":foreign_memory_rejected,
            "verified_memory_reference_accepted":verified_memory_reference_accepted,
            "repeated_draft_rejected":repeated_draft_rejected,"topic_cooldown_enforced":topic_cooldown_enforced,"unique_four_word_opening":unique_opening,
            "colon_in_opening":decision.text.split_whitespace().take(5).any(|word|word.contains(':')),
            "empty_context_abstention_valid":empty_abstention_valid}));
        if accepted_with_history && decision.choice == SocialChoice::Draft {
            history.insert(
                0,
                PreviousDraft {
                    text: decision.text,
                    event_ids: decision.event_ids,
                    topics: c.candidates.iter().map(|e| e.topic.clone()).collect(),
                },
            );
        }
    }
    json!({"mode":"offline_assessment","provider_calls":0,"model_outputs_generated_here":false,
        "provenance":saved.as_ref().map_or(json!(fixture["new_examples_provenance"]),|s|s["provenance"].clone()),
        "reference_example_used_as_template":false,"reference_example_assessed_as_new_output":false,
        "canonical_personality":"config/supa.json; actual runs use owned persisted personality",
        "style_count":styles.len(),"cases":results,
        "subjective_scores":{"personality_consistency":null,"humor":null,"naturalness":null},
        "human_rubric":"Rate voice fidelity, conversational rhythm, wit where appropriate, sincerity and diversity 1–5; inspect every factual assertion. Silence is valid. Do not reward forced jokes or copied references."})
}
fn main() {
    let mut args = std::env::args().skip(1);
    let saved = args.next().map(|path| {
        serde_json::from_slice(&std::fs::read(path).expect("reviewed input file"))
            .expect("saved JSON decisions")
    });
    assert!(args.next().is_none(), "at most one reviewed input file");
    println!("{}", serde_json::to_string_pretty(&assess(saved)).unwrap());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_authored_fixtures_are_grounded_and_diverse_without_claiming_model_quality() {
        let report = assess(None);
        assert_eq!(report["style_count"], 6);
        assert_eq!(report["provider_calls"], 0);
        for case in report["cases"].as_array().unwrap() {
            for metric in [
                "event_grounding_and_schema_valid",
                "accepted_with_history",
                "foreign_memory_rejected",
                "verified_memory_reference_accepted",
                "unique_four_word_opening",
                "empty_context_abstention_valid",
                "repeated_draft_rejected",
                "topic_cooldown_enforced",
            ] {
                assert_eq!(case[metric], true, "{metric}");
            }
        }
        assert!(report["subjective_scores"]["humor"].is_null());
    }
}
