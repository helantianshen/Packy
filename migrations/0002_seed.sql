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
 ('modrinth','Modrinth',1,0),('curseforge','CurseForge',1,1),('github','GitHub',1,2),('server','Rust 服务端',1,3);
