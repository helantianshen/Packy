use super::paths;
use crate::error::Result;
use fs2::FileExt;
use packy_common::{error::Error, models::*, validation};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

/// 锁句柄存活期间独占整个整合包，避免多个进程同时修改名单文件
pub struct PackLock {
    _file: File,
}
impl PackLock {
    /// 锁文件位于固定本地状态目录，文件本身不存业务状态
    pub fn acquire(root: &Path) -> Result<Self> {
        let updater = paths::checked(root, ".updater")?;
        std::fs::create_dir_all(&updater)?;
        let path = paths::checked(root, ".updater/packy.lock")?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        file.try_lock_exclusive()
            .map_err(|_| Error::invalid("此整合包已有另一个更新器正在运行"))?;
        Ok(Self { _file: file })
    }
}

/// 读取小型 JSON 前限制长度，损坏或丢失的版本不能通过猜测恢复
pub async fn read_json<T: DeserializeOwned>(root: &Path, relative: &str) -> Result<T> {
    let path = paths::checked(root, relative)?;
    let metadata = tokio::fs::metadata(&path).await.map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            crate::error::Error::Rule(Error::invalid(format!(
                "缺少本地文件 {relative}，请联系整合包管理员准备配置或版本记录"
            )))
        } else {
            crate::error::Error::Io(error)
        }
    })?;
    if metadata.len() > 2 * 1024 * 1024 {
        return Err(Error::invalid("本地配置或任务文件过大").into());
    }
    Ok(serde_json::from_slice(&tokio::fs::read(path).await?)?)
}

/// 同目录临时文件同步后替换目标，不依赖 staging 与目标位于同一磁盘卷
pub async fn write_json<T: Serialize>(root: &Path, relative: &str, value: &T) -> Result<()> {
    let path = paths::checked(root, relative)?;
    let parent = path
        .parent()
        .ok_or_else(|| Error::invalid("本地状态路径无效"))?;
    tokio::fs::create_dir_all(parent).await?;
    let temporary = parent.join(format!(".packy-{}.tmp", uuid::Uuid::new_v4()));
    let mut cleanup = TemporaryFile::new(temporary.clone());
    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .await?;
    file.write_all(&serde_json::to_vec_pretty(value)?).await?;
    file.sync_all().await?;
    drop(file);
    paths::checked(root, relative)?;
    tokio::fs::rename(&temporary, &path).await?;
    cleanup.keep();
    Ok(())
}

/// 配置约束与平台无关，Windows 不能接受 Linux 上看似正常的设备路径
pub fn validate_config(config: &LocalConfig, state: &LocalState) -> Result<()> {
    uuid::Uuid::parse_str(&config.pack_id).map_err(|_| Error::invalid("整合包 ID 不正确"))?;
    uuid::Uuid::parse_str(&state.installed_release_id)
        .map_err(|_| Error::invalid("本地已安装版本 ID 不正确"))?;
    if config.pack_name.trim().is_empty() || config.pack_name.chars().count() > 100 {
        return Err(Error::invalid("整合包名称应为 1–100 个字符").into());
    }
    let url = validation::http_url(&config.server_url)?;
    if url.query().is_some() {
        return Err(Error::invalid("服务器地址不能包含查询参数").into());
    }
    if config.game_dir != "." {
        validation::relative_path(&config.game_dir)?;
    }
    validation::relative_path(&config.launcher_path)?;
    Ok(())
}

/// 仅删除自己创建的单个临时文件，失败记录诊断而不掩盖原始业务错误
pub struct TemporaryFile {
    path: PathBuf,
    keep: bool,
}
impl TemporaryFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path, keep: false }
    }
    pub fn keep(&mut self) {
        self.keep = true;
    }
}
impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if !self.keep
            && let Err(error) = std::fs::remove_file(&self.path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, "清理客户端临时文件失败");
        }
    }
}
