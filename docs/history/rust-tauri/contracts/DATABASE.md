# P1 SQLite 与 RBAC 模型

核验日期：2026-10-15。实际迁移位于 ../../migrations；[SQL 参考](SCHEMA-REFERENCE.md) 仅用于追溯旧设计。测试使用临时库，不创建真实业务账号。

## 表与关系

| 表 | 约束与职责 |
|---|---|
| accounts | UUID 主键；username NOCASE 唯一；password_hash、enabled、token_version |
| roles | 仅 super_admin/user 两个代码 |
| permissions | API 契约中的固定权限编码 |
| account_roles / role_permissions | 复合主键和外键，实现用户角色/角色权限 |
| system_settings | 单行 id=1，registration_enabled，默认关闭 |
| download_source_types | 四种固定编码，名称、启用与排序 |
| mods | 稳定 ID，业务 modid 唯一、大小写敏感，名称可重复 |
| hosted_files | 不可变存储标识、文件名、非负大小与 SHA-512 |
| artifacts | 所属 Mod、版本、文件、哈希与可空托管文件引用 |
| download_sources | Artifact 的四类来源 URL 与优先级 |
| packs | 当前正式发布引用必须属于同包 |
| releases | pack/version 唯一；draft/published、revision、同包基础版本；发布后禁止改删 |
| release_items | release/mod 唯一，目标路径 NOCASE 唯一；Artifact 必须属于同一 Mod；名称/modid/哈希/来源保存快照 |
| schema_migrations | 已应用初始化版本，不重复执行种子覆盖设置 |

发布流程先写 draft/items，再在事务中切换 published 和 current_release_id。SQL 触发器保护发布快照及已发布 Artifact；跨包外键禁止错误指针。来源快照保存完整列表 JSON，字典调整不会改写快照。生成新计划时再按当前字典过滤来源，已经发给客户端的任务保持原内容。

SQLite NOCASE 主要处理 ASCII，Windows Unicode 路径碰撞、合法路径、HTTP/HTTPS 解析、哈希与实际文件一致性仍需服务层校验，SQL 不替代这些检查。current_release 必须已 published 的语义由发布服务事务校验。当前不提供托管文件删除接口，文件与发布快照保持保留。发布时 base_release_id 记录实际上一正式版，用于历史差异；复制草稿的起点不决定发布后的前驱。

## 连接与迁移

使用 SQLx 0.8.6 和直接 SQL。连接池最多一个连接，每个连接启用 foreign_keys、busy_timeout=5000 和 WAL。注册开关写入和注册创建使用 BEGIN IMMEDIATE 写事务，检查开关和插入账号在同一个事务内。

schema 初始化幂等，0001/0002 与版本标记在一个 BEGIN IMMEDIATE 事务应用；第一次初始化失败不留下半套结构。

## SQL 初始化超管

首个超管初始化与 schema 迁移分开执行。packy-server init-admin 从标准输入读取密码，生成 UUID 和 bcrypt 哈希（cost=12），通过绑定参数在一个事务写入账号及角色。不把参数做字符串替换，不把密码写入仓库，也不内置通用默认密码。bcrypt 密码限制与 API 的 72 字节一致。

已有超管时初始化不再创建账号或覆盖密码。数据库已有同用户名冲突时明确失败，由部署者选择名称；禁止以“初始化”为理由覆盖现有用户。注册接口固定分配 user，普通角色种子没有任何管理权限。

超管可管理用户、角色分配和权限；系统管理要求超管角色与对应权限。保持最后一位启用超管及超管系统管理权限，防止界面操作使管理入口失效。自定义角色、组织树与字段级权限不在本版范围。

## 结构验证

当前自动化覆盖及其实际结果见 [验证记录](../VERIFICATION.md)。数据库约束、HTTP 授权和原生桌面验收分别记录，不以旧 Go 测试代替 Rust 证据。
