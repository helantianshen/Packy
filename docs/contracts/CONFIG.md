# 配置、固定参数、路径与打包清单

以当前 Go / Electron 实现为准（2026-10-18）。本文收录应用配置字段、环境变量、运行目录、静态素材和影响部署/行为的固定参数；协议字段见 [API.md](API.md)，数据库业务字段见 [DATABASE.md](DATABASE.md)。普通局部变量不属于部署配置，视觉样式的全部 token 以 `web/desktop/src/shared/theme.css` 和 `DESIGN.md` 为准。

## 1. 哪些内容随构建携带

| 内容 | 源文件 | 构建后的去向 | 修改何时生效 |
| --- | --- | --- | --- |
| 客户端业务配置 | `config/client.<windows|linux|macos>.yaml` | Go embed 内嵌在 Sidecar 可执行文件 | 重新构建 Sidecar；分发时重新打包桌面 |
| 桌面 HTML/JS/CSS、默认 logo 和明暗背景 | `web/desktop/src/`、`web/desktop/public/branding/` | Vite dist → Electron `resources/app.asar` | 重新 desktop:build |
| 主进程/preload、日志策略、窗口参数 | `web/desktop/electron/` | dist-electron → app.asar | 重新 desktop:build |
| 运行时窗口/托盘默认图标 | `web/desktop/assets/icon.png` | `resources/assets/icon.png` | 重新 desktop:build |
| Windows EXE 图标 | `web/desktop/assets/icon.ico` | 打包时写入 Windows 程序图标 | 重新构建 Windows 包 |
| 服务端默认值与固定参数 | `internal/server/`、`cmd/server/` | 编译进 Go 可执行文件 | 重新 server:build |
| 服务端配置样例、SQL | `config/server.yaml.example`、`sql/` | 服务端分发目录的 config/、sql/ | 打包携带，首次手动配置/执行 |
| 管理后台页面、图片、样式、API 路径 | `web/admin/src/`、`web/admin/public/`（若有） | 独立 `web/admin/dist/` | npm run build 后部署整个 dist |

“随包携带”不等于所有内容都塞进一个 EXE：Electron 必须分发完整目录，Sidecar、Electron 运行库和 assets 都是包的一部分。桌面使用本地资源，不依赖开发机源码目录或远程素材 CDN；系统字体仍由目标系统提供。后台保持独立部署，不嵌入 Go 服务端，也不在服务端启动时运行 npm。

运行数据不能作为构建常量：数据库、管理员凭据、JWT 密钥、下载文件、玩家版本记录、任务进度、日志在运行环境中维护。不要将本机 `config/server.yaml`、数据库、凭据文件或 `.updater` 打入客户端。服务端无可用 JWT 密钥时拒绝启动，这是预期行为。

## 2. 服务端：全部配置字段

优先级：显式环境变量（包括空字符串）＞YAML＞程序默认值。默认 YAML 为 **工作目录** `config/server.yaml`；`PACKY_CONFIG` 可指定其他文件，指定后文件缺失即报错。默认文件缺失允许用环境变量启动。相对数据路径始终相对于工作目录，而不是 YAML 所在目录。

| YAML 字段 | 环境变量 | 程序默认值 | 含义 |
| --- | --- | --- | --- |
| listen | PACKY_LISTEN | `127.0.0.1:8080` | HTTP 监听；TLS 由反向代理负责 |
| database | PACKY_DATABASE | `data/packy-go.db` | SQLite 文件；不能为空 |
| files_dir | PACKY_FILES_DIR | `data/files` | 托管 JAR 目录 |
| public_url | PACKY_PUBLIC_URL | `http://127.0.0.1:8080` | 生成下载链接的公开 HTTP/HTTPS 地址，不能有查询参数 |
| jwt_secret | PACKY_JWT_SECRET | 空 | 至少32字节；由部署环境提供，不放客户端 |
| modrinth_user_agent | PACKY_MODRINTH_USER_AGENT | `Packy/0.2` | 上游请求 User-Agent；样例要求填写联系方式 |
| log_filter | PACKY_LOG_FILTER | `info` | Zap 日志级别：debug/info/warn/error/dpanic/panic/fatal |

入口 `cmd/server/main.go`：无子命令启动 API；`migrate` 手动 AutoMigrate；`password-hash` 仅生成密码哈希。`sql/init.sql` 可替换占位哈希后手动执行，也可由首次部署脚本执行一次，不随启动自动执行。Linux `deploy.sh` 自动生成 JWT 密钥，安装到 `/opt/packy`，配置和数据库分别位于其 `config/server.yaml` 与 `data/packy-go.db`；安装服务后需维护者配置公开地址并手动启动。`admin_dir` / `PACKY_ADMIN_DIR` 已移除。

