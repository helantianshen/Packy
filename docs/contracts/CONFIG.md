# 配置与工具链

核验日期：2026-10-15。精确依赖以 Cargo.lock、package-lock.json 为准。

## 工具链

Cargo workspace 使用 Rust edition 2024，最低 Rust 1.88；本机验证版本 1.99.0。Tauri 2.11.6、Axum 0.8.9、SQLx 0.8.6、Reqwest 0.12.28；Vue 3、TypeScript、Vite、Arco Design Vue 与共享 CSS 主题。Node 使用 .node-version 指定的 22.23.3，不修改全局运行时。

Linux 桌面使用 GTK3/WebKitGTK 4.1；Windows 构建需要 C++ 工具链和 WebView2。构建与启动命令见 [README](../../README.md)，部署和 Windows 操作见 [部署说明](../DEPLOYMENT.md)。不制作安装包，不包含自更新。

## 服务端

开发和生产共用一套配置，逐字段采用环境变量 > YAML > 内置默认值。默认读取工作目录下 `config/server.yaml`；`PACKY_CONFIG` 指定其他路径时，该文件必须存在且可读。默认文件不存在允许纯环境变量启动；YAML 语法、未知字段、字段类型和最终配置值错误均拒绝启动，不把错误当作缺失。显式环境空值覆盖文件值，必填项为空会报错。相对路径始终以工作目录解析。

| YAML 字段 | 环境变量 | 默认 / 约束 |
|---|---|---|
| listen | PACKY_LISTEN | 127.0.0.1:8080，仅 HTTP |
| database | PACKY_DATABASE | data/packy.db |
| files_dir | PACKY_FILES_DIR | data/files |
| admin_dir | PACKY_ADMIN_DIR | web/admin/dist；分发包模板为 admin |
| public_url | PACKY_PUBLIC_URL | http://127.0.0.1:8080；实际可达的 HTTP/HTTPS 基址，不含凭据、查询或片段 |
| jwt_secret | PACKY_JWT_SECRET | 必填，至少 32 字节随机密钥，无公共默认值 |
| modrinth_user_agent | PACKY_MODRINTH_USER_AGENT | Packy/0.1.0，部署时应补充联系信息 |
| log_filter | RUST_LOG | packy_server=info,tower_http=info |

`init-admin` 使用同一 YAML/环境合并逻辑和数据库路径，但不要求 JWT 或 HTTP 监听参数有效。配置文件错误不输出原文，避免泄露密钥。程序不会自动加载 `.env/server.env`。模板为 `config/server.yaml.example`，本地 `config/server.yaml` 忽略版本管理，不进入分发包。当前不使用 CurseForge API Key，仅录入直链。

普通 JSON 上限 1 MiB，托管 JAR 上传上限 512 MiB。registration_enabled 仅保存在 SQLite；每次注册在写事务内读取，设置提交后立即作用于后续注册判定。

## 客户端

项目分别维护 `config/client.windows.yaml`、`config/client.linux.yaml`、`config/client.macos.yaml`，各自填写实际 ID、名称、服务地址和相对路径。Rust 根据 target_os 选择目标系统文件，不依据宿主环境或运行时参数。三个文件的改动互不覆盖，再分别在对应系统构建客户端。Rust `include_str!` 把文件内容嵌入程序，配置变化会触发 Cargo 重新编译；发布时不分发此配置文件。桌面和命令行入口不读取整包 `.updater/config.json`，不存在运行时文件或环境变量配置覆盖。整包只需携带 `.updater/state.json` 和可选用户偏好。示例域名和 ID 不能用于真实更新。

本客户端专用于蒹葭服务器整合包。管理员在项目 `config/client.<platform>.yaml` 设置 `pack_name`，默认“蒹葭服务器整合包”，展示名称不使用服务端清单中的 pack_name。`pack_id` 仍用于识别真实发布和校验计划。`server_url` 必须填写完整 HTTP/HTTPS URL，不能使用本地文件相对路径；域名和部署子路径由组包时确定。

game_dir、launcher_path 相对整包可执行程序目录，game_dir 可为 .，路径使用 / 分隔，禁止绝对路径和 .. 越界。组包时可填写如 `.minecraft/versions/server-pack` 和 `HMCL.exe`；不可将工作目录或开发机器绝对路径写入分发配置。

用户后台偏好单独保存在 `.updater/preferences.json`，格式见 examples/updater-preferences.json。文件缺失时 `background_enabled` 默认 true。界面开关原子保存成功后立即生效，失败时保留原值，不修改内置配置。开启时关闭窗口隐藏到托盘；关闭时安全停止下载并退出，原任务保留供重试；安装和保存版本期间始终禁止退出。

task.json 保存 TaskRecord，schema_version=1；staging 保存当前任务下载。state 仅在全部文件安装成功后推进。读取配置、记录、路径安全检查与原记录重试均由 Rust 核心负责。生产程序始终以自身所在目录定位；调试构建可用 PACKY_GAME_ROOT 指定隔离副本。

## HTTP/HTTPS

客户端的服务地址与下载链接支持 HTTP/HTTPS。Nginx 终止 TLS 后反代 HTTP，服务端不自行配置证书。HTTPS 重定向不得降级 HTTP；初始 HTTP 可用。实际 HTTP 联调、HTTPS 文件下载和仍待验收的范围见 [验证记录](../VERIFICATION.md)。
