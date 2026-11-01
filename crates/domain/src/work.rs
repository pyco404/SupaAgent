//! Bounded supplied-input work. Quotes are evidence of supplied text, never external truth.
use crate::*;
use serde_json::{Value, json};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkRequest {
    pub objective: String,
    pub material: String,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub require_external_research: bool,
}
impl WorkRequest {
    pub fn parse(input: &str, kind: ServiceKind) -> Result<Self> {
        clean_text(input, 8000)?;
        let object = serde_json::from_str::<Value>(input).ok();
        let request = if object
            .as_ref()
            .is_some_and(|v| v.get("objective").is_some() && v.get("material").is_some())
        {
            serde_json::from_value(object.unwrap()).map_err(|_| Error::Invalid)?
        } else {
            Self {
                objective: format!("Produce a bounded {kind:?} report from supplied material"),
                material: input.into(),
                requirements: vec![],
                require_external_research: false,
            }
        };
        clean_text(&request.objective, 600)?;
        clean_text(&request.material, 8000)?;
        if request.requirements.len() > 8 {
            return Err(Error::Invalid);
        }
        for requirement in &request.requirements {
            clean_text(requirement, 120)?;
        }
        Ok(request)
    }
    pub fn tools(&self, kind: ServiceKind, cautious: bool) -> Vec<&'static str> {
        let analysis = match kind {
            ServiceKind::Summary | ServiceKind::ResearchReport => "extract_evidence",
            ServiceKind::CodeAnalysis => "review_code",
            ServiceKind::ApiReport => "analyze_data",
            ServiceKind::DelegationSimulation => "decompose_task",
        };
        if cautious {
            vec!["inspect_input", "check_requirements", analysis]
        } else {
            vec!["inspect_input", analysis, "check_requirements"]
        }
    }
}

/// Application-generated findings: a provider cannot supply claims or invent citations.
pub fn analyze_work(request: &WorkRequest, kind: ServiceKind) -> Result<Value> {
    let lines: Vec<_> = request.material.lines().collect();
    let lower = request.material.to_lowercase();
    let coverage: Vec<_> = request
        .requirements
        .iter()
        .map(|r| {
            json!({
                "requirement":r, "supported_by_literal_match":lower.contains(&r.to_lowercase()),
                "supporting_line":lower.find(&r.to_lowercase()).map(|offset|lower[..offset].bytes().filter(|b|*b==b'\n').count()+1)
            })
        })
        .collect();
    let missing = coverage
        .iter()
        .filter(|r| r["supported_by_literal_match"] == false)
        .count();
    let status = if request.require_external_research {
        "abstained"
    } else if missing > 0 {
        "incomplete"
    } else {
        "completed"
    };
    let terms: Vec<_> = request
        .objective
        .split_whitespace()
        .filter(|s| s.len() > 3)
        .map(str::to_lowercase)
        .collect();
    let mut ranked: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.trim().is_empty())
        .collect();
    ranked.sort_by_key(|(i, s)| {
        (
            std::cmp::Reverse(
                terms
                    .iter()
                    .filter(|t| s.to_lowercase().contains(t.as_str()))
                    .count(),
            ),
            *i,
        )
    });
    let evidence: Vec<_> = ranked
        .into_iter()
        .take(8)
        .map(|(i, line)| {
            json!({
                "source":"supplied_material", "line":i+1,
                "quote":line.chars().take(240).collect::<String>(),
                "classification":"unverified_supplied_statement"
            })
        })
        .collect();
    let findings = match kind {
        ServiceKind::Summary | ServiceKind::ResearchReport => {
            json!({"method":"objective_ranked_extracts","extracts":evidence})
        }
        ServiceKind::CodeAnalysis => {
            let warnings: Vec<_> = lines.iter().enumerate().flat_map(|(i,s)| {
                ["TODO", ".unwrap()", ".expect(", "unsafe ", "eval("].into_iter().filter(move |p|s.contains(p)).map(move |p| json!({"line":i+1,"pattern":p,"assessment":"Requires human review; textual match is not proof of a defect."}))
            }).take(16).collect();
            json!({"method":"static_pattern_review","line_count":lines.len(),"findings":warnings,"executed":false,"security_certification":false})
        }
        ServiceKind::ApiReport => {
            let data: Value =
                serde_json::from_str(&request.material).map_err(|_| Error::Invalid)?;
            let object = data.as_object().ok_or(Error::Invalid)?;
            let totals: Vec<_> = object.iter().filter_map(|(key,v)| {
                let array=v.as_array()?;
                if array.is_empty() || array.len()>1000 {return None;}
                let total=array.iter().try_fold(0i64,|sum,n|sum.checked_add(n.as_i64()?));
                Some(json!({"field":key,"integer_sum":total,"exact":total.is_some(),"count":array.len()}))
            }).take(16).collect();
            json!({"method":"checked_integer_analysis","keys":object.keys().take(32).collect::<Vec<_>>(),"totals":totals,"freshness_verified":false})
        }
        ServiceKind::DelegationSimulation => json!({"method":"bounded_task_decomposition","steps":[
            {"task":"Inspect supplied material and identify evidence gaps","completion_rule":"Every requested assertion has a supplied reference"},
            {"task":"Produce the requested artifact using read-only analysis","completion_rule":"No unsupported claims or external mutations"},
            {"task":"Check requirements and report limitations","completion_rule":"Missing evidence produces incompleteness or abstention"}
        ],"proposed_only":true,"child_agents_created":0,"external_actions":0}),
    };
    Ok(
        json!({"contract":"bounded_work_v1","status":status,"objective":request.objective,
            "evidence":evidence,"findings":findings,"requirements":coverage,
            "evaluation":{"missing_requirements":missing,"coverage_method":"literal text presence only; no semantic or truth certification"},
            "limitations":["Supplied statements are not independently verified.","No internet retrieval, code execution, external mutation or paid inference.","Extractive analysis can miss contradictions and semantic errors."],
            "abstention_reason":if request.require_external_research {Some("Independent retrieval is unavailable")}else{None}
        }),
    )
}

