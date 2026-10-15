# SQLite 结构参考

保留原 P1 SQL 作为数据库约束和种子的设计参考，不是当前可执行迁移。Rust P1 重建迁移并复验；业务语义保持不变。下方保留原 SQL 字节内容，server 字典显示名中的“Go 服务端”在新迁移中改为“Rust 服务端”，编码 server 不变。

## 0001_schema.sql

```sql
CREATE TABLE accounts (
 id TEXT PRIMARY KEY NOT NULL,
 username TEXT NOT NULL COLLATE NOCASE UNIQUE,
 password_hash TEXT NOT NULL,
 enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0,1)),
 token_version INTEGER NOT NULL DEFAULT 0 CHECK (token_version >= 0),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE roles (code TEXT PRIMARY KEY NOT NULL CHECK(code IN ('super_admin','user')), name TEXT NOT NULL);
CREATE TABLE permissions (code TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL);
CREATE TABLE account_roles (
 account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
 role_code TEXT NOT NULL REFERENCES roles(code) ON DELETE RESTRICT,
 PRIMARY KEY(account_id, role_code)
);
CREATE TABLE role_permissions (
 role_code TEXT NOT NULL REFERENCES roles(code) ON DELETE RESTRICT,
 permission_code TEXT NOT NULL REFERENCES permissions(code) ON DELETE RESTRICT,
 PRIMARY KEY(role_code, permission_code)
);
CREATE TABLE system_settings (
 id INTEGER PRIMARY KEY CHECK(id=1),
 registration_enabled INTEGER NOT NULL DEFAULT 0 CHECK(registration_enabled IN (0,1))
);
CREATE TABLE download_source_types (
 code TEXT PRIMARY KEY NOT NULL CHECK(code IN ('modrinth','curseforge','github','server')),
 name TEXT NOT NULL,
 enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)),
 sort_order INTEGER NOT NULL CHECK(sort_order >= 0)
);
CREATE TABLE mods (
 id TEXT PRIMARY KEY NOT NULL,
 modid TEXT NOT NULL UNIQUE CHECK(length(modid)>0),
 name TEXT NOT NULL CHECK(length(name)>0),
 modrinth_project_id TEXT,
 curseforge_project_id TEXT
);
CREATE TABLE hosted_files (
 id TEXT PRIMARY KEY NOT NULL,
 storage_key TEXT NOT NULL UNIQUE,
 filename TEXT NOT NULL,
 size INTEGER NOT NULL CHECK(size>=0),
 sha512 TEXT NOT NULL CHECK(length(sha512)=128 AND sha512 NOT GLOB '*[^0-9a-f]*'),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE artifacts (
 id TEXT PRIMARY KEY NOT NULL,
 mod_id TEXT NOT NULL REFERENCES mods(id) ON DELETE RESTRICT,
 version TEXT NOT NULL,
 filename TEXT NOT NULL,
 size INTEGER CHECK(size>=0),
 sha512 TEXT NOT NULL CHECK(length(sha512)=128 AND sha512 NOT GLOB '*[^0-9a-f]*'),
 hosted_file_id TEXT REFERENCES hosted_files(id) ON DELETE RESTRICT,
 UNIQUE(mod_id,id)
);
CREATE TABLE download_sources (
 id TEXT PRIMARY KEY NOT NULL,
 artifact_id TEXT NOT NULL REFERENCES artifacts(id) ON DELETE RESTRICT,
 source_code TEXT NOT NULL REFERENCES download_source_types(code) ON DELETE RESTRICT,
 url TEXT NOT NULL CHECK(url LIKE 'https://%'),
 priority INTEGER NOT NULL CHECK(priority>=0),
 UNIQUE(artifact_id,source_code,url)
);
CREATE TABLE packs (
 id TEXT PRIMARY KEY NOT NULL,
 name TEXT NOT NULL,
 minecraft_version TEXT NOT NULL,
 loader TEXT NOT NULL CHECK(loader IN ('neoforge','forge','fabric')),
 current_release_id TEXT,
 FOREIGN KEY(id,current_release_id) REFERENCES releases(pack_id,id) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE releases (
 id TEXT PRIMARY KEY NOT NULL,
 pack_id TEXT NOT NULL REFERENCES packs(id) ON DELETE RESTRICT,
 version TEXT NOT NULL,
 changelog TEXT NOT NULL DEFAULT '',
 minecraft_version TEXT NOT NULL,
 loader TEXT NOT NULL CHECK(loader IN ('neoforge','forge','fabric')),
 status TEXT NOT NULL CHECK(status IN ('draft','published')),
 revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>=1),
 base_release_id TEXT,
 published_at TEXT,
 UNIQUE(pack_id,version),
 UNIQUE(pack_id,id),
 FOREIGN KEY(pack_id,base_release_id) REFERENCES releases(pack_id,id) ON DELETE RESTRICT,
 CHECK((status='draft' AND published_at IS NULL) OR (status='published' AND published_at IS NOT NULL))
);
CREATE TABLE release_items (
 release_id TEXT NOT NULL REFERENCES releases(id) ON DELETE RESTRICT,
 mod_id TEXT NOT NULL REFERENCES mods(id) ON DELETE RESTRICT,
 artifact_id TEXT NOT NULL,
 modid_snapshot TEXT NOT NULL,
 name_snapshot TEXT NOT NULL,
 target_path TEXT NOT NULL COLLATE NOCASE,
 size_snapshot INTEGER CHECK(size_snapshot>=0),
 sha512_snapshot TEXT NOT NULL CHECK(length(sha512_snapshot)=128 AND sha512_snapshot NOT GLOB '*[^0-9a-f]*'),
 sources_snapshot TEXT NOT NULL CHECK(json_valid(sources_snapshot) AND json_type(sources_snapshot)='array'),
 PRIMARY KEY(release_id,mod_id),
 UNIQUE(release_id,target_path),
 FOREIGN KEY(mod_id,artifact_id) REFERENCES artifacts(mod_id,id) ON DELETE RESTRICT
);
CREATE INDEX releases_pack_status ON releases(pack_id,status);
CREATE INDEX download_sources_artifact ON download_sources(artifact_id);

CREATE TRIGGER published_release_update BEFORE UPDATE ON releases WHEN OLD.status='published'
BEGIN SELECT RAISE(ABORT,'published release is immutable'); END;
CREATE TRIGGER published_release_delete BEFORE DELETE ON releases WHEN OLD.status='published'
BEGIN SELECT RAISE(ABORT,'published release is immutable'); END;
CREATE TRIGGER published_item_insert BEFORE INSERT ON release_items
WHEN EXISTS(SELECT 1 FROM releases WHERE id=NEW.release_id AND status='published')
BEGIN SELECT RAISE(ABORT,'published snapshot is immutable'); END;
CREATE TRIGGER published_item_update BEFORE UPDATE ON release_items
WHEN EXISTS(SELECT 1 FROM releases WHERE id IN (OLD.release_id,NEW.release_id) AND status='published')
BEGIN SELECT RAISE(ABORT,'published snapshot is immutable'); END;
CREATE TRIGGER published_item_delete BEFORE DELETE ON release_items
WHEN EXISTS(SELECT 1 FROM releases WHERE id=OLD.release_id AND status='published')
BEGIN SELECT RAISE(ABORT,'published snapshot is immutable'); END;
CREATE TRIGGER referenced_artifact_update BEFORE UPDATE ON artifacts
WHEN EXISTS(SELECT 1 FROM release_items ri JOIN releases r ON r.id=ri.release_id WHERE ri.artifact_id=OLD.id AND r.status='published')
BEGIN SELECT RAISE(ABORT,'published artifact is immutable'); END;
CREATE TRIGGER hosted_file_update BEFORE UPDATE ON hosted_files
BEGIN SELECT RAISE(ABORT,'hosted file is immutable'); END;

```

