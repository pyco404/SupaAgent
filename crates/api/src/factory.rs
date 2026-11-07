//! Dedicated, default-off synthetic journey; existing execution routes unchanged.
use super::*;
use supa_persistence::FactoryRequest;
impl AppState {
    pub fn with_synthetic_factory(mut self) -> Result<Self> {
        if self.live_work_enabled
            || self.controlled.is_some()
            || self.wallets.is_some()
            || self.store.operational_context().is_some()
        {
            return Err(Error::Denied);
        }
        self.synthetic_factory_enabled = true;
        Ok(self)
    }
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/factory/agents", get(list).post(create))
        .route("/factory/agents/{id}", get(detail))
}
fn allowed(s: &AppState, actor: Principal) -> Result<()> {
    if !s.synthetic_factory_enabled || actor.role != Role::Owner {
        return Err(Error::Denied);
    }
    Ok(())
}
async fn list(State(s): State<AppState>, Extension(actor): Extension<Principal>) -> ApiResult {
    allowed(&s, actor)?;
    Ok(Json(
        json!({"agents":s.store.factory_owned(actor.id).await?}),
    ))
}
async fn detail(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
) -> ApiResult {
    allowed(&s, actor)?;
    Ok(Json(s.store.factory_owned_agent(actor.id, id).await?))
}
async fn create(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    body: std::result::Result<Json<FactoryRequest>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    allowed(&s, actor)?;
    let Json(body) = body.map_err(|_| Error::Invalid)?;
    if body.avatar_upload_id.is_some() && !s.agent_avatars_enabled {
        return Err(Error::Denied.into());
    }
    Ok(Json(s.store.factory_create(actor.id, &body).await?))
}
