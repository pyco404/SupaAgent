use super::*;
use supa_runtime::wallet::WalletService;
fn service(s: &AppState) -> Result<&WalletService> {
    s.wallets.as_ref().ok_or(Error::Denied)
}
async fn owner(s: &AppState, id: Uuid) -> Result<Uuid> {
    let actor = service(s)?.operator();
    if s.store.wallet(id).await?.owner_id != actor {
        return Err(Error::Denied);
    }
    Ok(actor)
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/agents/{id}/wallet", get(identity).post(create))
        .route("/agents/{id}/wallet/recover", post(recover))
        .route("/agents/{id}/wallet/sync", post(sync))
        .route("/agents/{id}/wallet/balances", get(balances))
        .route("/agents/{id}/wallet/payments", get(payments).post(verify))
        .route("/agents/{id}/wallet/payment-checks", get(checks))
        .route("/agents/{id}/wallet/treasury", get(treasury))
        .route("/agents/{id}/wallet/policy", get(policy).post(set_policy))
        .route(
            "/agents/{id}/wallet/reservations",
            get(reservations).post(reserve),
        )
        .route(
            "/agents/{id}/wallet/reservations/{key}/release",
            post(release),
        )
        .route("/agents/{id}/wallet/audit", get(audit))
        .route("/agents/{id}/wallet/sign", post(sign))
}
async fn create(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    let service = service(&s)?;
    Ok(Json(json!(service.create(id, service.operator()).await?)))
}
async fn identity(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    owner(&s, id).await?;
    Ok(Json(json!(s.store.wallet(id).await?)))
}
async fn recover(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    let actor = owner(&s, id).await?;
    Ok(Json(
        json!({"address":service(&s)?.recover(id,actor).await?,"private_material_exported":false}),
    ))
}
async fn sync(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    let actor = owner(&s, id).await?;
    Ok(Json(service(&s)?.sync(id, actor).await?))
}
async fn balances(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    owner(&s, id).await?;
    Ok(Json(json!(s.store.wallet_balance(id).await?)))
}
async fn treasury(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    owner(&s, id).await?;
    Ok(Json(json!(s.store.treasury(id).await?)))
}
async fn policy(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    owner(&s, id).await?;
    Ok(Json(json!(s.store.spending_policy(id).await?)))
}
async fn set_policy(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<SpendingPolicy>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = owner(&s, id).await?;
    let Json(p) = body.map_err(|_| Error::Invalid)?;
    s.store.set_spending_policy(id, actor, &p).await?;
    Ok(Json(json!({"updated":true,"signing_enabled":false})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payment {
    signature: String,
}
async fn verify(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<Payment>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = owner(&s, id).await?;
    let Json(p) = body.map_err(|_| Error::Invalid)?;
    Ok(Json(
        json!({"inserted":service(&s)?.verify_payment(id,actor,&p.signature).await?}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reserve {
    request_id: Uuid,
    destination: String,
    amount_atoms: i64,
    purpose: OperatingPurpose,
}
async fn reserve(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<Reserve>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = owner(&s, id).await?;
    let Json(p) = body.map_err(|_| Error::Invalid)?;
    s.store
        .reserve_operating(
            id,
            actor,
            p.request_id,
            &p.destination,
            p.amount_atoms,
            p.purpose,
        )
        .await?;
    Ok(Json(json!({"reserved":true,"funds_transferred":false})))
}
async fn release(State(s): State<AppState>, Path((id, key)): Path<(Uuid, Uuid)>) -> ApiResult {
    let actor = owner(&s, id).await?;
    s.store.release_operating(id, actor, key).await?;
    Ok(Json(json!({"released":true})))
}
async fn sign(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    let svc = service(&s)?;
    svc.attempt_signing(id, svc.operator()).await?;
    Err(Error::Denied.into())
}
macro_rules! history {
    ($name:ident,$kind:literal) => {
        async fn $name(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
            owner(&s, id).await?;
            Ok(Json(json!(s.store.wallet_history(id, $kind).await?)))
        }
    };
}
history!(payments, "payments");
history!(checks, "checks");
history!(reservations, "reservations");
history!(audit, "audit");
