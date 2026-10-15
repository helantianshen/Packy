use crate::error::{AppError, Result};
use packy_common::models::{SourceDictionaryItem, SourceType};
use sqlx::SqliteConnection;

/// 注册开关不缓存，每次在调用方事务内读取
pub async fn registration_enabled(conn: &mut SqliteConnection) -> Result<bool> {
    Ok(
        sqlx::query_scalar("SELECT registration_enabled FROM system_settings WHERE id=1")
            .fetch_one(conn)
            .await?,
    )
}

/// 设置提交后，之后取得写锁的注册请求立即使用新值
pub async fn set_registration(conn: &mut SqliteConnection, enabled: bool) -> Result<()> {
    sqlx::query("UPDATE system_settings SET registration_enabled=? WHERE id=1")
        .bind(enabled)
        .execute(conn)
        .await?;
    Ok(())
}

/// 来源字典只影响后续计划，已有任务中的 URL 不受影响
pub async fn sources(conn: &mut SqliteConnection) -> Result<Vec<SourceDictionaryItem>> {
    let rows: Vec<(String, String, bool, i64)> = sqlx::query_as(
        "SELECT code, name, enabled, sort_order FROM download_source_types ORDER BY sort_order,
     code",
    )
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|(code, name, enabled, sort_order)| {
            let code: SourceType = serde_json::from_value(serde_json::Value::String(code))?;
            Ok(SourceDictionaryItem {
                code,
                name,
                enabled,
                sort_order,
            })
        })
        .collect()
}

/// 固定编码只能修改名称、启用和排序，不提供删除或改码接口
pub async fn update_source(
    conn: &mut SqliteConnection,
    source: &SourceDictionaryItem,
) -> Result<()> {
    let result = sqlx::query(
        "UPDATE download_source_types SET name=?, enabled=?, sort_order=? WHERE code=?",
    )
    .bind(&source.name)
    .bind(source.enabled)
    .bind(source.sort_order)
    .bind(source.code.as_str())
    .execute(conn)
    .await?;
    if result.rows_affected() != 1 {
        return Err(AppError::not_found());
    }
    Ok(())
}
