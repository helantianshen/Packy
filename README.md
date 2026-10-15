# Packy

Minecraft 整合包更新器：Rust 服务端与更新核心、Tauri 2 + Vue 3 桌面、Vue/Arco 管理后台、SQLite。支持 HTTP/HTTPS 地址；服务端只监听 HTTP，由 Nginx 负责 HTTPS。更新器直接放入整合包，不制作安装包、不提供自更新。

目前已实现账号/JWT/RBAC、实时注册开关、用户/权限/下载源管理、Modrinth 搜索录入、Mod/文件版本、草稿/发布/差异计划、托管文件上传与 Range 下载，以及客户端并发下载、SHA-512、名单安装、原记录重试/取消和启动器。桌面支持明暗主题、托盘常驻和完成通知。具体已测与未测项见 [验证记录](docs/VERIFICATION.md)，不能将构建通过等同于 Windows 实机验收。

## 结构

```text
crates/common        共享 JSON/TypeScript 契约、错误和纯校验
crates/sources       四类下载来源 trait 适配、Modrinth 元数据客户端
crates/server        api / application / domain / infrastructure
crates/client        独立更新核心、状态机、文件和网络适配
apps/desktop         Vue 页面与 Tauri commands、事件、托盘、通知
web/admin            管理后台
web/shared           共用主题与 Rust 生成的 TypeScript 类型
migrations           SQLite 结构和种子
scripts              隔离联调环境
```

架构采用模块化分层，规则见 [架构决策](docs/decisions/0001-rust-layers.md) 和 [AGENTS.md](AGENTS.md)。服务端不依赖桌面系统库，核心不依赖 Tauri。

## 工具与构建

