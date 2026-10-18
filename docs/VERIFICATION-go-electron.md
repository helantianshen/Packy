# Go / Electron 重构验证记录

日期：2026-10-18。当前代码已切换为 Go + Electron，以下仅记录新实现证据，不借用历史 Rust/Tauri 验收。未提交、推送或部署，真实数据库、玩家目录和本地服务配置未迁移。

## 阶段状态

| 阶段 | 已完成 | 尚未验收 |
|---|---|---|
| 清理 | 删除旧 Rust/Tauri 源码、Cargo 文件、target、旧依赖和产物，历史文档归档 | 新栈 node_modules/dist/artifacts 是本轮重新生成的必要构建文件 |
| P0/P1 基础 | Go 分层、手写协议、JSON-RPC stdio、固定官方后台模板、独立依赖锁、平台配置内嵌 | 未单独运行未改动的官方模板基线；三平台原生探针不完整 |
| P2 服务与后台 | Gin/GORM/Viper、手动初始化、多包、Mod/文件/草稿/冻结发布、托管下载、审计、真实 API 页面 | 真实反代部署、真实 Modrinth 网络导入尚未验收 |
| P3 更新核心 | 名单更新、并发下载、哈希验证、原任务重试、状态最后写入、锁与路径隔离 | 真正断电和平台文件占用故障未实测 |
| P4 桌面 | 安全 IPC、Go Sidecar、原生 Linux 窗口/托盘关闭行为、便携包构建 | Windows/macOS 原生运行、成功通知、实际游戏启动器、干净机验收 |
| P5 迁移与交付 | 人工 SQL 导入、合成旧库验证、README/协议/CI 更新 | 真实旧库副本迁移、真实整包和来源、远端 CI 尚未执行 |

代码阶段已落地，表中待验项意味着完整发布验收尚未完成。

## 本机环境与检查

Linux x86_64，Go 1.27.1，GCC 16.2.1，Node 24.20.0（仅当前命令 PATH，不修改全局 Node）。依赖以 go.mod/go.sum、各 package-lock.json 为准。服务端 SQLite 使用 CGO；客户端无数据库依赖，可关闭 CGO。

通过的检查：

```bash
go test -race -timeout 60s ./cmd/... ./internal/... ./pkg/... ./config/...
go vet ./cmd/... ./internal/... ./pkg/... ./config/...
npm run build
npm run format:check
npm run test:smoke
npm run desktop:build
npm run server:build
git diff --check
```

前端 build 包含 Vue/TypeScript 检查。Go 测试使用明确包路径，避免 `./...` 进入 node_modules 中附带的 Go 源码。Windows amd64 和 Darwin arm64 Sidecar 的 CGO_ENABLED=0 交叉编译通过，不等于平台运行通过。最终重打包时 Electron 默认下载连接中断；指定 `ELECTRON_MIRROR=https://npmmirror.com/mirrors/electron/` 重试后完成。该环境变量仅用于构建下载，不改变业务配置。

admin/desktop 依赖审计在兼容升级后为 0 条已知漏洞（扫描当时结果，不是长期安全保证）。

核心自动化覆盖：

- 管理员认证、注销使旧 Token 失效、多整合包隔离、草稿 revision 冲突、发布后冻结
- 成功审计失败则业务事务回滚，审计敏感信息脱敏
- 全部下载验证后才写游戏目录，错误 SHA-512 不破坏原文件
- 重启继续原任务，不请求新发布；名单外哨兵文件保持不变
- 检查取消、重复运行锁、符号链接/目录冲突拒绝、跨平台非法路径和严格 JSON 校验

`scripts/smoke.mjs` 在临时目录完成：未初始化启动拒绝 → 显式 migrate → 手动 init.sql → 登录 → 创建整合包/Mod/文件/草稿 → 发布 → 上传与 Range 下载 → stdio Sidecar 更新 → state 落盘与名单外文件校验 → 审计查询 → 注销。配置使用临时构建 overlay，未改动仓库平台配置或真实目录。

旧库导入使用根据历史 schema 构造的临时 SQLite，包含包、Mod、文件、来源和发布快照；新库 migrate/init 后执行 import-legacy.sql，通过 ID、来源和条目校验。此证据不是用户真实数据库迁移成功。

## 页面与原生运行证据

后台通过 Playwright CLI 访问本地临时服务，完成真实登录、创建整合包、草稿和发布，以及 Mod 表单、审计与文件抽屉检查。桌面 1440×1000、手机 390×844 截图已实际查看：

