use crate::error::{Error, Result};
use url::Url;

/// 接受小写 SHA-512 十六进制，避免将其他哈希算法或非规范值带入安装流程
pub fn sha512(value: &str) -> Result<()> {
    if value.len() != 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::invalid("SHA-512 必须是 128 位小写十六进制"));
    }
    Ok(())
}

/// 校验下载直链和服务器地址，不允许嵌入凭据或片段
pub fn http_url(value: &str) -> Result<Url> {
    let url = Url::parse(value).map_err(|_| Error::invalid("URL 格式不正确"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::invalid(
            "地址必须使用 HTTP 或 HTTPS，且不能包含用户名、密码或片段",
        ));
    }
    Ok(url)
}

/// 在所有宿主系统上应用 Windows 相对路径规则，实际文件访问还须检查符号链接和联接
pub fn relative_path(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 1024 || value.contains('\\') || value.starts_with('/') {
        return Err(Error::invalid("路径必须是使用 / 分隔的相对路径"));
    }
    for part in value.split('/') {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with([' ', '.'])
            || part
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
        {
            return Err(Error::invalid("路径包含越界片段或 Windows 不允许的名称"));
        }
        let base = part.split('.').next().unwrap_or_default().to_uppercase();
        if matches!(
            base.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || ["COM", "LPT"].iter().any(|prefix| {
            base.strip_prefix(prefix).is_some_and(|n| {
                matches!(
                    n,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        }) {
            return Err(Error::invalid("路径不能使用 Windows 设备名称"));
        }
    }
    Ok(())
}

/// 更新计划只能操作实例目录下 mods 子树中的 JAR，保留合法嵌套路径
pub fn mod_path(value: &str) -> Result<()> {
    relative_path(value)?;
    if !value.starts_with("mods/") || !value.to_lowercase().ends_with(".jar") {
        return Err(Error::invalid("更新目标必须是 mods/ 下的 JAR 文件"));
    }
    Ok(())
}

/// 账号使用 ASCII 名称，唯一性由 SQLite NOCASE 约束负责
pub fn username(value: &str) -> Result<()> {
    if !(3..=32).contains(&value.len())
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err(Error::invalid("账号应为 3–32 位字母、数字、下划线或短横线"));
    }
    Ok(())
}

/// bcrypt 输入按 UTF-8 字节计数，拒绝超长输入以防止静默截断
pub fn password(value: &str) -> Result<()> {
    if !(8..=72).contains(&value.len()) {
        return Err(Error::invalid("密码长度应为 8–72 个 UTF-8 字节"));
    }
    Ok(())
}

/// 文件路径之间不能形成父子关系，否则部分安装后无法按原名单幂等重试
pub fn non_overlapping_paths<'a>(paths: impl IntoIterator<Item = &'a str>) -> Result<()> {
    let paths: std::collections::BTreeSet<String> =
        paths.into_iter().map(str::to_lowercase).collect();
    for path in &paths {
        for (index, _) in path.match_indices('/') {
            if paths.contains(&path[..index]) {
                return Err(Error::invalid(
                    "名单中的文件路径不能互为父目录，请调整目标路径",
                ));
            }
        }
    }
    Ok(())
}

/// 共享短文本约束，按 UTF-8 字节限制长度并拒绝空白和控制字符
pub fn text(value: &str, max: usize, label: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(Error::invalid(format!("{label}为空、过长或包含控制字符")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_follow_windows_rules_on_every_platform() {
        for value in [
            "/mods/a.jar",
            "mods/../a.jar",
            "mods/CON.jar",
            "mods/LPT¹.jar",
            "mods/a:ads.jar",
            "mods/a /b.jar",
            "mods\\a.jar",
            "mods/a.jar/",
            "mods//a.jar",
        ] {
            assert!(mod_path(value).is_err(), "{value}");
        }
        assert!(mod_path("mods/中文 路径/demo.jar").is_ok());
        assert!(http_url("https://u:p@example.org/a.jar").is_err());
        assert!(sha512(&"a".repeat(128)).is_ok());
        assert!(sha512(&"A".repeat(128)).is_err());
        assert!(password(&"中".repeat(25)).is_err());
        assert!(
            non_overlapping_paths(["mods/a.jar", "mods/a.jar-b.jar", "mods/a.jar/b.jar"]).is_err()
        );
        assert!(non_overlapping_paths(["mods/A.jar", "mods/a.jar/b.jar"]).is_err());
        assert!(non_overlapping_paths(["mods/a.jar", "mods/a.jar", "mods/b.jar"]).is_ok());
    }
}