## 0002_seed.sql

```sql
INSERT INTO roles(code,name) VALUES ('super_admin','超管'),('user','普通用户');
INSERT INTO permissions(code,name) VALUES
 ('pack:read','查看整合包'),('pack:write','维护整合包'),
 ('release:read','查看版本'),('release:write','维护草稿'),('release:publish','发布版本'),
 ('mod:read','查看 Mod'),('mod:write','维护 Mod'),
 ('source:read','查看来源'),('source:write','维护来源'),('file:write','上传文件'),
 ('user:read','查看用户'),('user:write','管理用户'),
 ('permission:read','查看权限'),('permission:write','管理权限'),
 ('settings:read','查看设置'),('settings:write','管理设置');
INSERT INTO role_permissions(role_code,permission_code) SELECT 'super_admin',code FROM permissions;
INSERT INTO system_settings(id,registration_enabled) VALUES(1,0);
INSERT INTO download_source_types(code,name,enabled,sort_order) VALUES
 ('modrinth','Modrinth',1,0),('curseforge','CurseForge',1,1),('github','GitHub',1,2),('server','Go 服务端',1,3);

```

## bootstrap_super_admin.sql

```sql
INSERT INTO accounts(id,username,password_hash)
SELECT :account_id,:username,:password_hash
WHERE NOT EXISTS (SELECT 1 FROM account_roles WHERE role_code='super_admin');
INSERT INTO account_roles(account_id,role_code)
SELECT id,'super_admin' FROM accounts WHERE id=:account_id AND username=:username
AND NOT EXISTS (SELECT 1 FROM account_roles WHERE role_code='super_admin')
ON CONFLICT DO NOTHING;

```
