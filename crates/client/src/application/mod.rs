mod worker;

use crate::{
    domain::plan,
    error::{Error, Result},
    infrastructure::{
        paths,
        storage::{self, PackLock},
    },
};
use packy_common::{error::Error as RuleError, models::*};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use tokio::sync::{Mutex, broadcast};
use tokio_util::sync::CancellationToken;

/// 文件状态由一个核心实例持有，窗口隐藏或重载不会改变任务生命周期
pub struct Client {
    sequence: AtomicU64,
    root: PathBuf,
    config: LocalConfig,
    preferences: Mutex<ClientPreferences>,
    installed: Mutex<LocalState>,
    http: reqwest::Client,
    inner: Mutex<Inner>,
    events: broadcast::Sender<TaskStatus>,
    _lock: PackLock,
}
struct Inner {
    record: Option<TaskRecord>,
    plan: Option<UpdatePlan>,
    status: TaskStatus,
    running: bool,
    cancel: CancellationToken,
    progress: Vec<i64>,
    started: Instant,
    last_event: Instant,
}
impl Client {
    /// 桌面和命令行使用构建时嵌入的专用配置，整合包中的配置文件不能覆盖它
    pub async fn open(root: &Path) -> Result<Arc<Self>> {
        let config = crate::infrastructure::config::bundled_config()?;
        Self::open_with_config(root, config).await
    }

    /// 显式配置供核心装配和隔离测试使用，所有文件路径仍相对给定整合包根目录
    pub async fn open_with_config(root: &Path, config: LocalConfig) -> Result<Arc<Self>> {
        let root = tokio::fs::canonicalize(root).await?;
        let lock = PackLock::acquire(&root)?;
        let state: LocalState = storage::read_json(&root, ".updater/state.json").await?;
        storage::validate_config(&config, &state)?;
        let preferences =
            if tokio::fs::try_exists(paths::checked(&root, ".updater/preferences.json")?).await? {
                storage::read_json(&root, ".updater/preferences.json").await?
            } else {
                ClientPreferences::default()
            };
        paths::game(&root, &config.game_dir)?;
        paths::checked(&root, &config.launcher_path)?;
        let mut record =
            if tokio::fs::try_exists(paths::checked(&root, ".updater/task.json")?).await? {
                Some(storage::read_json::<TaskRecord>(&root, ".updater/task.json").await?)
            } else {
                None
            };
        if let Some(task) = &record {
            plan::validate_record(task, &config, &state)?;
            if task.phase == "completed" {
                record = None;
            }
        }
        let mut status = idle_status();
        let plan = record.as_ref().map(|task| task.plan.clone());
        if let Some(record) = &record {
            status.task_id = record.task_id.clone();
            status.phase = "failed".into();
            status.error = Some(Error::conflict("检测到未完成更新，请按原记录重试").public());
            status.total_files = record
                .plan
                .operations
                .iter()
                .filter(|op| op.target.is_some())
                .count() as i64;
        }
        let (events, _) = broadcast::channel(32);
        let http = packy_sources::download::http_client("Packy/0.1.0")?;
        Ok(Arc::new(Self {
            sequence: AtomicU64::new(0),
            root,
            config,
            preferences: Mutex::new(preferences),
            installed: Mutex::new(state),
            http,
            inner: Mutex::new(Inner {
                record,
                plan,
                status,
                running: false,
                cancel: CancellationToken::new(),
                progress: Vec::new(),
                started: Instant::now(),
                last_event: Instant::now(),
            }),
            events,
            _lock: lock,
        }))
    }

    /// 窗口通过权威快照恢复显示，不能从丢失的进度事件推断任务结果
    pub async fn snapshot(&self) -> ClientSnapshot {
        let inner = self.inner.lock().await;
        let (status, plan) = (inner.status.clone(), inner.plan.clone());
        drop(inner);
        ClientSnapshot {
            config: self.config.clone(),
            preferences: self.preferences.lock().await.clone(),
            state: self.installed.lock().await.clone(),
            status,
            plan,
        }
    }
    /// 先原子保存用户偏好，再更新内存，保存失败时保持原有关闭行为
    pub async fn set_background_enabled(&self, enabled: bool) -> Result<()> {
        let mut preferences = self.preferences.lock().await;
        let next = ClientPreferences {
            background_enabled: enabled,
        };
        storage::write_json(&self.root, ".updater/preferences.json", &next).await?;
        *preferences = next;
        Ok(())
    }

