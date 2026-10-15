use packy_common::{
    error::{Error, Result},
    models::{DownloadSource, SourceType},
    validation,
};
use reqwest::{Client, RequestBuilder, header};
use std::time::Duration;

/// 将已下发的直链转换为来源请求，不能在失败重试时解析另一个版本或索取后台令牌
pub trait DownloadProvider: Send + Sync {
    /// 用于匹配数据库字典中的固定来源编码
    fn source_type(&self) -> SourceType;
    /// 只能添加公开的请求头，服务器密钥不得进入客户端下载链路
    fn request(&self, client: &Client, source: &DownloadSource) -> Result<RequestBuilder>;
}

/// Modrinth CDN 文件直链适配
pub struct ModrinthProvider;
/// CurseForge 已解析下载链接适配，不调用需要 API Key 的元数据接口
pub struct CurseForgeProvider;
/// GitHub Release 资产链接适配，允许客户端跟随 HTTPS 资产重定向
pub struct GitHubProvider;
/// 自托管文件端点适配，普通玩家下载不携带管理端 JWT
pub struct ServerProvider;

fn direct_request(
    client: &Client,
    source: &DownloadSource,
    expected: SourceType,
) -> Result<RequestBuilder> {
    if source.r#type != expected {
        return Err(Error::invalid("下载来源与适配器不匹配"));
    }
    let url = validation::http_url(&source.url)?;
    Ok(client.get(url).header(header::ACCEPT_ENCODING, "identity"))
}

impl DownloadProvider for ModrinthProvider {
    fn source_type(&self) -> SourceType {
        SourceType::Modrinth
    }
    fn request(&self, client: &Client, source: &DownloadSource) -> Result<RequestBuilder> {
        direct_request(client, source, self.source_type())
    }
}
impl DownloadProvider for CurseForgeProvider {
    fn source_type(&self) -> SourceType {
        SourceType::Curseforge
    }
    fn request(&self, client: &Client, source: &DownloadSource) -> Result<RequestBuilder> {
        direct_request(client, source, self.source_type())
    }
}
impl DownloadProvider for GitHubProvider {
    fn source_type(&self) -> SourceType {
        SourceType::Github
    }
    fn request(&self, client: &Client, source: &DownloadSource) -> Result<RequestBuilder> {
        Ok(direct_request(client, source, self.source_type())?
            .header(header::ACCEPT, "application/octet-stream"))
    }
}
impl DownloadProvider for ServerProvider {
    fn source_type(&self) -> SourceType {
        SourceType::Server
    }
    fn request(&self, client: &Client, source: &DownloadSource) -> Result<RequestBuilder> {
        direct_request(client, source, self.source_type())
    }
}

/// 固定四类来源的分派点，非法字典编码由协议反序列化直接拒绝
pub fn provider(source: SourceType) -> &'static dyn DownloadProvider {
    match source {
        SourceType::Modrinth => &ModrinthProvider,
        SourceType::Curseforge => &CurseForgeProvider,
        SourceType::Github => &GitHubProvider,
        SourceType::Server => &ServerProvider,
    }
}

/// 共用 HTTP/HTTPS 客户端，限制连接和读取停顿，拒绝重定向降级以及凭据 URL
pub fn http_client(user_agent: &str) -> std::result::Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 8
                || validation::http_url(attempt.url().as_str()).is_err()
                || (attempt.url().scheme() == "http"
                    && attempt.previous().iter().any(|url| url.scheme() == "https"))
            {
                attempt.error("下载重定向不符合安全策略")
            } else {
                attempt.follow()
            }
        }))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adapters_preserve_the_record_and_exclude_secrets()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let client = http_client("Packy/test")?;
        for kind in [
            SourceType::Modrinth,
            SourceType::Curseforge,
            SourceType::Github,
            SourceType::Server,
        ] {
            let source = DownloadSource {
                r#type: kind,
                url: "https://example.org/a.jar?version=1".into(),
                priority: 0,
            };
            let request = provider(kind).request(&client, &source)?.build()?;
            assert_eq!(request.url().as_str(), source.url);
            assert!(!request.headers().contains_key(header::AUTHORIZATION));
            assert!(!request.headers().contains_key("x-api-key"));
        }
        Ok(())
    }
}
