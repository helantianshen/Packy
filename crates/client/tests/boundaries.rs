use axum::{
    Json, Router,
    body::Body,
    routing::{get, post},
};
use packy_client::{application::Client, infrastructure::paths};
use packy_common::models::*;
use sha2::{Digest, Sha512};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
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

const CONTENT: &[u8] = b"PK\x03\x04concurrency-test";
async fn prepare(root: &Path, base: &str, pack: &str, release: &str) -> TestResult {
    tokio::fs::create_dir_all(root.join(".updater")).await?;
    tokio::fs::write(
        root.join(".updater/config.json"),
        serde_json::to_vec(&LocalConfig {
            pack_name: "测试整合包".into(),
            pack_id: pack.into(),
            server_url: base.into(),
            game_dir: "instance".into(),
            launcher_path: "launcher.exe".into(),
        })?,
    )
    .await?;
    tokio::fs::write(
        root.join(".updater/state.json"),
        serde_json::to_vec(&LocalState {
            installed_release_id: release.into(),
        })?,
    )
    .await?;
    Ok(())
}
async fn finished(client: &Client) -> Result<ClientSnapshot, Box<dyn std::error::Error>> {
    Ok(tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let state = client.snapshot().await;
            if matches!(
                state.status.phase.as_str(),
                "completed" | "failed" | "cancelled"
            ) {
                return state;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?)
}

#[tokio::test]
async fn download_concurrency_is_bounded_and_cancel_keeps_original_files() -> TestResult {
    let temp = tempfile::tempdir()?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(AtomicUsize::new(0));
    let pack = uuid::Uuid::new_v4().to_string();
    let initial = uuid::Uuid::new_v4().to_string();
    let plan = UpdatePlan {
        pack_id: pack.clone(),
        pack_name: "并发测试".into(),
        target_release_id: uuid::Uuid::new_v4().to_string(),
        target_version: "2".into(),
        changelog: String::new(),
        operations: (0..10)
            .map(|i| Operation {
                action: Action::Add,
                mod_id: uuid::Uuid::new_v4().to_string(),
                old_path: None,
                target: Some(TargetFile {
                    path: format!("mods/{i}.jar"),
                    size: Some(CONTENT.len() as i64),
                    sha512: hex::encode(Sha512::digest(CONTENT)),
                    sources: vec![DownloadSource {
                        r#type: SourceType::Server,
                        url: format!("{base}/file"),
                        priority: 0,
                    }],
                }),
            })
            .collect(),
    };
    let plan_copy = plan.clone();
    let current = active.clone();
    let max = maximum.clone();
    let requests = started.clone();
    let app = Router::new()
        .route(
            "/api/v1/packs/{id}/update-plan",
            post(move || {
                let plan = plan_copy.clone();
                async { Json(plan) }
            }),
        )
        .route(
            "/file",
            get(move || {
                let current = current.clone();
                let max = max.clone();
                let requests = requests.clone();
                async move {
                    let count = current.fetch_add(1, Ordering::SeqCst) + 1;
                    max.fetch_max(count, Ordering::SeqCst);
                    requests.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(150)).await;
                    current.fetch_sub(1, Ordering::SeqCst);
                    Body::from(CONTENT)
                }
            }),
        );
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
    });
    let first = temp.path().join("complete");
    prepare(&first, &base, &pack, &initial).await?;
    let client = open_fixture(&first).await?;
    client.check_update().await?;
    client.start_update().await?;
    let result = finished(&client).await?;
    assert_eq!(result.status.phase, "completed");
    assert_eq!(result.status.completed_files, 10);
    assert!(maximum.load(Ordering::SeqCst) <= 4);
    assert!(maximum.load(Ordering::SeqCst) >= 2);
    assert_eq!(started.load(Ordering::SeqCst), 10);
    let second = temp.path().join("cancel");
    prepare(&second, &base, &pack, &initial).await?;
    tokio::fs::create_dir_all(second.join("instance/mods")).await?;
    tokio::fs::write(second.join("instance/mods/keep.jar"), b"untouched").await?;
    let client = open_fixture(&second).await?;
    client.check_update().await?;
    let id = client.start_update().await?;
    tokio::time::timeout(Duration::from_secs(3), async {
        while started.load(Ordering::SeqCst) < 14 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;
    client.cancel_update(&id).await?;
    let result = finished(&client).await?;
    assert_eq!(result.status.phase, "cancelled");
    assert_eq!(result.state.installed_release_id, initial);
    assert_eq!(
        tokio::fs::read(second.join("instance/mods/keep.jar")).await?,
        b"untouched"
    );
    assert!(!tokio::fs::try_exists(second.join("instance/mods/0.jar")).await?);
    assert!(client.launch_game().await.is_err());
    client.prepare_exit().await?;
    assert!(!client.running().await);
    // 模拟版本文件被目录占用：安装完成仍不得提前报告成功或放行启动器
    tokio::fs::remove_file(second.join(".updater/state.json")).await?;
    tokio::fs::create_dir(second.join(".updater/state.json")).await?;
    client.retry_update(&id).await?;
    let failed = finished(&client).await?;
    assert_eq!(failed.status.phase, "failed");
    assert_eq!(failed.state.installed_release_id, initial);
    assert!(client.launch_game().await.is_err());
    let record: TaskRecord =
        serde_json::from_slice(&tokio::fs::read(second.join(".updater/task.json")).await?)?;
    assert_eq!(record.phase, "saving_state");
    tokio::fs::remove_dir(second.join(".updater/state.json")).await?;
    tokio::fs::write(
        second.join(".updater/state.json"),
        serde_json::to_vec(&LocalState {
            installed_release_id: initial,
        })?,
    )
    .await?;
    client.retry_update(&id).await?;
    let completed = finished(&client).await?;
    assert_eq!(completed.status.phase, "completed");
    assert_eq!(completed.state.installed_release_id, plan.target_release_id);
    stop.send(()).map_err(|_| "server stopped")?;
    task.await??;
    Ok(())
}

#[cfg(unix)]
#[test]
fn listed_symlink_paths_cannot_escape_the_instance() -> TestResult {
    let temp = tempfile::tempdir()?;
    let outside = temp.path().join("outside.jar");
    std::fs::write(&outside, b"outside-canary")?;
    let instance = temp.path().join("instance");
    std::fs::create_dir_all(instance.join("mods"))?;
    std::os::unix::fs::symlink(&outside, instance.join("mods/link.jar"))?;
    assert!(paths::checked(&instance, "mods/link.jar").is_err());
    std::os::unix::fs::symlink(temp.path(), instance.join("mods/escape"))?;
    assert!(paths::checked(&instance, "mods/escape/other.jar").is_err());
    assert_eq!(std::fs::read(&outside)?, b"outside-canary");
    Ok(())
}

/// 大小相同的错误内容必须由哈希识别，备用源与未知大小仍按原记录处理
#[tokio::test]
async fn hash_failure_switches_source_without_leaving_partial_files() -> TestResult {
    let temp = tempfile::tempdir()?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let calls = Arc::new(AtomicUsize::new(0));
    let bad_calls = calls.clone();
    let app = Router::new()
        .route(
            "/bad",
            get(move || {
                bad_calls.fetch_add(1, Ordering::SeqCst);
                async { Body::from(vec![b'x'; CONTENT.len()]) }
            }),
        )
        .route("/good", get(|| async { Body::from(CONTENT) }));
    let task = tokio::spawn(async move { axum::serve(listener, app).await });
    let mut target = TargetFile {
        path: "mods/demo.jar".into(),
        size: None,
        sha512: hex::encode(Sha512::digest(CONTENT)),
        sources: ["bad", "good"]
            .into_iter()
            .enumerate()
            .map(|(priority, name)| DownloadSource {
                r#type: SourceType::Server,
                url: format!("{base}/{name}"),
                priority: priority as i64,
            })
            .collect(),
    };
    let http = packy_sources::download::http_client("Packy/test")?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let path = packy_client::infrastructure::download::file(
        &http,
        temp.path(),
        "file.bin",
        &target,
        &cancel,
        |_| async {},
    )
    .await?;
    assert_eq!(tokio::fs::read(path).await?, CONTENT);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    target.sources.truncate(1);
    assert!(
        packy_client::infrastructure::download::file(
            &http,
            temp.path(),
            "failed.bin",
            &target,
            &cancel,
            |_| async {}
        )
        .await
        .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert!(!temp.path().join("failed.bin").exists());
    assert_eq!(std::fs::read_dir(temp.path())?.count(), 1);
    task.abort();
    Ok(())
}

/// 旧组包配置可直接使用，用户偏好独立保存且写入失败不会改变内存行为
#[tokio::test]
async fn dedicated_config_and_background_preference_survive_relocation() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("pack");
    let pack = uuid::Uuid::new_v4().to_string();
    let release = uuid::Uuid::new_v4().to_string();
    prepare(&root, "https://example.invalid/updates", &pack, &release).await?;
    let config_path = root.join(".updater/config.json");
    let mut config: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(&config_path).await?)?;
    config
        .as_object_mut()
        .ok_or("配置不是对象")?
        .remove("pack_name");
    let original = serde_json::to_vec(&config)?;
    tokio::fs::write(&config_path, &original).await?;
    let client = open_fixture(&root).await?;
    assert_eq!(client.snapshot().await.config.pack_name, "蒹葭服务器整合包");
    assert!(client.background_enabled().await);
    client.set_background_enabled(false).await?;
    assert!(!client.snapshot().await.preferences.background_enabled);
    assert_eq!(tokio::fs::read(&config_path).await?, original);
    drop(client);

    let moved = temp.path().join("moved-pack");
    tokio::fs::rename(&root, &moved).await?;
    let client = open_fixture(&moved).await?;
    assert!(!client.background_enabled().await);
    let snapshot = client.snapshot().await;
    assert_eq!(snapshot.config.game_dir, "instance");
    assert_eq!(snapshot.config.launcher_path, "launcher.exe");
    let preferences_path = moved.join(".updater/preferences.json");
    tokio::fs::remove_file(&preferences_path).await?;
    tokio::fs::create_dir(&preferences_path).await?;
    assert!(client.set_background_enabled(true).await.is_err());
    assert!(!client.background_enabled().await);
    tokio::fs::remove_dir(&preferences_path).await?;
    client.set_background_enabled(true).await?;
    assert!(client.background_enabled().await);
    Ok(())
}

/// 分发程序不依赖整包配置文件，也不接受玩家放入的配置覆盖
#[tokio::test]
async fn bundled_configuration_ignores_pack_local_override() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    tokio::fs::create_dir(root.join(".updater")).await?;
    tokio::fs::write(
        root.join(".updater/state.json"),
        serde_json::to_vec(&LocalState {
            installed_release_id: uuid::Uuid::new_v4().to_string(),
        })?,
    )
    .await?;
    let config_file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../config/client.{}.yaml", std::env::consts::OS));
    let expected: LocalConfig = serde_yaml_ng::from_slice(&tokio::fs::read(config_file).await?)?;
    let client = Client::open(root).await?;
    assert_eq!(
        serde_json::to_value(client.snapshot().await.config)?,
        serde_json::to_value(&expected)?
    );
    drop(client);
    tokio::fs::write(
        root.join(".updater/config.json"),
        b"invalid player override",
    )
    .await?;
    let client = Client::open(root).await?;
    assert_eq!(
        serde_json::to_value(client.snapshot().await.config)?,
        serde_json::to_value(&expected)?
    );
    Ok(())
}