- `.impeccable/review/desktop.png`、`mobile.png`：审计页面
- `.impeccable/review/mods-drawer.png`、`mods-mobile.png`：文件版本抽屉

修复了 /admin/ 自动重定向循环、缺失菜单翻译、手机导航标题挤压、来源优先级没有可见说明。最终 UI reviewer 的 ship 仅覆盖所列优先级修复及手机来源 URL 不溢出；手机首屏截图不覆盖抽屉底部全部操作，不声称所有页面视觉验收。模板风格保持不变。

Linux Electron 开发入口及最终便携包均实际启动（不是浏览器模拟桌面），在含中文与空格的临时整合包路径初始化 Sidecar，读取 get_status=idle；开启后台运行后关闭窗口验证隐藏，关闭后台运行后请求安全退出。原生截图保存在本轮临时证据 `/tmp/packy-refactor-smoke/electron-linux.png` 和 `electron-portable-linux.png`，该临时路径不作为永久发布附件。没有实际启动游戏或验证通知中心展示。

## 实现细节与方案差异

- Sidecar 采用成熟 JSON-RPC 库；Main 每 250ms 获取状态。进度总量未知时返回 null；速度当前为单文件平均值，并非方案中的聚合滑动窗口速率
- Windows 文件替换采用 Go os.Root.Rename 的目录句柄实现，避免重新拼接绝对路径造成竞态。Windows 重解析点和文件占用行为仍需原生测试；不能把编译通过解释为该测试通过
- 下载后在目标同目录创建临时文件、校验并同步后替换。Unix 同步父目录；不承诺跨多个 Mod 的整个安装事务原子性。失败保留原任务重试，不自动回滚
- schema 使用 GORM AutoMigrate，正常启动不补表、不补管理员。应用事务维护发布不可变与关联约束，直接 SQL 操作必须遵守 DATABASE.md
- 上传磁盘与数据库不是一个原子事务；DB 失败可能留下孤立托管文件，记录错误，不自动扫描删除
- Electron 打包采用 @electron/packager（没有引入 Forge 的安装器流程）。构建输出使用 resources/sidecar，而非早期方案示例 resources/bin；Main 使用一致路径。Electron 便携包是完整目录，macOS 是 app bundle，不是单文件
- 当前没有代码生成、Goose、RBAC、多租户或自动升级；首次部署已有 scripts/server/deploy.sh 与 systemd service，不承担后续更新

## 发布前待执行

1. 将各平台配置中的占位服务地址/pack ID 替换为真实值，并用真实整包副本验收；不使用真实玩家目录做破坏测试
2. Windows/macOS 原生运行：中文/空格/长路径、重解析点、锁、目标被占用、断管、托盘、通知、启动器、签名和干净机依赖
3. 真实网络来源与反代：超时、429、重定向、Range、TLS、来源切换及真实 Modrinth 导入
4. 真实旧库副本迁移、历史托管地址兼容和历史任务恢复；保留旧库备份
5. 经用户授权后运行远端 CI/发布；本次没有提交或推送

## 全仓审查修复复验（2026-10-18）

本轮修复七项问题：Windows SQLite URI 盘符解析、Modrinth 选择请求乱序、跨历史版本升级的路径别名冲突、发布前后指针审计、服务退出等待在途请求、客户端初始化误报、下载失败诊断丢失。发布与预览现检查全部历史正式版本到候选版本的计划；无法安全升级时拒绝发布，既有禁止大小写/NFC 别名改名的客户端约束不变。

实际通过：

- `go test -race ./...`、`go vet ./...`；新增 URI/特殊字符数据库路径、历史升级发布拒绝、审计前后值、HTTP 503 诊断与 URL 脱敏回归
- `npm run build`、`npm run format:check`、`git diff --check`
- `npm run test:smoke`：临时数据库的认证、发布、上传/Range、Sidecar 更新、中文文件名、改名替换、名单外文件保留、审计、注销；新增 SIGTERM 时在途上传返回 201 后进程退出
- Playwright CLI 执行 `web/admin/tests/modrinth-selection.cjs`：A 请求晚于 B 返回，最终展示和导入仍为 B；桌面与 390px 截图已查看。此项使用模拟接口控制响应顺序，不代表真实 Modrinth 网络验收
- Linux 原生 Electron `gui-native.cjs`、`download-settings-native.cjs`：初始化误报消失、品牌回退、1–2 倍缩放、关闭隐藏、设置保存、重启恢复、恢复默认；品牌文件安全测试 2 项通过
- Linux x64 服务端与桌面便携包构建；新便携包实际启动并通过 Sidecar 保存设置。桌面首次 GitHub 下载超时，使用命令级 `ELECTRON_MIRROR=https://npmmirror.com/mirrors/electron/` 重试成功
- `bash -n release.sh scripts/server/deploy.sh`、`bash release.sh --dry-run`；未执行真实 systemd 安装和全平台发布

