use packy_server::infrastructure::database::Database;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// 绕过 HTTP 直接写入，验证数据库自身的外键、唯一性和不可变快照保护
#[tokio::test]
async fn schema_rejects_invalid_identity_hash_and_published_mutation() -> TestResult {
    let temp = tempfile::tempdir()?;
    let db = Database::open(&temp.path().join("schema.db")).await?;
    sqlx::raw_sql("INSERT INTO mods(id,modid,name) VALUES ('m1','demo','同名'),('m2','other','同名');
        INSERT INTO packs(id,name,minecraft_version,loader) VALUES ('p1','测试','1.21.1','neoforge'),('p2','其他','1.21.1','neoforge');
        INSERT INTO releases(id,pack_id,version,minecraft_version,loader,status) VALUES ('r1','p1','1','1.21.1','neoforge','draft');")
        .execute(&db.pool).await?;
    let hash = "a".repeat(128);
    sqlx::query("INSERT INTO artifacts(id,mod_id,version,filename,sha512) VALUES ('a1','m1','1','demo.jar',?)")
        .bind(&hash).execute(&db.pool).await?;
    sqlx::query("INSERT INTO release_items(release_id,mod_id,artifact_id,modid_snapshot,name_snapshot,target_path,sha512_snapshot,sources_snapshot)
        VALUES ('r1','m1','a1','demo','同名','mods/demo.jar',?,'[]')")
        .bind(&hash).execute(&db.pool).await?;
    for sql in [
        "INSERT INTO mods(id,modid,name) VALUES ('m3','demo','另一名称')",
        "INSERT INTO account_roles(account_id,role_code) VALUES ('missing','user')",
        "INSERT INTO artifacts(id,mod_id,version,filename,sha512) VALUES ('bad','m1','1','bad.jar','invalid')",
        "UPDATE packs SET current_release_id='r1' WHERE id='p2'",
        "UPDATE release_items SET mod_id='m2' WHERE release_id='r1'",
    ] {
        assert!(sqlx::query(sql).execute(&db.pool).await.is_err(), "{sql}");
    }
    sqlx::query(
        "UPDATE releases SET status='published',published_at='2026-10-15T00:00:00Z' WHERE id='r1'",
    )
    .execute(&db.pool)
    .await?;
    for sql in [
        "UPDATE releases SET version='changed' WHERE id='r1'",
        "DELETE FROM releases WHERE id='r1'",
        "UPDATE release_items SET target_path='mods/changed.jar' WHERE release_id='r1'",
        "DELETE FROM release_items WHERE release_id='r1'",
        "UPDATE artifacts SET filename='changed.jar' WHERE id='a1'",
    ] {
        assert!(sqlx::query(sql).execute(&db.pool).await.is_err(), "{sql}");
    }
    sqlx::query("UPDATE mods SET modid='renamed' WHERE id='m1'")
        .execute(&db.pool)
        .await?;
    let snapshot: String =
        sqlx::query_scalar("SELECT modid_snapshot FROM release_items WHERE release_id='r1'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(snapshot, "demo");
    Ok(())
}
