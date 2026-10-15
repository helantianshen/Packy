use super::extract::{Actor, ApiJson, ApiQuery};
use crate::{
    application::{AppState, releases, system::Pagination},
    error::Result,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use packy_common::models::*;

/// 草稿与正式发布使用不同写入口，不能通过修改状态字段绕过发布事务
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/packs/{id}/releases", get(list))
        .route("/packs/{id}/releases/{release}", get(get_release))
        .route("/packs/{id}/drafts", post(create))
        .route("/packs/{id}/drafts/{release}", put(update))
        .route("/packs/{id}/drafts/{release}/preview", post(preview))
        .route("/packs/{id}/drafts/{release}/publish", post(publish))
}
async fn list(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path(p): Path<String>,
    ApiQuery(page): ApiQuery<Pagination>,
) -> Result<Json<Page<Release>>> {
    Ok(Json(releases::list(&s, &a, &p, page).await?))
}
async fn get_release(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path((p, r)): Path<(String, String)>,
) -> Result<Json<Release>> {
    Ok(Json(releases::get(&s, &a, &p, &r).await?))
}
async fn create(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path(p): Path<String>,
    ApiJson(i): ApiJson<CreateDraftRequest>,
) -> Result<(StatusCode, Json<Release>)> {
    Ok((
        StatusCode::CREATED,
        Json(releases::create(&s, &a, &p, i).await?),
    ))
}
async fn update(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path((p, r)): Path<(String, String)>,
    ApiJson(i): ApiJson<UpdateDraftRequest>,
) -> Result<Json<Release>> {
    Ok(Json(releases::update(&s, &a, &p, &r, i).await?))
}
async fn preview(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path((p, r)): Path<(String, String)>,
    ApiJson(i): ApiJson<RevisionRequest>,
) -> Result<Json<UpdatePlan>> {
    Ok(Json(
        releases::preview(&s, &a, &p, &r, i.revision, false)
            .await?
            .0,
    ))
}
async fn publish(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path((p, r)): Path<(String, String)>,
    ApiJson(i): ApiJson<RevisionRequest>,
) -> Result<Json<Release>> {
    Ok(Json(
        releases::preview(&s, &a, &p, &r, i.revision, true).await?.1,
    ))
}
/// 公开更新计划只返回文件操作，不接受客户端执行状态回传
pub async fn update_plan(
    State(s): State<AppState>,
    Path(p): Path<String>,
    ApiJson(i): ApiJson<UpdatePlanRequest>,
) -> Result<Json<UpdatePlan>> {
    Ok(Json(releases::plan(&s, &p, i).await?))
}
