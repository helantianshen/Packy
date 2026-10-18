-- 首次使用前将 REPLACE_WITH_BCRYPT_HASH 替换为 password-hash 命令产生的哈希
-- 由维护者手动执行或首次部署脚本调用，重复执行将因唯一管理员冲突而失败
.bail on
PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;
CREATE TEMP TABLE init_password_guard (hash TEXT CHECK(length(hash) = 60 AND substr(hash, 1, 4) = '$2a$'));
INSERT INTO init_password_guard VALUES ('REPLACE_WITH_BCRYPT_HASH');
INSERT INTO administrators (id, username, password_hash, token_version)
SELECT 1, 'admin', hash, 0 FROM init_password_guard;
INSERT INTO source_types (code, name, enabled, sort_order) VALUES
('modrinth', 'Modrinth', 1, 0),
('curseforge', 'CurseForge', 1, 10),
('github', 'GitHub', 1, 20),
('server', '服务端托管', 1, 30);
DROP TABLE init_password_guard;
COMMIT;
