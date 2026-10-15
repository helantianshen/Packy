use crate::error::Result;
use packy_common::models::*;
use sqlx::{FromRow, SqliteConnection};

#[derive(FromRow)]
struct ReleaseRow {
    id: String,
    pack_id: String,
    version: String,
    changelog: String,
    minecraft_version: String,
    loader: String,
    status: String,
    revision: i64,
    base_release_id: Option<String>,
    published_at: Option<String>,
}
impl From<ReleaseRow> for Release {
    fn from(r: ReleaseRow) -> Self {
        Self {
            id: r.id,
            pack_id: r.pack_id,
            version: r.version,
            changelog: r.changelog,
            minecraft_version: r.minecraft_version,
            loader: r.loader,
            status: r.status,
            revision: r.revision,
            base_release_id: r.base_release_id,
            published_at: r.published_at,
            items: Vec::new(),
        }
    }
}
#[derive(FromRow)]
struct ItemRow {
    mod_id: String,
    artifact_id: String,
    modid_snapshot: String,
    name_snapshot: String,
    target_path: String,
    size_snapshot: Option<i64>,
    sha512_snapshot: String,
    sources_snapshot: String,
}

/// 所有读取都约束所属整合包，不能跨包引用其他发布
pub async fn get(conn: &mut SqliteConnection, pack: &str, key: &str) -> Result<Release> {
    let row = sqlx::query_as::<_, ReleaseRow>(
        "SELECT id, pack_id, version, changelog, minecraft_version, loader, status, revision,
     base_release_id, published_at FROM releases WHERE pack_id=? AND id=?",
    )
    .bind(pack)
    .bind(key)
    .fetch_one(&mut *conn)
    .await?;
    let mut release: Release = row.into();
    let rows = sqlx::query_as::<_, ItemRow>(
        "SELECT mod_id, artifact_id, modid_snapshot, name_snapshot, target_path, size_snapshot,
     sha512_snapshot, sources_snapshot FROM release_items WHERE release_id=? ORDER BY mod_id",
    )
    .bind(key)
    .fetch_all(conn)
    .await?;
    release.items = rows
        .into_iter()
        .map(|i| -> Result<ReleaseItem> {
            Ok(ReleaseItem {
                mod_id: i.mod_id,
                artifact_id: i.artifact_id,
                modid_snapshot: i.modid_snapshot,
                name_snapshot: i.name_snapshot,
                target: TargetFile {
                    path: i.target_path,
                    size: i.size_snapshot,
                    sha512: i.sha512_snapshot,
                    sources: serde_json::from_str(&i.sources_snapshot)?,
                },
            })
        })
        .collect::<Result<_>>()?;
    Ok(release)
}
/// 发布列表不加载完整文件清单，详细记录由单条接口读取
pub async fn list(
    conn: &mut SqliteConnection,
    pack: &str,
    offset: i64,
    limit: i64,
) -> Result<(Vec<Release>, i64)> {
    let total = sqlx::query_scalar("SELECT count(*) FROM releases WHERE pack_id=?")
        .bind(pack)
        .fetch_one(&mut *conn)
        .await?;
    let rows = sqlx::query_as::<_, ReleaseRow>(
        "SELECT id, pack_id, version, changelog, minecraft_version, loader, status, revision,
     base_release_id, published_at FROM releases WHERE pack_id=? ORDER BY
     COALESCE(published_at, '9999') DESC, id DESC LIMIT ? OFFSET ?",
    )
    .bind(pack)
    .bind(limit)
    .bind(offset)
    .fetch_all(conn)
    .await?;
    Ok((rows.into_iter().map(Into::into).collect(), total))
}
/// 首个草稿允许空清单，其余草稿从指定或当前正式版复制
pub async fn insert(
    conn: &mut SqliteConnection,
    key: &str,
    pack: &Pack,
    input: &CreateDraftRequest,
    base: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO releases(id, pack_id, version, changelog, minecraft_version, loader, status,
     base_release_id) VALUES(?, ?, ?, ?, ?, ?, 'draft', ?)",
    )
    .bind(key)
    .bind(&pack.id)
    .bind(&input.version)
    .bind(&input.changelog)
    .bind(&pack.minecraft_version)
    .bind(&pack.loader)
    .bind(base)
    .execute(&mut *conn)
    .await?;
    if let Some(base) = base {
        sqlx::query(
            "INSERT INTO release_items(release_id, mod_id, artifact_id, modid_snapshot,
     name_snapshot, target_path, size_snapshot, sha512_snapshot, sources_snapshot) SELECT ?,
     mod_id, artifact_id, modid_snapshot, name_snapshot, target_path, size_snapshot,
     sha512_snapshot, sources_snapshot FROM release_items WHERE release_id=?",
        )
        .bind(key)
        .bind(base)
        .execute(conn)
        .await?;
    }
    Ok(())
}
/// 应用层已核对 revision，写事务在整个清单替换期间保持独占
pub async fn update(
    conn: &mut SqliteConnection,
    key: &str,
    input: &UpdateDraftRequest,
    items: &[ReleaseItem],
) -> Result<()> {
    sqlx::query(
        "UPDATE releases SET version=?, changelog=?, revision=revision+1 WHERE id=? AND
     status='draft' AND revision=?",
    )
    .bind(&input.version)
    .bind(&input.changelog)
    .bind(key)
    .bind(input.revision)
    .execute(&mut *conn)
    .await?;
    sqlx::query("DELETE FROM release_items WHERE release_id=?")
        .bind(key)
        .execute(&mut *conn)
        .await?;
    for item in items {
        sqlx::query(
            "INSERT INTO release_items(release_id, mod_id, artifact_id, modid_snapshot,
     name_snapshot, target_path, size_snapshot, sha512_snapshot, sources_snapshot) VALUES(?,
     ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(key)
        .bind(&item.mod_id)
        .bind(&item.artifact_id)
        .bind(&item.modid_snapshot)
        .bind(&item.name_snapshot)
        .bind(&item.target.path)
        .bind(item.target.size)
        .bind(&item.target.sha512)
        .bind(serde_json::to_string(&item.target.sources)?)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}
/// 发布时间与当前指针在同一事务提交，触发器从此拒绝快照改删
pub async fn publish(
    conn: &mut SqliteConnection,
    pack: &str,
    key: &str,
    previous: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "UPDATE releases SET status='published', published_at=?, base_release_id=?,
     revision=revision+1 WHERE id=? AND status='draft'",
    )
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(previous)
    .bind(key)
    .execute(&mut *conn)
    .await?;
    sqlx::query("UPDATE packs SET current_release_id=? WHERE id=?")
        .bind(key)
        .bind(pack)
        .execute(conn)
        .await?;
    Ok(())
}
