#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use packy_client::application::Client;
use packy_common::models::{ApiError, ClientSnapshot, ProbeInfo};
use std::{path::PathBuf, sync::Arc};
use tauri::{
    Emitter, Manager, State,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Mutex;

/// 核心独立于窗口常驻，配置缺失时保留可见界面并允许管理员修复后重新检查
struct DesktopState {
    root: PathBuf,
    core: Mutex<Option<Arc<Client>>>,
}
impl DesktopState {
    async fn core(&self, app: &tauri::AppHandle) -> Result<Arc<Client>, ApiError> {
        let mut core = self.core.lock().await;
        if let Some(client) = core.as_ref() {
            return Ok(client.clone());
        }
        let client = Client::open(&self.root).await.map_err(public_error)?;
        let mut events = client.subscribe();
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(status) => {
                        if let Err(error) = handle.emit("packy:update-status", &status) {
                            eprintln!("无法发送状态事件：{error}");
                        }
                        if status.phase == "completed" {
                            notify(
                                &handle,
                                "整合包更新完成",
                                "文件已校验并安装完成，可以打开启动器了",
                            );
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        *core = Some(client.clone());
        Ok(client)
    }
}
fn public_error(error: packy_client::error::Error) -> ApiError {
    let public = error.public();
    eprintln!("{}：{}", public.request_id, error);
    public
}
/// 系统通知失败只记录诊断，不改变已完成的安装结果
fn notify(app: &tauri::AppHandle, title: &str, body: &str) {
    if let Err(error) = app.notification().builder().title(title).body(body).show() {
        eprintln!("无法发送系统通知：{error}");
    }
}
/// 桌面桥接只返回核心版本，不承担下载或文件安装逻辑
#[tauri::command]
fn get_probe_info() -> ProbeInfo {
    let mut info = packy_client::probe();
    info.checked_at = chrono::Utc::now().to_rfc3339();
    info
}
#[tauri::command]
async fn get_status(
    state: State<'_, DesktopState>,
    app: tauri::AppHandle,
) -> Result<ClientSnapshot, ApiError> {
    Ok(state.core(&app).await?.snapshot().await)
}
/// 后台运行偏好由核心持久化，窗口关闭时读取同一份内存状态
#[tauri::command]
async fn set_background_enabled(
    state: State<'_, DesktopState>,
    app: tauri::AppHandle,
    enabled: bool,
) -> Result<ClientSnapshot, ApiError> {
    let client = state.core(&app).await?;
    client
        .set_background_enabled(enabled)
        .await
        .map_err(public_error)?;
    Ok(client.snapshot().await)
}

#[tauri::command]
async fn check_update(
    state: State<'_, DesktopState>,
    app: tauri::AppHandle,
) -> Result<ClientSnapshot, ApiError> {
    let client = state.core(&app).await?;
    client.check_update().await.map_err(public_error)?;
    Ok(client.snapshot().await)
}
#[tauri::command]
async fn start_update(
    state: State<'_, DesktopState>,
    app: tauri::AppHandle,
) -> Result<String, ApiError> {
    state
        .core(&app)
        .await?
        .start_update()
        .await
        .map_err(public_error)
}
#[tauri::command(rename_all = "snake_case")]
async fn retry_update(
    state: State<'_, DesktopState>,
    app: tauri::AppHandle,
    task_id: String,
) -> Result<String, ApiError> {
    state
        .core(&app)
        .await?
        .retry_update(&task_id)
        .await
        .map_err(public_error)
}
#[tauri::command(rename_all = "snake_case")]
async fn cancel_update(
    state: State<'_, DesktopState>,
    app: tauri::AppHandle,
    task_id: String,
) -> Result<(), ApiError> {
    state
        .core(&app)
        .await?
        .cancel_update(&task_id)
        .await
        .map_err(public_error)
}
#[tauri::command]
async fn launch_game(
    state: State<'_, DesktopState>,
    app: tauri::AppHandle,
) -> Result<(), ApiError> {
    state
        .core(&app)
        .await?
        .launch_game()
        .await
        .map_err(public_error)
}

/// 恢复隐藏窗口，任何平台失败均保留日志供定位
fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main")
        && let Err(error) = window
            .show()
            .and_then(|_| window.unminimize())
            .and_then(|_| window.set_focus())
    {
        eprintln!("无法显示主窗口：{error}");
    }
}
fn quit(app: &tauri::AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<DesktopState>();
        let client = state.core.lock().await.clone();
        if let Some(client) = client
            && let Err(error) = client.prepare_exit().await
        {
            notify(&handle, "暂时无法退出", &error.public().message);
            show_main(&handle);
            return;
        }
        handle.exit(0);
    });
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executable = std::env::current_exe()?;
    let mut root = executable
        .parent()
        .ok_or("无法定位整合包目录")?
        .to_path_buf();
    // 只有开发构建允许指定整合包副本，正式分发始终以可执行程序目录为根
    if cfg!(debug_assertions)
        && let Some(path) = std::env::var_os("PACKY_GAME_ROOT")
    {
        root = PathBuf::from(path);
    }
    tauri::Builder::default()
        .manage(DesktopState {
            root,
            core: Mutex::new(None),
        })
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            get_probe_info,
            get_status,
            set_background_enabled,
            check_update,
            start_update,
            retry_update,
            cancel_update,
            launch_game
        ])
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "打开更新器", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let icon = tauri::image::Image::new_owned(
                include_bytes!("../icons/tray.rgba").to_vec(),
                64,
                64,
            );
            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("蒹葭服务器整合包更新器")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => self::quit(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let window = window.clone();
                let handle = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    let client = handle.state::<DesktopState>().core.lock().await.clone();
                    let background = match client {
                        Some(client) => client.background_enabled().await,
                        None => false,
                    };
                    if background {
                        if let Err(error) = window.hide() {
                            eprintln!("无法隐藏窗口：{error}");
                        }
                    } else {
                        quit(&handle);
                    }
                });
            }
        })
        .run(tauri::generate_context!())?;
    Ok(())
}
