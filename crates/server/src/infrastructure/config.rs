use anyhow::Context;
use packy_common::validation;
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf};

/// 仅服务端持有的部署配置，密钥不实现 Debug，避免误写入日志
#[derive(Clone)]
pub struct Config {
    pub listen: SocketAddr,
    pub admin_dir: PathBuf,
    pub database: PathBuf,
    pub files_dir: PathBuf,
    pub public_url: String,
    pub jwt_secret: String,
    pub modrinth_user_agent: String,
}

/// 文件配置与进程环境合并后的参数，尚未执行 HTTP 服务专用校验
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    listen: String,
    admin_dir: PathBuf,
    pub database: PathBuf,
    files_dir: PathBuf,
    public_url: String,
    jwt_secret: String,
    modrinth_user_agent: String,
    pub log_filter: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:8080".into(),
            admin_dir: "web/admin/dist".into(),
            database: "data/packy.db".into(),
            files_dir: "data/files".into(),
            public_url: "http://127.0.0.1:8080".into(),
            jwt_secret: String::new(),
            modrinth_user_agent: "Packy/0.1.0".into(),
            log_filter: "packy_server=info,tower_http=info".into(),
        }
    }
}

impl Settings {
    /// 相对路径以工作目录解析，显式指定的配置文件缺失时拒绝启动
    pub fn load() -> anyhow::Result<Self> {
        let explicit = std::env::var_os("PACKY_CONFIG");
        let path = explicit
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| "config/server.yaml".into());
        let mut settings: Self = match std::fs::read_to_string(&path) {
            Ok(content) => serde_yaml_ng::from_str(&content).map_err(|_| {
                anyhow::anyhow!(
                    "配置文件 {} 无效，请检查 YAML 语法、字段名及字段类型",
                    path.display()
                )
            })?,
            Err(error) if explicit.is_none() && error.kind() == std::io::ErrorKind::NotFound => {
                Self::default()
            }
            Err(error) => {
                return Err(error).with_context(|| format!("无法读取配置文件 {}", path.display()));
            }
        };
        settings.listen = env_or("PACKY_LISTEN", settings.listen)?;
        settings.public_url = env_or("PACKY_PUBLIC_URL", settings.public_url)?;
        settings.jwt_secret = env_or("PACKY_JWT_SECRET", settings.jwt_secret)?;
        settings.modrinth_user_agent =
            env_or("PACKY_MODRINTH_USER_AGENT", settings.modrinth_user_agent)?;
        settings.log_filter = env_or("RUST_LOG", settings.log_filter)?;
        for (name, field) in [
            ("PACKY_DATABASE", &mut settings.database),
            ("PACKY_FILES_DIR", &mut settings.files_dir),
            ("PACKY_ADMIN_DIR", &mut settings.admin_dir),
        ] {
            if let Some(value) = std::env::var_os(name) {
                *field = value.into();
            }
            anyhow::ensure!(!field.as_os_str().is_empty(), "{name} 路径不能为空");
        }
        anyhow::ensure!(
            !settings.modrinth_user_agent.trim().is_empty(),
            "Modrinth User-Agent 不能为空"
        );
        anyhow::ensure!(!settings.log_filter.trim().is_empty(), "日志过滤器不能为空");
        Ok(settings)
    }

    /// 校验 HTTP 服务需要的密钥、外部地址与监听地址
    pub fn into_config(self) -> anyhow::Result<Config> {
        anyhow::ensure!(
            self.jwt_secret.len() >= 32 && !self.jwt_secret.trim().is_empty(),
            "必须通过 PACKY_JWT_SECRET 或配置文件的 jwt_secret 设置至少 32 字节的随机密钥"
        );
        let url = validation::http_url(&self.public_url).map_err(|_| {
            anyhow::anyhow!("PACKY_PUBLIC_URL / public_url 必须是有效的 HTTP/HTTPS 地址")
        })?;
        anyhow::ensure!(
            url.query().is_none(),
            "PACKY_PUBLIC_URL / public_url 不能包含查询参数"
        );
        Ok(Config {
            listen: self
                .listen
                .parse()
                .context("PACKY_LISTEN / listen 必须是有效的 IP:端口")?,
            admin_dir: self.admin_dir,
            database: self.database,
            files_dir: self.files_dir,
            public_url: self.public_url.trim_end_matches('/').into(),
            jwt_secret: self.jwt_secret,
            modrinth_user_agent: self.modrinth_user_agent,
        })
    }
}

fn env_or(name: &str, fallback: String) -> anyhow::Result<String> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Ok(fallback),
        Err(_) => anyhow::bail!("环境变量 {name} 必须是有效的 Unicode 文本"),
    }
}
