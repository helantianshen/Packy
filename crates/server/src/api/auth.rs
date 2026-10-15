use super::extract::{Actor, ApiJson};
use crate::{
    application::{AppState, auth, system},
    error::Result,
};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use packy_common::models::*;

/// 公开注册与登录入口，业务权限不由路由前缀隐式授予
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/registration-status", get(registration_status))
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
}
async fn registration_status(State(state): State<AppState>) -> Result<Json<SystemSettings>> {
    Ok(Json(system::settings(&state.db).await?))
}
async fn register(
    State(state): State<AppState>,
    ApiJson(input): ApiJson<Credentials>,
) -> Result<(StatusCode, Json<Account>)> {
    Ok((
        StatusCode::CREATED,
        Json(auth::register(&state.db, input).await?),
    ))
}
async fn login(
    State(state): State<AppState>,
    ApiJson(input): ApiJson<Credentials>,
) -> Result<Json<LoginResponse>> {
    Ok(Json(state.auth.login(&state.db, input).await?))
}
async fn logout(State(state): State<AppState>, Actor(actor): Actor) -> Result<StatusCode> {
    auth::logout(&state.db, &actor).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn me(Actor(actor): Actor) -> Json<Account> {
    Json(actor)
}
