# Packy 开发约定

## 范围与结构

需求以 docs/PRD-v0.2.md 和 docs/contracts 为基线，最新明确用户指令优先。完整实现任务状态在 .agent/tasks/rust-full-implementation.md；历史 Go/Wails 证据不能用于 Rust 验收。

采用模块化分层，依赖方向与目录见 docs/decisions/0001-rust-layers.md。common 只存共享契约和纯校验；sources 适配四种下载来源；server 的 api → application → infrastructure；client 的 application 编排独立更新核心。纯差异与计划规则归 domain。Tauri 只装配 commands、事件、托盘与通知。不要把 SQL 写入控制器，不建立只有一个实现的仓储接口，不创建空模块占位。

## 代码与注释

优先可读、可审查的简洁代码，使用明确类型和小函数。生产代码禁止 unwrap、expect、panic 和忽略实际 I/O 失败。公开接口、关键数据含义、事务、不变量、状态转换和平台限制使用完整中文注释；最后一行不以中文标点收尾。生成类型的工具头部保持生成器原样。

密码和管理员 Token 不记录日志、不写入客户端。注册检查与建号在同一 BEGIN IMMEDIATE 事务，权限以数据库当前值判断。只对本次名单执行 ADD/REPLACE/REMOVE，不扫描、整理或删除名单外 Mod。失败重试使用原记录，不刷新最新发布。

## 桌面与部署

支持 HTTP/HTTPS URL；server 本身只监听 HTTP，由 Nginx 负责 TLS。服务端默认读取工作目录下 config/server.yaml，环境变量逐项优先覆盖，PACKY_CONFIG 可指定其他路径；相对数据路径仍以工作目录解析。客户端专用于蒹葭服务器整合包，名称、HTTP/HTTPS 地址、游戏与启动器相对路径在项目 config/client.<platform>.yaml 中维护，通过 include_str! 构建嵌入；分发程序禁止读取整包 .updater/config.json 覆盖配置。桌面后台开关默认开启，关闭窗口按开关隐藏到托盘或安全退出，成功保存版本后通知用户。安装和版本保存阶段不可取消或退出。只分发整合包内可执行文件，不制作安装包、不做自更新。

## 验证

前端使用 .node-version 指定的 Node 22，不能擅自改全局 Node。Rust 依赖由 Cargo.lock 固定。可执行命令见 README。

- cargo fmt --all --check
- cargo clippy --locked --workspace --all-targets -- -D warnings
- cargo test --locked
- npm run build
- npm run format:check

桌面测试需要系统 GUI 库，服务器与核心可独立验证。数据库、文件和端到端测试用临时库及整合包副本，不接触真实玩家目录。分清构建、模拟来源、真实网络、Linux 和 Windows 的证据。不得以 Linux 编译或浏览器预览替代 Windows 实机和原生托盘验收。

Git 写操作和远程发布需要明确授权，不从修改代码授权推导。


## 目标架构调整（2026-10-17）

用户已确定下一轮重建采用单 Go module：`cmd/server` 为 Go 服务端入口，`cmd/client` 为 Go 客户端 Sidecar 入口，业务分别位于 `internal/server` 与 `internal/client`，共享模型与纯协议校验位于 `pkg/protocol`；`web/desktop` 使用 Electron + Vue，`web/admin` 使用 Arco Design Pro Vue。详细方案见 `docs/specs/go-electron-redevelopment.md`，任务状态见 `.agent/tasks/go-electron-redevelopment.md`。以上是目标架构，当前 Rust/Tauri 实现尚未替换；本文件前述 Rust 构建命令仅适用于旧实现。

Admin 必须从 Arco Design Pro Vue 官方完整模板接入业务，保留模板布局、主题和组件模式，不得仅安装 Arco 组件库后重新搭建后台。UI 风格后期另行调整，当前只做必要的业务字段、菜单、接口、权限和渲染兼容修复。模板不免除真实接口与浏览器渲染验证；生产禁用 mock。

Electron Main 负责窗口、托盘、通知与 Sidecar 管理，Go Sidecar 负责更新状态机、下载校验、文件安装、任务恢复、整包锁与启动器。Node 不承担服务端或更新核心业务。保留既有业务契约和数据安全边界，旧 Go/Wails 或 Rust/Tauri 证据不能用于新栈验收。


### Go 重建的依赖选型原则

用户已确定 HTTP 框架使用 Gin，数据库访问使用 GORM（当前数据库仍为 SQLite），配置管理使用 Viper。其余通用能力同样优先采用成熟、维护良好、适配业务和目标平台的第三方库，不以依赖最小集、标准库优先、零 CGO 或减少包数量为目标，不自行重复实现成熟生态能力。自研仅用于项目业务规则及库未覆盖的必要适配。核查版本、许可证、默认行为和跨平台兼容性，不因采用框架而放松事务、路径安全、权限和原任务恢复约束。此前方案或技能中与本条冲突的依赖最小化原则不用于本次重建。


### 业务与初始化简化（2026-10-18，覆盖旧约束）

协议以人工维护文档为契约，Go 双端继续共享 pkg/protocol 实际模型，前端只手写必要类型，不建立协议/类型生成流水线。数据库结构使用 GORM AutoMigrate，不引入 Goose；结构准备与种子初始化分开，首次管理员和基础数据由维护者手动执行 init.sql，程序不自行检测后建号或补种子，暂不提供部署脚本。

每个服务端只有一个管理员，可管理多个整合包；多包是业务资源而非租户。取消注册、注册开关、RBAC、角色/权限/用户管理和多租户；保留管理员登录认证与 pack/release 归属校验。本条替代此前注册事务和最后超管等旧要求。

服务端必须持久化业务审计，记录管理员操作的对象、动作、结果及字段前后变化；成功审计与数据库业务写入在同一 GORM 事务提交，审计失败不得报告业务成功。密码、密码哈希、Token、密钥和带签名参数的完整 URL 不写审计。后台基于官方模板提供只读审计列表/详情，不重新搭建 UI。


### 当前执行基线（2026-10-18）

Go/Electron 重构已进入实现，上述 Rust 命令/目录仅为历史约束。当前构建与测试以 README、docs/contracts 和 .agent/tasks/go-electron-redevelopment.md 为准。旧实现及依赖已按用户明确指令清理，历史证据位于 docs/history/rust-tauri。Node 使用 .node-version 的 24.20.0，仅调整命令 PATH，不切换全局环境。客户端通过 Go embed 内嵌配置，分发完整 Electron 便携目录。正常启动不迁移或初始化，migrate 与 init.sql 必须手动执行。Git 提交与推送按用户最新明确授权执行。


### 首次部署与升级兼容（2026-10-18，覆盖此前无部署脚本约束）

用户已授权 Linux 服务端首次部署脚本执行 migrate 和一次 init.sql，并随机生成 JWT 密钥。正常服务启动仍不得迁移或初始化；已有安装时脚本拒绝执行，不提供更新功能。发布与预览必须验证所有历史正式版本到目标版本的路径兼容性；继续拒绝仅大小写或 Unicode 规范不同的改名，不发布旧客户端无法执行的计划。Git 提交与推送按用户最新明确授权执行。
