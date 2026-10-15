use crate::error::{AppError, Result};
use sha2::{Digest, Sha512};
use sqlx::{FromRow, SqliteConnection};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

/// 托管文件大小上限与 API 契约一致，multipart 包装开销由接口层另行限制
pub const MAX_FILE_SIZE: u64 = 512 * 1024 * 1024;

/// 数据库记录只在服务端内部包含存储键，API 不返回实际磁盘路径
#[derive(FromRow)]
pub struct FileRecord {
    pub id: String,
    pub storage_key: String,
    pub filename: String,
    pub size: i64,
    pub sha512: String,
}

/// 按不可变文件 ID 查找，下载地址中的任意路径片段不能变成存储路径
pub async fn record(conn: &mut SqliteConnection, id: &str) -> Result<FileRecord> {
    Ok(sqlx::query_as(
        "SELECT id, storage_key, filename, size, sha512 FROM hosted_files WHERE id=?",
    )
    .bind(id)
    .fetch_one(conn)
    .await?)
}
/// 只保存已落盘文件的真实大小和哈希，事务失败时由上传守卫清理文件
pub async fn insert(conn: &mut SqliteConnection, file: &FileRecord) -> Result<()> {
    sqlx::query(
        "INSERT INTO hosted_files(id, storage_key, filename, size, sha512) VALUES(?, ?, ?, ?, ?)",
    )
    .bind(&file.id)
    .bind(&file.storage_key)
    .bind(&file.filename)
    .bind(file.size)
    .bind(&file.sha512)
    .execute(conn)
    .await?;
    Ok(())
}

/// 上传守卫在取消、无效输入或数据库失败时删除自己的临时文件
pub struct Upload {
    file: Option<tokio::fs::File>,
    path: PathBuf,
    root: PathBuf,
    id: String,
    filename: String,
    size: u64,
    hasher: Sha512,
    prefix: Vec<u8>,
    persisted: bool,
}
impl Upload {
    /// 随机名称配合 create_new 避免覆盖已有文件，上传文件名不参与磁盘寻址
    pub async fn new(root: &Path, filename: String) -> Result<Self> {
        tokio::fs::create_dir_all(root).await?;
        let id = uuid::Uuid::new_v4().to_string();
        let path = root.join(format!("{id}.upload"));
        let file = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .await?;
        Ok(Self {
            file: Some(file),
            path,
            root: root.to_path_buf(),
            id,
            filename,
            size: 0,
            hasher: Sha512::new(),
            prefix: Vec::with_capacity(4),
            persisted: false,
        })
    }
    /// 分块写入和计数防止内存缓冲整个 JAR，超限立即停止
    pub async fn write(&mut self, chunk: &[u8]) -> Result<()> {
        if self.size + chunk.len() as u64 > MAX_FILE_SIZE {
            return Err(packy_common::error::Error::invalid("JAR 文件超过 512 MiB").into());
        }
        let required = 4 - self.prefix.len();
        self.prefix
            .extend_from_slice(&chunk[..chunk.len().min(required)]);
        self.file
            .as_mut()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("上传文件已关闭")))?
            .write_all(chunk)
            .await?;
        self.hasher.update(chunk);
        self.size += chunk.len() as u64;
        Ok(())
    }
    /// 文件落盘并关闭后再更名，Windows 下不能依赖打开文件可被重命名
    pub async fn seal(&mut self) -> Result<FileRecord> {
        if self.size == 0
            || ![
                b"PK\x03\x04".as_slice(),
                b"PK\x05\x06".as_slice(),
                b"PK\x07\x08".as_slice(),
            ]
            .contains(&self.prefix.as_slice())
        {
            return Err(packy_common::error::Error::invalid("上传内容不是 JAR/ZIP 文件").into());
        }
        if let Some(file) = self.file.take() {
            file.sync_all().await?;
            drop(file);
        }
        let storage_key = format!("{}.jar", self.id);
        let destination = self.root.join(&storage_key);
        tokio::fs::rename(&self.path, &destination).await?;
        self.path = destination;
        Ok(FileRecord {
            id: self.id.clone(),
            storage_key,
            filename: self.filename.clone(),
            size: self.size as i64,
            sha512: hex::encode(self.hasher.clone().finalize()),
        })
    }
    /// 只有数据库提交成功后才保留文件，避免失败上传留下可发布记录
    pub fn persist(mut self) {
        self.persisted = true;
    }
}
impl Drop for Upload {
    fn drop(&mut self) {
        // Drop 不能等待异步 I/O，关闭句柄后删除单个自有文件，不扫描托管目录
        drop(self.file.take());
        if !self.persisted
            && let Err(error) = std::fs::remove_file(&self.path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, "清理未完成上传失败");
        }
    }
}
