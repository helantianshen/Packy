use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::get,
};
use packy_client::application::Client;
use packy_common::models::*;
use packy_server::{
    api,
    application::{AppState, auth},
    infrastructure::config::Config,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha512};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// 测试源地址和发布 ID 仅来自临时夹具，不覆盖实际程序的编译配置
async fn open_fixture(root: &Path) -> Result<Arc<Client>, Box<dyn std::error::Error>> {
    let config =
        serde_json::from_slice(&tokio::fs::read(root.join(".updater/config.json")).await?)?;
    Ok(Client::open_with_config(root, config).await?)
}

async fn resource<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    base: &str,
    token: &str,
    method: &str,
    path: &str,
    input: Value,
) -> Result<T, Box<dyn std::error::Error>> {
    let response = http
        .request(method.parse()?, format!("{base}{path}"))
        .bearer_auth(token)
        .json(&input)
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(format!("{method} {path}: {status} {body}").into());
    }
    Ok(serde_json::from_str(&body)?)
}
async fn draft(
    http: &reqwest::Client,
    base: &str,
    token: &str,
    pack: &str,
    version: &str,
    items: Value,
) -> Result<Release, Box<dyn std::error::Error>> {
    let draft: Release = resource(
        http,
        base,
        token,
        "POST",
        &format!("/api/v1/admin/packs/{pack}/drafts"),
        json!({"version":version,"changelog":"联调版本","base_release_id":null}),
    )
    .await?;
    resource(
        http,
        base,
        token,
        "PUT",
        &format!("/api/v1/admin/packs/{pack}/drafts/{}", draft.id),
        json!({"revision":draft.revision,"version":version,"changelog":"联调版本","items":items}),
    )
    .await
}
async fn publish(
    http: &reqwest::Client,
    base: &str,
    token: &str,
    draft: &Release,
) -> Result<Release, Box<dyn std::error::Error>> {
    resource(
        http,
        base,
        token,
        "POST",
        &format!(
            "/api/v1/admin/packs/{}/drafts/{}/publish",
            draft.pack_id, draft.id
        ),
        json!({"revision":draft.revision}),
    )
    .await
}
async fn artifact(
    http: &reqwest::Client,
    base: &str,
    token: &str,
    mod_id: &str,
    version: &str,
    bytes: &[u8],
    download: (&str, &str),
) -> Result<Artifact, Box<dyn std::error::Error>> {
    let (source, kind) = download;
    resource(http,base,token,"POST","/api/v1/admin/artifacts",json!({"mod_id":mod_id,"version":version,"filename":format!("{version}.jar"),"size":bytes.len(),"sha512":hex::encode(Sha512::digest(bytes)),"hosted_file_id":null,"sources":[{"type":kind,"url":format!("{base}{source}"),"priority":0}]})).await
}
async fn terminal(client: &Client) -> Result<ClientSnapshot, Box<dyn std::error::Error>> {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let snapshot = client.snapshot().await;
            if matches!(
                snapshot.status.phase.as_str(),
                "completed" | "failed" | "cancelled"
            ) {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .map_err(Into::into)
}

#[tokio::test]
async fn real_http_download_install_and_original_record_retry() -> TestResult {
    let temp = tempfile::tempdir()?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let config = Config {
        admin_dir: temp.path().join("admin"),
        listen: listener.local_addr()?,
        database: temp.path().join("server.db"),
        files_dir: temp.path().join("hosted"),
        public_url: base.clone(),
        jwt_secret: "integration-only-key-32-bytes-not-production".into(),
        modrinth_user_agent: "Packy/e2e".into(),
    };
    let server = AppState::open(config).await?;
    let credentials = || Credentials {
        username: "admin".into(),
        password: "integration-password".into(),
    };
    auth::bootstrap(&server.db, credentials()).await?;
    let token = server
        .auth
        .login(&server.db, credentials())
        .await?
        .access_token;
    let available = Arc::new(AtomicBool::new(false));
    let plan_requests = Arc::new(AtomicUsize::new(0));
    let flag = available.clone();
    let count = plan_requests.clone();
    let app = api::router(server.clone())
        .route(
            "/unstable.jar",
            get(move || {
                let flag = flag.clone();
                async move {
                    if flag.load(Ordering::SeqCst) {
                        (StatusCode::OK, Body::from("PK\x03\x04new-a")).into_response()
                    } else {
                        StatusCode::SERVICE_UNAVAILABLE.into_response()
                    }
                }
            }),
        )
        .route("/add.jar", get(|| async { "PK\x03\x04added-c" }))
        .route("/future.jar", get(|| async { "PK\x03\x04future-a" }))
        .layer(axum::middleware::from_fn(
            move |request: Request<Body>, next: axum::middleware::Next| {
                let count = count.clone();
                async move {
                    if request.uri().path().ends_with("/update-plan") {
                        count.fetch_add(1, Ordering::SeqCst);
                    }
                    next.run(request).await
                }
            },
        ));
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stop_rx.await;
            })
            .await
    });
    let http = reqwest::Client::new();
    let boundary = "packy-integration-boundary";
    let upload_bytes = b"PK\x03\x04hosted-file-body";
    let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"hosted.jar\"\r\nContent-Type: application/java-archive\r\n\r\n").into_bytes();
    body.extend_from_slice(upload_bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let upload = http
        .post(format!("{base}/api/v1/admin/files"))
        .bearer_auth(&token)
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await?;
    assert_eq!(upload.status(), reqwest::StatusCode::CREATED);
    let hosted: HostedFile = upload.json().await?;
    assert_eq!(hosted.sha512, hex::encode(Sha512::digest(upload_bytes)));
    assert_eq!(hosted.size, upload_bytes.len() as i64);
    let range = http
        .get(&hosted.url)
        .header("range", "bytes=0-3")
        .send()
        .await?;
    assert_eq!(range.status(), reqwest::StatusCode::PARTIAL_CONTENT);
    assert_eq!(range.bytes().await?.as_ref(), b"PK\x03\x04");
    let pack: Pack = resource(
        &http,
        &base,
        &token,
        "POST",
        "/api/v1/admin/packs",
        json!({"name":"联调整合包","minecraft_version":"1.21.1","loader":"neoforge"}),
    )
    .await?;
    let mut mods = Vec::new();
    for modid in ["demo_a", "demo_b", "demo_c"] {
        mods.push(resource::<ModRecord>(&http,&base,&token,"POST","/api/v1/admin/mods",json!({"modid":modid,"name":"允许重复的名称","modrinth_project_id":null,"curseforge_project_id":null})).await?);
    }
    // 列表和字面通配符搜索必须返回资源，不能让 SQL 转义规则影响基础录入流程
    let listed: Page<ModRecord> = resource(
        &http,
        &base,
        &token,
        "GET",
        "/api/v1/admin/mods?page=1&page_size=20",
        Value::Null,
    )
    .await?;
    assert_eq!(listed.total, 3);
    let searched: Page<ModRecord> = resource(
        &http,
        &base,
        &token,
        "GET",
        "/api/v1/admin/mods?keyword=demo_a",
        Value::Null,
    )
    .await?;
    assert_eq!(searched.total, 1);
    let literal: Page<ModRecord> = resource(
        &http,
        &base,
        &token,
        "GET",
        "/api/v1/admin/mods?keyword=%25",
        Value::Null,
    )
    .await?;
    assert_eq!(literal.total, 0);
    let old = artifact(
        &http,
        &base,
        &token,
        &mods[0].id,
        "old",
        b"old-a",
        ("/unused-old.jar", "server"),
    )
    .await?;
    let deleted = artifact(
        &http,
        &base,
        &token,
        &mods[1].id,
        "delete",
        b"delete-b",
        ("/unused-delete.jar", "server"),
    )
    .await?;
    let first = draft(
        &http,
        &base,
        &token,
        &pack.id,
        "1.0.0",
        json!([
            {"mod_id":mods[0].id,"artifact_id":old.id,"target_path":"mods/old.jar"},
            {"mod_id":mods[1].id,"artifact_id":deleted.id,"target_path":"mods/delete.jar"}
        ]),
    )
    .await?;
    let first = publish(&http, &base, &token, &first).await?;
    let updated = artifact(
        &http,
        &base,
        &token,
        &mods[0].id,
        "new",
        b"PK\x03\x04new-a",
        ("/unstable.jar", "github"),
    )
    .await?;
    let added = artifact(
        &http,
        &base,
        &token,
        &mods[2].id,
        "added",
        b"PK\x03\x04added-c",
        ("/add.jar", "github"),
    )
    .await?;
    let second = draft(
        &http,
        &base,
        &token,
        &pack.id,
        "1.1.0",
        json!([
            {"mod_id":mods[0].id,"artifact_id":updated.id,"target_path":"mods/new.jar"},
            {"mod_id":mods[2].id,"artifact_id":added.id,"target_path":"mods/added.jar"}
        ]),
    )
    .await?;
    let second = publish(&http, &base, &token, &second).await?;
    let game = temp.path().join("game");
    tokio::fs::create_dir_all(game.join(".updater")).await?;
    tokio::fs::create_dir_all(game.join("instance/mods")).await?;
    tokio::fs::write(game.join("instance/mods/old.jar"), b"old-a").await?;
    tokio::fs::write(game.join("instance/mods/delete.jar"), b"delete-b").await?;
    tokio::fs::write(
        game.join("instance/mods/player.jar"),
        b"player-custom-unchanged",
    )
    .await?;
    tokio::fs::write(
        game.join(".updater/config.json"),
        serde_json::to_vec(&LocalConfig {
            pack_name: "测试整合包".into(),
            pack_id: pack.id.clone(),
            server_url: base.clone(),
            game_dir: "instance".into(),
            launcher_path: "launcher.exe".into(),
        })?,
    )
    .await?;
    tokio::fs::write(
        game.join(".updater/state.json"),
        serde_json::to_vec(&LocalState {
            installed_release_id: first.id.clone(),
        })?,
    )
    .await?;
    let client = open_fixture(&game).await?;
    assert!(open_fixture(&game).await.is_err());
    let plan = client.check_update().await?;
    assert_eq!(plan.target_release_id, second.id);
    assert_eq!(plan.operations.len(), 3);
    let task_id = client.start_update().await?;
    assert!(client.start_update().await.is_err());
    let failed = terminal(&client).await?;
    assert_eq!(failed.status.phase, "failed");
    assert_eq!(
        tokio::fs::read(game.join("instance/mods/old.jar")).await?,
        b"old-a"
    );
    assert_eq!(
        tokio::fs::read(game.join("instance/mods/delete.jar")).await?,
        b"delete-b"
    );
    assert_eq!(failed.state.installed_release_id, first.id);
    assert!(client.launch_game().await.is_err());
    // 服务端发布新版并禁用字典，原任务仍必须使用已经取得的记录
    let future = artifact(
        &http,
        &base,
        &token,
        &mods[0].id,
        "future",
        b"PK\x03\x04future-a",
        ("/future.jar", "server"),
    )
    .await?;
    let third = draft(
        &http,
        &base,
        &token,
        &pack.id,
        "1.2.0",
        json!([
            {"mod_id":mods[0].id,"artifact_id":future.id,"target_path":"mods/future.jar"},
            {"mod_id":mods[2].id,"artifact_id":added.id,"target_path":"mods/added.jar"}
        ]),
    )
    .await?;
    let third = publish(&http, &base, &token, &third).await?;
    let _: SourceDictionaryItem = resource(
        &http,
        &base,
        &token,
        "PUT",
        "/api/v1/admin/system/download-source-types/github",
        json!({"name":"GitHub","enabled":false,"sort_order":2}),
    )
    .await?;
    available.store(true, Ordering::SeqCst);
    // 制造安装阶段占用，证明失败不推进版本，重试允许旧路径已经删除
    tokio::fs::create_dir(game.join("instance/mods/new.jar")).await?;
    client.retry_update(&task_id).await?;
    let failed = terminal(&client).await?;
    assert_eq!(failed.status.phase, "failed");
    assert_eq!(failed.state.installed_release_id, first.id);
    assert!(!tokio::fs::try_exists(game.join("instance/mods/old.jar")).await?);
    tokio::fs::remove_dir(game.join("instance/mods/new.jar")).await?;
    drop(client);
    let client = open_fixture(&game).await?;
    assert_eq!(client.snapshot().await.status.task_id, task_id);
    client.retry_update(&task_id).await?;
    let completed = terminal(&client).await?;
    assert_eq!(completed.status.phase, "completed");
    assert_eq!(completed.state.installed_release_id, second.id);
    assert_eq!(
        tokio::fs::read(game.join("instance/mods/new.jar")).await?,
        b"PK\x03\x04new-a"
    );
    assert_eq!(
        tokio::fs::read(game.join("instance/mods/added.jar")).await?,
        b"PK\x03\x04added-c"
    );
    assert_eq!(
        tokio::fs::read(game.join("instance/mods/player.jar")).await?,
        b"player-custom-unchanged"
    );
    assert!(!tokio::fs::try_exists(game.join("instance/mods/delete.jar")).await?);
    assert_eq!(plan_requests.load(Ordering::SeqCst), 1);
    assert_eq!(client.check_update().await?.target_release_id, third.id);
    // 正式快照和旧 modid 不被主表编辑覆盖
    let _:ModRecord=resource(&http,&base,&token,"PATCH",&format!("/api/v1/admin/mods/{}",mods[0].id),json!({"modid":"renamed_a","name":"新名称","modrinth_project_id":null,"curseforge_project_id":null})).await?;
    let published: Release = resource(
        &http,
        &base,
        &token,
        "GET",
        &format!("/api/v1/admin/packs/{}/releases/{}", pack.id, second.id),
        Value::Null,
    )
    .await?;
    assert!(
        published
            .items
            .iter()
            .any(|item| item.modid_snapshot == "demo_a")
    );
    stop_tx.send(()).map_err(|_| "server stopped early")?;
    task.await??;
    Ok(())
}
