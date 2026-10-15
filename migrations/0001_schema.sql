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
 url TEXT NOT NULL CHECK(url LIKE 'https://%' OR url LIKE 'http://%'),
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
