# Packy

为蒹葭服务器整合包提供名单式 Mod 更新。当前实现使用 Go 服务端/Sidecar、Electron + Vue 桌面、官方 Arco Design Pro Vue 管理后台。

配置与素材的修改位置、全部配置字段、固定行为参数、路径基准及构建归属见 [配置、固定参数、路径与打包清单](docs/contracts/CONFIG.md)。默认客户端配置和素材随程序构建携带，服务端部署密钥与运行数据独立管理。

## 工程

```text
cmd/server/          服务装配、migrate、password-hash
cmd/client/          stdio Sidecar
internal/server/     API → application → infrastructure，domain 为纯差异规则
internal/client/     更新编排、文件系统、JSON-RPC
pkg/protocol/        Go DTO 与跨平台校验
config/              Viper 配置、按目标平台内嵌客户端 YAML
web/desktop/         Electron main/preload 与 Vue
web/admin/           固定官方模板，真实 API，无 Mock/RBAC/注册
sql/                 手动初始化、旧库副本导入
```

Go 版本见 `go.mod`；Node 版本见 `.node-version`（24.20.0）。服务端 GORM SQLite 使用 CGO，构建需 C 编译器；Sidecar 不依赖数据库，支持 `CGO_ENABLED=0`。所有依赖分别固定在 `go.sum`、根及两个前端的 `package-lock.json`。

```bash
npm ci
npm --prefix web/admin ci
npm --prefix web/desktop ci
go mod download
npm run build
npm test
npm run test:smoke
```

`test:smoke` 需要 sqlite3 CLI，在临时目录生成数据库、配置、整合包和二进制，验证手动初始化、HTTP API、上传下载、JSON-RPC 更新、版本落盘、审计和注销，结束后清理。不会连接现有服务或修改真实数据。

## 首次准备服务端

1. 复制 `config/server.yaml.example` 为 `config/server.yaml`，填入随机 JWT 密钥（至少 32 字节），确认数据库是**新路径**，配置 `public_url`。
2. `go run ./cmd/server migrate`：显式 GORM AutoMigrate，仅准备结构。
3. `go run ./cmd/server password-hash`：从终端输入密码，生成 bcrypt hash；此命令不连接数据库。
4. 复制 `sql/init.sql`，将 `REPLACE_WITH_BCRYPT_HASH` 替换为上一步结果，然后手动运行 `sqlite3 data/packy-go.db < /path/to/prepared-init.sql`。只创建管理员 `admin` 和四类下载来源；重复执行会失败，不覆盖旧密码。
5. `go run ./cmd/server`，仅启动 HTTP API，默认监听 `127.0.0.1:8080`。后台单独启动，见下方说明。

正常启动不会创建数据库、迁移表或初始化管理员。缺少结构/基础数据时明确报错。默认读取 cwd 下 `config/server.yaml`；`PACKY_CONFIG` 指定其他文件。各字段 `PACKY_<大写字段名>` 覆盖 YAML，显式空值不回退。所有相对数据路径以服务端 cwd 解析。`public_url` 生成托管链接；反代子路径必须由 Nginx 去除后转发。服务本身只监听 HTTP，TLS 由反代负责。Linux 服务端包提供首次部署脚本 `deploy.sh`，它执行 migrate 和一次 init.sql，并随机生成 JWT 密钥；已有部署目录或服务时拒绝运行。

一个管理员管理多个整合包。先在「Mod 与文件」手动录入/托管上传，或从 Modrinth 导入，再在整合包中创建草稿、选择文件及路径、预览与发布。已发布快照及文件版本不可编辑；新变化创建新版本。管理写操作记录审计，不存密码、JWT 或 URL 查询凭据。

## 管理后台独立开发与构建

Go 服务端不托管前端、不读取 `admin_dir`，启动及服务端打包均不构建前端。

```bash
# 终端一：API（首次使用需先完成上述数据库初始化）
go run ./cmd/server

# 终端二：后台，Node 使用 .node-version 指定版本
cd web/admin
npm run dev
```

浏览器打开 `http://127.0.0.1:5173/`。Vite 将 `/api` 代理到 `http://127.0.0.1:8080`，可在 `web/admin/.env.development.local` 设置 `VITE_DEV_API_TARGET` 修改开发代理目标；客户端配置不受影响。GoLand 的服务端配置不需要任何 npm 启动前任务；另建 npm 配置，package.json 选择 `web/admin/package.json`，脚本选 `dev`。

在 `web/admin` 执行 `npm run build`，产物独立输出到 `web/admin/dist/`；构建后可执行 `npm run preview` 在 `http://127.0.0.1:4173/` 检查，预览代理同样默认指向8080。`preview` 不是生产服务器。

生产部署时将 dist 放在 Nginx 等静态服务器，前端路由回退到 `/index.html`，`/api/` 反向代理到 Go 服务端并保留路径。浏览器始终通过前端同源 `/api/v1/...` 请求，无需开放跨域通配符。根目录 `npm run build` 仍是显式全项目构建命令，不会被 Go 启动调用。

## 桌面配置与便携分发

编辑 `config/client.windows.yaml`、`config/client.linux.yaml`、`config/client.macos.yaml`。现有 `example.invalid` 仅为占位地址，分发前必须填写真实服务和 pack ID。Go embed 构建内嵌全部平台文件，运行时按编译目标选取；不接受玩家目录 `.updater/config.json` 或环境变量覆盖业务配置。

```bash
npm run desktop:build                 # 当前平台
npm run desktop:build -- win32 x64    # Windows x64
npm run desktop:build -- darwin arm64 # macOS arm64
npm run server:build                  # 当前主机 API 服务端包，含 SQL/配置样例，不含前端
```

