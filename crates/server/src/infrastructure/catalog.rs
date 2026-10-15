use crate::error::{AppError, Result};
use packy_common::models::*;
use sqlx::{FromRow, SqliteConnection};

#[derive(FromRow)]
struct PackRow {
    id: String,
    name: String,
    minecraft_version: String,
    loader: String,
    current_release_id: Option<String>,
}
impl From<PackRow> for Pack {
    fn from(p: PackRow) -> Self {
        Self {
            id: p.id,
            name: p.name,
            minecraft_version: p.minecraft_version,
            loader: p.loader,
            current_release_id: p.current_release_id,
        }
    }
}
#[derive(FromRow)]
struct ModRow {
    id: String,
    modid: String,
    name: String,
    modrinth_project_id: Option<String>,
    curseforge_project_id: Option<String>,
}
impl From<ModRow> for ModRecord {
    fn from(m: ModRow) -> Self {
        Self {
            id: m.id,
            modid: m.modid,
            name: m.name,
            modrinth_project_id: m.modrinth_project_id,
            curseforge_project_id: m.curseforge_project_id,
        }
    }
}
#[derive(FromRow)]
struct ArtifactRow {
    id: String,
    mod_id: String,
    version: String,
    filename: String,
    size: Option<i64>,
    sha512: String,
    hosted_file_id: Option<String>,
}

