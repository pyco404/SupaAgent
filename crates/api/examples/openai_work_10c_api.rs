//! Milestone 10C exact disposable evaluation API. No migrations, recovery writes, worker or startup provider calls.
use axum::{
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use supa_api::{AppState, OperatorCredential, router};
use supa_domain::*;
use supa_persistence::{Role, Store};
use supa_providers::{Offline, OpenAi, controlled::ControlledProvider};
use supa_runtime::{Runtime, controlled::ControlledInference};

const AGENT: &str = "82c254ff-4659-4404-94c4-c497565567e6";
const JOB: &str = "7bf3067c-ea30-4557-8c70-c06aeb1c429b";
const OWNER: &str = "acede749-746b-4541-b178-0085434dcd39";
const BUDGET: &str = "3248e1c5-b306-4806-825a-f9b0a79d8f22";
const NONCE: &str = "41316d3e-40b8-4b7d-8605-83db7aab36b8";
const PROJECT: &str = "proj_npsyg93Ofk7ivKheGa8IDw5q";
const SCOPE: &str = "3c3e5a2f689f6af3347b78c9c19f639842ae6ec1472aea61ace5c853287471ba";
const INPUT: &str = "613e8933d52bf52f46091fb9b67ab05562393145bd5c60e0204c8a61d21fd35e";
const DATABASE: &str = "supa_m10a_8c6356653dc2_test";
const SOCKET: &str = "/tmp/supa-m10a-pg-8c6356653dc2/s";
const ROLE: &str = "supa_openai_10c_eval";

#[derive(Clone)]
struct Gate {
    paid: bool,
    used: Arc<AtomicBool>,
    owner_token: Arc<String>,
}
fn matches_scope(c: &LiveWorkConsent) -> bool {
    c.job_id.to_string() == JOB
        && c.expected_revision == 0
        && c.input_sha256 == INPUT
        && !c.use_memory
        && c.approved_tools == ["inspect_input", "extract_evidence", "check_requirements"]
        && c.consent.request_id.to_string() == NONCE
        && c.consent.model == "gpt-6-luna"
        && c.consent.goal == "Compare the supplied recovery strategies"
        && c.consent.max_requests == 1
        && c.consent.max_input_tokens == 24192
        && c.consent.max_output_tokens == 1024
        && c.consent.max_cost_microusd == 4000
        && c.consent.validate().is_ok()
}
fn token_matches(actual: &str, expected: &str) -> bool {
    actual.len() == expected.len()
        && actual
            .bytes()
            .zip(expected.bytes())
            .fold(0u8, |n, (a, b)| n | (a ^ b))
            == 0
}
async fn scoped(State(g): State<Gate>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let live = format!("/agents/{AGENT}/live-work");
    if request.method() == Method::GET || request.method() == Method::HEAD {
        if [
            "/health",
            "/agents",
            &format!("/agents/{AGENT}"),
            &live,
            &format!("/jobs/{JOB}"),
            &format!("/agents/{AGENT}/live-work/diagnostics/{NONCE}"),
        ]
        .contains(&path)
        {
            return next.run(request).await;
        }
        return StatusCode::FORBIDDEN.into_response();
    }
    if !g.paid || request.method() != Method::POST || path != live {
        return StatusCode::FORBIDDEN.into_response();
    }
    let authenticated = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|v| token_matches(v, &g.owner_token));
    if !authenticated {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let (parts, body) = request.into_parts();
    let Ok(bytes) = to_bytes(body, 16_384).await else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(consent) = serde_json::from_slice::<LiveWorkConsent>(&bytes) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if !matches_scope(&consent) {
        return StatusCode::FORBIDDEN.into_response();
    }
    // Consume before dispatch; ambiguous failures never reopen this process gate.
    if g.used
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return StatusCode::CONFLICT.into_response();
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}
#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("scoped_openai_api_failed_closed error_code={error}");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let paid = match args.as_slice() {
        [] => false,
        [flag, digest] if flag == "--authorize-one-paid-scope" && digest == SCOPE => true,
        _ => return Err(Error::Denied),
    };
    // The launcher supplies an isolated filesystem: no repository, primary data, private files or other sockets.
    if std::path::Path::new("/home/pikoo/Desktop/supa-agent/postgres-data").exists()
        || std::path::Path::new("/home/pikoo/Desktop/supa-agent/.env.eval").exists()
        || std::env::var("SUPA_EVALUATION_SANDBOX").as_deref() != Ok(SCOPE)
    {
        return Err(Error::Denied);
    }
    let password = std::env::var("PGPASSWORD").map_err(|_| Error::Invalid)?;
    if password.len() != 64 || !password.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Denied);
    }
    let url = format!("postgresql://{ROLE}@localhost/{DATABASE}?host={SOCKET}&port=55681");
    let store = Store::connect(&url).await?;
    let identity: (String,String,String,String,bool,bool,bool) = sqlx::query_as("SELECT current_database(), current_setting('data_directory'), system_identifier::text,current_user, r.rolsuper,r.rolcreatedb,r.rolcreaterole FROM pg_control_system(),pg_roles r WHERE r.rolname=current_user")
        .fetch_one(store.pool()).await.map_err(|_|Error::Database)?;
    if identity
        != (
            DATABASE.into(),
            "/tmp/supa-m10a-pg-8c6356653dc2/data".into(),
            "7703596362529484555".into(),
            ROLE.into(),
            false,
            false,
            false,
        )
    {
        return Err(Error::Denied);
    }
    if store.application_epoch().await?.is_some() {
        return Err(Error::Denied);
    }
    let untouched: bool = sqlx::query_scalar("SELECT (SELECT count(*) FROM agents)=1 AND (SELECT count(*) FROM economic_jobs)=1 AND (SELECT count(*) FROM live_budgets)=1 AND (SELECT count(*) FROM live_sessions)=0 AND EXISTS(SELECT 1 FROM live_budgets WHERE id=$1 AND limit_microusd=5000 AND committed_microusd=0) AND EXISTS(SELECT 1 FROM economic_jobs WHERE id=$2 AND state='created' AND received_atoms=0 AND earned_atoms=0 AND expires_at>clock_timestamp()+interval '5 minutes')")
        .bind(BUDGET.parse::<uuid::Uuid>().map_err(|_|Error::Invalid)?).bind(JOB.parse::<uuid::Uuid>().map_err(|_|Error::Invalid)?)
        .fetch_one(store.pool()).await.map_err(|_|Error::Database)?;
    if !untouched {
        return Err(Error::Denied);
    }
    let provider = OpenAi::controlled_from_env()?;
    let policy = provider.policy();
    if provider.project() != PROJECT
        || policy.model != "gpt-6-luna"
        || policy.budget_id.map(|v| v.to_string()).as_deref() != Some(BUDGET)
        || policy.budget_limit_microusd != 5000
        || policy.input_token_bound != 24192
        || policy.max_output_tokens != 1024
        || policy.reservation()? != 3536
    {
        return Err(Error::Denied);
    }
    let actor = OWNER.parse().map_err(|_| Error::Invalid)?;
    let token = std::env::var("SUPA_WORK_OPERATOR_TOKEN").map_err(|_| Error::Invalid)?;
    let public = std::env::var("SUPA_EVALUATION_SERVICE_TOKEN").map_err(|_| Error::Invalid)?;
    if token == public {
        return Err(Error::Denied);
    }
    let deadline: u64 = std::env::var("SUPA_EVALUATION_CREDENTIAL_DEADLINE")
        .map_err(|_| Error::Invalid)?
        .parse()
        .map_err(|_| Error::Invalid)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Invalid)?
        .as_secs();
    if deadline <= now || deadline > now + 3600 {
        return Err(Error::Denied);
    }
    let gate = Gate {
        paid,
        used: Arc::new(AtomicBool::new(false)),
        owner_token: Arc::new(token.clone()),
    };
    let state = AppState::new(
        store.clone(),
        Runtime::new(store.clone(), Arc::new(Offline)),
        public,
    )?
    .with_credentials(vec![OperatorCredential {
        id: actor,
        role: Role::Owner,
        token,
    }])?
    .with_credential_deadlines(BTreeMap::from([(actor, deadline)]))?
    .with_controlled_inference(ControlledInference {
        store,
        provider: Arc::new(provider),
    })
    .with_live_work_evaluation();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:55682")
        .await
        .map_err(|_| Error::Invalid)?;
    println!("scoped_openai_api_ready paid_dispatch_authorized={paid}");
    axum::serve(listener,router(state).layer(middleware::from_fn_with_state(gate,scoped)))
        .with_graceful_shutdown(async move {
            tokio::select! {
                _=tokio::time::sleep(std::time::Duration::from_secs(deadline-now))=>{},
                _=async { if let Ok(mut s)=tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {s.recv().await;} }=>{}
            }
        }).await.map_err(|_|Error::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn consent() -> LiveWorkConsent {
        let now = chrono::Utc::now();
        serde_json::from_value(serde_json::json!({"job_id":JOB,"expected_revision":0,"input_sha256":INPUT,"approved_tools":["inspect_input","extract_evidence","check_requirements"],"use_memory":false,"consent":{"request_id":NONCE,"goal":"Compare the supplied recovery strategies","model":"gpt-6-luna","authorize_paid":true,"consented_at":now,"expires_at":now+chrono::Duration::minutes(2),"max_requests":1,"max_input_tokens":24192,"max_output_tokens":1024,"max_cost_microusd":4000}})).unwrap()
    }
    #[test]
    fn rejects_changed_job_nonce_limits_and_expired_consent() {
        let good = consent();
        assert!(matches_scope(&good));
        let mut c = good.clone();
        c.job_id = uuid::Uuid::new_v4();
        assert!(!matches_scope(&c));
        let mut c = good.clone();
        c.consent.request_id = uuid::Uuid::new_v4();
        assert!(!matches_scope(&c));
        let mut c = good.clone();
        c.consent.max_cost_microusd = 4001;
        assert!(!matches_scope(&c));
        let mut c = good.clone();
        c.use_memory = true;
        assert!(!matches_scope(&c));
        let mut c = good;
        c.consent.expires_at = chrono::Utc::now();
        assert!(!matches_scope(&c));
    }
    fn test_app(paid: bool) -> axum::Router {
        axum::Router::new()
            .route(
                &format!("/agents/{AGENT}/live-work"),
                axum::routing::post(|| async { StatusCode::OK }),
            )
            .layer(middleware::from_fn_with_state(
                Gate {
                    paid,
                    used: Arc::new(AtomicBool::new(false)),
                    owner_token: Arc::new("synthetic-test-token".into()),
                },
                scoped,
            ))
    }
    fn post(c: &LiveWorkConsent, token: &str) -> Request {
        Request::builder()
            .method(Method::POST)
            .uri(format!("/agents/{AGENT}/live-work"))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(serde_json::to_vec(c).unwrap()))
            .unwrap()
    }
    #[tokio::test]
    async fn locked_dispatch_rejects_even_exact_paid_consent_without_a_provider() {
        use tower::ServiceExt;
        let response = test_app(false)
            .oneshot(post(&consent(), "synthetic-test-token"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    #[tokio::test]
    async fn armed_gate_authenticates_binds_scope_and_consumes_only_once() {
        use tower::ServiceExt;
        let app = test_app(true);
        assert_eq!(
            app.clone()
                .oneshot(post(&consent(), "wrong"))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let mut bad = consent();
        bad.consent.request_id = uuid::Uuid::new_v4();
        assert_eq!(
            app.clone()
                .oneshot(post(&bad, "synthetic-test-token"))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            app.clone()
                .oneshot(post(&consent(), "synthetic-test-token"))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            app.oneshot(post(&consent(), "synthetic-test-token"))
                .await
                .unwrap()
                .status(),
            StatusCode::CONFLICT
        );
    }
}
