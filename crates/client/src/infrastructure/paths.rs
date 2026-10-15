use crate::error::Result;
use packy_common::{error::Error, validation};
use std::path::{Path, PathBuf};

/// 校验指定相对路径的每一段，拒绝符号链接和 Windows 重解析点，不扫描名单外目录
pub fn checked(root: &Path, relative: &str) -> Result<PathBuf> {
    validation::relative_path(relative)?;
    let mut current = root.to_path_buf();
    for component in relative.split('/') {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if linked(&metadata) {
                    return Err(Error::invalid("更新路径包含符号链接或目录联接").into());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(current)
}

/// 游戏实例允许整包根目录本身，其他位置必须使用安全相对路径
pub fn game(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative == "." {
        Ok(root.to_path_buf())
    } else {
        checked(root, relative)
    }
}

fn linked(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