/// Only delivery of a complete application report can earn job revenue.
pub fn work_delivered(output: &Value) -> Result<bool> {
    match output.get("work") {
        None => Ok(true), // Historical structured_report_v1 output remains readable.
        Some(work) if work["contract"] == "bounded_work_v1" => match work["status"].as_str() {
            Some("completed") => Ok(true),
            Some("incomplete" | "abstained") => Ok(false),
            _ => Err(Error::Invalid),
        },
        _ => Err(Error::Invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evidence_is_exact_and_objective_ranked() {
        let r=WorkRequest::parse(r#"{"objective":"Analyze storage failures","material":"unrelated\nstorage failures were observed\nmore","requirements":["storage"]}"#,ServiceKind::ResearchReport).unwrap();
        let v = analyze_work(&r, ServiceKind::ResearchReport).unwrap();
        assert_eq!(v["status"], "completed");
        assert_eq!(v["evidence"][0]["line"], 2);
        assert_eq!(
            v["evidence"][0]["quote"],
            r.material.lines().nth(1).unwrap()
        );
    }
    #[test]
    fn unsupported_requirements_and_external_research_do_not_earn_revenue() {
        for (external, status) in [(false, "incomplete"), (true, "abstained")] {
            let r = WorkRequest {
                objective: "Report".into(),
                material: "limited input".into(),
                requirements: vec!["missing evidence".into()],
                require_external_research: external,
            };
            let v = analyze_work(&r, ServiceKind::Summary).unwrap();
            assert_eq!(v["status"], status);
            assert!(!work_delivered(&json!({"work":v})).unwrap());
        }
        assert!(work_delivered(&json!({"work":{"status":"completed"}})).is_err());
    }
    #[test]
    fn data_is_checked_without_floating_point_or_overflow_claims() {
        let r = WorkRequest::parse(
            r#"{"values":[2,3,5],"overflow":[9223372036854775807,1],"fraction":[0.1,0.2]}"#,
            ServiceKind::ApiReport,
        )
        .unwrap();
        let v = analyze_work(&r, ServiceKind::ApiReport).unwrap();
        let totals = v["findings"]["totals"].as_array().unwrap();
        assert_eq!(
            totals.iter().find(|x| x["field"] == "values").unwrap()["integer_sum"],
            10
        );
        for field in ["overflow", "fraction"] {
            assert_eq!(
                totals.iter().find(|x| x["field"] == field).unwrap()["exact"],
                false
            );
        }
    }
    #[test]
    fn static_review_and_planning_never_execute_input() {
        let r = WorkRequest::parse(
            "unsafe { data.unwrap() } // TODO",
            ServiceKind::CodeAnalysis,
        )
        .unwrap();
        let v = analyze_work(&r, ServiceKind::CodeAnalysis).unwrap();
        assert_eq!(v["findings"]["findings"].as_array().unwrap().len(), 3);
        let v = analyze_work(&r, ServiceKind::DelegationSimulation).unwrap();
        assert_eq!(v["findings"]["proposed_only"], true);
    }
    #[test]
    fn request_bounds_and_unrecognized_authority_are_rejected() {
        let legacy = WorkRequest::parse(
            r#"{"objective":"business label","value":2}"#,
            ServiceKind::ApiReport,
        )
        .unwrap();
        assert_eq!(
            analyze_work(&legacy, ServiceKind::ApiReport).unwrap()["status"],
            "completed"
        );
        assert!(
            WorkRequest::parse(
                r#"{"objective":"Review","material":"data","verified":true}"#,
                ServiceKind::Summary
            )
            .is_err()
        );
        let r = WorkRequest {
            objective: "Review".into(),
            material: "data".into(),
            requirements: vec![],
            require_external_research: false,
        };
        assert_ne!(
            r.tools(ServiceKind::Summary, false),
            r.tools(ServiceKind::Summary, true)
        );
        for kind in [
            ServiceKind::Summary,
            ServiceKind::ResearchReport,
            ServiceKind::CodeAnalysis,
            ServiceKind::ApiReport,
            ServiceKind::DelegationSimulation,
        ] {
            assert_eq!(r.tools(kind, false).len(), 3);
        }
    }
}
