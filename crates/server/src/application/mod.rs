pub mod auth;
pub mod system;

use crate::{
    error::Result,
    infrastructure::{config::Config, database::Database},
};
use std::sync::Arc;

/// 应用状态只共享线程安全资源，连接和事务由具体用例按需取得
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Arc<Config>,
    pub auth: auth::Auth,
    pub modrinth: packy_sources::modrinth::Modrinth,
}

impl AppState {
    /// 启动时验证配置和数据库初始化，失败不进入 HTTP 监听
    pub async fn open(config: Config) -> Result<Self> {
        let db = Database::open(&config.database).await?;
        let auth = auth::Auth::new(&config.jwt_secret).await?;
        let modrinth = packy_sources::modrinth::Modrinth::new(&config.modrinth_user_agent)
            .map_err(anyhow::Error::from)?;
        Ok(Self {
            db,
            config: Arc::new(config),
            auth,
            modrinth,
        })
    }
}

pub mod catalog;

pub mod releases;

pub mod files;
