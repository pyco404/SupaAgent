//! Authenticated customer contracts; all identities come from server credentials.
use super::*;
use supa_providers::solana::HttpSolanaRpc;
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/services", get(services))
        .route("/jobs", get(jobs).post(create_job))
        .route("/jobs/{id}", get(job))
        .route("/jobs/{id}/commands", post(command))
        .route("/jobs/{id}/simulated-payment", post(simulated_payment))
        .route("/jobs/{id}/verify-payment", post(verify_payment))
        .route("/agents/{id}/services", post(offer))
        .route("/agents/{id}/economy", get(accounts))
        .route("/agents/{id}/economy/recipient", post(recipient))
        .route("/agents/{id}/economy/capital", post(capital))
        .route("/agents/{id}/economy/policy", post(policy))
        .route("/agents/{id}/economy/spend", post(spend))
}
async fn services(State(s): State<AppState>) -> ApiResult {
    let services=s.store.economic_services(None).await?.into_iter().map(|offer|json!({"id":offer.id,"agent_id":offer.agent_id,"spec":offer.spec,"created_at":offer.created_at,"input_contract":"supplied text or {objective,material,requirements,require_external_research}; API material must be a JSON object; no secrets","output_contract":offer.spec.output_contract(),"permission_requirements":["evaluate_goal"],"execution_limits":{"lease_seconds":30,"provider_timeout_seconds":10,"max_provider_requests":2,"max_tool_steps":4,"max_revisions":1,"external_mutations":0}})).collect::<Vec<_>>();
    Ok(Json(json!({"services":services,"mainnet_enabled":false})))
}
async fn jobs(State(s): State<AppState>, Extension(p): Extension<Principal>) -> ApiResult {
    Ok(Json(
        json!({"jobs":s.store.economic_jobs(p.id,None).await?}),
    ))
}
async fn create_job(
    State(s): State<AppState>,
    Extension(p): Extension<Principal>,
    Json(r): Json<CreateJob>,
) -> ApiResult {
    let saved = s.store.create_economic_job(p.id, r).await?;
    Ok(Json(json!(s.store.economic_job(saved.id, p.id).await?)))
}
async fn job(
    State(s): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<Uuid>,
) -> ApiResult {
    let j = s.store.economic_job(id, p.id).await?;
    let wallet = match s.store.wallet(j.agent_id).await {
        Ok(w) => Some(w.address),
        Err(Error::NotFound) => None,
        Err(e) => return Err(e.into()),
    };
    Ok(Json(
        json!({"job":j,"refund_eligible_atoms":j.refund_eligible_atoms(),"payment_request":{"network":j.network,"amount_atoms":j.spec.price_atoms,"decimals":6,"mint":if j.network==EconomicNetwork::Devnet{Some(SolanaNetwork::Devnet.usdc_mint())}else{None},"recipient":wallet,"memo":id,"expires_at":j.expires_at,"signing_enabled":false}}),
    ))
}
async fn command(
    State(s): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<Uuid>,
    Json(c): Json<JobCommand>,
) -> ApiResult {
    let _permit = s.gate.try_acquire().map_err(|_| Error::Busy)?;
    let saved = if c.operation == "execute" {
        s.runtime.execute_economic_job(id, p.id, c).await?
    } else {
        s.store.economic_command(id, p.id, c).await?
    };
    Ok(Json(json!(s.store.economic_job(saved.id, p.id).await?)))
}
async fn simulated_payment(
    State(s): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<Uuid>,
    Json(r): Json<SimulatedReceipt>,
) -> ApiResult {
    s.store.simulated_job_payment(id, p.id, r).await?;
    Ok(Json(json!(s.store.economic_job(id, p.id).await?)))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payment {
    signature: String,
    authorize_read_only_verification: bool,
}
async fn verify_payment(
    State(s): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<Uuid>,
    Json(r): Json<Payment>,
) -> ApiResult {
    if !r.authorize_read_only_verification {
        return Err(Error::Denied.into());
    }
    // Fixed server-side configuration; customer/model input can never select a remote URL.
    let endpoint = std::env::var("SUPA_ECONOMY_DEVNET_RPC_URL").map_err(|_| Error::Denied)?;
    let rpc = HttpSolanaRpc::new(&endpoint, SolanaNetwork::Devnet)?;
    s.runtime
        .verify_economic_payment(id, p.id, &r.signature, &rpc)
        .await?;
    Ok(Json(json!(s.store.economic_job(id, p.id).await?)))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Offer {
    id: Uuid,
    spec: ServiceSpec,
    authorize: bool,
}
async fn offer(
    State(s): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<Uuid>,
    Json(r): Json<Offer>,
) -> ApiResult {
    owner(p)?;
    if !r.authorize {
        return Err(Error::Denied.into());
    }
    Ok(Json(json!(
        s.store.offer_service(id, p.id, r.id, r.spec).await?
    )))
}
async fn accounts(
    State(s): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<Uuid>,
) -> ApiResult {
    Ok(Json(
        json!({"accounts":s.store.economic_accounts(id,p.id).await?,"services":s.store.economic_services(Some(id)).await?,"jobs":s.store.economic_jobs(p.id,Some(id)).await?}),
    ))
}
fn owner(p: Principal) -> Result<()> {
    if p.role == Role::Owner {
        Ok(())
    } else {
        Err(Error::Denied)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipient {
    address: String,
    confirm_watch_only: bool,
}
/// Register only a public external identity. No vault, key generation or signing.
async fn recipient(
    State(s): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<Uuid>,
    Json(r): Json<Recipient>,
) -> ApiResult {
    owner(p)?;
    s.store.require_access(id, p.id, Role::Owner).await?;
    if !r.confirm_watch_only {
        return Err(Error::Denied.into());
    }
    validate_solana_address(&r.address)?;
    let wallet = match s.store.wallet(id).await {
        Ok(w)
            if w.address == r.address
                && w.owner_id == p.id
                && w.network == SolanaNetwork::Devnet =>
        {
            w
        }
        Ok(_) => return Err(Error::Stale.into()),
        Err(Error::NotFound) => {
            let w = WalletIdentity {
                id: Uuid::new_v4(),
                agent_id: id,
                network: SolanaNetwork::Devnet,
                address: r.address,
                owner_id: p.id,
                origin: IdentityOrigin::HumanOperator,
            };
            s.store.register_wallet(&w, p.id).await?;
            w
        }
        Err(e) => return Err(e.into()),
    };
    Ok(Json(
        json!({"address":wallet.address,"network":"devnet","watch_only":true,"signing_enabled":false,"external_control_verified":false}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Money {
    request_id: Uuid,
    amount_atoms: i64,
    authorize_simulated: bool,
}
async fn capital(
    State(s): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<Uuid>,
    Json(r): Json<Money>,
) -> ApiResult {
    owner(p)?;
    if !r.authorize_simulated {
        return Err(Error::Denied.into());
    }
    s.store
        .economic_capital(id, p.id, r.request_id, r.amount_atoms)
        .await?;
    accounts(State(s), Extension(p), Path(id)).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    ceiling_atoms: i64,
    reserve_atoms: i64,
    destinations: Vec<String>,
    authorize_simulated: bool,
}
async fn policy(
    State(s): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<Uuid>,
    Json(r): Json<Policy>,
) -> ApiResult {
    owner(p)?;
    if !r.authorize_simulated {
        return Err(Error::Denied.into());
    }
    s.store
        .economic_policy(id, p.id, r.ceiling_atoms, r.reserve_atoms, r.destinations)
        .await?;
    accounts(State(s), Extension(p), Path(id)).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spend {
    request_id: Uuid,
    amount_atoms: i64,
    destination: String,
    authorize_simulated: bool,
}
async fn spend(
    State(s): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<Uuid>,
    Json(r): Json<Spend>,
) -> ApiResult {
    owner(p)?;
    if !r.authorize_simulated {
        return Err(Error::Denied.into());
    }
    s.store
        .economic_spend(id, p.id, r.request_id, r.amount_atoms, &r.destination)
        .await?;
    accounts(State(s), Extension(p), Path(id)).await
}
