use crate::error::AppError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use packy_common::{error::ErrorCode, models::ApiError};

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (code, message) = match self {
            Self::Business(error) => (error.code, error.message),
            Self::Internal(error) => {
                tracing::error!(%request_id, error = %error, "请求处理失败");
                (
                    ErrorCode::InternalError,
                    "服务暂时不可用，请稍后重试".into(),
                )
            }
        };
        let status = match code {
            ErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
            ErrorCode::Unauthenticated => StatusCode::UNAUTHORIZED,
            ErrorCode::Forbidden | ErrorCode::RegistrationClosed => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Conflict | ErrorCode::NoPublishedRelease => StatusCode::CONFLICT,
            ErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::UpstreamUnavailable => StatusCode::BAD_GATEWAY,
            ErrorCode::SourceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(ApiError {
                code: code.as_str().into(),
                message,
                request_id,
            }),
        )
            .into_response()
    }
}
