//! Runs only inside the caller's verified disposable PG18 fixture.
use super::*;
use ring::{digest, hmac};
fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../artifacts/avatar-tests")
            .join(name),
    )
    .unwrap()
}
fn encoded(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn upload_body(token: &str, n: u128, name: &str) -> Value {
    let image = bytes(name);
    let sha = encoded(digest::digest(&digest::SHA256, &image).as_ref());
    let request = id(n);
    let signature = hmac::sign(
        &hmac::Key::new(hmac::HMAC_SHA256, SERVICE.as_bytes()),
        format!("agent-avatar-v1\n{token}\n{request}\n{sha}\nupload").as_bytes(),
    );
    json!({"operation":"upload","request_id":request,"kind":"upload","sha256":sha,"png_hex":encoded(&image),"proof":encoded(signature.as_ref())})
}
pub async fn exercise(a: &Router, admin: &Store, runtime: &Store, agent: Uuid) {
    let b = upload_body(OWNER, 100, "first.png");
    let mut bad = b.clone();
    bad["proof"] = json!("0".repeat(64));
    assert_eq!(
        http(a, Some(OWNER), "POST", "/factory/avatars", bad)
            .await
            .0,
        403
    );
    assert_eq!(
        http(a, None, "POST", "/factory/avatars", b.clone()).await.0,
        401
    );
    assert_eq!(
        http(a, Some(VIEWER), "POST", "/factory/avatars", b.clone())
            .await
            .0,
        403
    );
    assert_eq!(
        http(a, Some(OTHER), "POST", "/factory/avatars", b.clone())
            .await
            .0,
        403
    );
    let first = http(a, Some(OWNER), "POST", "/factory/avatars", b.clone()).await;
    assert_eq!(first.0, 200, "{first:?}");
    assert_eq!(
        http(a, Some(OWNER), "POST", "/factory/avatars", b).await,
        first
    );
    let asset = first.1["asset_id"].as_str().unwrap();
    let original_sha = first.1["sha256"].clone();
    assert_eq!(
        http(
            a,
            Some(OTHER),
            "GET",
            &format!("/factory/avatar-assets/{asset}"),
            Value::Null
        )
        .await
        .0,
        403
    );
    let original = http(
        a,
        Some(OWNER),
        "GET",
        &format!("/factory/avatar-assets/{asset}"),
        Value::Null,
    )
    .await;
    assert_eq!(original.0, 200);
    assert_eq!(original.1["png_hex"], json!(encoded(&bytes("first.png"))));
    let set = json!({"operation":"set_avatar","request_id":id(101),"agent_id":agent,"asset_id":asset,"expected_revision":0});
    assert_eq!(
        http(a, Some(OTHER), "POST", "/factory/avatars", set.clone())
            .await
            .0,
        403
    );
    let attached = http(a, Some(OWNER), "POST", "/factory/avatars", set.clone()).await;
    assert_eq!(attached.0, 200, "{attached:?}");
    assert_eq!(
        http(a, Some(OWNER), "POST", "/factory/avatars", set).await,
        attached
    );
    let snapshot = json!({"operation":"confirm_token_image","request_id":id(102),"agent_id":agent,"expected_revision":1,"sha256":original_sha,"confirmed":true});
    let mut bad = snapshot.clone();
    bad["confirmed"] = json!(false);
    assert_eq!(
        http(a, Some(OWNER), "POST", "/factory/avatars", bad)
            .await
            .0,
        409
    );
    assert_eq!(
        http(a, Some(OWNER), "POST", "/factory/avatars", snapshot.clone())
            .await
            .0,
        200
    );
    assert_eq!(
        http(a, Some(OWNER), "POST", "/factory/avatars", snapshot)
            .await
            .0,
        200
    );
    let next = http(
        a,
        Some(OWNER),
        "POST",
        "/factory/avatars",
        upload_body(OWNER, 103, "second.png"),
    )
    .await;
    assert_eq!(next.0, 200);
    let update = json!({"operation":"set_avatar","request_id":id(104),"agent_id":agent,"asset_id":next.1["asset_id"],"expected_revision":1});
    assert_eq!(
        http(a, Some(OWNER), "POST", "/factory/avatars", update)
            .await
            .0,
        200
    );
    let metadata = http(
        a,
        Some(OWNER),
        "GET",
        &format!("/factory/avatars/{agent}"),
        Value::Null,
    )
    .await;
    assert_eq!(metadata.1["revision"], 2);
    assert_eq!(metadata.1["token_image"]["revision"], 1);
    assert_eq!(metadata.1["token_image"]["sha256"], original_sha);
    assert_eq!(http(a,Some(OWNER),"POST","/factory/avatars",json!({"operation":"set_avatar","request_id":id(105),"agent_id":agent,"asset_id":asset,"expected_revision":1})).await.0,409);
    assert_eq!(http(a,Some(OWNER),"POST","/factory/avatars",json!({"operation":"confirm_token_image","request_id":id(106),"agent_id":agent,"expected_revision":2,"sha256":next.1["sha256"],"confirmed":true})).await.0,409);
    assert!(
        sqlx::query("UPDATE agent_avatars.token_images SET sha256=$1 WHERE agent_id=$2")
            .bind("0".repeat(64))
            .bind(agent)
            .execute(admin.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE agent_avatars.assets SET png=''::bytea")
            .execute(runtime.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM agent_avatars.revisions")
            .execute(runtime.pool())
            .await
            .is_err()
    );
    assert_eq!(
        http(
            a,
            Some(SERVICE),
            "GET",
            &format!("/public/agent-avatars/{agent}/2"),
            Value::Null
        )
        .await
        .0,
        404
    );
    assert_eq!(http(a,Some(OWNER),"POST","/factory/avatars",json!({"operation":"approve_public","request_id":id(107),"agent_id":agent,"expected_revision":2,"approved":true})).await.0,200);
    assert_eq!(
        http(
            a,
            Some(SERVICE),
            "GET",
            &format!("/public/agent-avatars/{agent}/2"),
            Value::Null
        )
        .await
        .0,
        404,
        "image approval alone is not public listing consent"
    );
    // Independent creator-consent/public-purpose fixtures: never run on a live DB.
    let mut purpose:Value=sqlx::query_scalar("SELECT body FROM ecosystem_social.purposes WHERE agent_id=$1 ORDER BY revision DESC LIMIT 1").bind(agent).fetch_one(admin.pool()).await.unwrap();
    purpose["id"] = json!(id(108));
    purpose["revision"] = json!(2);
    purpose["public_summary"] = json!("Approved synthetic public summary");
    purpose["objective"] = json!("PRIVATE_PURPOSE_DO_NOT_DISCLOSE");
    sqlx::query("INSERT INTO ecosystem_social.purposes VALUES($1,$2,$3,2,$4)")
        .bind(id(108))
        .bind(agent)
        .bind(id(1))
        .bind(purpose)
        .execute(admin.pool())
        .await
        .unwrap();
    let now = chrono::Utc::now();
    let consent = json!({"request_id":id(109),"expected_revision":1,"public":true,"revoked":false,"valid_from":now,"expires_at":now+chrono::Duration::hours(1),"categories":["agent_created"]});
    sqlx::query("INSERT INTO ecosystem_social.consents(request_id,agent_id,creator_id,revision,body) VALUES($1,$2,$3,2,$4)").bind(id(109)).bind(agent).bind(id(1)).bind(consent).execute(admin.pool()).await.unwrap();
    let public = http(
        a,
        Some(SERVICE),
        "GET",
        "/public/agent-profiles",
        Value::Null,
    )
    .await;
    assert_eq!(public.0, 200);
    assert_eq!(public.1["agents"].as_array().unwrap().len(), 1);
    let text = public.1.to_string();
    assert!(
        !text.contains("PRIVATE_PURPOSE")
            && !text.contains("personality")
            && !text.contains("goals")
    );
    let public_asset = http(
        a,
        Some(SERVICE),
        "GET",
        &format!("/public/agent-avatars/{agent}/2"),
        Value::Null,
    )
    .await;
    assert_eq!(public_asset.0, 200);
    assert_eq!(public_asset.1["sha256"], next.1["sha256"]);
    assert_eq!(
        http(
            a,
            Some(SERVICE),
            "GET",
            &format!("/public/agent-avatars/{agent}/1"),
            Value::Null
        )
        .await
        .0,
        404
    );
    // Abandoned upload cleanup never removes bound/snapshotted bytes.
    sqlx::query("INSERT INTO agent_avatars.assets(owner_id,request_id,kind,png,sha256,expires_at) SELECT owner_id,$1,kind,png,sha256,clock_timestamp()-interval '1 minute' FROM agent_avatars.assets WHERE id=$2")
  .bind(id(110)).bind(Uuid::parse_str(asset).unwrap()).execute(admin.pool()).await.unwrap();
    let cleanup = http(
        a,
        Some(OWNER),
        "POST",
        "/factory/avatars",
        json!({"operation":"cleanup"}),
    )
    .await;
    assert_eq!(cleanup.0, 200);
    assert_eq!(cleanup.1["removed"], 1);
    assert_eq!(
        http(
            a,
            Some(OWNER),
            "GET",
            &format!("/factory/avatar-assets/{asset}"),
            Value::Null
        )
        .await
        .0,
        200
    );
    // The creation wrapper must atomically reject foreign images without leaving an agent.
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM agents")
        .fetch_one(admin.pool())
        .await
        .unwrap();
    let mut r = request(111);
    r["avatar_upload_id"] = json!(asset);
    assert_eq!(
        http(a, Some(OTHER), "POST", "/factory/agents", r).await.0,
        403
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM agents")
            .fetch_one(admin.pool())
            .await
            .unwrap(),
        count
    );
    let large = upload_body(OWNER, 150, "noise.png");
    assert!(large["png_hex"].as_str().unwrap().len() > 16000);
    assert_eq!(
        http(a, Some(OWNER), "POST", "/factory/avatars", large)
            .await
            .0,
        200,
        "normalized large PNG must use the isolated image body limit"
    );
    for n in 200..220 {
        assert_eq!(
            http(
                a,
                Some(OTHER),
                "POST",
                "/factory/avatars",
                upload_body(OTHER, n, "first.png")
            )
            .await
            .0,
            200
        );
    }
    assert_eq!(
        http(
            a,
            Some(OTHER),
            "POST",
            "/factory/avatars",
            upload_body(OTHER, 220, "first.png")
        )
        .await
        .0,
        409,
        "durable hourly limit must reject upload 21"
    );
    sqlx::query("INSERT INTO agent_avatars.assets(owner_id,request_id,kind,png,sha256) SELECT $1,gen_random_uuid(),'upload',$2,$3 FROM generate_series(1,80)")
        .bind(id(2)).bind(bytes("first.png")).bind(original_sha.as_str().unwrap()).execute(admin.pool()).await.unwrap();
    let quota = sqlx::query("SELECT agent_avatars.command($1,'upload',$2)")
        .bind(id(2))
        .bind(upload_body(OTHER, 221, "first.png"))
        .execute(runtime.pool())
        .await
        .unwrap_err();
    assert_eq!(
        quota.as_database_error().unwrap().code().as_deref(),
        Some("54000")
    );
    let default_created = http(a, Some(OTHER), "POST", "/factory/agents", request(300)).await;
    assert_eq!(
        default_created.0, 200,
        "image quotas must never prevent image-free creation"
    );
    let default_id = Uuid::parse_str(default_created.1["agent_id"].as_str().unwrap()).unwrap();
    let default_metadata = http(
        a,
        Some(OTHER),
        "GET",
        &format!("/factory/avatars/{default_id}"),
        Value::Null,
    )
    .await;
    assert_eq!(default_metadata.0, 200);
    assert_eq!(default_metadata.1["revision"], 0);
    assert!(default_metadata.1["asset_id"].is_null());
    sqlx::query("DELETE FROM agent_access WHERE agent_id=$1")
        .bind(default_id)
        .execute(admin.pool())
        .await
        .unwrap();
}
