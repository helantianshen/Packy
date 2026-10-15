use packy_common::{
    error::{Error as RuleError, ErrorCode},
    models::ApiError,
};

/// 本地 I/O 和网络诊断保留在核心，界面接收安全摘要而非完整路径或上游正文
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Rule(#[from] RuleError),
    #[error("文件操作失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("本地配置或任务记录损坏")]
    Json(#[from] serde_json::Error),
    #[error("内置 YAML 配置无效")]
    Yaml(#[from] serde_yaml_ng::Error),
    #[error("网络请求失败")]
    Network(#[from] reqwest::Error),
    #[error("操作已取消")]
    Cancelled,
}
impl Error {
    /// 只有路径检查等主动构造的规则错误直接展示，其他错误使用稳定提示
    pub fn public(&self) -> ApiError {
        let (code, message) = match self {
            Self::Rule(error) => (error.code.as_str(), error.message.clone()),
            Self::Io(error) => (
                "LOCAL_IO_ERROR",
                format!(
                    "无法完成文件操作（{}），请检查权限、磁盘空间或文件占用",
                    error.kind()
                ),
            ),
            Self::Json(_) => (
                "LOCAL_STATE_INVALID",
                "本地配置或任务记录损坏，请联系整合包管理员恢复文件".into(),
            ),
            Self::Yaml(_) => (
                "CLIENT_CONFIG_INVALID",
                "内置配置无效，请联系整合包管理员重新构建客户端".into(),
            ),
            Self::Network(_) => (
                "NETWORK_ERROR",
                "无法连接下载来源或服务器，请检查网络后重试".into(),
            ),
            Self::Cancelled => ("CANCELLED", "更新已取消，可稍后重试原任务".into()),
        };
        ApiError {
            code: code.into(),
            message,
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }
    /// 状态冲突不会触发文件修改
    pub fn conflict(message: &str) -> Self {
        Self::Rule(RuleError::new(ErrorCode::Conflict, message))
    }
}
/// 客户端核心不依赖 HTTP 服务端的错误实现
pub type Result<T> = std::result::Result<T, Error>;
