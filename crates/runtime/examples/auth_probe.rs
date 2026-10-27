//! Explicitly authorized model-metadata GET, with no generation or database access.
mod live_eval_support;
use live_eval_support::{EvalResult, Report, at, environment::load_local_environment};
use serde_json::{Value, json};
use supa_domain::Error;
use supa_providers::OpenAi;
use uuid::Uuid;

fn authorized(args: &[String]) -> supa_domain::Result<bool> {
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["--check"] => Ok(false),
        ["--probe", "--authorize-auth-probe"] => Ok(true),
        _ => Err(Error::Denied),
    }
}
fn main() {
    let result = run();
    if let Err(error) = result {
        let mut safe = error.json();
        safe["generation_requests"] = json!(0);
        eprintln!("{safe}");
        std::process::exit(1);
    }
}
fn run() -> EvalResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let permit = at("authentication_probe_authorization", authorized(&args))?;
    let environment = at("environment_loading", load_local_environment())?;
    let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-6-luna".into());
    at("model_configuration", supa_providers::model_rates(&model))?;
    let mut diagnostic = json!({"environment":environment,"model":model,
        "base_url":"https://api.openai.com/v1","authorization_scheme":"Bearer",
        "explicit_organization_header":false,"explicit_project_header":false,
        "project_verification":"unknown; credential scope must match the intended account project",
        "authorized":permit,"request_attempts_started":0,"generation_requests":0,
        "evaluation_budget_modified":false});
    if !permit {
        println!("{diagnostic}");
        return Ok(());
    }
    if diagnostic["environment"]["key_format"]["basic_format_valid"] != true {
        return at("credential_format", Err(Error::Invalid));
    }
    let runtime = at(
        "runtime_initialization",
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| Error::Invalid),
    )?;
    let probe_id = Uuid::new_v4();
    let mut audit = Report::create(std::path::Path::new("artifacts/auth-probe"), probe_id, None)?;
    // Crash-safe upper bound: a prepared record means dispatch may have started, never retry it automatically.
    audit.write(&json!({"probe_id":probe_id,"status":"prepared","request_attempts_upper_bound":1,
        "generation_requests":0,"inference_reservations_created":0,"evaluation_budget_modified":false}))?;
    let outcome = runtime.block_on(OpenAi::authentication_probe(true));
    let report: Value = match &outcome {
        Ok(value) => value.clone(),
        Err(error) => {
            json!({"result":error.to_string(),"stage":"probe_configuration","request_attempts_started":0,"generation_requests":0,"evaluation_budget_modified":false})
        }
    };
    diagnostic["probe"] = report;
    diagnostic["request_attempts_started"] =
        diagnostic["probe"]["request_attempts_started"].clone();
    diagnostic["probe_id"] = json!(probe_id);
    diagnostic["audit_report"] = json!(audit.path);
    audit.write(&diagnostic)?;
    println!("{diagnostic}");
    at("probe_configuration", outcome)?;
    if diagnostic["probe"]["authentication_verified"] != true {
        std::process::exit(2);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_probe_authorization_can_enable_the_get() {
        for args in [
            vec!["--probe"],
            vec!["--authorize-auth-probe"],
            vec!["--run", "--authorize-paid"],
            vec!["--check", "--probe", "--authorize-auth-probe"],
        ] {
            assert!(authorized(&args.into_iter().map(String::from).collect::<Vec<_>>()).is_err());
        }
        assert!(!authorized(&["--check".into()]).unwrap());
        assert!(authorized(&["--probe".into(), "--authorize-auth-probe".into()]).unwrap());
    }
}
