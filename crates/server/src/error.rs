use packy_common::error::{Error, ErrorCode};

/// 应用层错误保留内部诊断，只有接口层负责转换 HTTP 响应
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Business(#[from] Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    /// 存储冲突与输入错误使用固定业务代码，不向用户泄露 SQL 内容
    pub fn business(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Business(Error::new(code, message))
    }

    /// 对不存在的业务资源采用一致响应
    pub fn not_found() -> Self {
        Self::business(ErrorCode::NotFound, "记录不存在")
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        match &error {
            sqlx::Error::RowNotFound => Self::not_found(),
            sqlx::Error::Database(db)
                if db.is_unique_violation() || db.is_foreign_key_violation() =>
            {
                Self::business(ErrorCode::Conflict, "记录重复或关联关系冲突")
            }
            _ => Self::Internal(error.into()),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::Internal(error.into())
    }
}
impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::Internal(error.into())
    }
}

/// 所有业务用例显式传播错误，HTTP 层统一映射
pub type Result<T> = std::result::Result<T, AppError>;
