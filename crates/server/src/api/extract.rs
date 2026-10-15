use crate::{
    application::AppState,
    error::{AppError, Result},
};
use axum::{
    Json,
    extract::{FromRequest, FromRequestParts, Query, Request},
    http::request::Parts,
};
use packy_common::{
    error::{Error, ErrorCode},
    models::Account,
};
use serde::de::DeserializeOwned;

/// 所有受保护接口都使用同一身份提取器，缺失和错误 Token 不泄露解析细节
pub struct Actor(pub Account);
impl FromRequestParts<AppState> for Actor {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        let token = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .filter(|v| !v.is_empty() && v.len() <= 8192)
            .ok_or_else(|| AppError::business(ErrorCode::Unauthenticated, "请先登录"))?;
        Ok(Self(state.auth.authenticate(&state.db, token).await?))
    }
}

/// JSON 错误统一采用业务错误体，类型的 deny_unknown_fields 防止批量赋值越权
pub struct ApiJson<T>(pub T);
impl<T, S> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = AppError;
    async fn from_request(request: Request, state: &S) -> Result<Self> {
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|_| Error::invalid("JSON 格式、字段或请求体大小不符合接口要求").into())
    }
}

/// 查询参数错误与 JSON 输入错误保持相同响应格式
pub struct ApiQuery<T>(pub T);
impl<T, S> FromRequestParts<S> for ApiQuery<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        Query::<T>::from_request_parts(parts, state)
            .await
            .map(|Query(value)| Self(value))
            .map_err(|_| Error::invalid("查询参数格式不正确").into())
    }
}
