use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 客户端和服务端共享的稳定错误分类，界面不得依赖错误消息判断分支
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidRequest,
    Unauthenticated,
    Forbidden,
    RegistrationClosed,
    NotFound,
    Conflict,
    NoPublishedRelease,
    RateLimited,
    UpstreamUnavailable,
    SourceUnavailable,
    InternalError,
}

impl ErrorCode {
    /// 错误编码是稳定协议，HTTP 与桌面必须使用同一拼写
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "INVALID_REQUEST",
            Self::Unauthenticated => "UNAUTHENTICATED",
            Self::Forbidden => "FORBIDDEN",
            Self::RegistrationClosed => "REGISTRATION_CLOSED",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::NoPublishedRelease => "NO_PUBLISHED_RELEASE",
            Self::RateLimited => "RATE_LIMITED",
            Self::UpstreamUnavailable => "UPSTREAM_UNAVAILABLE",
            Self::SourceUnavailable => "SOURCE_UNAVAILABLE",
            Self::InternalError => "INTERNAL_ERROR",
        }
    }
}

/// 只携带可安全展示的业务错误，底层诊断由调用边界单独记录
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}

impl Error {
    /// 构造具有稳定类别的可预期错误
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// 表示输入不符合接口或平台约束
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidRequest, message)
    }
}

/// 通用规则层不携带 HTTP、Tauri 或数据库实现类型
pub type Result<T> = std::result::Result<T, Error>;
