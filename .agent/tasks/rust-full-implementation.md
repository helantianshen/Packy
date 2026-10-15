# Packy Rust 完整实现

更新时间：2026-10-15。状态：代码、本机自动化验证及 Linux 原生交互人工验收完成，Windows 和实际部署验收待完成。

## 目标与授权

完整实现 Rust server/client、Tauri 2 + Vue 3 明暗主题桌面、管理后台；模块化分层、共享契约独立、四种来源 trait。用户已授权各阶段自动推进和自审，未授权 Git 提交或远程发布。业务以 PRD v0.2、contracts 和最新用户补充为准。

## 稳定边界

- 仅名单 ADD/REPLACE/REMOVE，名单外不扫描、不清理；modid 唯一，名称可重复
- 重试和重启恢复固定原计划，不刷新最新发布；来源字典只影响新计划
- JWT 与实时数据库 RBAC；注册只创建 user，开关与建号在同一写事务判定；SQL 初始化首个超管，无默认密码
- 托盘常驻，关闭窗口隐藏，成功保存版本后通知；安装/保存阶段禁止取消和退出
- 可执行程序随整包分发，无安装包和自更新；HTTP/HTTPS 客户端，服务端只 HTTP，Nginx 终止 TLS
- 保持精简、可读、可审查，必要中文注释完整；生产代码禁 unwrap/expect/panic

## 当前实现

P1—P5 的工程、服务端、核心与界面已落盘；P6 的部署文档和隔离联调已完成本机部分。目录见 AGENTS.md 与 docs/decisions/0001-rust-layers.md；运行见 README.md，证据见 docs/VERIFICATION.md。

common 统一 DTO、错误码与纯校验，通过 ts-rs 生成前端类型。sources 支持四种直链和 Modrinth 元数据。server 分 api/application/domain/infrastructure，完成认证、系统管理、Mod/Artifact、草稿/发布/不可变快照/差异、托管上传与 Range。client 独立于 Tauri，完成锁、路径、并发下载/校验、安装、原记录重试/取消/恢复、版本与启动器。桌面接入真实 IPC/托盘/通知；后台五类页面接入真实 API。前端使用 Arco Design Vue、共享 CSS 明暗主题，不使用 Tailwind 或重型 shader。

## 环境与实际验证

Linux CachyOS x86_64；执行命令使用 Bash，登录 Shell 为 fish。本机 Rust/Cargo 1.99.0；GTK3/WebKitGTK 4.1 可用。前端通过 npm exec 临时 Node 22.23.3 验证，未改全局 Node。无 Git 仓库，未创建 Git 或进行远程操作。原始 PRD 已核对与附件字节一致。

- cargo fmt --all --check、workspace/all-targets Clippy -D warnings、cargo test --locked 通过
- 前端格式、类型检查与生产构建通过，npm audit 为 0
- Tauri CLI --no-bundle Linux release 构建通过，最后一轮核心审查后的增量构建也通过；server/client release 构建通过，release 服务端在临时库实际初始化、启动、探针、JWT 登录和身份读取通过
- 真实本地 HTTP server/client 联调：三类操作、托管上传/hash/Range、失败和重启重试、计划固定、名单外不变
- 边界：并发上限 4、取消、状态保存失败、符号链接、错误哈希/备用源/未知大小/三次尝试、路径父子冲突
- 认证：签名/算法/过期/签发方/受众/未来时间拒绝，旧 Token 实时权限撤销和禁用；数据库直接写入的外键/唯一/哈希/不可变保护通过
- 后台真实浏览器登录、创建草稿/预览/发布/回读和明暗截图；真实 Modrinth Sodium 搜索、录入和 HTTPS CDN 下载安装通过
- 真实 HTTPS 联调含 HTTP 测试文件源、SHA-512、版本和名单外文件核对；测试启动器只写标记，未启动 Minecraft

审查修正了 multipart 字段释放、LIKE 字面通配符、进度序号、父子路径完整检查、并发失败后等待所有写入结束、保存失败不报告完成等问题。已同步当前架构/API/配置/数据库/PRD/计划，旧 Go 证据仅作历史追溯。

## 原生与外部验收边界

Linux 原生 debug 程序已启动，真实 Vue get_status 初始化核心并持有隔离整包锁，系统托盘注册存在。当前工具原生 UI 能力被禁用；2026-10-15 用户按约定流程人工检查，确认窗口恢复、后台更新和完成通知“均正常”。该隔离环境的 Linux 调试构建三项人工验收通过，不代表 Windows 或其他构建已验证。

当前测试窗口 PID 386231，session 74880，使用 /tmp/packy-smoke-z8v4q61n/game；fixture session 98701，后台 http://127.0.0.1:40723/admin/。保持该窗口和 fixture 以供用户检查；同一整包不要再启动 CLI 抢锁。后台临时数据已发布至 1.3.0；旧检查计划继续执行原记录符合约定。这些临时信息仅用于本次测试，不是部署配置。

尚待 Windows 10/11 x64 构建与 WebView2/文件占用/联接/托盘通知、真实 HMCL/PCL2 和 Minecraft、实际 Nginx TLS 反代、CurseForge/GitHub 真实发布文件。仅安装/解压到 /tmp 的 WebDriver 探索未完成原生交互，不计入证据；没有修改系统包或全局配置。

## 下一步

Linux 原生人工检查已通过；在对应 Windows、Nginx 和真实整合包环境执行余下验收。完整目标尚不能标记全部验收完成，不用 Linux 或浏览器结果替代实机条件。
