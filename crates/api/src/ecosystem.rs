//! Authenticated, review-only preparation. No model or publishing dependencies.
use super::*;
use supa_domain::agent_identity::{PurposeRevision, QualificationReview};
use supa_persistence::EcosystemAchievementRequest;
use supa_persistence::EcosystemConsentRequest;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/agents/{id}/purpose", post(purpose))
        .route("/agents/{id}/qualification-review", post(qualification))
        .route("/agents/{id}/achievement-review", post(achievement))
        .route("/agents/{id}/ecosystem-consent", post(consent))
        .route("/agents/{id}/ecosystem-prepare", post(prepare))
}
async fn purpose(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<PurposeRevision>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(body) = body.map_err(|_| Error::Invalid)?;
    // Identity claims in the versioned domain object are binding assertions,
    // never authority. Only the authenticated principal can record its own purpose.
    if id != body.agent_id || actor.id != body.creator_id {
        return Err(Error::Denied.into());
    }
    s.store.record_ecosystem_purpose(actor.id, &body).await?;
    Ok(Json(json!({"recorded":true,"publishing_enabled":false})))
}
async fn consent(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
    body: std::result::Result<
        Json<EcosystemConsentRequest>,
        axum::extract::rejection::JsonRejection,
    >,
) -> ApiResult {
    let Json(body) = body.map_err(|_| Error::Invalid)?;
    let revision = s
        .store
        .record_ecosystem_consent(id, actor.id, &body)
        .await?;
    Ok(Json(
        json!({"revision":revision,"publishing_enabled":false}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Prepare {
    request_id: Uuid,
}
async fn prepare(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<Prepare>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(body) = body.map_err(|_| Error::Invalid)?;
    Ok(Json(
        s.store
            .prepare_ecosystem_social(id, actor.id, body.request_id)
            .await?,
    ))
}

async fn qualification(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<QualificationReview>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(body) = body.map_err(|_| Error::Invalid)?;
    s.store
        .record_ecosystem_qualification(id, actor.id, &body)
        .await?;
    Ok(Json(json!({"recorded":true,"publishing_enabled":false})))
}
async fn achievement(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
    body: std::result::Result<
        Json<EcosystemAchievementRequest>,
        axum::extract::rejection::JsonRejection,
    >,
) -> ApiResult {
    let Json(body) = body.map_err(|_| Error::Invalid)?;
    s.store
        .record_ecosystem_achievement(id, actor.id, &body)
        .await?;
    Ok(Json(json!({"recorded":true,"publishing_enabled":false})))
}
