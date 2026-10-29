//! No credential presence or lifecycle authorization enables paid inference implicitly.
use super::*;
use supa_runtime::controlled::ControlledInference;
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/agents/{id}/live", get(list).post(run))
        .route("/agents/{id}/live/{session}/approval", post(approve))
}
impl AppState {
    pub fn with_controlled_inference(mut self, service: ControlledInference) -> Self {
        self.controlled = Some(service);
        self
    }
}
async fn list(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    let policy = s.controlled.as_ref().map(|c| c.provider.policy());
    let minimum_reservation_microusd = policy
        .as_ref()
        .map(InferencePolicy::reservation)
        .transpose()?;
    Ok(Json(
        json!({"enabled":s.controlled.is_some(),"mode":"controlled_live","continuous":false,"policy":policy,"minimum_reservation_microusd":minimum_reservation_microusd,"sessions":s.store.live_sessions(id).await?}),
    ))
}
async fn run(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
    Json(input): Json<LiveConsent>,
) -> ApiResult {
    if !actor.role.permits(Role::Owner) {
        return Err(Error::Denied.into());
    }
    s.store.require_access(id, actor.id, Role::Owner).await?;
    let service = s.controlled.as_ref().ok_or(Error::Denied)?;
    let session = service.run(id, actor.id, input).await?;
    Ok(Json(
        serde_json::to_value(session).map_err(|_| Error::Invalid)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Approval {
    approve: bool,
    confirm_review: bool,
}
async fn approve(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path((id, session)): Path<(Uuid, Uuid)>,
    Json(input): Json<Approval>,
) -> ApiResult {
    if !actor.role.permits(Role::Owner) || !input.confirm_review {
        return Err(Error::Denied.into());
    }
    let result = s
        .controlled
        .as_ref()
        .ok_or(Error::Denied)?
        .approve(id, session, actor.id, input.approve)
        .await?;
    Ok(Json(
        serde_json::to_value(result).map_err(|_| Error::Invalid)?,
    ))
}
