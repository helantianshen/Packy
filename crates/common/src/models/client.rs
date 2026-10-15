use super::*;

/// 整合包内相对路径配置，所有路径以更新器所在目录定位
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct LocalConfig {
    pub pack_id: String,
    /// 组包时确定的客户端名称，不使用更新清单覆盖
    #[serde(default = "default_pack_name")]
    pub pack_name: String,
    /// 管理员填写的完整 HTTP/HTTPS 服务地址
    pub server_url: String,
    /// 相对更新器目录的游戏实例路径
    pub game_dir: String,
    /// 相对更新器目录的启动器路径
    pub launcher_path: String,
}

fn default_pack_name() -> String {
    "蒹葭服务器整合包".into()
}

/// 用户在当前整合包中的偏好，与管理员组包配置分别保存
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ClientPreferences {
    /// 开启时关闭窗口隐藏到托盘，关闭时按核心规则安全退出
    pub background_enabled: bool,
}
impl Default for ClientPreferences {
    fn default() -> Self {
        Self {
            background_enabled: true,
        }
    }
}

/// 全部名单操作成功后才允许保存的已安装版本
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct LocalState {
    pub installed_release_id: String,
}

/// 原始更新计划的本地记录，重启重试只使用其中计划
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct TaskRecord {
    #[ts(type = "number")]
    pub schema_version: i64,
    pub task_id: String,
    pub installed_release_id: String,
    pub plan: UpdatePlan,
    /// 由核心状态机产生的阶段
    pub phase: String,
}

/// 核心状态权威快照，事件可以合并而不能改变结果
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct TaskStatus {
    pub task_id: String,
    #[ts(type = "number")]
    /// 同一核心实例内单调递增的事件序号
    pub sequence: u64,
    /// 由核心状态机产生的阶段
    pub phase: String,
    #[ts(type = "number")]
    pub completed_files: i64,
    #[ts(type = "number")]
    pub total_files: i64,
    #[ts(type = "number")]
    pub downloaded_bytes: i64,
    #[ts(type = "number | null")]
    /// 任意文件大小未知时为 null，不虚构百分比
    pub total_bytes: Option<i64>,
    #[ts(type = "number")]
    pub bytes_per_second: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

/// 版本与 IPC 探针，不用于判断安装成功
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ProbeInfo {
    pub name: String,
    pub version: String,
    /// 工程能力阶段，仅用于诊断，不是更新任务阶段
    pub phase: String,
    pub business_enabled: bool,
    pub checked_at: String,
}

/// 桌面一次取得配置、已安装版本和任务快照，窗口重载无需重启任务
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ClientSnapshot {
    pub config: LocalConfig,
    pub preferences: ClientPreferences,
    pub state: LocalState,
    pub status: TaskStatus,
    pub plan: Option<UpdatePlan>,
}