## 3. 客户端：全部业务配置字段

文件名中的 macos 对应 Go 的 darwin。当前 embed 包含三份 YAML，程序按编译目标选择。字段没有环境变量覆盖，不读取玩家 `.updater/config.json`。

| 字段 | 含义与路径基准 |
| --- | --- |
| pack_name | 界面整合包名称，必填 |
| pack_id | 服务器真实整合包 UUID，必填；不是发布版本 ID |
| server_url | HTTP/HTTPS 服务地址，不能含查询参数 |
| game_dir | 相对整合包根目录的游戏目录，例如 `.minecraft`；允许 `.` |
| launcher_path | 相对整合包根目录的启动器路径，必填 |
| java_path | JAR 启动器使用的 Java 路径，相对整合包根目录；其他启动器可留空，不搜索系统 Java |

当前源码 Windows/macOS 地址仍为 `https://example.invalid`，Linux 为本地 `http://127.0.0.1:8080`；三个 pack_id 仍是 `11111111-1111-4111-8111-111111111111`。这些是占位/开发配置，构建不会自动取得后台 ID，分发前必须改成真实值。

所有协议相对路径统一 `/`，Windows 配置也写 `runtime/bin/java.exe`，不要写反斜杠、盘符或 UNC。禁止穿越、链接、设备名、ADS、非法字符、末尾点/空格。更新清单 `mods/...` 相对于 game_dir。新发布 JAR 安装名为 `[后台名称快照]原始文件名.jar`，完整规则见 API.md。

## 4. 环境变量与构建入口

| 变量/参数 | 使用位置 | 生效范围 |
| --- | --- | --- |
| PACKY_CONFIG、PACKY_各字段 | 服务端 Viper | 运行时 |
| PACKY_DEV_ROOT | Electron Main | 仅开发模式，必须为整合包绝对路径；打包程序忽略 |
| VITE_DEV_API_TARGET | admin/.env.development.local | 后台 dev/preview 的代理目标，默认8080；不是生产运行时配置 |
| GOOS / GOARCH / CGO_ENABLED | Go 工具链 | 桌面脚本按目标设定；Sidecar CGO=0；SQLite 服务端需要 CGO/C 编译器 |
| npm_execpath | 构建脚本 | npm CLI 定位；通常由 npm 自动提供 |
| PACKY_PLAYWRIGHT_MODULE | 桌面原生测试脚本 | 测试用 Playwright 模块路径，不是生产配置 |
| XDG_CONFIG_HOME / APPDATA 等系统变量 | Electron 平台目录解析 | 系统决定日志及应用数据基目录，不写成开发机绝对路径 |
| desktop:build 后的 platform arch | scripts/build-desktop.mjs | linux/win32/darwin，x64/arm64；默认当前主机 |

Node 版本由 `.node-version` 固定24.20.0；Go版本见 go.mod。不要把 GoLand 中的绝对工具路径、测试目录或用户 HOME 写入产品配置。后台生产 API 基址固定同源 `/api/v1/admin`，认证使用 `/api/v1/auth/*`，Nginx 需保留 `/api` 路径转发；Vite 的代理不随 dist 变成生产代理。

## 5. 静态资源：改哪里、如何打包

默认素材直接在以下源码位置替换，然后运行 `npm run desktop:build`，无需用户再复制外部图片：

