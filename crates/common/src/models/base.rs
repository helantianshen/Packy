use super::*;

/// 有界分页结果，空列表固定序列化为空数组
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Page<T> {
    pub items: Vec<T>,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub page: i64,
    #[ts(type = "number")]
    pub page_size: i64,
}

/// 可公开的错误信息，诊断通过 request_id 关联服务端日志
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub request_id: String,
}

/// 进程健康响应，不代表外部下载源可用
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Health {
    pub status: String,
    pub version: String,
    /// 由核心状态机产生的阶段
    pub phase: String,
}
