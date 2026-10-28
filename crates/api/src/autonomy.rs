//! Operator-authorized, bounded offline rehearsal. Never uses the configured provider.
use crate::{ApiError, ApiResult, AppState};
use axum::{
    Extension, Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use serde_json::json;
use std::{sync::Arc, time::Duration};
use supa_domain::*;
use supa_providers::Offline;
use supa_runtime::Runtime;
use uuid::Uuid;

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/autonomy", get(capabilities).post(run))
}
async fn capabilities() -> Json<serde_json::Value> {
    Json(
        json!({"mode":"offline","max_cycles":3,"simulated_seed_microusd":10000,
        "simulated_expense_limit_microusd":3300,"paid_requests":0,
        "tools":["read_memory","remember","evaluate_goal"]}),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    request_id: Uuid,
    goal: String,
    authorize_offline: bool,
}
impl Input {
    fn validate(&self) -> Result<()> {
        if !self.authorize_offline {
            return Err(Error::Denied);
        }
        if self.request_id.is_nil() {
            return Err(Error::Invalid);
        }
        if self.goal.trim().len() < 3 {
            return Err(Error::Invalid);
        }
        clean_text(self.goal.trim(), 500)
    }
}
async fn run(
    State(state): State<AppState>,
    Extension(actor): Extension<crate::Principal>,
    body: std::result::Result<Json<Input>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let input = match body {
        Ok(Json(input)) => input,
        Err(_) => return ApiError(Error::Invalid).into_response(),
    };
    if let Err(error) = input.validate() {
        return ApiError(error).into_response();
    }
    let permit = match state.gate.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => return ApiError(Error::Busy).into_response(),
    };
    // Continue the bounded task and final pause even if the HTTP caller disconnects.
    match tokio::spawn(async move {
        let _permit = permit;
        execute(state, input, actor.id).await
    })
    .await
    {
        Ok(response) => response,
        Err(_) => ApiError(Error::Provider).into_response(),
    }
}
async fn execute(state: AppState, input: Input, actor: Uuid) -> Response {
    let id = input.request_id;
    match state.store.get(id).await {
        Ok(_) => {
            return (
                StatusCode::CONFLICT,
                Json(json!({"error":"request_id_exists","stage":"create","agent_id":id,"mode":"offline","paid_requests":0})),
            )
                .into_response();
        }
        Err(Error::NotFound) => {}
        Err(error) => return ApiError(error).into_response(),
    }
    let config = CycleConfig {
        scheduler_enabled: false,
        interval_seconds: 1,
        max_steps: 3,
        timeout_seconds: 5,
        max_provider_calls: 1,
        inference_cost_microusd: 1000,
        operating_cost_microusd: 100,
        ..CycleConfig::default()
    };
    let mut agent = match Agent::new(
        "Supa offline autonomy".into(),
        Personality::default(),
        config,
        [
            Capability::ReadMemory,
            Capability::Remember,
            Capability::EvaluateGoal,
        ]
        .into_iter()
        .collect(),
    ) {
        Ok(agent) => agent,
        Err(error) => return ApiError(error).into_response(),
    };
    agent.id = id;
    if let Err(error) = state.store.create_owned(&agent, actor).await {
        // Another API process may have won the primary-key race. Never touch its agent.
        return if state.store.get(id).await.is_ok() {
            (
                StatusCode::CONFLICT,
                Json(json!({"error":"request_id_exists","stage":"create","agent_id":id,"mode":"offline","paid_requests":0})),
            )
                .into_response()
        } else {
            ApiError(error).into_response()
        };
    }
    let mut stage = "setup";
    let mut cycles = vec![];
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        state
            .store
            .add_goal(id, input.goal.trim().to_owned())
            .await?;
        state.store.credit(id, LedgerKind::Seed, 10000, id).await?;
        state
            .store
            .transition(
                id,
                Lifecycle::Active,
                "operator_authorized_offline_rehearsal",
                true,
            )
            .await?;
        let runtime = Runtime::new(state.store.clone(), Arc::new(Offline));
        stage = "cycles";
        for index in 0..3 {
            if index > 0 {
                tokio::time::sleep(Duration::from_millis(1100)).await;
            }
            cycles.push(runtime.cycle(id, false).await?);
        }
        stage = "pause";
        state
            .store
            .transition(id, Lifecycle::Paused, "offline_rehearsal_completed", true)
            .await?;
        Ok::<(), Error>(())
    })
    .await
    .unwrap_or(Err(Error::Timeout));
    if let Err(error) = result {
        let fenced = tokio::time::timeout(
            Duration::from_secs(5),
            state.store.emergency_stop(id, "offline_rehearsal_failed"),
        )
        .await
        .is_ok_and(|result| result.is_ok());
        tracing::warn!(agent_id=%id,stage,error_code=%error,fenced,"offline_rehearsal_failed");
        let status = if matches!(error, Error::Timeout) {
            StatusCode::GATEWAY_TIMEOUT
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };
        return (
            status,
            Json(
                json!({"error":error.to_string(),"stage":stage,"agent_id":id,
            "cycles":cycles,"mode":"offline","paid_requests":0,"emergency_stop_confirmed":fenced}),
            ),
        )
            .into_response();
    }
    tracing::info!(agent_id=%id,completed_cycles=cycles.len(),"offline_rehearsal_completed");
    let response: ApiResult = Ok(Json(
        json!({"agent_id":id,"mode":"offline","simulated":true,
        "cycles":cycles,"lifecycle":"paused","paid_requests":0,"simulated_seed_microusd":10000,
        "simulated_expense_limit_microusd":3300}),
    ));
    response.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation_never_accepts_provider_or_policy_overrides() {
        assert!(serde_json::from_value::<Input>(json!({"request_id":Uuid::new_v4(),"goal":"Learn","authorize_offline":true,"model":"live"})).is_err());
        for goal in ["", "private_key=secret", "postgresql://secret"] {
            assert!(
                Input {
                    request_id: Uuid::new_v4(),
                    goal: goal.into(),
                    authorize_offline: true
                }
                .validate()
                .is_err()
            );
        }
        assert!(matches!(
            Input {
                request_id: Uuid::new_v4(),
                goal: "Learn".into(),
                authorize_offline: false
            }
            .validate(),
            Err(Error::Denied)
        ));
    }
}
