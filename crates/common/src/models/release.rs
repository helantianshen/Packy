use super::*;

/// 只针对本次名单路径的文件操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
pub enum Action {
    Add,
    Replace,
    Remove,
}

/// 草稿或正式发布快照，正式发布后禁止修改
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub id: String,
    pub pack_id: String,
    pub version: String,
    pub changelog: String,
    pub minecraft_version: String,
    pub loader: String,
    pub status: String,
    #[ts(type = "number")]
    /// 草稿乐观锁版本，每次更新递增
    pub revision: i64,
    pub base_release_id: Option<String>,
    pub published_at: Option<String>,
    pub items: Vec<ReleaseItem>,
}

/// 稳定 Mod 身份和发布时信息，业务 modid 后续变化不改历史
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ReleaseItem {
    /// 稳定内部 ID，与可修改的业务 modid 区分
    pub mod_id: String,
    pub artifact_id: String,
    pub modid_snapshot: String,
    pub name_snapshot: String,
    /// 发布快照中该 Mod 的完整目标文件
    pub target: TargetFile,
}

/// 创建草稿的起点，未指定基础版本时采用当前正式版
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CreateDraftRequest {
    pub version: String,
    pub changelog: String,
    pub base_release_id: Option<String>,
}

/// 草稿中的完整文件引用，目标路径相对游戏实例
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DraftItemInput {
    /// 稳定内部 ID，与可修改的业务 modid 区分
    pub mod_id: String,
    pub artifact_id: String,
    /// 相对实例目录的 mods 子树路径
    pub target_path: String,
}

/// 草稿完整清单和并发版本，缺失条目表示从该草稿移除
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct UpdateDraftRequest {
    #[ts(type = "number")]
    /// 草稿乐观锁版本，每次更新递增
    pub revision: i64,
    pub version: String,
    pub changelog: String,
    pub items: Vec<DraftItemInput>,
}

/// 发布或预览前的乐观并发版本
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RevisionRequest {
    #[ts(type = "number")]
    /// 草稿乐观锁版本，每次更新递增
    pub revision: i64,
}

/// 已经解析的文件直链，重试保持此记录不变
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct DownloadSource {
    pub r#type: SourceType,
    pub url: String,
    #[ts(type = "number")]
    pub priority: i64,
}

/// 安装目标与下载校验要求，不表示扫描现存文件
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct TargetFile {
    pub path: String,
    #[ts(type = "number | null")]
    /// 文件字节数，未知时为 null
    pub size: Option<i64>,
    /// 实际文件的 128 位小写十六进制 SHA-512
    pub sha512: String,
    /// 同一文件的备用直链，必须对应相同字节
    pub sources: Vec<DownloadSource>,
}

/// 名单内操作，ADD 无旧路径，REMOVE 无目标，REPLACE 同时具有两者
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub action: Action,
    /// 稳定内部 ID，与可修改的业务 modid 区分
    pub mod_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// 仅 REPLACE 和 REMOVE 使用，缺省时不序列化
    pub old_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// 仅 ADD 和 REPLACE 使用，REMOVE 必须省略
    pub target: Option<TargetFile>,
}

/// 客户端本地版本，不向服务器上报执行进度
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct UpdatePlanRequest {
    pub installed_release_id: String,
    pub client_version: String,
}

/// 一次检查取得的完整更新记录，失败重试不得刷新为新发布
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct UpdatePlan {
    pub pack_id: String,
    pub pack_name: String,
    pub target_release_id: String,
    pub target_version: String,
    pub changelog: String,
    pub operations: Vec<Operation>,
}
