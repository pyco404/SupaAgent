//! Pure preflight: never constructs a provider, connects to PostgreSQL, or sends HTTP.
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const ENVIRONMENT: &[&str] = &[
    "OPENAI_API_KEY",
    "SUPA_ALLOW_OPENAI",
    "SUPA_LIVE_BUDGET_ID",
    "SUPA_LIVE_BUDGET_MICROUSD",
    "SUPA_EVAL_DATABASE_URL",
    "OPENAI_MODEL",
    "OPENAI_COMPLEX_MODEL",
    "SUPA_PLANNING_MODE",
    "SUPA_MAX_OUTPUT_TOKENS",
    "SUPA_INPUT_RATE_MICROUSD_PER_MILLION",
    "SUPA_CACHED_RATE_MICROUSD_PER_MILLION",
    "SUPA_CACHE_WRITE_RATE_MICROUSD_PER_MILLION",
    "SUPA_OUTPUT_RATE_MICROUSD_PER_MILLION",
];
pub fn readiness(args: &[String], env: &BTreeMap<String, String>) -> Value {
    let get = |name: &str| env.get(name).map(String::as_str).unwrap_or("");
    let has = |name: &str| !get(name).trim().is_empty();
    let ceiling = get("SUPA_LIVE_BUDGET_MICROUSD").parse::<i64>().ok();
    let mut blockers = vec![];
    if !args.iter().any(|s| s == "--run") || !args.iter().any(|s| s == "--authorize-paid") {
        blockers.push("explicit --run --authorize-paid required");
    }
    if get("SUPA_ALLOW_OPENAI") != "true" {
        blockers.push("SUPA_ALLOW_OPENAI must be true");
    }
    if !has("OPENAI_API_KEY") {
        blockers.push("OPENAI_API_KEY missing");
    }
    if !ceiling.is_some_and(|v| (1..=1_000_000).contains(&v)) {
        blockers.push("SUPA_LIVE_BUDGET_MICROUSD must be 1..1000000");
    }
    if Uuid::parse_str(get("SUPA_LIVE_BUDGET_ID")).is_err() {
        blockers.push("stable SUPA_LIVE_BUDGET_ID UUID required");
    }
    if !has("SUPA_EVAL_DATABASE_URL") {
        blockers.push("dedicated local SUPA_EVAL_DATABASE_URL required");
    }
    for key in ["OPENAI_MODEL", "OPENAI_COMPLEX_MODEL"] {
        if has(key) && supa_providers::model_rates(get(key)).is_err() {
            blockers.push("unsupported model configuration");
        }
    }
    if has("SUPA_PLANNING_MODE") && !matches!(get("SUPA_PLANNING_MODE"), "routine" | "complex") {
        blockers.push("SUPA_PLANNING_MODE must be routine or complex");
    }
    if has("SUPA_MAX_OUTPUT_TOKENS")
        && !get("SUPA_MAX_OUTPUT_TOKENS")
            .parse::<u64>()
            .ok()
            .is_some_and(|v| (256..=4096).contains(&v))
    {
        blockers.push("SUPA_MAX_OUTPUT_TOKENS must be 256..4096");
    }
    let routine = if has("OPENAI_MODEL") {
        get("OPENAI_MODEL")
    } else {
        "gpt-6-luna"
    };
    let complex = if has("OPENAI_COMPLEX_MODEL") {
        get("OPENAI_COMPLEX_MODEL")
    } else {
        "gpt-6.1-sol"
    };
    for model in [
        Some(routine),
        if get("SUPA_PLANNING_MODE") == "complex" {
            Some(complex)
        } else {
            None
        },
    ]
    .into_iter()
    .flatten()
    {
        if let Ok(mut rates) = supa_providers::model_rates(model) {
            for (slot, key) in [
                "SUPA_INPUT_RATE_MICROUSD_PER_MILLION",
                "SUPA_CACHED_RATE_MICROUSD_PER_MILLION",
                "SUPA_CACHE_WRITE_RATE_MICROUSD_PER_MILLION",
                "SUPA_OUTPUT_RATE_MICROUSD_PER_MILLION",
            ]
            .iter()
            .enumerate()
            {
                if has(key) {
                    match get(key).parse::<i64>() {
                        Ok(rate) if (rates[slot]..=1_000_000_000).contains(&rate) => {
                            rates[slot] = rate
                        }
                        _ => blockers.push("pricing override below baseline or invalid"),
                    }
                }
            }
            if rates[1] > rates[0] {
                blockers.push("cached rate exceeds ordinary input rate");
            }
        }
    }
    json!({"openai_api_key_detected":if has("OPENAI_API_KEY"){"yes"}else{"no"},"evaluation_database_configured":has("SUPA_EVAL_DATABASE_URL"),"configuration_ready":blockers.iter().all(|b|*b=="explicit --run --authorize-paid required"),"authorization_supplied":args.iter().any(|s|s=="--run")&&args.iter().any(|s|s=="--authorize-paid"),"ready_for_paid":blockers.is_empty(),"blockers":blockers,"routine_model":if has("OPENAI_MODEL")&&supa_providers::model_rates(get("OPENAI_MODEL")).is_err(){"unsupported"}else if has("OPENAI_MODEL"){get("OPENAI_MODEL")}else{"gpt-6-luna"},"complex_model":if has("OPENAI_COMPLEX_MODEL")&&supa_providers::model_rates(get("OPENAI_COMPLEX_MODEL")).is_err(){"unsupported"}else if has("OPENAI_COMPLEX_MODEL"){get("OPENAI_COMPLEX_MODEL")}else{"gpt-6.1-sol"},"account_access_verified":false,"paid_calls_made":0,"database_connectivity_checked":false,"price_basis":"standard short-context, verified 2026-10-27; no external tools; explicit caching without breakpoints"})
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> BTreeMap<String, String> {
        [
            ("OPENAI_API_KEY", "sk-fake-never-display"),
            ("SUPA_ALLOW_OPENAI", "true"),
            (
                "SUPA_LIVE_BUDGET_ID",
                "59bb09d1-a445-45cf-96ec-e70122a6cd01",
            ),
            ("SUPA_LIVE_BUDGET_MICROUSD", "100000"),
            ("SUPA_EVAL_DATABASE_URL", "postgresql://local"),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect()
    }
    #[test]
    fn key_and_ceiling_never_imply_authorization() {
        let c = config();
        let result = readiness(&["--check".into()], &c);
        assert_eq!(result["ready_for_paid"], false);
        assert!(!result.to_string().contains("sk-fake"));
        assert_eq!(readiness(&["--run".into()], &c)["ready_for_paid"], false);
        assert_eq!(
            readiness(&["--run".into(), "--authorize-paid".into()], &c)["ready_for_paid"],
            true
        );
    }
    #[test]
    fn every_required_safeguard_fails_closed() {
        let args = vec!["--run".into(), "--authorize-paid".into()];
        for k in [
            "OPENAI_API_KEY",
            "SUPA_ALLOW_OPENAI",
            "SUPA_LIVE_BUDGET_ID",
            "SUPA_LIVE_BUDGET_MICROUSD",
            "SUPA_EVAL_DATABASE_URL",
        ] {
            let mut c = config();
            c.remove(k);
            assert_eq!(readiness(&args, &c)["ready_for_paid"], false);
        }
        for value in ["0", "-1", "1000001", "invalid"] {
            let mut c = config();
            c.insert("SUPA_LIVE_BUDGET_MICROUSD".into(), value.into());
            assert_eq!(readiness(&args, &c)["ready_for_paid"], false);
        }
    }
}
