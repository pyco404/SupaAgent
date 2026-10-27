mod wallet;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeSet, net::SocketAddr, sync::Arc};
use supa_domain::*;
use supa_persistence::Store;
use supa_runtime::Runtime;
use tokio::sync::Semaphore;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub runtime: Runtime,
    token: Arc<String>,
    pub gate: Arc<Semaphore>,
    pub wallets: Option<supa_runtime::wallet::WalletService>,
}
impl AppState {
    pub fn new(store: Store, runtime: Runtime, token: String) -> Result<Self> {
        if token.len() < 32
            || token.len() > 256
            || !token.is_ascii()
            || token.chars().any(char::is_whitespace)
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            store,
            runtime,
            token: Arc::new(token),
            gate: Arc::new(Semaphore::new(1)),
            wallets: None,
        })
    }
}
pub fn local_address(value: &str) -> Result<SocketAddr> {
    let address: SocketAddr = value.parse().map_err(|_| Error::Invalid)?;
    if !address.ip().is_loopback() {
        return Err(Error::Denied);
    }
    Ok(address)
}
fn token_matches(actual: &str, expected: &str) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    actual
        .bytes()
        .zip(expected.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}
async fn auth(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let valid = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .is_some_and(|s| token_matches(s, &state.token));
    if !valid {
        return ApiError(Error::Unauthorized).into_response();
    }
    next.run(request).await
}
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(wallet::routes())
        .route("/health", get(health))
        .route("/agents", get(list).post(create))
        .route("/agents/{id}", get(status))
        .route("/agents/{id}/lifecycle", post(lifecycle))
        .route("/agents/{id}/pause", post(pause))
        .route("/agents/{id}/resume", post(resume))
        .route("/agents/{id}/emergency-stop", post(emergency_stop))
        .route("/agents/{id}/inference", get(inference))
        .route("/agents/{id}/goal-evaluations", get(evaluations))
        .route("/agents/{id}/goals/{goal}/priority", post(priority))
        .route("/agents/{id}/goals", get(goals).post(add_goal))
        .route("/agents/{id}/goals/{goal}/complete", post(complete_goal))
        .route("/agents/{id}/memory", get(memory))
        .route("/agents/{id}/activity", get(activity))
        .route("/agents/{id}/financials", get(financials))
        .route("/agents/{id}/simulated-funding", post(fund))
        .route("/agents/{id}/cycles", get(cycles).post(cycle))
        .route("/agents/{id}/actions", get(actions))
        .route("/agents/{id}/drafts", get(drafts))
        .route("/agents/{id}/social", get(social))
        .route("/agents/{id}/transitions", get(transitions))
        .route("/agents/{id}/simulated-ledger", get(ledger))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state)
}
pub struct ApiError(Error);
impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::Denied | Error::FinancialPolicy => StatusCode::FORBIDDEN,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Invalid | Error::InvalidPayment => StatusCode::BAD_REQUEST,
            Error::Unconfirmed => StatusCode::CONFLICT,
            Error::Busy
            | Error::Inactive
            | Error::InvalidTransition
            | Error::Stale
            | Error::InsufficientBalance
            | Error::LiveBudget
            | Error::ResourceBudget
            | Error::SocialRepetition => StatusCode::CONFLICT,
            Error::Timeout => StatusCode::GATEWAY_TIMEOUT,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        };
        (status, Json(json!({"error":self.0.to_string()}))).into_response()
    }
}
type ApiResult = std::result::Result<Json<Value>, ApiError>;
async fn health(State(s): State<AppState>) -> ApiResult {
    s.store.ping().await?;
    Ok(Json(
        json!({"status":"ok","economic_mode":"simulation","publishing_enabled":false}),
    ))
}
async fn list(State(s): State<AppState>) -> ApiResult {
    Ok(Json(json!(s.store.list().await?)))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    name: String,
    #[serde(default)]
    personality: Personality,
    #[serde(default)]
    config: CycleConfig,
    #[serde(default)]
    capabilities: BTreeSet<Capability>,
}
async fn create(
    State(s): State<AppState>,
    body: std::result::Result<Json<Create>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(b) = body.map_err(|_| Error::Invalid)?;
    let a = Agent::new(b.name, b.personality, b.config, b.capabilities)?;
    s.store.create(&a).await?;
    Ok(Json(json!(a)))
}
async fn status(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    Ok(Json(
        json!({"agent":s.store.get(id).await?,"financials":s.store.financials(id).await?}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Transition {
    state: Lifecycle,
    reason: String,
}
async fn lifecycle(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<Transition>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(b) = body.map_err(|_| Error::Invalid)?;
    s.store.transition(id, b.state, &b.reason, true).await?;
    Ok(Json(json!({"agent":s.store.get(id).await?})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    reason: String,
}
macro_rules! lifecycle_control {
    ($name:ident,$next:expr)=>{
        async fn $name(State(s):State<AppState>,Path(id):Path<Uuid>,body:std::result::Result<Json<Control>,axum::extract::rejection::JsonRejection>)->ApiResult {
            let Json(b)=body.map_err(|_|Error::Invalid)?;
            s.store.transition(id,$next,&b.reason,true).await?;
            Ok(Json(json!({"agent":s.store.get(id).await?})))
        }
    };
}
lifecycle_control!(pause, Lifecycle::Paused);
lifecycle_control!(resume, Lifecycle::Active);
async fn emergency_stop(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<Control>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(b) = body.map_err(|_| Error::Invalid)?;
    s.store.emergency_stop(id, &b.reason).await?;
    Ok(Json(json!({"agent":s.store.get(id).await?})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Priority {
    priority: u8,
}
async fn priority(
    State(s): State<AppState>,
    Path((id, goal)): Path<(Uuid, Uuid)>,
    body: std::result::Result<Json<Priority>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(b) = body.map_err(|_| Error::Invalid)?;
    s.store.set_goal_priority(id, goal, b.priority).await?;
    Ok(Json(json!({"updated":true})))
}
async fn goals(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    Ok(Json(json!(s.store.get(id).await?.goals)))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoalInput {
    description: String,
}
async fn add_goal(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<GoalInput>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(b) = body.map_err(|_| Error::Invalid)?;
    Ok(Json(json!(s.store.add_goal(id, b.description).await?)))
}
async fn complete_goal(
    State(s): State<AppState>,
    Path((id, goal)): Path<(Uuid, Uuid)>,
) -> ApiResult {
    s.store.complete_goal(id, goal).await?;
    Ok(Json(json!({"completed":true})))
}
async fn memory(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    Ok(Json(json!(s.store.get(id).await?.memory)))
}
async fn activity(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    s.store.get(id).await?;
    Ok(Json(json!(s.store.events(id).await?)))
}
async fn financials(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    Ok(Json(json!(s.store.financials(id).await?)))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Funding {
    kind: LedgerKind,
    amount_microusd: i64,
    idempotency_key: Uuid,
    simulated: bool,
}
async fn fund(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<Funding>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(b) = body.map_err(|_| Error::Invalid)?;
    if !b.simulated {
        return Err(Error::Invalid.into());
    }
    s.store
        .credit(id, b.kind, b.amount_microusd, b.idempotency_key)
        .await?;
    Ok(Json(json!(s.store.financials(id).await?)))
}
async fn cycle(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    let _permit = s.gate.try_acquire().map_err(|_| Error::Busy)?;
    let cycle = s.runtime.cycle(id, false).await?;
    Ok(Json(json!({"cycle_id":cycle})))
}
macro_rules! history_handler {
    ($name:ident,$resource:literal) => {
        async fn $name(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
            Ok(Json(json!(s.store.history(id, $resource).await?)))
        }
    };
}
history_handler!(inference, "inference");
history_handler!(evaluations, "evaluations");
history_handler!(cycles, "cycles");
history_handler!(actions, "actions");
history_handler!(drafts, "drafts");
history_handler!(social, "social");
history_handler!(transitions, "transitions");
history_handler!(ledger, "ledger");

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn never_binds_publicly() {
        assert!(local_address("0.0.0.0:8080").is_err());
        assert!(local_address("[::]:8080").is_err());
        assert!(local_address("127.0.0.1:8080").is_ok());
        assert!(local_address("[::1]:8080").is_ok());
    }
    #[test]
    fn token_comparison() {
        assert!(token_matches("abc", "abc"));
        assert!(!token_matches("abc", "abd"));
        assert!(!token_matches("ab", "abc"));
    }
}
