use crate::error::Result;
use sqlx::{
    Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::{path::Path, time::Duration};

/// 单连接写库保证本进程事务顺序，SQLite 写锁同时约束外部初始化进程
#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    /// 每个连接启用外键和等待时间，WAL 持久化在真实文件库上生效
    pub async fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            tokio::fs::create_dir_all(parent).await?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5))
            .journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        let database = Self { pool };
        database.migrate().await?;
        Ok(database)
    }

    /// 提前取得写锁，注册判定和设置更新均以该事务的提交顺序生效
    pub async fn begin_write(&self) -> Result<Transaction<'static, Sqlite>> {
        Ok(self.pool.begin_with("BEGIN IMMEDIATE").await?)
    }

    /// 结构与种子在一个事务中初始化，失败时不留下半套结构或覆盖已有配置
    async fn migrate(&self) -> Result<()> {
        let mut transaction = self.begin_write().await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY)")
            .execute(&mut *transaction)
            .await?;
        for (version, sql) in [
            (1, include_str!("../../../../migrations/0001_schema.sql")),
            (2, include_str!("../../../../migrations/0002_seed.sql")),
        ] {
            let exists: i64 =
                sqlx::query_scalar("SELECT count(*) FROM schema_migrations WHERE version=?")
                    .bind(version)
                    .fetch_one(&mut *transaction)
                    .await?;
            if exists == 0 {
                sqlx::raw_sql(sql).execute(&mut *transaction).await?;
                sqlx::query("INSERT INTO schema_migrations(version) VALUES (?)")
                    .bind(version)
                    .execute(&mut *transaction)
                    .await?;
            }
        }
        transaction.commit().await?;
        Ok(())
    }
}
