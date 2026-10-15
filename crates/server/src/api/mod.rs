mod auth;
mod catalog;
mod error;
mod extract;
mod files;
mod modrinth;
mod releases;
mod system;

use crate::{application::AppState, error::AppError};
use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use packy_common::models::Health;
use tower_http::trace::TraceLayer;

/// HTTP 装配入口，服务端不承担 TLS，部署时由 Nginx 反向代理
pub fn router(state: AppState) -> Router {
    let admin_dir = state.config.admin_dir.clone();
    Router::new()
        .nest_service("/admin", tower_http::services::ServeDir::new(admin_dir))
        .route("/healthz", get(health))
        .nest("/api/v1/auth", auth::routes())
        .nest(
            "/api/v1/admin",
            catalog::routes()
                .merge(releases::routes())
                .merge(files::routes())
                .nest("/system", system::routes())
                .nest("/modrinth", modrinth::routes()),
        )
        .route(
            "/api/v1/packs/{id}/update-plan",
            post(releases::update_plan),
        )
        .route("/api/v1/files/{id}/download", get(files::download))
        .fallback(|| async { AppError::not_found() })
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        phase: "P3".into(),
    })
}
