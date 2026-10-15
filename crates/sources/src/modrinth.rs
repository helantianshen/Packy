use packy_common::{
    error::{Error, ErrorCode, Result},
    validation,
};
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;

/// 服务端元数据查询条件，按 Modrinth facets 规范编码而非拼接原始查询字符串
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub query: Option<String>,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}

/// 仅访问公开 Modrinth v2 元数据，返回上游 JSON 以保留平台特有字段
#[derive(Clone)]
pub struct Modrinth {
    client: Client,
}

impl Modrinth {
    /// User-Agent 由部署配置提供，客户端超时与重定向策略沿用下载模块
    pub fn new(user_agent: &str) -> std::result::Result<Self, reqwest::Error> {
        Ok(Self {
            client: crate::download::http_client(user_agent)?,
        })
    }

    /// 搜索只限定 Mod 项目，版本与加载器分别组成 AND 条件
    pub async fn search(&self, input: &Search) -> Result<Value> {
        let limit = input.limit.unwrap_or(20);
        if !(1..=100).contains(&limit) || input.query.as_ref().is_some_and(|s| s.len() > 256) {
            return Err(Error::invalid("搜索参数超出范围"));
        }
        let mut facets = vec![vec!["project_type:mod".to_owned()]];
        if let Some(version) = &input.minecraft_version {
            facets.push(vec![format!("versions:{version}")]);
        }
        if let Some(loader) = &input.loader {
            facets.push(vec![format!("categories:{loader}")]);
        }
        self.json(
            self.client
                .get("https://api.modrinth.com/v2/search")
                .query(&[
                    ("query", input.query.clone().unwrap_or_default()),
                    ("limit", limit.to_string()),
                    ("offset", input.offset.unwrap_or(0).to_string()),
                    ("facets", json!(facets).to_string()),
                ]),
        )
        .await
    }

    /// ID 或 slug 作为单个路径段处理，拒绝斜杠与 URL 注入
    pub async fn project(&self, id: &str) -> Result<Value> {
        valid_id(id)?;
        self.json(
            self.client
                .get(format!("https://api.modrinth.com/v2/project/{id}")),
        )
        .await
    }

    /// 上游版本筛选参数是 JSON 数组，不能直接传入逗号分隔文本
    pub async fn versions(&self, id: &str, input: &Search) -> Result<Value> {
        valid_id(id)?;
        let mut request = self
            .client
            .get(format!("https://api.modrinth.com/v2/project/{id}/version"));
        if let Some(v) = &input.minecraft_version {
            request = request.query(&[("game_versions", json!([v]).to_string())]);
        }
        if let Some(v) = &input.loader {
            request = request.query(&[("loaders", json!([v]).to_string())]);
        }
        self.json(request).await
    }

    /// 查询一个不可变版本的文件元数据
    pub async fn version(&self, id: &str) -> Result<Value> {
        valid_id(id)?;
        self.json(
            self.client
                .get(format!("https://api.modrinth.com/v2/version/{id}")),
        )
        .await
    }

    /// 批量文件查询只接受与安装校验相同的 SHA-512 算法
    pub async fn version_files(&self, hashes: &[String], algorithm: &str) -> Result<Value> {
        if algorithm != "sha512" || hashes.is_empty() || hashes.len() > 100 {
            return Err(Error::invalid("文件查询需要 1–100 个 SHA-512"));
        }
        for hash in hashes {
            validation::sha512(hash)?;
        }
        self.json(
            self.client
                .post("https://api.modrinth.com/v2/version_files")
                .json(&json!({"hashes": hashes, "algorithm": algorithm})),
        )
        .await
    }

    /// 限制响应体防止上游异常耗尽内存，不把上游正文或诊断直接返回管理端
    async fn json(&self, request: RequestBuilder) -> Result<Value> {
        let upstream = || Error::new(ErrorCode::UpstreamUnavailable, "Modrinth 暂时不可用");
        let mut response = request
            .timeout(Duration::from_secs(25))
            .send()
            .await
            .map_err(|_| upstream())?;
        match response.status().as_u16() {
            429 => {
                return Err(Error::new(
                    ErrorCode::RateLimited,
                    "Modrinth 请求过于频繁，请稍后重试",
                ));
            }
            404 => return Err(Error::new(ErrorCode::NotFound, "Modrinth 项目或版本不存在")),
            200..=299 => {}
            _ => return Err(upstream()),
        }
        let mut data = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| upstream())? {
            if data.len() + chunk.len() > 8 * 1024 * 1024 {
                return Err(upstream());
            }
            data.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&data).map_err(|_| upstream())
    }
}

fn valid_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(Error::invalid("Modrinth ID 或 slug 格式不正确"));
    }
    Ok(())
}
