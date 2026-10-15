use super::extract::{Actor, ApiJson, ApiQuery};
use crate::{
    application::{AppState, catalog, system::Pagination},
    error::Result,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
};
use packy_common::models::*;

/// 目录接口不读写文件或执行 SQL，统一经过应用层输入和权限检查
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/packs", get(packs).post(create_pack))
        .route("/packs/{id}", get(pack).put(update_pack))
        .route("/mods", get(mods).post(create_mod))
        .route("/mods/{id}", patch(update_mod))
        .route("/artifacts", get(artifacts).post(create_artifact))
}
async fn packs(
    State(s): State<AppState>,
    Actor(a): Actor,
    ApiQuery(p): ApiQuery<Pagination>,
) -> Result<Json<Page<Pack>>> {
    Ok(Json(catalog::packs(&s, &a, p).await?))
}
async fn pack(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path(id): Path<String>,
) -> Result<Json<Pack>> {
    Ok(Json(catalog::pack(&s, &a, &id).await?))
}
async fn create_pack(
    State(s): State<AppState>,
    Actor(a): Actor,
    ApiJson(i): ApiJson<PackInput>,
) -> Result<(StatusCode, Json<Pack>)> {
    Ok((
        StatusCode::CREATED,
        Json(catalog::save_pack(&s, &a, None, i).await?),
    ))
}
async fn update_pack(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path(id): Path<String>,
    ApiJson(i): ApiJson<PackInput>,
) -> Result<Json<Pack>> {
    Ok(Json(catalog::save_pack(&s, &a, Some(&id), i).await?))
}
async fn mods(
    State(s): State<AppState>,
    Actor(a): Actor,
    ApiQuery(p): ApiQuery<catalog::ModQuery>,
) -> Result<Json<Page<ModRecord>>> {
    Ok(Json(catalog::mods(&s, &a, p).await?))
}
async fn create_mod(
    State(s): State<AppState>,
    Actor(a): Actor,
    ApiJson(i): ApiJson<ModInput>,
) -> Result<(StatusCode, Json<ModRecord>)> {
    Ok((
        StatusCode::CREATED,
        Json(catalog::save_mod(&s, &a, None, i).await?),
    ))
}
async fn update_mod(
    State(s): State<AppState>,
    Actor(a): Actor,
    Path(id): Path<String>,
    ApiJson(i): ApiJson<ModInput>,
) -> Result<Json<ModRecord>> {
    Ok(Json(catalog::save_mod(&s, &a, Some(&id), i).await?))
}
async fn artifacts(
    State(s): State<AppState>,
    Actor(a): Actor,
    ApiQuery(p): ApiQuery<catalog::ArtifactQuery>,
) -> Result<Json<Page<Artifact>>> {
    Ok(Json(catalog::artifacts(&s, &a, p).await?))
}
async fn create_artifact(
    State(s): State<AppState>,
    Actor(a): Actor,
    ApiJson(i): ApiJson<ArtifactInput>,
) -> Result<(StatusCode, Json<Artifact>)> {
    Ok((
        StatusCode::CREATED,
        Json(catalog::create_artifact(&s, &a, i).await?),
    ))
}
