-- 对旧库的副本执行导入，先在新库执行 migrate 和 init.sql
-- 替换下面两个占位值后，用 sqlite3 NEW_DB < import-legacy.sql 手动执行
-- 托管目录应单独复制，沿用 ID.jar 文件名；不迁移旧账号、角色和权限
.bail on
ATTACH DATABASE 'file:REPLACE_WITH_LEGACY_COPY?mode=ro' AS legacy;
CREATE TEMP TABLE import_settings(public_url TEXT CHECK(public_url LIKE 'http://%' OR public_url LIKE 'https://%'));
INSERT INTO import_settings VALUES ('REPLACE_WITH_PUBLIC_URL');
BEGIN IMMEDIATE;
INSERT INTO mods SELECT id,modid,name,modrinth_project_id,curseforge_project_id FROM legacy.mods;
INSERT INTO hosted_files(id,filename,size,sha512,url)
SELECT id,filename,size,sha512,(SELECT rtrim(public_url,'/') FROM import_settings)||'/api/v1/files/'||id||'/download' FROM legacy.hosted_files;
INSERT INTO artifacts(id,mod_id,version,filename,size,sha512,hosted_file_id,sources)
SELECT a.id,a.mod_id,a.version,a.filename,a.size,a.sha512,a.hosted_file_id,
 (SELECT json_group_array(json_object('type',source_code,'url',url,'priority',priority)) FROM (SELECT * FROM legacy.download_sources WHERE artifact_id=a.id ORDER BY priority,id))
FROM legacy.artifacts a;
INSERT INTO packs(id,name,minecraft_version,loader,current_release_id)
SELECT id,name,minecraft_version,loader,current_release_id FROM legacy.packs;
INSERT INTO releases(id,pack_id,version,changelog,minecraft_version,loader,status,revision,base_release_id,published_at,items)
SELECT r.id,r.pack_id,r.version,r.changelog,r.minecraft_version,r.loader,r.status,r.revision,r.base_release_id,r.published_at,
 (SELECT json_group_array(json_object('mod_id',mod_id,'artifact_id',artifact_id,'modid_snapshot',modid_snapshot,'name_snapshot',name_snapshot,'target',json_object('path',target_path,'size',size_snapshot,'sha512',sha512_snapshot,'sources',json(sources_snapshot)))) FROM (SELECT * FROM legacy.release_items WHERE release_id=r.id ORDER BY mod_id))
FROM legacy.releases r;
UPDATE source_types SET name=(SELECT name FROM legacy.download_source_types l WHERE l.code=source_types.code),enabled=(SELECT enabled FROM legacy.download_source_types l WHERE l.code=source_types.code),sort_order=(SELECT sort_order FROM legacy.download_source_types l WHERE l.code=source_types.code);
INSERT INTO audit_logs(id,occurred_at,actor_id,actor_name,action,result,resource_type,resource_id,changes,summary)
VALUES(lower(hex(randomblob(16))),strftime('%Y-%m-%dT%H:%M:%fZ','now'),1,'manual','legacy.import','success','database','legacy','{}','维护者手动导入旧库副本');
COMMIT;
DROP TABLE import_settings;
DETACH DATABASE legacy;
