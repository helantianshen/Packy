use super::auth::require;
use crate::{
    error::{AppError, Result},
    infrastructure::{accounts, database::Database, system},
};
use packy_common::{
    error::{Error, ErrorCode},
    models::*,
};
use serde::Deserialize;
use std::collections::HashSet;

/// 分页参数在进入仓储前统一验证，避免负偏移和无限列表
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pagination {
    #[serde(default = "first_page")]
    pub page: i64,
    #[serde(default = "page_size")]
    pub page_size: i64,
}
fn first_page() -> i64 {
    1
}
fn page_size() -> i64 {
    20
}
impl Pagination {
    /// 返回安全偏移，不允许乘法溢出
    pub fn offset(&self) -> Result<i64> {
        if self.page < 1 || !(1..=100).contains(&self.page_size) {
            return Err(Error::invalid("分页参数超出范围").into());
        }
        (self.page - 1)
            .checked_mul(self.page_size)
            .ok_or_else(|| Error::invalid("页码超出范围").into())
    }
}

/// 公开注册状态仅用于界面展示，注册用例仍在写事务内重新检查
pub async fn settings(db: &Database) -> Result<SystemSettings> {
    let registration_enabled = system::registration_enabled(&mut *db.pool.acquire().await?).await?;
    Ok(SystemSettings {
        registration_enabled,
    })
}

/// 注册开关与注册创建使用相同的 SQLite 写锁顺序
pub async fn update_settings(
    db: &Database,
    actor: &Account,
    input: SystemSettings,
) -> Result<SystemSettings> {
    require(actor, "settings:write", true)?;
    let mut tx = db.begin_write().await?;
    system::set_registration(&mut tx, input.registration_enabled).await?;
    tx.commit().await?;
    Ok(input)
}

/// 返回账号及其当前有效权限，不暴露密码或 Token 撤销版本
pub async fn users(db: &Database, actor: &Account, page: Pagination) -> Result<Page<Account>> {
    require(actor, "user:read", true)?;
    let offset = page.offset()?;
    let (items, total) =
        accounts::list(&mut *db.pool.acquire().await?, offset, page.page_size).await?;
    Ok(Page {
        items,
        total,
        page: page.page,
        page_size: page.page_size,
    })
}

/// 用户只能持有已定义的角色，重复角色直接拒绝，保留最后一位超管的检查位于事务中
pub async fn update_user(
    db: &Database,
    actor: &Account,
    id: &str,
    input: UpdateAccountRequest,
) -> Result<Account> {
    require(actor, "user:write", true)?;
    let unique: HashSet<_> = input.roles.iter().collect();
    if unique.len() != input.roles.len()
        || input.roles.is_empty()
        || input
            .roles
            .iter()
            .any(|role| role != "super_admin" && role != "user")
    {
        return Err(Error::invalid("账号需要有效且不重复的角色").into());
    }
    let mut tx = db.begin_write().await?;
    accounts::update(&mut tx, id, input.enabled, &input.roles).await?;
    let account = accounts::identity(&mut tx, id).await?.0;
    tx.commit().await?;
    Ok(account)
}

/// 角色列表包含当前权限，供管理端完整编辑
pub async fn roles(db: &Database, actor: &Account) -> Result<Vec<Role>> {
    require(actor, "permission:read", true)?;
    accounts::roles(&mut *db.pool.acquire().await?).await
}

/// 固定权限目录的中文显示名与初始化 SQL 一致
pub async fn permissions(db: &Database, actor: &Account) -> Result<Vec<Permission>> {
    require(actor, "permission:read", true)?;
    accounts::permissions(&mut *db.pool.acquire().await?).await
}

/// 系统管理权限必须保留在超管角色，其他权限可按业务需要授予普通用户
pub async fn update_permissions(
    db: &Database,
    actor: &Account,
    role: &str,
    input: UpdateRolePermissionsRequest,
) -> Result<Role> {
    require(actor, "permission:write", true)?;
    if !matches!(role, "super_admin" | "user") {
        return Err(AppError::not_found());
    }
    let unique: HashSet<_> = input.permissions.iter().map(String::as_str).collect();
    if unique.len() != input.permissions.len() {
        return Err(Error::invalid("权限列表不能重复").into());
    }
    if role == "super_admin"
        && [
            "user:read",
            "user:write",
            "permission:read",
            "permission:write",
            "settings:read",
            "settings:write",
        ]
        .iter()
        .any(|code| !unique.contains(code))
    {
        return Err(AppError::business(
            ErrorCode::Conflict,
            "必须保留超管的用户、权限和设置管理权限",
        ));
    }
    let mut tx = db.begin_write().await?;
    let known = accounts::permissions(&mut tx).await?;
    if input
        .permissions
        .iter()
        .any(|code| !known.iter().any(|item| &item.code == code))
    {
        return Err(Error::invalid("包含未知权限编码").into());
    }
    accounts::set_permissions(&mut tx, role, &input.permissions).await?;
    let result = accounts::roles(&mut tx)
        .await?
        .into_iter()
        .find(|r| r.code == role)
        .ok_or_else(AppError::not_found)?;
    tx.commit().await?;
    Ok(result)
}

/// 来源字典支持被授权的业务管理员维护，不要求超管身份
pub async fn sources(db: &Database, actor: &Account) -> Result<Vec<SourceDictionaryItem>> {
    require(actor, "source:read", false)?;
    system::sources(&mut *db.pool.acquire().await?).await
}

/// 请求体不接受修改来源代码，编码只来自路由参数
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceInput {
    pub name: String,
    pub enabled: bool,
    pub sort_order: i64,
}

/// 字典禁用只影响后续生成的计划，不删除已托管文件或修改历史快照
pub async fn update_source(
    db: &Database,
    actor: &Account,
    code: SourceType,
    input: SourceInput,
) -> Result<SourceDictionaryItem> {
    require(actor, "source:write", false)?;
    packy_common::validation::text(&input.name, 80, "来源名称")?;
    if input.sort_order < 0 {
        return Err(Error::invalid("来源名称或排序不正确").into());
    }
    let source = SourceDictionaryItem {
        code,
        name: input.name,
        enabled: input.enabled,
        sort_order: input.sort_order,
    };
    let mut tx = db.begin_write().await?;
    system::update_source(&mut tx, &source).await?;
    tx.commit().await?;
    Ok(source)
}
