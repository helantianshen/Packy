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
