use super::{AppState, auth::require, catalog::id};
use crate::{
    error::Result,
    infrastructure::files::{self, Upload},
};
use packy_common::{
    models::{Account, HostedFile},
    validation,
};
use std::path::PathBuf;

/// 上传前校验权限和单个文件名，玩家下载端点不复用此权限
pub async fn begin_upload(s: &AppState, a: &Account, filename: &str) -> Result<Upload> {
    require(a, "file:write", false)?;
    validation::relative_path(filename)?;
    if filename.contains('/') || !filename.to_lowercase().ends_with(".jar") {
        return Err(packy_common::error::Error::invalid("只允许上传单个 JAR 文件").into());
    }
    Upload::new(&s.config.files_dir, filename.into()).await
}
/// 保证文件同步成功后才提交数据库，不把尚未写完的内容暴露为文件资源
pub async fn finish_upload(s: &AppState, mut upload: Upload) -> Result<HostedFile> {
    let file = upload.seal().await?;
    let mut tx = s.db.begin_write().await?;
    files::insert(&mut tx, &file).await?;
    tx.commit().await?;
    upload.persist();
    Ok(HostedFile {
        id: file.id.clone(),
        filename: file.filename,
        size: file.size,
        sha512: file.sha512,
        url: format!("{}/api/v1/files/{}/download", s.config.public_url, file.id),
    })
}
/// 存储键由服务器生成，但读取时仍校验以防损坏数据突破托管目录
pub async fn download(s: &AppState, key: &str) -> Result<PathBuf> {
    id(key)?;
    let file = files::record(&mut *s.db.pool.acquire().await?, key).await?;
    validation::relative_path(&file.storage_key)?;
    if file.storage_key != format!("{key}.jar") {
        return Err(crate::error::AppError::not_found());
    }
    let path = s.config.files_dir.join(&file.storage_key);
    let metadata = match tokio::fs::symlink_metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(crate::error::AppError::not_found());
        }
        Err(error) => return Err(error.into()),
    };
    if !metadata.file_type().is_file() || metadata.len() != file.size as u64 {
        return Err(crate::error::AppError::not_found());
    }
    Ok(path)
}