    /// 窗口关闭时读取当前偏好，开关保存后立即作用于后续关闭请求
    pub async fn background_enabled(&self) -> bool {
        self.preferences.lock().await.background_enabled
    }

    /// 事件接收端可以落后或重建，核心不会因此停止更新
    pub fn subscribe(&self) -> broadcast::Receiver<TaskStatus> {
        self.events.subscribe()
    }
    /// 托盘退出和窗口按钮共用核心的安装阶段判定
    pub async fn can_exit(&self) -> bool {
        !plan::installing(&self.inner.lock().await.status.phase)
    }
    /// 用于退出前等待取消完成，进程不得遗留后台文件写入
    pub async fn running(&self) -> bool {
        self.inner.lock().await.running
    }

    /// 托盘退出先在状态锁内取消可取消任务，再等待写入完全停止
    pub async fn prepare_exit(&self) -> Result<()> {
        {
            let inner = self.inner.lock().await;
            if plan::installing(&inner.status.phase) {
                return Err(Error::conflict("正在安装文件，请等待更新结束后退出"));
            }
            if inner.running {
                inner.cancel.cancel();
            }
        }
        while self.running().await {
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }
        Ok(())
    }

    /// 一次网络检查只获取一份完整记录，已有未完成任务时必须先恢复原任务
    pub async fn check_update(self: &Arc<Self>) -> Result<UpdatePlan> {
        let cancel = {
            let mut inner = self.inner.lock().await;
            if inner.running || inner.record.is_some() {
                return Err(Error::conflict("请先完成或重试当前更新任务"));
            }
            inner.running = true;
            inner.plan = None;
            inner.cancel = CancellationToken::new();
            inner.status = idle_status();
            inner.status.phase = "checking".into();
            self.publish(&mut inner);
            inner.cancel.clone()
        };
        let result = tokio::select! {biased;_ = cancel.cancelled()=>Err(Error::Cancelled),result=self.fetch_plan()=>result};
        let mut inner = self.inner.lock().await;
        inner.running = false;
        match result {
            Ok(plan) => {
                inner.status.phase = if plan.operations.is_empty() {
                    "ready"
                } else {
                    "available"
                }
                .into();
                inner.status.total_files = plan
                    .operations
                    .iter()
                    .filter(|op| op.target.is_some())
                    .count() as i64;
                inner.plan = Some(plan.clone());
                self.publish(&mut inner);
                Ok(plan)
            }
            Err(error) => {
                inner.status.phase = if matches!(error, Error::Cancelled) {
                    "cancelled"
                } else {
                    "failed"
                }
                .into();
                inner.status.error = Some(error.public());
                self.publish(&mut inner);
                Err(error)
            }
        }
    }

