# 数据库与审计

结构唯一来源：`internal/server/infrastructure/models.go`；显式 `server migrate` 调用 GORM AutoMigrate。初始化唯一管理员与四来源由维护者手动执行 `sql/init.sql`，不引入 Goose，也不自动创建用户。SQLite DSN 启用 foreign_keys、WAL、busy_timeout，业务事务 BEGIN IMMEDIATE，单写连接。

表：administrators、packs、mods、artifacts、releases、source_types、hosted_files、audit_logs。主键为 UUID 字符串，管理员固定 id=1。Mod modid 唯一；发布 (pack_id,version) 唯一。Artifact 来源和 Release items 使用 GORM JSON serializer，发布引用与不可变规则在 application 事务中校验；没有提供修改已发布快照或删除接口。当前版本不是通用 SQL 管理后台，维护者直接写 SQL 需自行遵守业务不变量。

成功业务与 audit 同事务，审计写失败则业务回滚；业务失败记录 failed，不记录请求体秘密。登录成功记录管理员身份；字段变更 old/new；条目按稳定 mod_id 记录；发布包含实际前驱/新版本指针及操作清单。密码、Token、secret 字段过滤；URL 查询、用户信息、fragment 过滤。上传文件系统与数据库不构成分布式事务：元数据提交失败可能遗留本次孤立文件，不做自动扫删。

旧库必须以只读副本手动导入新库，见 `sql/import-legacy.sql`。不迁移旧 RBAC；不自动操作现有 data。
