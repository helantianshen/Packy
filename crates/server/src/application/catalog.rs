use super::{AppState, auth::require, system::Pagination};
use crate::{error::Result, infrastructure::catalog};
use packy_common::validation::text;
use packy_common::{error::Error, models::*, validation};
use serde::Deserialize;
use uuid::Uuid;

/// API 资源 ID 必须是 UUID，不能把任意路径当成资源标识
pub fn id(value: &str) -> Result<()> {
    Uuid::parse_str(value).map_err(|_| Error::invalid("资源 ID 格式不正确"))?;
    Ok(())
}
fn pack_input(input: &PackInput) -> Result<()> {
    text(&input.name, 200, "整合包名称")?;
    text(&input.minecraft_version, 40, "游戏版本")?;
    if !matches!(input.loader.as_str(), "neoforge" | "forge" | "fabric") {
        return Err(Error::invalid("不支持的加载器").into());
    }
    Ok(())
}
fn mod_input(input: &ModInput) -> Result<()> {
    text(&input.modid, 128, "modid")?;
    text(&input.name, 200, "Mod 名称")?;
    for value in [&input.modrinth_project_id, &input.curseforge_project_id]
        .into_iter()
        .flatten()
    {
        text(value, 128, "平台项目 ID")?;
    }
    Ok(())
}
/// 列表请求包含稳定分页，不能返回无界数据集
pub async fn packs(s: &AppState, a: &Account, page: Pagination) -> Result<Page<Pack>> {
    require(a, "pack:read", false)?;
    let offset = page.offset()?;
    let (items, total) =
        catalog::packs(&mut *s.db.pool.acquire().await?, offset, page.page_size).await?;
    Ok(Page {
        items,
        total,
        page: page.page,
        page_size: page.page_size,
    })
}
/// 获取单个整合包，权限依据数据库实时账号视图
pub async fn pack(s: &AppState, a: &Account, key: &str) -> Result<Pack> {
    require(a, "pack:read", false)?;
    id(key)?;
    catalog::pack(&mut *s.db.pool.acquire().await?, key).await
}
/// 创建和修改仅共享输入约束，不允许修改当前发布指针
pub async fn save_pack(
    s: &AppState,
    a: &Account,
    key: Option<&str>,
    input: PackInput,
) -> Result<Pack> {
    require(a, "pack:write", false)?;
    pack_input(&input)?;
    let is_new = key.is_none();
    let created = Uuid::new_v4().to_string();
    let key = key.unwrap_or(&created);
    id(key)?;
    let mut tx = s.db.begin_write().await?;
    if is_new {
        catalog::insert_pack(&mut tx, key, &input).await?;
    } else {
        catalog::update_pack(&mut tx, key, &input).await?;
    }
    let result = catalog::pack(&mut tx, key).await?;
    tx.commit().await?;
    Ok(result)
}

/// 搜索关键词和分页独立验证，仓储对 LIKE 通配符转义
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub keyword: Option<String>,
}
/// Mod 搜索只作用于受管理主表，不访问玩家文件
pub async fn mods(s: &AppState, a: &Account, input: ModQuery) -> Result<Page<ModRecord>> {
    require(a, "mod:read", false)?;
    let page = Pagination {
        page: input.page.unwrap_or(1),
        page_size: input.page_size.unwrap_or(20),
    };
    let offset = page.offset()?;
    let keyword = input.keyword.unwrap_or_default();
    if keyword.len() > 200 {
        return Err(Error::invalid("搜索词过长").into());
    }
    let (items, total) = catalog::mods(
        &mut *s.db.pool.acquire().await?,
        &keyword,
        offset,
        page.page_size,
    )
    .await?;
    Ok(Page {
        items,
        total,
        page: page.page,
        page_size: page.page_size,
    })
}
/// 修改业务 modid 时保留内部 ID，唯一键冲突作为 409 返回
pub async fn save_mod(
    s: &AppState,
    a: &Account,
    key: Option<&str>,
    input: ModInput,
) -> Result<ModRecord> {
    require(a, "mod:write", false)?;
    mod_input(&input)?;
    let is_new = key.is_none();
    let created = Uuid::new_v4().to_string();
    let key = key.unwrap_or(&created);
    id(key)?;
    let mut tx = s.db.begin_write().await?;
    if is_new {
        catalog::insert_mod(&mut tx, key, &input).await?;
    } else {
        catalog::update_mod(&mut tx, key, &input).await?;
    }
    let result = catalog::mod_record(&mut tx, key).await?;
    tx.commit().await?;
    Ok(result)
}

/// 文件版本列表可按 Mod 内部 ID 过滤
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub mod_id: Option<String>,
}
/// 录入的来源未按字典禁用过滤，管理员仍能查看和维护完整记录
pub async fn artifacts(s: &AppState, a: &Account, input: ArtifactQuery) -> Result<Page<Artifact>> {
    require(a, "mod:read", false)?;
    let page = Pagination {
        page: input.page.unwrap_or(1),
        page_size: input.page_size.unwrap_or(20),
    };
    let offset = page.offset()?;
    if let Some(key) = &input.mod_id {
        id(key)?;
    }
    let (items, total) = catalog::artifacts(
        &mut *s.db.pool.acquire().await?,
        input.mod_id.as_deref(),
        offset,
        page.page_size,
    )
    .await?;
    Ok(Page {
        items,
        total,
        page: page.page,
        page_size: page.page_size,
    })
}
/// 所有来源共享同一实际文件哈希，托管文件引用还要验证存储记录一致
pub async fn create_artifact(s: &AppState, a: &Account, input: ArtifactInput) -> Result<Artifact> {
    require(a, "mod:write", false)?;
    id(&input.mod_id)?;
    text(&input.version, 100, "Mod 版本")?;
    validation::relative_path(&input.filename)?;
    if input.filename.contains('/') || !input.filename.to_lowercase().ends_with(".jar") {
        return Err(Error::invalid("文件名必须是单个 JAR 名称").into());
    }
    validation::sha512(&input.sha512)?;
    if input.size.is_some_and(|size| size < 0)
        || input.sources.is_empty()
        || input.sources.len() > 16
    {
        return Err(Error::invalid("文件大小或来源数量不正确").into());
    }
    let mut urls = std::collections::HashSet::new();
    for source in &input.sources {
        validation::http_url(&source.url)?;
        if source.priority < 0 || !urls.insert((source.r#type.as_str(), &source.url)) {
            return Err(Error::invalid("来源排序不能为负数且来源不能重复").into());
        }
    }
    let mut tx = s.db.begin_write().await?;
    catalog::mod_record(&mut tx, &input.mod_id).await?;
    if let Some(file_id) = &input.hosted_file_id {
        id(file_id)?;
        let file = crate::infrastructure::files::record(&mut tx, file_id).await?;
        if file.sha512 != input.sha512 || input.size != Some(file.size) {
            return Err(Error::invalid("托管文件大小或哈希与录入信息不一致").into());
        }
        let url = format!("{}/api/v1/files/{file_id}/download", s.config.public_url);
        if !input
            .sources
            .iter()
            .any(|source| source.r#type == SourceType::Server && source.url == url)
        {
            return Err(Error::invalid("托管文件必须包含本服务对应的下载链接").into());
        }
    }
    let key = Uuid::new_v4().to_string();
    catalog::insert_artifact(&mut tx, &key, &input).await?;
    let result = catalog::artifact(&mut tx, &key).await?;
    tx.commit().await?;
    Ok(result)
}