Rust 1.88+、Node 22.23.3。Cargo.lock/package-lock.json 固定实际依赖；`.node-version` 记录前端工具链。Linux 桌面需要 GTK3/WebKitGTK 4.1，Windows 需要对应 C++ 构建工具和 WebView2，详见 [Tauri 官方依赖](https://v2.tauri.app/start/prerequisites/)。

```bash
npm ci
npm run build
cargo build --locked -p packy-server -p packy-client
```

`npm run build` 先从 Rust 导出 TypeScript 类型，再构建桌面页面与后台。不要手改 `web/shared/generated`。无 GUI 的服务器无需执行桌面 Rust 构建。

桌面开发：

```bash
npm run types
npm run desktop
```

生产桌面可执行文件：

```bash
npm run desktop:build
```

此命令按本机平台选择配置，使用 Tauri CLI `build --no-bundle --target <本机目标>` 并锁定 Cargo 依赖。归档输出至 `artifacts/desktop/packy-desktop-<platform>-<target>.tar.gz`，包含可执行文件和 build-info.json（配置/程序 SHA-256），不含可修改的组包配置。只打包程序，不制作安装器。Windows 程序带 `.exe`；归档保留 Linux/macOS 的执行权限。

三端配置独立维护，Rust 根据编译目标 `target_os` 选择 `config/client.windows.yaml`、`config/client.linux.yaml` 或 `config/client.macos.yaml`，无需复制覆盖公共文件。服务地址、pack_id、实例路径和启动器均可分别配置；Windows 示例为 HMCL.exe，Linux/macOS 示例为 launcher.sh，组包时替换为实际相对路径。

可以显式执行 `npm run desktop:build -- --platform windows`、`--platform linux`、`--platform macos`；必须在对应系统运行。`--check` 仅打印选用配置和产物路径，`npm run test:build` 验证平台选择和跨系统误用拦截。构建脚本要求本机 SDK 和 tar 命令，Windows 10/11 提供 tar.exe。

GitHub Actions 工作流 `.github/workflows/build.yml` 分别使用 Windows 2022 x64、Ubuntu 24.04 x64、macOS 15 ARM64 原生 runner，独立测试/构建/上传三个客户端归档，另有独立 Linux 服务端任务，无发布或推送步骤。Intel Mac 本机也可构建，产物按其目标架构命名；当前 CI 不额外构建 Intel Mac 或其他架构。

也可先构建前端，再执行 `cargo build --locked --release -p packy-desktop --features custom-protocol`。直接用 Cargo 启动嵌入前端的桌面也需要 `--features custom-protocol`；开发模式由 Tauri CLI 启动 Vite。

## 统一构建客户端与服务端

以下命令在项目根目录执行，使用上述 Node 22、Rust 和当前系统的桌面构建依赖。

1. 编辑目标平台的组包配置：[Windows](config/client.windows.yaml)、[Linux](config/client.linux.yaml)、[macOS](config/client.macos.yaml)。填写真实的整合包 ID、完整 HTTP/HTTPS 服务地址、游戏目录和启动器相对路径；默认 `example.invalid` 和示例 ID 不能用于实际更新。配置在构建时嵌入，修改后需要重新构建。
2. 首次检出或依赖锁文件变化后执行 `npm ci`。
3. 检查构建计划，再执行构建：

```bash
npm run release:build -- --check
npm run release:build
```

`--check` 只打印计划，不编译。实际构建依次生成共享类型、客户端前端及 Tauri 程序、管理后台及服务端程序，最后整理分发包。仅构建其中一端时执行：

```bash
npm run release:build -- --only desktop
npm run release:build -- --only server
```

若全局 Node 版本不符，可通过临时 Node 22 执行，不修改全局环境；其他 npm 命令同样可使用此前缀：

```bash
npm exec --yes --package=node@22.23.3 -- npm ci
npm exec --yes --package=node@22.23.3 -- npm run release:build -- --check
npm exec --yes --package=node@22.23.3 -- npm run release:build
```

### 产物目录与使用

以 Linux x64 为例，构建完成后目录如下，其他平台按实际平台和 Rust target 命名，Windows 可执行文件带 `.exe`：

```text
artifacts/
├── desktop/
│   ├── packy-desktop-linux-x86_64-unknown-linux-gnu/
│   │   ├── packy-desktop
│   │   └── build-info.json
│   └── packy-desktop-linux-x86_64-unknown-linux-gnu.tar.gz
└── server/
    ├── packy-server-linux-x86_64-unknown-linux-gnu/
    │   ├── packy-server
    │   ├── admin/
    │   │   ├── index.html
    │   │   └── assets/
    │   ├── config/
    │   │   └── server.yaml.example
    │   ├── README.txt
    │   └── build-info.json
    └── packy-server-linux-x86_64-unknown-linux-gnu.tar.gz
```

`artifacts/` 保存整理后的分发目录及压缩包；`target/` 保存 Cargo 编译缓存和原始产物。压缩包直接包含对应分发目录内的文件，不额外套一层目录，解压时应指定独立目标目录。

- 客户端：将归档中的可执行文件放入整合包根目录；前端和组包配置已嵌入，无需分发项目配置文件。`build-info.json` 用于核对平台及配置、程序摘要，程序运行不依赖它。
- 服务端：完整解压归档，保留 `admin/` 管理后台静态文件，按包内 `README.txt` 配置并启动。将 `config/server.yaml.example` 复制为 `config/server.yaml` 后填写 JWT 密钥和对外地址，程序自动读取；环境变量可逐项覆盖。详细步骤见下方“服务端运行”和[部署文档](docs/DEPLOYMENT.md)。数据库和上传数据不包含在构建包中，升级时应保留。

### 三平台 CI 构建

本地脚本一次构建当前系统的客户端和服务端；`--platform` 必须与本机系统一致，不能在 Linux 上用参数生成 Windows/macOS 程序。

完整三端客户端加服务端由 [Packy builds 工作流](.github/workflows/build.yml) 一次执行：Windows x64、Linux x64、macOS ARM64 客户端与 Linux x64 服务端四个任务并行，分别上传四个产物。仓库推送到 GitHub 且启用 Actions 后，可在 Actions → Packy builds → Run workflow 手动运行，完成后在该次运行的 Artifacts 中分别下载。工作流不自动发布 Release。

2026-10-15 已在 Linux x64 实际完成统一构建，并核对两个归档的文件列表；该结果只证明 Linux 本机构建和打包通过。Windows/macOS 构建、远程 CI 以及各平台实际运行验收需分别验证，工作流文件存在不代表已经执行。

## RustRover 启动客户端

Cargo 配置的 Command 使用 `run --locked -p packy-desktop --features custom-protocol`，工作目录为项目根目录，环境变量 `PACKY_GAME_ROOT` 指向包含 `.updater` 的测试整合包目录。该覆盖仅对 debug 构建有效。

内嵌前端模式不运行 Vite。直接执行 Cargo 不会运行 Tauri CLI 的 beforeBuildCommand；修改 Vue/CSS 后必须先构建 dist，再由 Cargo 编译嵌入资源。Tauri 框架依赖无需每次重新编译，Cargo 按依赖变化增量构建。

建议在此运行配置的 **Before launch** 中添加前端构建，并放在 Cargo Build 之前。可通过 **External Tools** 配置：

| 字段 | 内容 |
|---|---|
| Program | `npm` 的完整路径，用 `command -v npm` 查询；本机为 `/home/helan/.xlings/subos/current/bin/npm` |
| Arguments | `exec --yes --package=node@22.23.3 -- npm run build` |
| Working directory | `$ProjectFileDir$` |

然后在 Before launch 中选择 Run External Tool，添加该工具。它会等待前端构建成功才继续启动 Rust，避免全局 Node 18 或旧 dist 带来的混淆；不更改全局 Node。

也可在 IDE 终端手工先运行上述 npm 命令，再运行 Cargo 配置。需要前端热更新时，使用 `PACKY_GAME_ROOT=<测试整包目录> npm run desktop`，此时由 Tauri CLI 启动 Vite；不要同时以两个模式操作同一整合包。后台开关开启时关闭窗口只隐藏到托盘，重启前需从托盘退出旧进程。

依据：[Tauri 开发与构建配置](https://v2.tauri.app/reference/config/)、[JetBrains 启动前外部工具](https://www.jetbrains.com/help/clion/configuring-third-party-tools.html)。

## 服务端运行

后端是单个可执行文件，不需要在部署机器安装 Rust、Node 或独立数据库服务，但管理后台静态资源当前在程序外，需一同部署。统一构建生成的服务端包已包含这些文件。数据库及内置迁移自动初始化，上传目录按需创建；数据目录必须持久化。

开发和生产使用同一套配置：**环境变量 > YAML 字段 > 内置默认值**。默认读取工作目录下的 `config/server.yaml`，也可通过 `PACKY_CONFIG` 指定其他路径。现有 `PACKY_*` 环境变量保持有效，`RUST_LOG` 覆盖 YAML 的 `log_filter`。默认文件不存在时可以只使用环境变量；显式指定的文件缺失、YAML 错误或配置值无效会拒绝启动，空环境变量不会静默回退。

源码运行时在项目根目录准备配置，RustRover 的 Working directory 也设为项目根目录：

```bash
cp config/server.yaml.example config/server.yaml
openssl rand -hex 32
```

将生成的随机字符串填入 `config/server.yaml` 的 `jwt_secret`，将 `admin_dir` 设为 `web/admin/dist`。本地 `config/server.yaml` 已在 `.gitignore` 中忽略，模板不含可直接使用的密钥。其他字段和环境变量对应关系见[配置契约](docs/contracts/CONFIG.md)。相对文件路径均以工作目录解析，不按开发/生产切换。

部署分发包时，在解压目录将 `config/server.yaml.example` 复制为 `config/server.yaml`，保留 `admin_dir: admin`，按实际情况设置 `public_url` 和持久化目录。程序不自动读取 `.env/server.env`。HTTPS 仍由 Nginx 处理。

已有根目录 `server.yaml` 的配置应原样移动至 `config/server.yaml`，保留原密钥和配置值，避免用模板覆盖。已有 `PACKY_CONFIG` 显式指向旧路径时也需同步调整。配置搬迁不改变数据目录的解析基准，RustRover 工作目录仍为项目根目录。

初始化首个超管，密码从标准输入传入，避免放在参数或 shell 历史中：

```bash
read -rs -p '初始超管密码: ' PACKY_INIT_PASSWORD
printf '%s\n' "$PACKY_INIT_PASSWORD" | cargo run --locked -p packy-server -- init-admin admin
unset PACKY_INIT_PASSWORD
cargo run --locked -p packy-server -- serve
```

后台访问 `http://127.0.0.1:8080/admin/`，探针为 `/healthz`。结构/种子随打开数据库幂等初始化；超管需显式初始化，已有超管时命令不覆盖账号。注册默认关闭，注册固定普通用户。所有管理权限由后端校验。

前端独立开发可运行 `npm run dev --workspace @packy/admin`，API 代理至本机 8080。生产部署和 Nginx 示例见 [部署说明](docs/DEPLOYMENT.md)。

## 整合包分发与本地联调

先在项目 [Windows](config/client.windows.yaml)、[Linux](config/client.linux.yaml)、[macOS](config/client.macos.yaml) 配置 填写专用整合包参数，然后重新构建程序。该文件通过 Rust `include_str!` 编译嵌入，桌面和命令行均不会读取整包 `.updater/config.json`，修改旁置配置不能覆盖内置参数。将构建后的程序放在整合包根目录，根据 [examples](examples) 准备 `.updater/state.json`。客户端专用于蒹葭服务器整合包，`pack_name` 默认“蒹葭服务器整合包”，不随更新清单改名。`server_url` 在构建前填写完整 HTTP/HTTPS 服务地址。项目默认的 example.invalid 和示例 ID 必须替换，不能用于实际更新。已安装 release ID 必须来自本服务的正式发布，不能猜测。`game_dir` 和 `launcher_path` 相对程序所在目录；游戏实例在根目录时 `game_dir` 可为 `.`。

后台运行开关默认开启，用户选择保存在 `.updater/preferences.json`，无需修改管理员配置。开启时关闭窗口隐藏到托盘；关闭时关闭窗口与托盘“退出”一样，会取消可取消的任务并保留原记录，安装与保存版本阶段禁止退出。失败后只重试原计划，名单外文件不处理。完成通知不可用时仍可从托盘查看结果。

隔离演示环境只使用新建的临时目录，生成的是测试 JAR，不会启动 Minecraft：

```bash
python scripts/smoke_fixture.py
```

脚本生成的临时 client-build-config.yaml 仅作为联调参数参考；使用桌面或命令行联调前，需要把这些参数填写到项目 config/client.<platform>.yaml 并重新构建，不能靠旁置配置或环境变量覆盖。脚本打印临时后台地址、测试账号和 game 路径，Ctrl+C 停止服务。开发桌面可用 `PACKY_GAME_ROOT=<game 路径> npm run desktop`，生产构建忽略此覆盖。核心命令行可执行 `target/debug/packy-client <game 路径> update`、`retry`、`status`、`check`、`launch`。桌面与命令行不能同时操作同一整合包。

## 验证

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked
npm run format:check
npm run build
```

自动测试使用临时 SQLite、临时整合包与真实本机 HTTP 监听，覆盖认证、发布、托管上传/Range、下载、安装、重启重试、取消和路径边界。测试来源、真实 Modrinth HTTPS、原生桌面、Windows 和真实游戏验收分别记录在 [验证记录](docs/VERIFICATION.md)。

## 文档

[需求](docs/PRD-v0.2.md) · [架构](docs/ARCHITECTURE.md) · [计划](docs/DEVELOPMENT-PLAN.md) · [API](docs/contracts/API.md) · [数据库](docs/contracts/DATABASE.md) · [配置](docs/contracts/CONFIG.md)。[原始 PRD](docs/PRD-v0.1.md) 保留原文；Go/Wails 的旧 P1 证据只作历史记录。
