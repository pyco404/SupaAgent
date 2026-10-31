//! Read-only caller status remains available after permit revocation, never private agent records.
use super::*;
pub(super) async fn status(
    State(state): State<AppState>,
    Extension(actor): Extension<Principal>,
) -> Response {
    let response = async {
        let mut status = state.store.operator_status(actor.id).await?;
        status["role"] = json!(actor.role);
        status["api_session_expires_at"] = json!(state.credential_deadlines.get(&actor.id));
        if actor.role != Role::Owner {
            for key in [
                "create_agent",
                "grant_owner",
                "activate_offline",
                "execute_offline",
            ] {
                status["eligibility"][key] = json!(false);
            }
        }
        Ok::<_, Error>(status)
    }
    .await;
    let mut response = match response {
        Ok(value) => Json(value).into_response(),
        Err(error) => ApiError(error).into_response(),
    };
    response.headers_mut().insert(
        "cache-control",
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}
