use crate::{
    error::{AppError, Result},
    infrastructure::{accounts, database::Database, system},
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use packy_common::{
    error::ErrorCode,
    models::{Account, Credentials, LoginResponse},
    validation,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// 密码计算使用有界并发，避免公开认证入口无限占用阻塞线程
static HASH_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(8);
fn hash_slot() -> Result<tokio::sync::SemaphorePermit<'static>> {
    HASH_SLOTS
        .try_acquire()
        .map_err(|_| AppError::business(ErrorCode::RateLimited, "认证请求繁忙，请稍后重试"))
}
use uuid::Uuid;

/// JWT 只保存身份和撤销版本，角色权限始终由数据库判定
#[derive(Clone, Serialize, Deserialize)]
struct Claims {
    sub: String,
    iss: String,
    aud: String,
    iat: i64,
    exp: i64,
    jti: String,
    token_version: i64,
}

/// 预生成不存在账号使用的密码哈希，避免每个无效请求重新生成高成本哈希
#[derive(Clone)]
pub struct Auth {
    secret: Arc<str>,
    dummy_hash: Arc<str>,
}

impl Auth {
    /// 密钥必须由部署者提供，长度不足时拒绝启动
    pub async fn new(secret: &str) -> Result<Self> {
        if secret.len() < 32 {
            return Err(AppError::business(
                ErrorCode::InvalidRequest,
                "JWT 密钥长度不足",
            ));
        }
        let dummy_hash = hash_password(Uuid::new_v4().to_string()).await?;
        Ok(Self {
            secret: secret.into(),
            dummy_hash: dummy_hash.into(),
        })
    }

    /// 密码验证在阻塞线程执行，签发前再次确认账号和撤销版本
    pub async fn login(&self, db: &Database, input: Credentials) -> Result<LoginResponse> {
        validation::username(&input.username)?;
        validation::password(&input.password)?;
        let record =
            accounts::login_record(&mut *db.pool.acquire().await?, &input.username).await?;
        let hash = record
            .as_ref()
            .map(|r| r.password_hash.clone())
            .unwrap_or_else(|| self.dummy_hash.to_string());
        let slot = hash_slot()?;
        let valid = tokio::task::spawn_blocking(move || bcrypt::verify(input.password, &hash))
            .await
            .map_err(anyhow::Error::from)?
            .map_err(anyhow::Error::from)?;
        drop(slot);
        let record = record
            .filter(|r| valid && r.enabled)
            .ok_or_else(unauthenticated)?;
        let (account, version) =
            accounts::identity(&mut *db.pool.acquire().await?, &record.id).await?;
        if !account.enabled || version != record.token_version {
            return Err(unauthenticated());
        }
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: account.id,
            iss: "packy".into(),
            aud: "packy-admin".into(),
            iat: now,
            exp: now + 1800,
            jti: Uuid::new_v4().to_string(),
            token_version: version,
        };
        let access_token = jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(anyhow::Error::from)?;
        Ok(LoginResponse {
            access_token,
            token_type: "Bearer".into(),
            expires_in: 1800,
        })
    }

    /// 固定算法、签发方和受众，注销、禁用或改角色均可使旧 Token 立即失效
    pub async fn authenticate(&self, db: &Database, token: &str) -> Result<Account> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["packy"]);
        validation.set_audience(&["packy-admin"]);
        validation.set_required_spec_claims(&["sub", "iss", "aud", "iat", "exp", "jti"]);
        validation.leeway = 0;
        let claims = jsonwebtoken::decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map_err(|_| unauthenticated())?
        .claims;
        if claims.iat > chrono::Utc::now().timestamp() {
            return Err(unauthenticated());
        }
        let identity = accounts::identity(&mut *db.pool.acquire().await?, &claims.sub).await;
        let (account, version) = match identity {
            Err(AppError::Business(error)) if error.code == ErrorCode::NotFound => {
                return Err(unauthenticated());
            }
            other => other?,
        };
        if !account.enabled || version != claims.token_version {
            return Err(unauthenticated());
        }
        Ok(account)
    }
}

/// 哈希在写事务之外生成，避免密码计算期间阻塞数据库所有读写
pub async fn register(db: &Database, input: Credentials) -> Result<Account> {
    validation::username(&input.username)?;
    validation::password(&input.password)?;
    if !system::registration_enabled(&mut *db.pool.acquire().await?).await? {
        return Err(registration_closed());
    }
    let hash = hash_password(input.password).await?;
    let mut tx = db.begin_write().await?;
    // 预检查仅节省计算，事务内判定才决定是否允许创建账号
    if !system::registration_enabled(&mut tx).await? {
        return Err(registration_closed());
    }
    let id = Uuid::new_v4().to_string();
    accounts::insert_user(&mut tx, &id, &input.username, &hash).await?;
    let account = accounts::identity(&mut tx, &id).await?.0;
    tx.commit().await?;
    Ok(account)
}

/// 部署命令通过参数化 SQL 创建首个超管，重复调用不改变已有账号
pub async fn bootstrap(db: &Database, input: Credentials) -> Result<bool> {
    validation::username(&input.username)?;
    validation::password(&input.password)?;
    let hash = hash_password(input.password).await?;
    let mut tx = db.begin_write().await?;
    let created =
        accounts::bootstrap(&mut tx, &Uuid::new_v4().to_string(), &input.username, &hash).await?;
    tx.commit().await?;
    Ok(created)
}

/// 注销使用持久化版本撤销此账号所有已签发 Token
pub async fn logout(db: &Database, account: &Account) -> Result<()> {
    let mut tx = db.begin_write().await?;
    accounts::revoke(&mut tx, &account.id).await?;
    tx.commit().await?;
    Ok(())
}

/// 以当前 RBAC 为准，系统管理还必须具有超管角色
pub fn require(account: &Account, permission: &str, super_only: bool) -> Result<()> {
    if !account.permissions.iter().any(|p| p == permission)
        || (super_only && !account.roles.iter().any(|r| r == "super_admin"))
    {
        return Err(AppError::business(
            ErrorCode::Forbidden,
            "没有执行此操作的权限",
        ));
    }
    Ok(())
}

async fn hash_password(password: String) -> Result<String> {
    let _slot = hash_slot()?;
    Ok(
        tokio::task::spawn_blocking(move || bcrypt::hash(password, 12))
            .await
            .map_err(anyhow::Error::from)?
            .map_err(anyhow::Error::from)?,
    )
}

fn unauthenticated() -> AppError {
    AppError::business(ErrorCode::Unauthenticated, "账号、密码或登录状态无效")
}
fn registration_closed() -> AppError {
    AppError::business(ErrorCode::RegistrationClosed, "当前未开放注册")
}
