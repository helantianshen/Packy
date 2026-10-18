> 历史记录：2026-10-15 已切换 Rust + Tauri，Go/Wails 代码已清理。以下完成状态和命令仅适用于旧实现，不能用于当前工程验收。

# Go/Wails 架构与计划审查

更新时间：2026-10-15。
状态：已落实用户审查及澄清；后续已获 P1 初始化授权，进展见 p1-initialization.md。

## 当前决定

- Go 服务端/更新核心，Wails v3 + Vue 3，单 Module，SQLite。
- 客户端取得完整记录后独立执行/重试，不协调最新发布；本地可保留原任务，不建服务端任务系统。
- 来源仅 Modrinth、CurseForge、GitHub、Go 服务端，字典管理。server 来源提供文件流量，不是第三方通用代理。
- 仅名单内 ADD/REPLACE/REMOVE，名单外完全不处理；无玩家 Mod 管理及不同发布 ID 空差异流程。
- 用户澄清：modid 唯一，名称可重复；修改 modid 保留内部记录 ID，名称不作匹配依据。
- JWT + RBAC，初始超管/普通用户；SQL 初始化角色和首个超管；注册固定普通用户。
- 超管用户管理、权限管理、系统注册开关；注册检查和建号与开关写入按 SQLite 事务顺序生效。

## 文件

PRD-v0.1.md 原稿保留；PRD-v0.2.md、ARCHITECTURE.md、DEVELOPMENT-PLAN.md 与 README 同步当前决定。

## 验证边界

仅文档一致性、链接和原稿完整性检查；没有业务代码、依赖安装、Git 写操作、SQL 初始化、构建或运行验收。CurseForge API 哈希目前列 SHA-1/MD5，录入阶段仍须准备 SHA-512；API Key 不下发客户端。

## 下一步

用户已明确授权 P1。当前任务作为审查记录保留；实施范围和验证见 .agent/tasks/p1-initialization.md。P2 及后续仍需后续指令。