| 源文件 | 用途 |
| --- | --- |
| web/desktop/public/branding/logo.png | 页面默认品牌图 |
| web/desktop/public/branding/background-light.webp | 浅色背景 |
| web/desktop/public/branding/background-dark.webp | 深色背景 |
| web/desktop/assets/icon.png | 默认窗口/托盘图标 |
| web/desktop/assets/icon.ico | Windows 文件管理器中 EXE 图标 |
| web/desktop/public/branding/*.json | 素材来源说明，随 Vite public 复制 |

当前 macOS 打包脚本未配置自定义 `.icns`，不能将替换 icon.png 等同于更改 Finder 中的 APP 图标。

保留可选运行时替换：整合包 `.updater/branding/` 下的 `logo.png`、`background-light.webp`、`background-dark.webp`、`tray.png`、`tray@2x.png`、`trayTemplate.png`、`trayTemplate@2x.png`。重启读取，缺失或无效回退内置素材。每个文件≤8MiB，单边≤8192，像素总数≤3200万；仅PNG/WebP固定名称，不接收远程URL或任意文件路径。此目录是可选覆盖，不是安装必需依赖。

主题 token、字号、间距、响应式及动画在 `web/desktop/src/shared/theme.css`、`theme.ts`、`App.vue`；这些会编译进包。后台样式与模板设置在 `web/admin/src/assets/` 和 `src/config/`，当前保留官方模板风格。

## 6. 安装及运行路径

| 路径 | 基准/用途 |
| --- | --- |
| artifacts/desktop/Packy-platform-arch | 仓库构建输出；Windows/Linux 整个目录放在整合包根下 |
| Packy.app | macOS 将此应用包放在整合包根下 |
| resources/app.asar | 桌面编译代码、依赖、默认页面资源；macOS 位于 Contents/Resources |
| resources/sidecar/packy-client[.exe] | 更新核心，不应独立作为 GUI 启动 |
| resources/assets/icon.png | 原生图标 |
| artifacts/server-platform-arch | 服务端二进制、配置样例、SQL、README |
| web/admin/dist | 独立静态后台部署产物 |
| `.updater/state.json` | 整合包根下，installed_release_id 必须匹配真实已发布版本；发行整合包时另行提供，不能猜测 |
| `.updater/preferences.json` | 整合包根下，background_enabled 默认true |
| `.updater/task.json` | 整合包根下，原任务计划及恢复进度 |
| `.updater/staging/<task_id>/<操作索引>.jar` | 整合包根下，下载暂存；下载中另加 `.<UUID>.part` 后缀 |
| `.packy-<UUID>.tmp` | JSON或目标JAR所在目录中，用于原子替换的临时文件 |
| `.updater/packy.lock` | 整合包根下，进程互斥锁 |
| `<game_dir>/mods/` | 只安装/删除清单里的目标，不扫描其他 Mod |
| `<files_dir>/<ID>.jar` | 服务端托管文件，文件名不是客户端安装名；上传中使用 `.upload-*` 临时文件 |
| SQLite 的 -wal / -shm 文件 | 数据库旁，由 SQLite 管理，备份须考虑一致性 |

生产整合包根目录：Windows/Linux 为 Electron可执行文件所在目录的上一级；macOS 为 .app 外的目录。开发模式只用 PACKY_DEV_ROOT。

日志位置为 Electron `appData/Packy/logs/client.log`，5MiB轮转为 client.old.log。Linux通常 `~/.config/Packy/logs/`，Windows `%APPDATA%/Packy/logs/`，macOS `~/Library/Application Support/Packy/logs/`。Electron userData 下使用整合包根路径SHA-256前24位区分会话；桌面主题 localStorage 键为 `packy-theme`。后台Token存在 sessionStorage，键为 `token`（`web/admin/src/utils/auth.ts`）。服务端日志输出终端，业务审计持久化数据库，不能误认为也写客户端 client.log。

## 7. 固定行为参数索引

下表列出程序固定行为及默认值；下载并发、超时和尝试次数已支持 YAML 默认值及玩家覆盖，见末尾下载偏好章节。调整前应核对相关协议和测试。

| 参数 | 当前值 | 权威源码 |
| --- | --- | --- |
| HTTP API版本/程序版本 | /api/v1、0.2.0 | server/api/router.go、client/application/update.go、client/rpc/server.go、desktop/package.json |
| JWT有效期/issuer/audience | 30分钟、packy、packy-admin | server/application/auth.go |
| 登录限流 | 全局每秒1次，突发5次 | server/api/router.go |
| HTTP请求头/空闲/关闭等待 | 10秒/60秒/10秒 | cmd/server/main.go |
| JSON请求上限 | 1MiB | server/api/router.go |
| 上传JAR上限 | 512MiB；multipart请求额外1MiB | server/application/files.go、api/router.go |
| Modrinth地址 | https://api.modrinth.com/v2 | server/api/router.go |
| Modrinth超时/响应上限/哈希数量 | 30秒/8MiB/100 | server/api/router.go |
| 分页默认/最大 | 20/100 | server/api/router.go |
| SQLite | WAL、foreign_keys=on、busy_timeout=5000ms、immediate事务、最大连接数1 | server/infrastructure/database.go |
| 检查更新超时/响应上限 | 30秒/8MiB | client/application/update.go |
| 单次下载超时/并发 | 10分钟/4 | client/application/update.go |
| 下载尝试 | 总计3次，轮换来源，间隔500ms/1000ms；Resty内部重试0 | client/application/update.go |
| 重定向 | 最多5次，禁止HTTPS降级，移除认证头和Cookie | client/application/update.go |
| 下载缓冲区 | 64KiB | client/application/update.go |
| 文件替换重试 | 3次，等待200ms | client/application/update.go |
| 本地JSON读取上限 | 8MiB | client/infrastructure/files.go |
| RPC版本/帧/头部 | 1 / 8MiB / 4KiB | client/rpc/server.go、desktop/electron/main.ts |
| 任务记录schema | 2，兼容1 | client/application/client.go |
| Sidecar握手超时/状态轮询 | 15秒/250ms | desktop/electron/main.ts |
| 窗口初始/最小尺寸 | 1060×760 / 720×560 | desktop/electron/main.ts |
| 托盘尺寸 | macOS18，其他24 | desktop/electron/main.ts |
| 应用标识 | Packy；bundle ID top.hikaritish.packy | desktop/electron/main.ts、scripts/build-desktop.mjs |
| 日志大小/单条文本 | 5MiB / 16384字符 | desktop/electron/logging.ts |
| 后台dev/preview端口 | 127.0.0.1:5173 / :4173，strictPort | admin/config/vite.config.dev.ts |
| 桌面Vite预览开发端口 | 127.0.0.1:5174；不是Electron启动服务 | desktop/vite.config.ts |
| JAR名称组件限制 | 255 UTF-8字节及255 UTF-16码元 | server/application/filenames.go |
| SHA-512与路径规则 | 128位小写十六进制、mods/下.jar、大小写/NFC冲突检查 | pkg/protocol/validation.go |

表中 server/client 源码前缀为 `internal/`，desktop/admin 前缀为 `web/`。完整路由、状态机、下载来源类型和审计契约见 API.md；视觉尺寸/颜色/动效 token 请直接查看上述样式文件，不通过环境变量修改。

## 8. 验证与发布前检查

运行 `npm run desktop:build`、`npm run server:build` 与 `npm --prefix web/admin run build`。检查桌面 app.asar 内 dist/branding、dist-electron 和生产依赖，包外 sidecar/assets 是否齐全。将包放在临时整合包根目录验证，不能从开发目录可启动推断发布包完整。

客户端 YAML 改动后只执行 npm build 不会重新编译 Sidecar；应执行 desktop:build。服务端默认值改动后需要重新编译，外部部署配置改动后重启即可。后台 VITE_DEV_API_TARGET 只改变开发代理，生产目标在反向代理中配置。

跨平台路径约束由协议和 Go os.Root 实施；Windows检查Reparse Point，使用Go原生文件API；Unix同目录rename并同步父目录。构建成功不代表Windows/macOS原生托盘、通知、锁占用和断电恢复已验收。

## 9. 本轮构建核验（2026-10-18）

Linux x64 实际执行 desktop:build、server:build、后台 build 均通过。读取新生成的 app.asar，逐字节比对 logo/明暗背景与源码资源一致，确认 main/preload 和运行依赖存在；逐字节比对包外 icon.png；确认 Sidecar 二进制内包含三份平台 YAML 原文。API 服务端包包含二进制、样例、SQL、README，不包含本机 server.yaml 或后台；独立后台 dist/index.html 存在。

本次修改为文档完善，沿用已有内嵌和打包机制，没有更改部署配置、密钥、素材或业务行为。以上是构建及包内容证据，不是重新进行 GUI 或跨平台实机验收。

## 客户端下载偏好补充

客户端现在支持 download 默认配置块，以下内容写入三个平台YAML并随Go embed构建：

```yaml
download:
  max_concurrent_downloads: 4
  check_timeout_seconds: 30
  download_timeout_seconds: 600
  max_attempts: 3
  speed_limit_kib: 0
  proxies:
    modrinth: ''
    curseforge: ''
    github: ''
    server: ''
```

玩家在设置页逐字段覆盖，保存在整合包 `.updater/preferences.json`；恢复默认删除覆盖，空代理显式直连，限速0不限速。不读取系统代理环境变量。代理按渠道同时覆盖HTTP/HTTPS，server渠道还作用于检查更新。支持无认证HTTP/HTTPS代理地址。限速单位KiB/s，按所有下载合计，允许64KiB突发。

上述参数替代第7节对应的固定超时/并发/尝试次数；来源轮换、等待间隔、RPC大小等其余限制仍固定。当前操作使用启动时的设置，新值下次检查、开始或重试生效；服务地址等业务配置仍不可编辑。字段范围、兼容和测试见 [下载设置](../specs/client-download-settings.md)。