分发 `artifacts/desktop/Packy-<platform>-<arch>` 的**完整目录**，不要仅复制 Electron 可执行文件。Windows/Linux 把该目录放在整合包根目录下；macOS 把其中 `Packy.app` 放到整合包根目录。根目录需已有 `.updater/state.json`，其中 `installed_release_id` 必须是该包的真实已发布版本。样例见 `examples/updater-state.json`。路径支持空格和中文，协议相对路径统一 `/`；不接受盘符、UNC、反斜杠、设备名、ADS、路径穿越或链接。

游戏目录与启动器路径来自内嵌配置。支持 Windows `.exe`、Linux/macOS `.sh`、macOS `.app`；JAR 启动器需设置相对根目录的 `java_path`。不搜索系统 Java，不通过 shell 拼接参数。桌面后台运行默认开启；安装/保存版本不可取消退出；下载可取消，重启只恢复原任务。

开发运行先 `npm --prefix web/desktop run electron:install` 显式下载 Electron 运行时，再 `npm --prefix web/desktop run build`，再 `go build -o web/desktop/sidecar/packy-client ./cmd/client`（Windows 加 `.exe`），设置绝对临时目录 `PACKY_DEV_ROOT` 后执行 `npm --prefix web/desktop start`。此变量仅开发定位根目录，不能覆盖内嵌服务地址。

客户端默认图片位于 `web/desktop/public/branding/`，替换后执行 `npm run desktop:build` 即可随包分发，无需玩家另放图片；原生图标源位于 `web/desktop/assets/`。

客户端采用 B 森林明暗主题；图标、两张背景及托盘图可在整合包 `.updater/branding/` 中按固定文件名替换，重启生效。具体格式、限制及 GUI 验证见 [客户端 GUI 与素材替换](docs/client-gui-verification.md)。

客户端“配置与说明”现提供下载设置：渠道代理、并发、超时、尝试次数及全局限速。默认值来自平台 YAML 的 `download`，玩家覆盖保存于 `.updater/preferences.json`；空代理强制直连，限速0不限速，保存后下次操作生效。详见 [下载设置](docs/specs/client-download-settings.md)。

## 一键发布与首次部署

运行根目录 `bash release.sh` 或 PowerShell `./release.ps1`。构建结果集中到 `release/build-<随机后缀>/`，分别包含当前主机服务端、独立 admin 静态文件和六种平台/架构客户端；失败目录保留 `.incomplete` 后缀。`--dry-run` 只预览命令。非 Windows 主机构建 Windows 图标需 Wine；服务端 CGO 仍需 C 编译器。产物不包含本机密钥和运行数据。

Linux 首次安装：在服务端分发目录执行 `sudo bash deploy.sh`，交互输入管理员密码。脚本安装到 `/opt/packy`，生成随机 JWT 密钥，准备数据库并安装、启用 `packy-server.service`，但不立即启动。编辑 `/opt/packy/config/server.yaml` 的 `public_url` 后执行 `sudo systemctl start packy-server`。此路径已经初始化，不要再次执行手动 init.sql；脚本不提供更新功能。

## 旧版数据

不要对 Rust 数据库原地 AutoMigrate。保留旧库和托管目录备份，在**新库**先 migrate + 手动 init，再按 `sql/import-legacy.sql` 注释填写旧库副本路径和 public URL，手动导入业务数据。旧账号、角色、权限不导入。托管文件复制到新 `files_dir`，沿用 `<ID>.jar`。原快照来源 URL 保留，切换域名/路径需确保旧链接仍有效。旧客户端 schema 1 的任务可读取，重试重新验证或下载 staging，绝不扫描玩家 Mod。

## 验证边界

见 [当前验证记录](docs/VERIFICATION-go-electron.md)、[人工协议](docs/contracts/API.md)、[架构方案](docs/specs/go-electron-redevelopment.md)。旧 Rust/Tauri 证据归档在 `docs/history/rust-tauri`，不能用于新栈验收。Linux 构建与交叉编译不等于 Windows/macOS 原生托盘、通知、文件占用或签名验收；CI 配置不代表远端工作流已经运行。

## GitHub CI

`.github/workflows/build.yml` 在 push、pull request 和手动触发时运行。先完成 Go race/vet、全前端构建、格式、API/Sidecar 烟测、品牌文件安全、后台乱序导入和 Linux Electron 启动/设置回归，通过后才构建交付产物。

- 客户端：Linux、Windows、macOS 各 x64/arm64，使用对应原生 runner；各平台额外运行客户端核心测试
- 服务端：Linux x64，包含配置样例、SQL、首次部署脚本与 service
- 后台：独立 admin 静态文件，不嵌入服务端

共八个独立构建产物。完整目录先生成 tar.gz，再上传 GitHub Artifact，以保留 Unix 可执行权限和符号链接；下载后需解开 Artifact 外层 ZIP，再解开 tar.gz。客户端包仍须按照上面的整合包目录约定放置，构建成功不代表真实来源、托盘、通知和游戏启动器已完成跨平台验收。

本地运行后台回归：根目录 `npm ci`，安装后台依赖后执行 `npx --no-install playwright install --no-shell chromium`、`npm run test:browser`。测试自动启动并停止 127.0.0.1:5187 的独立 Vite 服务，使用模拟 API 控制响应顺序，不访问真实业务库。Linux CI 通过 Xvfb 执行原生 Electron 测试；失败时保留测试诊断产物。已完成一次六平台远端运行及八份下载产物核验，链接与范围见 [验证记录](docs/VERIFICATION-go-electron.md)。
