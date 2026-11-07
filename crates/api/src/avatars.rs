//! Image bytes require an authenticated normalizer receipt in addition to owner auth.
use super::*;
use axum::http::HeaderMap;
use serde::Serialize;
impl AppState {
    pub fn with_agent_avatars(mut self) -> Result<Self> {
        if !self.synthetic_factory_enabled {
            return Err(Error::Denied);
        }
        self.agent_avatars_enabled = true;
        Ok(self)
    }
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/factory/avatars", post(write))
        .route("/factory/avatars/{id}", get(metadata))
        .route("/factory/avatar-assets/{id}", get(asset))
        .route("/public/agent-profiles", get(directory))
        .route("/public/agent-profiles/{id}", get(profile))
        .route("/public/agent-avatars/{id}/{revision}", get(public_asset))
        .layer(DefaultBodyLimit::max(2_300_000))
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Upload {
        request_id: Uuid,
        kind: String,
        sha256: String,
        png_hex: String,
        #[serde(skip_serializing)]
        proof: String,
    },
    SetAvatar {
        request_id: Uuid,
        agent_id: Uuid,
        asset_id: Uuid,
        expected_revision: u32,
    },
    ApprovePublic {
        request_id: Uuid,
        agent_id: Uuid,
        expected_revision: u32,
        approved: bool,
    },
    ConfirmTokenImage {
        request_id: Uuid,
        agent_id: Uuid,
        expected_revision: u32,
        sha256: String,
        confirmed: bool,
    },
    RemoveUpload {
        asset_id: Uuid,
    },
    Cleanup,
}
fn hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    s.as_bytes()
        .chunks_exact(2)
        .map(|c| {
            let n = |v: u8| match v {
                b'0'..=b'9' => Some(v - b'0'),
                b'a'..=b'f' => Some(v - b'a' + 10),
                _ => None,
            };
            Some(n(c[0])? * 16 + n(c[1])?)
        })
        .collect()
}
pub fn image_receipt_valid(
    service: &str,
    bearer: &str,
    request: Uuid,
    sha: &str,
    kind: &str,
    png: &str,
    proof: &str,
) -> bool {
    if !matches!(kind, "upload" | "default")
        || request.is_nil()
        || sha.len() != 64
        || proof.len() != 64
        || png.len() > 2_200_000
    {
        return false;
    }
    let (Some(bytes), Some(signature)) = (hex(png), hex(proof)) else {
        return false;
    };
    let digest = ring::digest::digest(&ring::digest::SHA256, &bytes);
    let actual: String = digest.as_ref().iter().map(|v| format!("{v:02x}")).collect();
    if actual != sha {
        return false;
    }
    let message = format!("agent-avatar-v1\n{bearer}\n{request}\n{sha}\n{kind}");
    ring::hmac::verify(
        &ring::hmac::Key::new(ring::hmac::HMAC_SHA256, service.as_bytes()),
        message.as_bytes(),
        &signature,
    )
    .is_ok()
}
fn allowed(s: &AppState, actor: Principal) -> Result<()> {
    if !s.agent_avatars_enabled || !s.synthetic_factory_enabled || actor.role != Role::Owner {
        return Err(Error::Denied);
    }
    Ok(())
}
async fn write(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    headers: HeaderMap,
    body: std::result::Result<Json<Command>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    allowed(&s, actor)?;
    let Json(command) = body.map_err(|_| Error::Invalid)?;
    if let Command::Upload {
        request_id,
        kind,
        sha256,
        png_hex,
        proof,
    } = &command
    {
        let bearer = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(Error::Denied)?;
        if !image_receipt_valid(&s.token, bearer, *request_id, sha256, kind, png_hex, proof) {
            return Err(Error::Denied.into());
        }
    }
    let mut value = serde_json::to_value(command).map_err(|_| Error::Invalid)?;
    let operation = value
        .as_object_mut()
        .ok_or(Error::Invalid)?
        .remove("operation")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or(Error::Invalid)?;
    Ok(Json(
        s.store
            .avatar_command(Some(actor.id), &operation, &value)
            .await?,
    ))
}
async fn metadata(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
) -> ApiResult {
    allowed(&s, actor)?;
    Ok(Json(
        s.store
            .avatar_command(Some(actor.id), "metadata", &json!({"agent_id":id}))
            .await?,
    ))
}
async fn asset(
    State(s): State<AppState>,
    Extension(actor): Extension<Principal>,
    Path(id): Path<Uuid>,
) -> ApiResult {
    allowed(&s, actor)?;
    Ok(Json(
        s.store
            .avatar_command(Some(actor.id), "read_asset", &json!({"asset_id":id}))
            .await?,
    ))
}
async fn directory(State(s): State<AppState>) -> ApiResult {
    if !s.agent_avatars_enabled {
        return Err(Error::Denied.into());
    }
    Ok(Json(
        s.store
            .avatar_command(None, "public_directory", &json!({}))
            .await?,
    ))
}
async fn profile(State(s): State<AppState>, Path(id): Path<Uuid>) -> ApiResult {
    if !s.agent_avatars_enabled {
        return Err(Error::Denied.into());
    }
    Ok(Json(
        s.store
            .avatar_command(None, "public_profile", &json!({"agent_id":id}))
            .await?,
    ))
}
async fn public_asset(
    State(s): State<AppState>,
    Path((id, revision)): Path<(Uuid, u32)>,
) -> ApiResult {
    if !s.agent_avatars_enabled {
        return Err(Error::Denied.into());
    }
    Ok(Json(
        s.store
            .avatar_command(
                None,
                "public_asset",
                &json!({"agent_id":id,"revision":revision}),
            )
            .await?,
    ))
}