    async fn fetch_plan(&self) -> Result<UpdatePlan> {
        let installed = self.installed.lock().await.installed_release_id.clone();
        let url = format!(
            "{}/api/v1/packs/{}/update-plan",
            self.config.server_url.trim_end_matches('/'),
            self.config.pack_id
        );
        let mut response = self
            .http
            .post(url)
            .timeout(std::time::Duration::from_secs(25))
            .json(&UpdatePlanRequest {
                installed_release_id: installed,
                client_version: env!("CARGO_PKG_VERSION").into(),
            })
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(RuleError::invalid(format!(
                "检查更新失败（HTTP {}），请确认整合包配置和服务器状态",
                response.status().as_u16()
            ))
            .into());
        }
        let mut data = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if data.len() + chunk.len() > 2 * 1024 * 1024 {
                return Err(RuleError::invalid("更新记录过大").into());
            }
            data.extend_from_slice(&chunk);
        }
        let plan: UpdatePlan = serde_json::from_slice(&data)
            .map_err(|_| RuleError::invalid("服务端返回的更新记录格式不正确"))?;
        plan::validate(&plan, &self.config.pack_id)?;
        Ok(plan)
    }

    /// 开始前先保存完整原始记录，UI 只能引用核心已经检查并展示的计划
    pub async fn start_update(self: &Arc<Self>) -> Result<String> {
        let mut inner = self.inner.lock().await;
        if inner.running || inner.record.is_some() {
            return Err(Error::conflict("已有未完成更新任务"));
        }
        let plan = inner
            .plan
            .clone()
            .filter(|p| !p.operations.is_empty())
            .ok_or_else(|| Error::conflict("请先检查并确认可用更新"))?;
        let record = TaskRecord {
            schema_version: 1,
            task_id: uuid::Uuid::new_v4().to_string(),
            installed_release_id: self.installed.lock().await.installed_release_id.clone(),
            plan,
            phase: "downloading".into(),
        };
        storage::write_json(&self.root, ".updater/task.json", &record).await?;
        let id = record.task_id.clone();
        inner.record = Some(record.clone());
        self.begin(&mut inner, &record);
        drop(inner);
        self.spawn(record);
        Ok(id)
    }
    /// 重试只使用本地原记录，不调用更新计划 API，即使服务器已有其他发布也不换目标
    pub async fn retry_update(self: &Arc<Self>, task_id: &str) -> Result<String> {
        let mut inner = self.inner.lock().await;
        if inner.running {
            return Err(Error::conflict("更新任务仍在执行"));
        }
        let record = inner
            .record
            .clone()
            .filter(|r| r.task_id == task_id)
            .ok_or_else(|| Error::conflict("原更新任务不存在"))?;
        plan::validate_record(&record, &self.config, &*self.installed.lock().await)?;
        self.begin(&mut inner, &record);
        drop(inner);
        self.spawn(record);
        Ok(task_id.into())
    }
    /// 取消仅中断检查、下载和校验，安装与保存版本阶段拒绝普通取消
    pub async fn cancel_update(&self, task_id: &str) -> Result<()> {
        let inner = self.inner.lock().await;
        if !inner.running
            || inner.status.task_id != task_id
            || !plan::cancellable(&inner.status.phase)
        {
            return Err(Error::conflict("当前阶段不允许取消"));
        }
        inner.cancel.cancel();
        Ok(())
    }
    /// 只有检查失败且没有未完成任务时才允许离线启动，文件修改失败不能绕过恢复
    pub async fn launch_game(&self) -> Result<()> {
        let inner = self.inner.lock().await;
        if inner.running || inner.record.is_some() {
            return Err(Error::conflict("更新尚未完成，不能启动游戏"));
        }
        let launcher = paths::checked(&self.root, &self.config.launcher_path)?;
        tokio::process::Command::new(launcher)
            .current_dir(&self.root)
            .spawn()?;
        Ok(())
    }
    fn begin(&self, inner: &mut Inner, record: &TaskRecord) {
        inner.running = true;
        inner.cancel = CancellationToken::new();
        inner.started = Instant::now();
        inner.last_event = Instant::now();
        inner.progress = vec![
            0;
            record
                .plan
                .operations
                .iter()
                .filter(|op| op.target.is_some())
                .count()
        ];
        let sequence = if inner.status.task_id == record.task_id {
            inner.status.sequence
        } else {
            0
        };
        inner.status = idle_status();
        inner.status.sequence = sequence;
        inner.status.task_id = record.task_id.clone();
        inner.status.phase = "downloading".into();
        inner.status.total_files = inner.progress.len() as i64;
        inner.status.total_bytes = record
            .plan
            .operations
            .iter()
            .filter_map(|op| op.target.as_ref())
            .try_fold(0_i64, |sum, target| sum.checked_add(target.size?));
        self.publish(inner);
    }
    fn spawn(self: &Arc<Self>, record: TaskRecord) {
        let this = self.clone();
        tokio::spawn(async move {
            let result = this.execute(&record).await;
            let mut inner = this.inner.lock().await;
            inner.running = false;
            match result {
                Ok(()) => {
                    inner.record = None;
                    inner.status.phase = "completed".into();
                    inner.status.error = None;
                }
                Err(error) => {
                    let public = error.public();
                    tracing::warn!(error=%error,request_id=%public.request_id,"更新任务未完成");
                    inner.status.phase = if matches!(error, Error::Cancelled) {
                        "cancelled"
                    } else {
                        "failed"
                    }
                    .into();
                    inner.status.error = Some(public);
                }
            }
            this.publish(&mut inner);
        });
    }
    fn publish(&self, inner: &mut Inner) {
        inner.status.sequence = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        // 无窗口订阅属于常驻托盘的正常情况，状态仍保存在核心快照中
        let _ = self.events.send(inner.status.clone());
        inner.last_event = Instant::now();
    }
}
fn idle_status() -> TaskStatus {
    TaskStatus {
        task_id: String::new(),
        sequence: 0,
        phase: "idle".into(),
        completed_files: 0,
        total_files: 0,
        downloaded_bytes: 0,
        total_bytes: None,
        bytes_per_second: 0,
        error: None,
    }
}