/// 整合包 ID 稳定，修改基本信息不会覆盖历史发布快照
pub async fn pack(conn: &mut SqliteConnection, id: &str) -> Result<Pack> {
    Ok(sqlx::query_as::<_, PackRow>(
        "SELECT id, name, minecraft_version, loader, current_release_id FROM packs WHERE id=?",
    )
    .bind(id)
    .fetch_one(conn)
    .await?
    .into())
}
/// 列表排序由稳定主键确定，避免分页结果随查询计划改变
pub async fn packs(
    conn: &mut SqliteConnection,
    offset: i64,
    limit: i64,
) -> Result<(Vec<Pack>, i64)> {
    let total = sqlx::query_scalar("SELECT count(*) FROM packs")
        .fetch_one(&mut *conn)
        .await?;
    let rows = sqlx::query_as::<_, PackRow>(
        "SELECT id, name, minecraft_version, loader, current_release_id FROM packs ORDER BY id
     LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(conn)
    .await?;
    Ok((rows.into_iter().map(Into::into).collect(), total))
}
/// 创建整合包时尚无正式发布，当前指针保持 null
pub async fn insert_pack(conn: &mut SqliteConnection, id: &str, input: &PackInput) -> Result<()> {
    sqlx::query("INSERT INTO packs(id, name, minecraft_version, loader) VALUES(?, ?, ?, ?)")
        .bind(id)
        .bind(&input.name)
        .bind(&input.minecraft_version)
        .bind(&input.loader)
        .execute(conn)
        .await?;
    Ok(())
}
/// 只更新可编辑信息，不接受覆盖当前正式版本指针
pub async fn update_pack(conn: &mut SqliteConnection, id: &str, input: &PackInput) -> Result<()> {
    let changed = sqlx::query("UPDATE packs SET name=?, minecraft_version=?, loader=? WHERE id=?")
        .bind(&input.name)
        .bind(&input.minecraft_version)
        .bind(&input.loader)
        .bind(id)
        .execute(conn)
        .await?;
    if changed.rows_affected() != 1 {
        return Err(AppError::not_found());
    }
    Ok(())
}
/// 搜索业务 modid 或名称，重复名称不合并为同一个 Mod
pub async fn mods(
    conn: &mut SqliteConnection,
    keyword: &str,
    offset: i64,
    limit: i64,
) -> Result<(Vec<ModRecord>, i64)> {
    let pattern = format!(
        "%{}%",
        keyword
            .replace('!', "!!")
            .replace('%', "!%")
            .replace('_', "!_")
    );
    let total = sqlx::query_scalar(
        "SELECT count(*) FROM mods WHERE modid LIKE ? ESCAPE '!' OR name LIKE ? ESCAPE '!'",
    )
    .bind(&pattern)
    .bind(&pattern)
    .fetch_one(&mut *conn)
    .await?;
    let rows = sqlx::query_as::<_, ModRow>(
        "SELECT id, modid, name, modrinth_project_id, curseforge_project_id FROM mods WHERE modid
     LIKE ? ESCAPE '!' OR name LIKE ? ESCAPE '!' ORDER BY id LIMIT ? OFFSET ?",
    )
    .bind(&pattern)
    .bind(&pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(conn)
    .await?;
    Ok((rows.into_iter().map(Into::into).collect(), total))
}
/// 单条 Mod 的身份来自内部 ID，编辑业务 modid 后仍指向原记录
pub async fn mod_record(conn: &mut SqliteConnection, id: &str) -> Result<ModRecord> {
    Ok(sqlx::query_as::<_, ModRow>(
        "SELECT id, modid, name, modrinth_project_id, curseforge_project_id FROM mods WHERE id=?",
    )
    .bind(id)
    .fetch_one(conn)
    .await?
    .into())
}
/// 名称可重复，业务 modid 唯一性由数据库最终约束
pub async fn insert_mod(conn: &mut SqliteConnection, id: &str, input: &ModInput) -> Result<()> {
    sqlx::query(
        "INSERT INTO mods(id, modid, name, modrinth_project_id, curseforge_project_id) VALUES(?,
     ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(&input.modid)
    .bind(&input.name)
    .bind(&input.modrinth_project_id)
    .bind(&input.curseforge_project_id)
    .execute(conn)
    .await?;
    Ok(())
}
/// 更新原记录而不改历史 release_items 中的名称和 modid 快照
pub async fn update_mod(conn: &mut SqliteConnection, id: &str, input: &ModInput) -> Result<()> {
    let result = sqlx::query(
        "UPDATE mods SET modid=?, name=?, modrinth_project_id=?, curseforge_project_id=? WHERE
     id=?",
    )
    .bind(&input.modid)
    .bind(&input.name)
    .bind(&input.modrinth_project_id)
    .bind(&input.curseforge_project_id)
    .bind(id)
    .execute(conn)
    .await?;
    if result.rows_affected() != 1 {
        return Err(AppError::not_found());
    }
    Ok(())
}
/// 文件来源独立存储，按文件优先级和字典顺序生成稳定候选列表
pub async fn artifact(conn: &mut SqliteConnection, id: &str) -> Result<Artifact> {
    let a = sqlx::query_as::<_, ArtifactRow>(
        "SELECT id, mod_id, version, filename, size, sha512, hosted_file_id FROM artifacts WHERE
     id=?",
    )
    .bind(id)
    .fetch_one(&mut *conn)
    .await?;
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT ds.source_code, ds.url, ds.priority FROM download_sources ds JOIN
     download_source_types st ON st.code=ds.source_code WHERE artifact_id=? ORDER BY
     ds.priority, st.sort_order, ds.id",
    )
    .bind(id)
    .fetch_all(conn)
    .await?;
    let sources = rows
        .into_iter()
        .map(|(code, url, priority)| -> Result<DownloadSource> {
            Ok(DownloadSource {
                r#type: serde_json::from_value(serde_json::Value::String(code))?,
                url,
                priority,
            })
        })
        .collect::<Result<_>>()?;
    Ok(Artifact {
        id: a.id,
        mod_id: a.mod_id,
        version: a.version,
        filename: a.filename,
        size: a.size,
        sha512: a.sha512,
        hosted_file_id: a.hosted_file_id,
        sources,
    })
}
/// Artifact 不提供修改接口，正式发布后数据库还会保护其不可变性
pub async fn insert_artifact(
    conn: &mut SqliteConnection,
    id: &str,
    input: &ArtifactInput,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO artifacts(id, mod_id, version, filename, size, sha512, hosted_file_id)
     VALUES(?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(&input.mod_id)
    .bind(&input.version)
    .bind(&input.filename)
    .bind(input.size)
    .bind(&input.sha512)
    .bind(&input.hosted_file_id)
    .execute(&mut *conn)
    .await?;
    for source in &input.sources {
        sqlx::query(
            "INSERT INTO download_sources(id, artifact_id, source_code, url, priority) VALUES(?, ?,
     ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(id)
        .bind(source.r#type.as_str())
        .bind(&source.url)
        .bind(source.priority)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}
/// 可按所属 Mod 过滤，分页返回关联的完整来源记录
pub async fn artifacts(
    conn: &mut SqliteConnection,
    mod_id: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<(Vec<Artifact>, i64)> {
    let total = sqlx::query_scalar("SELECT count(*) FROM artifacts WHERE (? IS NULL OR mod_id=?)")
        .bind(mod_id)
        .bind(mod_id)
        .fetch_one(&mut *conn)
        .await?;
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM artifacts WHERE (? IS NULL OR mod_id=?) ORDER BY id LIMIT ? OFFSET ?",
    )
    .bind(mod_id)
    .bind(mod_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let mut result = Vec::new();
    for id in ids {
        result.push(artifact(conn, &id).await?);
    }
    Ok((result, total))
}
