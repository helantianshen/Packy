use super::extract::{Actor, ApiJson, ApiQuery};
use crate::{
    application::{AppState, auth::require},
    error::Result,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use packy_sources::modrinth::Search;
use serde::Deserialize;
use serde_json::Value;

/// 只代理需求中的公开元数据查询，管理端不能指定任意上游 URL
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/search", get(search))
        .route("/projects/{id}", get(project))
        .route("/projects/{id}/versions", get(versions))
        .route("/versions/{id}", get(version))
        .route("/version-files", post(version_files))
}
async fn search(
    State(s): State<AppState>,
    Actor(a): Actor,
    ApiQuery(input): ApiQuery<Search>,
) -> Result<Json<Value>> {
    require(&a, "mod:read", false)?;
    Ok(Json(s.modrinth.search(&input).await?))
}
async fn project(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&a, "mod:read", false)?;
    Ok(Json(s.modrinth.project(&id).await?))
}
async fn versions(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path(id): Path<String>,
    ApiQuery(input): ApiQuery<Search>,
) -> Result<Json<Value>> {
    require(&a, "mod:read", false)?;
    Ok(Json(s.modrinth.versions(&id, &input).await?))
}
async fn version(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    require(&a, "mod:read", false)?;
    Ok(Json(s.modrinth.version(&id).await?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Hashes {
    hashes: Vec<String>,
    algorithm: String,
}
async fn version_files(
    State(s): State<AppState>,
    Actor(a): Actor,
    ApiJson(input): ApiJson<Hashes>,
) -> Result<Json<Value>> {
    require(&a, "mod:read", false)?;
    Ok(Json(
        s.modrinth
            .version_files(&input.hashes, &input.algorithm)
            .await?,
    ))
}
