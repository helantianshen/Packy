use super::*;

/// 固定来源编码，与下载源字典一致
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Modrinth,
    Curseforge,
    Github,
    Server,
}

impl SourceType {
    /// 编码是数据库和网络协议的一部分，不能使用本地化显示名替代
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Modrinth => "modrinth",
            Self::Curseforge => "curseforge",
            Self::Github => "github",
            Self::Server => "server",
        }
    }
}

/// 整合包当前信息，正式发布历史由 Release 保存
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub loader: String,
    /// 当前正式发布，尚无发布时为 null
    pub current_release_id: Option<String>,
}

/// 管理员可编辑的整合包属性，不接受客户端指定资源 ID
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct PackInput {
    pub name: String,
    pub minecraft_version: String,
    pub loader: String,
}

/// 具体文件版本及其不可变校验信息
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub id: String,
    /// 稳定内部 ID，与可修改的业务 modid 区分
    pub mod_id: String,
    pub version: String,
    pub filename: String,
    #[ts(type = "number | null")]
    /// 文件字节数，未知时为 null
    pub size: Option<i64>,
    /// 实际文件的 128 位小写十六进制 SHA-512
    pub sha512: String,
    pub hosted_file_id: Option<String>,
    /// 同一文件的备用直链，必须对应相同字节
    pub sources: Vec<DownloadSource>,
}

/// 录入文件时提交的哈希与直链，不能把平台密钥作为下载凭据
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ArtifactInput {
    /// 稳定内部 ID，与可修改的业务 modid 区分
    pub mod_id: String,
    pub version: String,
    pub filename: String,
    #[ts(type = "number | null")]
    /// 文件字节数，未知时为 null
    pub size: Option<i64>,
    /// 实际文件的 128 位小写十六进制 SHA-512
    pub sha512: String,
    pub hosted_file_id: Option<String>,
    /// 同一文件的备用直链，必须对应相同字节
    pub sources: Vec<DownloadSource>,
}

/// 公开下载资源，不包含服务器存储路径
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct HostedFile {
    pub id: String,
    pub filename: String,
    #[ts(type = "number")]
    /// 托管文件已知的实际字节数
    pub size: i64,
    /// 实际文件的 128 位小写十六进制 SHA-512
    pub sha512: String,
    pub url: String,
}

/// 来源显示与排序，禁用只影响后续计划
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SourceDictionaryItem {
    pub code: SourceType,
    pub name: String,
    pub enabled: bool,
    #[ts(type = "number")]
    pub sort_order: i64,
}

/// 稳定内部身份与可修改业务 modid，名称允许重复
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ModRecord {
    pub id: String,
    /// 大小写敏感的业务唯一键
    pub modid: String,
    pub name: String,
    pub modrinth_project_id: Option<String>,
    pub curseforge_project_id: Option<String>,
}

/// Mod 可编辑字段，缺省平台 ID 按完整提交语义清空
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ModInput {
    /// 大小写敏感的业务唯一键
    pub modid: String,
    pub name: String,
    pub modrinth_project_id: Option<String>,
    pub curseforge_project_id: Option<String>,
}
