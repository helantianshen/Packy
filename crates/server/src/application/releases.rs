use super::{AppState, auth::require, catalog::id, system::Pagination};
use crate::{
    domain::releases as rules,
    error::{AppError, Result},
    infrastructure::{catalog, releases, system},
};
use packy_common::{error::ErrorCode, models::*, validation::text};

/// 管理员看到草稿和正式发布，公开计划接口只接受正式发布
pub async fn list(
    s: &AppState,
    a: &Account,
    pack: &str,
    page: Pagination,
) -> Result<Page<Release>> {
    require(a, "release:read", false)?;
    id(pack)?;
    let offset = page.offset()?;
    let mut conn = s.db.pool.acquire().await?;
    catalog::pack(&mut conn, pack).await?;
    let (items, total) = releases::list(&mut conn, pack, offset, page.page_size).await?;
    Ok(Page {
        items,
        total,
        page: page.page,
        page_size: page.page_size,
    })
}
/// 完整文件清单中的名字和 modid 均来自发布快照
pub async fn get(s: &AppState, a: &Account, pack: &str, key: &str) -> Result<Release> {
    require(a, "release:read", false)?;
    id(pack)?;
    id(key)?;
    releases::get(&mut *s.db.pool.acquire().await?, pack, key).await
}
/// 创建事务不包含网络请求，基础版本必须是同包正式版本
pub async fn create(
    s: &AppState,
    a: &Account,
    pack: &str,
    input: CreateDraftRequest,
) -> Result<Release> {
    require(a, "release:write", false)?;
    id(pack)?;
    text(&input.version, 80, "版本号")?;
    changelog(&input.changelog)?;
    let mut tx = s.db.begin_write().await?;
    let pack = catalog::pack(&mut tx, pack).await?;
    let base = input
        .base_release_id
        .as_deref()
        .or(pack.current_release_id.as_deref());
    if let Some(key) = base {
        let base = releases::get(&mut tx, &pack.id, key).await?;
        if base.status != "published" {
            return Err(AppError::not_found());
        }
    }
    let key = uuid::Uuid::new_v4().to_string();
    releases::insert(&mut tx, &key, &pack, &input, base).await?;
    let result = releases::get(&mut tx, &pack.id, &key).await?;
    tx.commit().await?;
    Ok(result)
}
/// 清单一次性替换，任一非法路径或引用导致整个事务回滚
pub async fn update(
    s: &AppState,
    a: &Account,
    pack: &str,
    key: &str,
    input: UpdateDraftRequest,
) -> Result<Release> {
    require(a, "release:write", false)?;
    id(pack)?;
    id(key)?;
    text(&input.version, 80, "版本号")?;
    changelog(&input.changelog)?;
    if input.items.len() > 2000 {
        return Err(packy_common::error::Error::invalid("清单过大").into());
    }
    let mut tx = s.db.begin_write().await?;
    let draft = releases::get(&mut tx, pack, key).await?;
    check_revision(&draft, input.revision)?;
    let mut items = Vec::with_capacity(input.items.len());
    for item in &input.items {
        id(&item.mod_id)?;
        id(&item.artifact_id)?;
        let m = catalog::mod_record(&mut tx, &item.mod_id).await?;
        let artifact = catalog::artifact(&mut tx, &item.artifact_id).await?;
        if artifact.mod_id != m.id {
            return Err(packy_common::error::Error::invalid("文件不属于指定 Mod").into());
        }
        items.push(ReleaseItem {
            mod_id: m.id,
            artifact_id: artifact.id,
            modid_snapshot: m.modid,
            name_snapshot: m.name,
            target: TargetFile {
                path: item.target_path.clone(),
                size: artifact.size,
                sha512: artifact.sha512,
                sources: artifact.sources,
            },
        });
    }
    rules::validate_snapshot(&items)?;
    releases::update(&mut tx, key, &input, &items).await?;
    let result = releases::get(&mut tx, pack, key).await?;
    tx.commit().await?;
    Ok(result)
}
/// 预览与发布都对比当前正式版，避免管理员从旧版起草时忽略后来发生的变更
pub async fn preview(
    s: &AppState,
    a: &Account,
    pack: &str,
    key: &str,
    revision: i64,
    publish: bool,
) -> Result<(UpdatePlan, Release)> {
    require(
        a,
        if publish {
            "release:publish"
        } else {
            "release:read"
        },
        false,
    )?;
    id(pack)?;
    id(key)?;
    let mut tx = s.db.begin_write().await?;
    let pack = catalog::pack(&mut tx, pack).await?;
    let draft = releases::get(&mut tx, &pack.id, key).await?;
    check_revision(&draft, revision)?;
    let old = match &pack.current_release_id {
        Some(key) => releases::get(&mut tx, &pack.id, key).await?.items,
        None => Vec::new(),
    };
    rules::validate_snapshot(&draft.items)?;
    let mut operations = rules::difference(&old, &draft.items)?;
    let dictionary = system::sources(&mut tx).await?;
    filter_sources(&mut operations, &dictionary)?;
    if publish {
        // 正式快照保留完整来源，后续计划只使用当时仍启用的来源
        let mut all: Vec<_> = draft
            .items
            .iter()
            .map(|item| Operation {
                action: Action::Add,
                mod_id: item.mod_id.clone(),
                old_path: None,
                target: Some(item.target.clone()),
            })
            .collect();
        filter_sources(&mut all, &dictionary)?;
        releases::publish(&mut tx, &pack.id, key, pack.current_release_id.as_deref()).await?;
    }
    let result = releases::get(&mut tx, &pack.id, key).await?;
    tx.commit().await?;
    Ok((
        UpdatePlan {
            pack_id: pack.id,
            pack_name: pack.name,
            target_release_id: draft.id,
            target_version: draft.version,
            changelog: draft.changelog,
            operations,
        },
        result,
    ))
}
/// 公开接口只在本次检查时读取当前正式指针，不创建服务端执行任务
pub async fn plan(s: &AppState, pack: &str, input: UpdatePlanRequest) -> Result<UpdatePlan> {
    id(pack)?;
    id(&input.installed_release_id)?;
    text(&input.client_version, 80, "客户端版本")?;
    let mut conn = s.db.pool.acquire().await?;
    let pack = catalog::pack(&mut conn, pack).await?;
    let old = releases::get(&mut conn, &pack.id, &input.installed_release_id).await?;
    if old.status != "published" {
        return Err(AppError::not_found());
    }
    let current = pack
        .current_release_id
        .as_ref()
        .ok_or_else(|| AppError::business(ErrorCode::NoPublishedRelease, "尚无正式发布"))?;
    let latest = releases::get(&mut conn, &pack.id, current).await?;
    if latest.status != "published" {
        return Err(AppError::business(
            ErrorCode::NoPublishedRelease,
            "当前发布状态异常",
        ));
    }
    let mut operations = rules::difference(&old.items, &latest.items)?;
    filter_sources(&mut operations, &system::sources(&mut conn).await?)?;
    Ok(UpdatePlan {
        pack_id: pack.id,
        pack_name: pack.name,
        target_release_id: latest.id,
        target_version: latest.version,
        changelog: latest.changelog,
        operations,
    })
}
fn check_revision(draft: &Release, revision: i64) -> Result<()> {
    if draft.status != "draft" || draft.revision != revision {
        return Err(AppError::business(
            ErrorCode::Conflict,
            "草稿已被修改或发布，请重新加载",
        ));
    }
    Ok(())
}
fn changelog(value: &str) -> Result<()> {
    if value.len() > 64 * 1024 {
        return Err(packy_common::error::Error::invalid("更新说明过长").into());
    }
    Ok(())
}
fn filter_sources(operations: &mut [Operation], dictionary: &[SourceDictionaryItem]) -> Result<()> {
    for operation in operations {
        if let Some(target) = &mut operation.target {
            target.sources.retain(|source| {
                dictionary
                    .iter()
                    .any(|entry| entry.code == source.r#type && entry.enabled)
            });
            target.sources.sort_by_key(|source| {
                (
                    source.priority,
                    dictionary
                        .iter()
                        .find(|entry| entry.code == source.r#type)
                        .map_or(i64::MAX, |entry| entry.sort_order),
                )
            });
            if target.sources.is_empty() {
                return Err(AppError::business(
                    ErrorCode::SourceUnavailable,
                    "更新文件没有启用的下载来源",
                ));
            }
        }
    }
    Ok(())
}
