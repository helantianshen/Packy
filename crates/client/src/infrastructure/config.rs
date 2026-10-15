use crate::error::Result;
use packy_common::models::LocalConfig;

// 根据编译目标选择配置，交叉编译时不受构建宿主系统影响
#[cfg(target_os = "windows")]
const CONFIG: &str = include_str!("../../../../config/client.windows.yaml");
#[cfg(target_os = "linux")]
const CONFIG: &str = include_str!("../../../../config/client.linux.yaml");
#[cfg(target_os = "macos")]
const CONFIG: &str = include_str!("../../../../config/client.macos.yaml");
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
compile_error!("Packy 客户端仅支持 Windows、Linux 和 macOS");

/// 返回当前编译目标的内置参数，不读取运行时文件或配置覆盖
pub fn bundled_config() -> Result<LocalConfig> {
    Ok(serde_yaml_ng::from_str(CONFIG)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_platform_yaml_configs_are_valid() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        for (yaml, launcher) in [
            (
                include_str!("../../../../config/client.windows.yaml"),
                "HMCL.exe",
            ),
            (
                include_str!("../../../../config/client.linux.yaml"),
                "launcher.sh",
            ),
            (
                include_str!("../../../../config/client.macos.yaml"),
                "launcher.sh",
            ),
        ] {
            let config: LocalConfig = serde_yaml_ng::from_str(yaml)?;
            uuid::Uuid::parse_str(&config.pack_id)?;
            packy_common::validation::http_url(&config.server_url)?;
            assert_eq!(config.pack_name, "蒹葭服务器整合包");
            assert_eq!(config.launcher_path, launcher);
            assert_eq!(config.game_dir, ".minecraft");
        }
        Ok(())
    }
}
