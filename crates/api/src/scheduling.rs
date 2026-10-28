//! Narrow operator control plane. No arbitrary configuration or funding endpoint.
use crate::{ApiError, ApiResult, AppState};
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::get,
};
use supa_domain::*;
use uuid::Uuid;
pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/agents/{id}/schedule", get(status).post(control))
}
async fn status(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    Ok(Json(
        serde_json::to_value(state.store.schedule_status(id).await?).map_err(|_| Error::Invalid)?,
    ))
}
async fn control(
    State(state): State<AppState>,
    Extension(actor): Extension<crate::Principal>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<ScheduleCommand>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(command) = body.map_err(|_| ApiError(Error::Invalid))?;
    if matches!(
        command.operation,
        ScheduleOperation::Start | ScheduleOperation::Renew
    ) && !actor.role.permits(supa_persistence::Role::Owner)
    {
        return Err(Error::Denied.into());
    }
    let result = state
        .store
        .schedule_control_as(id, &command, Some(actor.id))
        .await;
    match &result {
        Ok(status) => {
            tracing::info!(agent_id=%id,request_id=%command.request_id,operation=?command.operation,revision=status.revision,"schedule_control_saved")
        }
        Err(error) => {
            tracing::warn!(agent_id=%id,request_id=%command.request_id,operation=?command.operation,error_code=%error,"schedule_control_rejected")
        }
    }
    Ok(Json(
        serde_json::to_value(result?).map_err(|_| Error::Invalid)?,
    ))
}
