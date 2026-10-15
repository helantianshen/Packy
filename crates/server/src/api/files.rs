use super::extract::Actor;
use crate::{
    application::{AppState, files},
    error::{AppError, Result},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use packy_common::{error::Error, models::HostedFile};
use tower::ServiceExt;
use tower_http::services::ServeFile;

/// multipart 体的限制稍高于文件上限，文件字节仍在上传守卫内精确计数
pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/files",
        post(upload).layer(DefaultBodyLimit::max(512 * 1024 * 1024 + 64 * 1024)),
    )
}
async fn upload(
    State(s): State<AppState>,
    Actor(a): Actor,
    multipart: std::result::Result<Multipart, axum::extract::multipart::MultipartRejection>,
) -> Result<(StatusCode, Json<HostedFile>)> {
    let mut multipart =
        multipart.map_err(|_| Error::invalid("需要 multipart/form-data 文件上传"))?;
    let mut field = multipart
        .next_field()
        .await
        .map_err(|_| Error::invalid("上传内容不完整"))?
        .ok_or_else(|| Error::invalid("缺少文件字段"))?;
    if field.name() != Some("file") {
        return Err(Error::invalid("文件字段必须命名为 file").into());
    }
    let filename = field
        .file_name()
        .ok_or_else(|| Error::invalid("缺少 JAR 文件名"))?
        .to_owned();
    let mut upload = files::begin_upload(&s, &a, &filename).await?;
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|_| Error::invalid("上传被中断或超过大小限制"))?
    {
        upload.write(&chunk).await?;
    }
    // multer 在字段句柄释放后才允许读取下一个字段，即使内容已经耗尽
    drop(field);
    if multipart
        .next_field()
        .await
        .map_err(|_| Error::invalid("上传内容不完整"))?
        .is_some()
    {
        return Err(Error::invalid("每次只能上传一个文件").into());
    }
    Ok((
        StatusCode::CREATED,
        Json(files::finish_upload(&s, upload).await?),
    ))
}
/// 标准文件响应处理 Range，路径先由应用层按文件 ID 解析
pub async fn download(
    State(s): State<AppState>,
    Path(id): Path<String>,
    request: Request,
) -> Result<Response> {
    let path = files::download(&s, &id).await?;
    let response = ServeFile::new(path)
        .oneshot(request)
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))?;
    if response.status() == StatusCode::NOT_FOUND {
        return Err(AppError::not_found());
    }
    Ok(response.into_response())
}
