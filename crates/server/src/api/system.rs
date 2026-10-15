use super::extract::{Actor, ApiJson, ApiQuery};
use crate::{
    application::{AppState, auth, system},
    error::Result,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, patch, put},
};
use packy_common::models::*;

/// 系统管理接口只编排提取与响应，权限和事务由应用层负责
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/settings", get(settings).put(update_settings))
        .route("/users", get(users))
        .route("/users/{id}", patch(update_user))
        .route("/roles", get(roles))
        .route("/permissions", get(permissions))
        .route("/roles/{code}/permissions", put(update_permissions))
        .route("/download-source-types", get(sources))
        .route("/download-source-types/{code}", put(update_source))
}
async fn settings(State(s): State<AppState>, Actor(a): Actor) -> Result<Json<SystemSettings>> {
    auth::require(&a, "settings:read", true)?;
    Ok(Json(system::settings(&s.db).await?))
}
async fn update_settings(
    State(s): State<AppState>,
    Actor(a): Actor,
    ApiJson(input): ApiJson<SystemSettings>,
) -> Result<Json<SystemSettings>> {
    Ok(Json(system::update_settings(&s.db, &a, input).await?))
}
async fn users(
    State(s): State<AppState>,
    Actor(a): Actor,
    ApiQuery(page): ApiQuery<system::Pagination>,
) -> Result<Json<Page<Account>>> {
    Ok(Json(system::users(&s.db, &a, page).await?))
}
async fn update_user(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path(id): Path<String>,
    ApiJson(input): ApiJson<UpdateAccountRequest>,
) -> Result<Json<Account>> {
    Ok(Json(system::update_user(&s.db, &a, &id, input).await?))
}
async fn roles(State(s): State<AppState>, Actor(a): Actor) -> Result<Json<Vec<Role>>> {
    Ok(Json(system::roles(&s.db, &a).await?))
}
async fn permissions(State(s): State<AppState>, Actor(a): Actor) -> Result<Json<Vec<Permission>>> {
    Ok(Json(system::permissions(&s.db, &a).await?))
}
async fn update_permissions(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path(code): Path<String>,
    ApiJson(input): ApiJson<UpdateRolePermissionsRequest>,
) -> Result<Json<Role>> {
    Ok(Json(
        system::update_permissions(&s.db, &a, &code, input).await?,
    ))
}
async fn sources(
    State(s): State<AppState>,
    Actor(a): Actor,
) -> Result<Json<Vec<SourceDictionaryItem>>> {
    Ok(Json(system::sources(&s.db, &a).await?))
}
async fn update_source(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path(code): Path<String>,
    ApiJson(input): ApiJson<system::SourceInput>,
) -> Result<Json<SourceDictionaryItem>> {
    let code: SourceType = serde_json::from_value(serde_json::Value::String(code))
        .map_err(|_| packy_common::error::Error::invalid("未知下载源"))?;
    Ok(Json(system::update_source(&s.db, &a, code, input).await?))
}
