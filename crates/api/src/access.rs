//! Server-authenticated principals; browser-supplied identity/role headers are never trusted.
use super::*;
use axum::Extension;
use supa_persistence::Role;
#[derive(Clone, Copy)]
pub struct Principal {
    pub id: Uuid,
    pub role: Role,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorCredential {
    pub id: Uuid,
    pub role: Role,
    pub token: String,
}
impl AppState {
    pub fn with_operator(mut self, id: Uuid) -> Result<Self> {
        self.add_credential(OperatorCredential {
            id,
            role: Role::Owner,
            token: self.token.to_string(),
        })?;
        Ok(self)
    }
    pub fn with_credentials(mut self, credentials: Vec<OperatorCredential>) -> Result<Self> {
        if credentials.len() > 100 {
            return Err(Error::Invalid);
        }
        for credential in credentials {
            self.add_credential(credential)?;
        }
        Ok(self)
    }
    fn add_credential(&mut self, credential: OperatorCredential) -> Result<()> {
        if credential.id.is_nil()
            || credential.token.len() < 32
            || credential.token.len() > 256
            || !credential.token.is_ascii()
            || credential.token.bytes().any(|b| !(33..=126).contains(&b))
            || self
                .operators
                .iter()
                .any(|c| c.id == credential.id || token_matches(&c.token, &credential.token))
        {
            return Err(Error::Invalid);
        }
        Arc::make_mut(&mut self.operators).push(credential);
        Ok(())
    }
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/agents/{id}/access", get(scope).post(grant))
        .route("/agents/{id}/execution-health", get(health))
        .route("/agents/{id}/authorizations", get(history))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    principal_id: Uuid,
    role: Role,
}
async fn grant(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<Grant>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(body) = body.map_err(|_| Error::Invalid)?;
    s.store
        .grant_access(id, actor.id, body.principal_id, body.role)
        .await?;
    Ok(Json(json!({"updated":true})))
}
async fn health(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    Ok(Json(s.store.execution_health(id).await?))
}
async fn history(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    Ok(Json(json!(s.store.authorization_history(id).await?)))
}

async fn scope(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
) -> ApiResult {
    let mut role = Role::Viewer;
    for candidate in [Role::Operator, Role::Owner] {
        if actor.role.permits(candidate)
            && s.store
                .require_access(id, actor.id, candidate)
                .await
                .is_ok()
        {
            role = candidate;
        }
    }
    Ok(Json(json!({"role":role})))
}