依赖：`image-size` 升至 2.0.4、`quic-go` 升至 0.59.1，锁文件同步。根目录、admin、desktop 的 npm audit 均为 0。govulncheck 的受影响调用与导入包均为 0；模块级仍报告 GO-2026-5932，涉及不再维护且无修复版本的 `golang.org/x/crypto/openpgp`。`go list -deps` 确认服务端/客户端不导入 openpgp，实际使用同模块 bcrypt，不为消除无关告警移除密码哈希依赖。

边界：Windows SQLite 修复已验证 URI 构造，未在 Windows 执行数据库；Windows/macOS 原生、真实外部来源及生产部署仍待对应环境验证。未提交、推送，未操作真实玩家目录或现有业务数据库。

## CI 补齐复验（2026-10-18）

工作流现以 verify 为产物构建门禁，配置 Linux/Windows/macOS 各 x64/arm64 六种原生 runner，另有 Linux x64 服务端、独立 admin 产物。每份产物先 tar.gz 归档后上传，缺失文件时报错；没有配置发布 Release 或自动部署。

接入 Go race/vet、全前端构建与格式、API/Sidecar/停服烟测、品牌资源测试、Playwright 后台乱序导入，以及 Linux Xvfb 下的 Electron 启动/缩放/关闭隐藏和下载设置持久化测试。六种客户端构建前均运行本平台客户端核心测试。Playwright 使用根目录固定版本依赖及独立 webServer，测试控制响应顺序，不依赖外网 Modrinth。

本机通过 actionlint 1.7.12、根 npm ci、npm audit（0）、Go race/vet、CGO_ENABLED=0 客户端测试、全前端构建、格式、品牌测试、烟测、两个 Electron 原生测试及 `npm run test:browser`（1 项）。Linux x64 桌面/服务端重打包通过，三类 tar.gz 检查确认客户端主程序/Sidecar 和服务端可执行权限，以及 SQL、部署脚本、service、admin index.html 均存在。`git diff --check` 通过。

本机使用 CachyOS 图形会话验证 Electron，没有安装/执行 Xvfb；CI 中的 Ubuntu 系统依赖和 Xvfb 启动，以及其余五个原生平台的实际运行结果，须由远端工作流确认。未触发远端 CI，未提交、推送。此前手动 Playwright CLI 夹具现由标准 `npm run test:browser` 入口调用。

## GitHub CI 与下载产物验收（2026-10-18）

代码提交 `382e7665b2aef0ae2a12cec35bc665642e3a3456` 的 [GitHub Actions 运行](https://github.com/helantianshen/Packy/actions/runs/39107967836) 已全部成功（8 个 job）。此前两轮暴露的 Electron 运行时未显式下载、Windows ESM import 直接使用盘符路径的问题均已修复。最终 CI 包含完整 verify、六个平台的原生客户端核心/品牌测试与打包，以及 Linux x64 服务端和独立后台构建。

八份 Artifact 全部实际下载，核对 ELF/PE/Mach-O 的架构、主程序和 Sidecar 配对、Unix 可执行权限、macOS bundle 符号链接、app.asar、初始化 SQL/配置样例/部署脚本/service、独立后台 index.html 和 JS 文件。下载的 Linux x64 便携包还在中文和空格临时路径中实际启动、连接内嵌 Sidecar 并保存下载设置，未接触真实玩家目录。详细大小、摘要和逐项记录见 [CI 证据](evidence/ci-2026-10-18.json)。

六种客户端为 linux/win32/darwin 各 x64/arm64，另有 packy-server-linux-x64 与 packy-admin。产物均为 tar.gz，再包在 GitHub Artifact ZIP 中；需依次解开 ZIP 和 tar.gz。此轮已验证真实远端 Ubuntu Xvfb 测试，覆盖前文待验项；Windows/macOS 的原生 GUI、托盘、通知、启动器与真实下载来源仍须单独验收。最终文档记录提交使用 [skip ci]，未修改已通过验证的应用或工作流代码。
