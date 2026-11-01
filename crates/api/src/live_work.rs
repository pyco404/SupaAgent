//! Owner-only evaluation endpoint. Environment enablement alone cannot authorize a request.
use super::*;
pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/agents/{id}/live-work", get(list).post(run))
}
impl AppState {
    pub fn with_live_work_evaluation(mut self) -> Self {
        self.live_work_enabled = true;
        self
    }
}
async fn owner(s: &AppState, actor: &Principal, id: Uuid) -> Result<()> {
    if !actor.role.permits(Role::Owner) {
        return Err(Error::Denied);
    }
    s.store.require_access(id, actor.id, Role::Owner).await
}
async fn list(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
) -> ApiResult {
    owner(&s, &actor, id).await?;
    let policy = s.controlled.as_ref().map(|c| c.provider.policy());
    let reservation = policy
        .as_ref()
        .map(InferencePolicy::reservation)
        .transpose()?;
    let jobs = s.store.economic_jobs(actor.id, Some(id)).await?;
    let mut candidates = vec![];
    for job in jobs.into_iter().filter(|j| {
        j.customer_id == actor.id
            && j.state == JobState::Created
            && j.network == EconomicNetwork::Simulated
            && j.received_atoms == 0
    }) {
        let hash = s.store.work_input_sha256(job.id, actor.id).await?;
        let request = WorkRequest::parse(&job.input, job.spec.kind)?;
        candidates.push(json!({"id":job.id,"revision":job.revision,"input_sha256":hash,"objective":request.objective,"kind":job.spec.kind,"hypothetical_service_revenue_atoms":job.spec.price_atoms,"approved_tools":request.tools(job.spec.kind,false)}));
    }
    Ok(Json(
        json!({"enabled":s.live_work_enabled&&s.controlled.is_some(),"mode":"live_work_evaluation","continuous":false,"policy":policy,"minimum_reservation_microusd":reservation,"commercial_delivery":false,"project_id":s.controlled.as_ref().map(|c|c.provider.project()),"jobs":candidates,"sessions":s.store.live_sessions(id).await?.into_iter().filter(|s|s.work.is_some()).collect::<Vec<_>>()}),
    ))
}
async fn run(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
    Json(authorization): Json<LiveWorkConsent>,
) -> ApiResult {
    owner(&s, &actor, id).await?;
    if !s.live_work_enabled {
        return Err(Error::Denied.into());
    }
    let _permit = s.gate.try_acquire().map_err(|_| Error::Busy)?;
    let result = s
        .controlled
        .as_ref()
        .ok_or(Error::Denied)?
        .run_work(id, actor.id, authorization)
        .await?;
    owner(&s, &actor, id).await?;
    Ok(Json(json!(result)))
}
