use crate::error::{AppError, Result};
use packy_common::models::{Account, Permission, Role};
use sqlx::{FromRow, SqliteConnection};

/// 密码哈希仅在认证用例内可见，不序列化为 API 资源
#[derive(FromRow)]
pub struct LoginRecord {
    pub id: String,
    pub password_hash: String,
    pub enabled: bool,
    pub token_version: i64,
}

/// 账号不存在时返回 None，认证层仍执行一次密码验证以缩小时间差
pub async fn login_record(
    conn: &mut SqliteConnection,
    username: &str,
) -> Result<Option<LoginRecord>> {
    Ok(sqlx::query_as(
        "SELECT id, password_hash, enabled, token_version FROM accounts WHERE username=?",
    )
    .bind(username)
    .fetch_optional(conn)
    .await?)
}

/// 按稳定账号 ID 读取实时状态，旧 JWT 内不存角色和权限
pub async fn identity(conn: &mut SqliteConnection, id: &str) -> Result<(Account, i64)> {
    let row: Option<(String, bool, i64)> =
        sqlx::query_as("SELECT username, enabled, token_version FROM accounts WHERE id=?")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?;
    let (username, enabled, token_version) = row.ok_or_else(AppError::not_found)?;
    let roles = sqlx::query_scalar(
        "SELECT role_code FROM account_roles WHERE account_id=? ORDER BY role_code",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    let permissions = sqlx::query_scalar(
        "SELECT DISTINCT rp.permission_code FROM role_permissions rp JOIN account_roles ar ON
     rp.role_code=ar.role_code WHERE ar.account_id=? ORDER BY rp.permission_code",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    Ok((
        Account {
            id: id.into(),
            username,
            enabled,
            roles,
            permissions,
        },
        token_version,
    ))
}

/// 新注册账号的角色由服务器固定，不能接受请求体传入角色
pub async fn insert_user(
    conn: &mut SqliteConnection,
    id: &str,
    username: &str,
    hash: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO accounts(id, username, password_hash) VALUES(?, ?, ?)")
        .bind(id)
        .bind(username)
        .bind(hash)
        .execute(&mut *conn)
        .await?;
    sqlx::query("INSERT INTO account_roles(account_id, role_code) VALUES(?, 'user')")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 增加版本使此账号全部旧 Token 失效
pub async fn revoke(conn: &mut SqliteConnection, id: &str) -> Result<()> {
    sqlx::query("UPDATE accounts SET token_version=token_version+1 WHERE id=?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 仅在没有超管时建立首个超管，账号和角色写入同一事务
pub async fn bootstrap(
    conn: &mut SqliteConnection,
    id: &str,
    username: &str,
    hash: &str,
) -> Result<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_roles WHERE role_code='super_admin'")
            .fetch_one(&mut *conn)
            .await?;
    if count > 0 {
        return Ok(false);
    }
    sqlx::query("INSERT INTO accounts(id, username, password_hash) VALUES(?, ?, ?)")
        .bind(id)
        .bind(username)
        .bind(hash)
        .execute(&mut *conn)
        .await?;
    sqlx::query("INSERT INTO account_roles(account_id, role_code) VALUES(?, 'super_admin')")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(true)
}

/// 分页查询只返回可公开账号字段，不读取密码哈希
pub async fn list(
    conn: &mut SqliteConnection,
    offset: i64,
    limit: i64,
) -> Result<(Vec<Account>, i64)> {
    let total = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&mut *conn)
        .await?;
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM accounts ORDER BY created_at, id LIMIT ? OFFSET ?")
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *conn)
            .await?;
    let mut items = Vec::with_capacity(ids.len());
    for id in ids {
        items.push(identity(conn, &id).await?.0);
    }
    Ok((items, total))
}

/// 更新账号时始终在同一事务检查最后一位启用超管，避免并发管理操作互相绕过
pub async fn update(
    conn: &mut SqliteConnection,
    id: &str,
    enabled: bool,
    roles: &[String],
) -> Result<()> {
    let current = identity(conn, id).await?.0;
    if current.enabled
        && current.roles.iter().any(|r| r == "super_admin")
        && (!enabled || !roles.iter().any(|r| r == "super_admin"))
    {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM accounts a JOIN account_roles ar ON ar.account_id=a.id WHERE
     a.enabled=1 AND ar.role_code='super_admin'",
        )
        .fetch_one(&mut *conn)
        .await?;
        if count <= 1 {
            return Err(AppError::business(
                packy_common::error::ErrorCode::Conflict,
                "至少保留一位启用的超管",
            ));
        }
    }
    sqlx::query("UPDATE accounts SET enabled=?,  token_version=token_version+1 WHERE id=?")
        .bind(enabled)
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM account_roles WHERE account_id=?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    for role in roles {
        sqlx::query("INSERT INTO account_roles(account_id, role_code) VALUES(?, ?)")
            .bind(id)
            .bind(role)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

/// 角色和权限始终从字典读取，返回顺序稳定
pub async fn roles(conn: &mut SqliteConnection) -> Result<Vec<Role>> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT code, name FROM roles ORDER BY code")
        .fetch_all(&mut *conn)
        .await?;
    let mut items = Vec::new();
    for (code, name) in rows {
        let permissions = sqlx::query_scalar(
    "SELECT permission_code FROM role_permissions WHERE role_code=? ORDER BY permission_code",
)
            .bind(&code)
.fetch_all(&mut *conn).await?;
        items.push(Role {
            code,
            name,
            permissions,
        });
    }
    Ok(items)
}

/// 权限编码由初始化 SQL 定义，界面不能创建任意权限
pub async fn permissions(conn: &mut SqliteConnection) -> Result<Vec<Permission>> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT code, name FROM permissions ORDER BY code")
            .fetch_all(conn)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(code, name)| Permission { code, name })
        .collect())
}

/// 由外键拒绝未知权限，事务失败时保留原角色权限
pub async fn set_permissions(
    conn: &mut SqliteConnection,
    role: &str,
    permissions: &[String],
) -> Result<()> {
    sqlx::query("DELETE FROM role_permissions WHERE role_code=?")
        .bind(role)
        .execute(&mut *conn)
        .await?;
    for code in permissions {
        sqlx::query("INSERT INTO role_permissions(role_code, permission_code) VALUES(?, ?)")
            .bind(role)
            .bind(code)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}
