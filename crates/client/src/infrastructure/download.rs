use super::{paths, storage::TemporaryFile};
use crate::error::{Error, Result};
use packy_common::{error::Error as RuleError, models::TargetFile};
use sha2::{Digest, Sha512};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

/// 每次使用 staging 文件都重新核对哈希，绝不把文件存在视为完整下载
pub async fn verified(path: &Path, target: &TargetFile) -> Result<bool> {
    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let mut hash = Sha512::new();
    let mut buffer = vec![0; 64 * 1024];
    let mut length = 0_i64;
    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
        length += read as i64;
    }
    Ok(target.size.is_none_or(|size| size == length)
        && hex::encode(hash.finalize()) == target.sha512)
}

/// 每个文件共尝试三次，按原记录的来源排序轮换，不请求服务端刷新链接
pub async fn file<F, Fut>(
    client: &reqwest::Client,
    root: &Path,
    relative: &str,
    target: &TargetFile,
    cancel: &CancellationToken,
    mut progress: F,
) -> Result<PathBuf>
where
    F: FnMut(i64) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    if target.sources.is_empty() {
        return Err(RuleError::invalid("下载记录缺少来源").into());
    }
    let destination = paths::checked(root, relative)?;
    if verified(&destination, target).await? {
        progress(tokio::fs::metadata(&destination).await?.len() as i64).await;
        return Ok(destination);
    }
    let mut sources = target.sources.clone();
    sources.sort_by_key(|source| source.priority);
    let mut last = Error::conflict("没有可用的下载来源");
    for attempt in 0..3 {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        progress(0).await;
        let temporary = destination.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
        let mut cleanup = TemporaryFile::new(temporary.clone());
        let source = &sources[attempt % sources.len()];
        let request = packy_sources::download::provider(source.r#type).request(client, source)?;
        let result = async {
            let response = tokio::select! { biased;
                _ = cancel.cancelled() => return Err(Error::Cancelled),
                result = request.send() => result?,
            };
            let mut response = response.error_for_status()?;
            let mut output = tokio::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .await?;
            let mut hash = Sha512::new();
            let mut length = 0_i64;
            loop {
                let chunk = tokio::select! { biased;
                    _ = cancel.cancelled() => return Err(Error::Cancelled),
                    result = response.chunk() => result?,
                };
                let Some(chunk) = chunk else {
                    break;
                };
                length = length
                    .checked_add(chunk.len() as i64)
                    .ok_or_else(|| RuleError::invalid("下载文件过大"))?;
                if target.size.is_some_and(|size| length > size) {
                    return Err(RuleError::invalid("下载内容超过记录中的文件大小").into());
                }
                output.write_all(&chunk).await?;
                // 等待已提交的暂存写入完成，再在下一次网络等待时响应取消
                output.flush().await?;
                hash.update(&chunk);
                progress(length).await;
            }
            output.sync_all().await?;
            drop(output);
            if target.size.is_some_and(|size| size != length)
                || hex::encode(hash.finalize()) != target.sha512
            {
                return Err(RuleError::invalid("下载文件大小或 SHA-512 校验失败").into());
            }
            paths::checked(root, relative)?;
            tokio::fs::rename(&temporary, &destination).await?;
            cleanup.keep();
            Ok::<_, Error>(())
        }
        .await;
        match result {
            Ok(()) => return Ok(destination),
            Err(Error::Cancelled) => return Err(Error::Cancelled),
            Err(error) => last = error,
        }
        if attempt < 2 {
            tokio::select! {_ = cancel.cancelled()=>return Err(Error::Cancelled),_ = tokio::time::sleep(std::time::Duration::from_millis(500*(attempt as u64+1)))=>{}}
        }
    }
    Err(last)
}
