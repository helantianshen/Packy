# Rust 版本验证记录

日期：2026-10-15。本文只记录实际证据；旧 Go/Wails P1 记录不计入本次验收。测试均使用临时库和整合包副本，未触碰真实玩家数据。

## YAML 配置迁移（2026-10-15）

后续已将默认服务端配置和分发包模板统一至 config 目录，原配置内容、密钥及权限原样保留。服务端测试、Clippy、格式、构建脚本检查与 Linux 重打包通过；解包后按默认 config/server.yaml 验证初始化、healthz、后台及 JWT 登录通过，相对数据路径仍以工作目录解析。见[目录迁移验证](evidence/config-directory-verification.json)。

服务端统一采用环境变量 > config/server.yaml > 默认值，三端客户端使用构建嵌入的 YAML。配置进程回归、既有 Rust 测试、全工作区 Clippy、格式检查、构建脚本测试、两端前端构建和 Linux 桌面 debug 构建通过。Linux 服务端包已重建并在临时目录验证 YAML 初始化、healthz、后台 HTML 和 JWT 登录。详情见[任务记录](../.agent/tasks/yaml-configuration.md)与[产物验证](evidence/yaml-config-verification.json)。Windows/macOS 原生运行和 GUI 交互未重新验收；之前 JSON 配置版本的归档记录只作为历史证据。

## 当前阶段

| 阶段 | 实现 | 已取得证据 | 剩余验收 |
|---|---|---|---|
| P1 工程/契约 | common/sources/server/client/Tauri、SQL、类型生成 | Rust 编译与前端构建通过，Linux 桌面启动并注册托盘 | Windows 构建与实机 |
| P2 账号/权限 | JWT/RBAC、初始化、注册开关、用户/权限/字典 | HTTP 集成测试通过，后台实际登录通过 | 生产部署环境 |
| P3 发布/文件 | Modrinth、Mod/Artifact、草稿、快照、差异、托管上传/Range | 真实 HTTP 测试；后台创建/预览/发布/回读通过；真实 Modrinth 搜索和文件录入通过 | 第三方四源真实环境完整链路 |
| P4 更新核心 | 并发、校验、安装、原记录重试/取消、版本、启动器 | 真实 HTTP 联调与边界测试通过 | Windows 文件语义、真实整合包游戏 |
| P5 界面/生命周期 | 完整明暗桌面和后台、IPC、托盘、通知 | 类型/生产构建、后台交互和主题截图；Linux 原生进程持有整包锁且托盘注册成功；用户人工确认窗口恢复、后台更新和完成通知均正常；原生窗口帧截图复现并验证导航闪动修正 | Windows 原生交互与通知 |
| P6 分发/部署 | 无安装包的可执行文件构建、Nginx/整包说明、隔离联调脚本 | 自动化与本机联调记录 | Windows、Nginx 实际部署、真实游戏验收 |

## 自动化覆盖

- `crates/server/tests/auth_system.rs`：参数化首个超管、重复初始化不覆盖；大小写不敏感登录；缺 JWT、错误签名/算法、过期/错误签发方/受众、未来签发时间；注册关闭/开放；夹带角色拒绝；普通用户与系统角色边界；最后一位超管和关键权限保护；旧 Token 实时权限撤销、账号禁用；注销撤销；重开数据库不覆盖设置。
- `crates/server/tests/schema.rs`：直接 SQL 验证 modid 唯一但名称可重复、外键、非法哈希、跨包指针、Artifact 所属 Mod、已发布记录/条目/Artifact 拒绝修改和历史身份保留。
- `crates/server/src/domain/releases.rs`：两个完整清单计算最终新增/替换/删除、同版无操作、重复名称不承担身份、目标路径冲突拒绝。
- `crates/client/tests/end_to_end.rs`：真实 HTTP 服务和真实文件目录；账号、Mod、Artifact、草稿、正式发布、计划 API；托管上传的真实 SHA-512 与 Range；下载失败不修改旧文件；服务端发新版/禁用字典后原记录不变；安装失败不推进版本、旧路径已不存在时可重试；重启恢复原任务；名单外文件字节不变；业务 modid 编辑后历史快照不变；Mod 列表与字面 `%/_` 搜索。
- `crates/client/tests/boundaries.rs`：最多 4 个文件并发；可取消下载不修改实例；取消后可安全退出；版本记录保存失败不得完成或放行启动器；恢复原记录后可重试；Unix 链接逃逸拒绝；同大小错误内容由哈希拒绝、备用源成功、未知大小、有界三次尝试和临时文件清理。
- `crates/common/src/validation.rs`：Windows 设备名、越界、非法字符、中文/空格路径、哈希格式、密码字节长度、包含中间相似名称时的父子路径冲突。
- `crates/sources/src/download.rs`：四类来源保持原 URL，不附带管理员认证或 API Key。

本轮 `cargo fmt --all --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`、`cargo test --locked`、前端格式/类型/生产构建和 `npm audit` 均通过；审计报告为 0。`npm run desktop:build` 通过，生成 Linux `target/release/packy-desktop`，没有生成安装包。server/client 的 release 构建通过；release 服务端在独立临时库完成 init-admin、HTTP health、JWT 登录和认证身份读取。后续新增检查仍以对应实际结果为准。

## 真实后台与网络

隔离环境脚本：`scripts/smoke_fixture.py`。后台经真实 HTTP 提供静态资源和 API，浏览器已完成登录、从正式版创建草稿、改目标路径、保存/预览 REPLACE、发布并回读完整快照。Modrinth 搜索 `sodium`，以 Minecraft 1.21.1 / NeoForge 筛选并录入实际文件元数据；不存在模拟成功响应。

后台截图：[深色](evidence/rust-admin-dark.png)、[浅色](evidence/rust-admin-light.png)。客户端已从真实 Modrinth HTTPS CDN 下载 Sodium 1.21.1 NeoForge，并与 HTTP 文件源在同一次计划安装；SHA-512、目标版本、名单外文件和测试启动器标记均核对通过。原始结果见 [HTTPS 联调记录](evidence/rust-https-smoke.json)。测试启动器只写标记，不代表启动过 Minecraft。

## 原生桌面边界

Linux 已编译并启动 Tauri 原生应用。启用 custom-protocol 后嵌入前端，由 Vue 初始化 get_status 进入 Rust 核心，进程实际持有隔离整合包 `.updater/packy.lock`；系统 StatusNotifierWatcher 中可读取该进程的托盘注册。未经该 IPC 初始化不会创建持久持锁的核心实例。

2026-10-15，用户在已打开的 Linux 隔离测试窗口按“检查更新 → 查看并安装 → 确认安装”流程检查，并在下载时关闭窗口、从托盘重新打开；用户反馈窗口恢复、后台更新和完成通知“均正常”。这三项记为本次 Linux 调试构建的人工验收通过，不扩大为 Windows 或其他构建的实测证据。当前自动化工具没有原生窗口操作能力，此结论来源于用户人工反馈。GTK 的系统主题 color-mix 警告和 libappindicator 弃用提示未修改系统配置。临时 WebDriver 探索未完成原生交互，不计入通过证据。

## 未通过或未执行

- Windows 10/11 x64 的实际构建/启动、WebView2、托盘/通知、联接、占用、路径和启动器。
- 使用真实完整 Minecraft 整合包启动 HMCL/PCL2 和游戏。
- 实际 Nginx 证书/反代部署与完整公网链路；当前机器未安装 Nginx。
- CurseForge/GitHub 实际发布文件和平台权限条件；已通过统一直链协议与受控 HTTP 测试，不能据此声称所有外部链接都可用。

## 客户端与管理后台动效调整

2026-10-15：两端统一页面淡入/淡出、轻微位移和主题颜色渐变；客户端加入首次读取骨架、请求/任务加载反馈、进度平滑变化和确认弹窗入场，后台登录和五个页面加入真实请求加载提示。尊重系统减少动态效果设置，无新增动画依赖。

前端类型/构建/格式检查与桌面 debug、release no-bundle 构建通过。浏览器实测两端页面切换、客户端连续导航、主题中间色及 280ms 过渡、后台真实加载提示；模拟减少动态效果时过渡为 0s，之后已恢复默认偏好。客户端控制台未见 warn/error。预览截图：[浅色](evidence/ui-motion-client-light.png)、[深色](evidence/ui-motion-client-dark.png)。这些是浏览器预览，不能替代新构建的原生骨架/加载和动画体验验收；此前托盘生命周期人工结果仍属于原先测试构建。

### 客户端加载交接与检查更新布局回归

2026-10-15：骨架与实际内容保留同一卡片，只过渡透明度；版本行与状态文本预留高度，初始化卡片使用 inert。checking 不再挂载下载进度卡，真实 task_id 的任务仍显示进度。

隔离浏览器以真实 App.vue 和受控 composable 状态测量：普通窗口检查前/中/后说明顶部均 584.59375px；760×560 下均 563.59375px；骨架交接前后卡片高均 267.59375px。模拟文件任务的进度条仍存在。记录见 [布局回归数据](evidence/client-layout-regression.json)，截图见 [布局测试页](evidence/client-layout-regression.png)。此测试没有调用原生 IPC 或执行更新，不代表原生帧率已测量。前端类型/构建/格式以及桌面 debug/release no-bundle 构建通过。

### 客户端导航动画结束闪动回归

2026-10-15：客户端首页/配置页保留挂载，在同一个 grid 单元共同占位，只交叉淡化 opacity/visibility；不使用整页 transform，不在切页动画结束时卸载页面。隐藏页设置 inert、aria-hidden 和 pointer-events，较短页面保留共同占位以稳定页脚。

受控浏览器对前向、反向、760×560 连续导航各采样 181 帧：根页面始终两个，根节点 childList 变化为 0，内容高度、页脚位置和滚动区域宽度在各次采样内不变；没有两页同时全透明的采样帧，整页 transform 始终 none。减少动态效果时两页 transition 均为 0s。浏览器控制台未见 warn/error。数据见 [导航回归记录](evidence/client-navigation-regression.json)，截图见 [导航布局测试页](evidence/client-navigation-regression.png)。这不是原生帧率测试。

前端类型/生产构建/格式检查、Linux desktop debug/release no-bundle 构建通过。原生窗口最终视觉仍待用户用新构建确认；不从浏览器数据推断显卡或 WebKit 层面的问题已经排除。

### 实际 Tauri/WebKit 原生暗帧复现与第一轮修复

2026-10-15，用户明确授权使用本机 WebKit 调试接口和窗口录屏，仅操作隔离副本。此次没有用浏览器预览替代原生验收，也没有修改系统设置或停止用户原有客户端。

环境为 CachyOS / niri Wayland、GTK3 / WebKitGTK 4.1，运行真实 debug `packy-desktop`。WebKit 调试端点只绑定测试进程的 127.0.0.1 地址，niri 按该进程的窗口 ID 截取实际合成画面。测试目录为 `/tmp/packy-native-flash-apjr5u33/game`；原始逐帧截图及时间记录保留在同级测试目录，导出慢放见下方。

核对运行时资源：修复前实际窗口加载 `index-538zqyJg.js` 与 `index-B42pwRfF.css`，与当时 dist 及运行二进制中记录一致，因此不是旧前端资源导致。原生修复前的 100 张窗口截图中第 24 张出现内容近空白；内容区域相对背景的颜色差异降到稳定基线的约 5.3%。该指标只辅助定位近空白帧，不是帧率或普适性能指标。

原生 A/B：给页面添加 will-change:opacity 仍能录到异常暗帧，未采用。保留透明度过渡、取消动画结束时 visibility:hidden 的切换，正反方向对照未再出现近空白帧。结果支持本机 WebKit 闪动与结束时的合成可见性切换有关，未据此判定跨平台引擎缺陷或归因于某张显卡。

该轮只修改客户端导航 CSS：不设置 visibility，也不强制 will-change；隐藏页仍用 opacity:0、pointer-events:none、inert 与 aria-hidden 隔离。重新构建并重启隔离原生进程后，确认加载 `index-DrbC-B-H.js` / `index-CbvBXiaq.css`，没有注入临时样式，隐藏页 inert=true。两轮正反向切换共 400 张原生窗口截图，没有再出现上述近空白帧。debug / release no-bundle、前端类型/构建/格式检查通过；release 只做了构建，实际窗口记录来自 debug，Windows 未验证。

证据：[原生对照数据](evidence/client-native-flash-verification.json)、[修复前异常帧](evidence/client-native-before-dark-frame.png)、[修复后原生窗口](evidence/client-native-after.png)。慢放逐帧保留 100 张截图、以 30fps 导出，不能按视频时长推算真实性能：[修复前慢放](evidence/client-native-before.mp4)、[修复后慢放](evidence/client-native-after.mp4)。

RustRover 启动说明已补入 README：纯 Cargo 启动不执行 npm 构建，内嵌模式需在 Before launch 构建前端；Tauri CLI dev 才启动 Vite。当前问题已核验并非漏构建，但这个启动步骤仍需固定下来，避免后续加载旧 dist。

### 原生旧页回显与文字重影回归

2026-10-15 用户提供配置页叠加旧首页的截图。复查上轮原生录像：旧标题在首次清空后又于第 28/29 帧出现，反向切换的旧配置内容于第 35/36 帧回显。此前近空白帧检查未覆盖该问题，因此“无暗帧”不等于“无旧页回显”。

当前方案只让新页进行 180ms、opacity 0.86→1 的轻度入场，旧页立即 opacity=0，取消旧页退场动画。双页继续共同占位并用 inert/aria-hidden/pointer-events 隔离，不在结束时切换 visibility。

全新构建的实际 Tauri/WebKit 窗口加载 index-C5LF-pwx.js / index-BlV5D5ha.css，无临时样式。两轮正反导航共 400 张原生截图；逐帧样本只有一个页面可见，隐藏页透明度始终为 0，旧标题和旧配置内容的专用像素区域一旦清空，之后均未再次出现旧内容。未录到近空白帧。该像素检测只针对本机固定窗口中的这两处旧内容，不是通用视觉或性能保证。

数据：[原生重影回归](evidence/client-native-ghost-verification.json)。视频均逐帧保留、以 30fps 慢放：[修正前旧页回显](evidence/client-native-ghost-before.mp4)、[修正后切换](evidence/client-native-ghost-after.mp4)。原生截图：[修正前](evidence/client-native-ghost-before.png)、[修正后](evidence/client-native-ghost-after.png)。

前端格式/类型/构建与桌面 debug/release no-bundle 构建通过。此轮实际窗口验证来自 Linux debug；release 仅构建，Windows 未验证。


## 客户端 Motion 切页与取消晃动（2026-10-15）

客户端使用固定版本 motion-v 2.7.0 的 AnimatePresence wait 模式管理页面挂载/移除，清理原双页叠放及整页透明度关键帧。按用户最新要求，页面没有水平位移、淡入淡出或整页关键帧；按钮、加载提示、进度条等局部反馈保留。Rust 业务代码及 useUpdater 未修改。

前端类型/生产构建/格式检查，Linux desktop debug/release custom-protocol 构建均通过。原生隔离窗口确认加载最终嵌入资产 index-ClNO6d_y.js / index-BFJvmnSp.css。无清单、有清单两种状态，共 10 轮、1800 张合成窗口截图、1118 个 RAF 样本；其中 4 轮分别连续点击 25 次、间隔 30ms，最终页均正确。原生采样始终一个页面、opacity=1、transform=none，同页位置固定。主内容以每 6 像素采样对照两个稳定页面，未出现无法匹配的混合/空白帧，普通切换未发现旧页回显；人工查看截图联系表未见重影。

[验证数据](evidence/client-motion-native-verification.json)、[原生截图联系表](evidence/client-motion-navigation.png)、[普通切页录制](evidence/client-motion-navigation.mp4)、[快速连续切页录制](evidence/client-motion-rapid.mp4)。录制按 30fps 慢放并保留每张截图，不是实际 FPS。测试仅在临时整合包运行，取得清单后未执行下载和安装；截图采样不等同于覆盖每次显示刷新，Windows 尚未实机验证。本节替代前面历史修正的当前实现说明。

本轮隔离客户端和 WebKit 调试端口已随测试进程停止关闭；既有测试服务和用户窗口未停止。


## 检查更新组件闪动回归（2026-10-15）

原生基线确认 checking 期间主按钮/启动器被移除并重建，按钮 disabled 导致 opacity=0.5 后渐变恢复，且页面文案短暂进入后台更新展示。App.vue 将检查反馈与下载任务展示分开，主操作节点保持，检查期间启动器保留但禁用；请求超过 150ms 才显示加载反馈，顶部文案和页脚保持稳定，加载图标尺寸固定。业务核心与 IPC 未修改。

前端类型/生产构建/格式检查及 Linux desktop debug/release custom-protocol 构建通过。实际隔离 Tauri 加载最终资源 index-DOl_b1pV.js / index-CBrS1Wu0.css；2 次快速检查和 1 次 400ms 延迟检查共 540 张原生截图、411 个 RAF 样本，节点、按钮亮度及位置检查通过。快速检查主内容的每 8 像素网格样本全程不变；慢检查可见真实加载反馈、按钮禁用且不移动。测试用临时本机代理转发真实测试服务端请求，没有安装或下载文件。

[验证数据](evidence/client-check-native-verification.json)、[修正前](evidence/client-check-before.mp4)、[快速检查修正后](evidence/client-check-after.mp4)、[慢响应反馈](evidence/client-check-slow.mp4)。视频按 30fps 慢放，不是实际 FPS；截图采样不覆盖每次显示刷新，Windows 未实机验证。临时客户端、调试接口及代理已关闭，原有服务未停止。


## 蒹葭专用配置与后台开关（2026-10-15）

本地 config.json 的 pack_name 控制首页名称，旧配置默认蒹葭服务器整合包；服务端 URL 由组包填写完整 HTTP/HTTPS 地址，实例与启动器仍使用相对更新器目录的安全路径。preferences.json 独立保存后台运行开关，默认开启。

新增核心测试通过：旧配置兼容、偏好重启与移动后保留、开关不改写管理员配置、保存失败保留原行为。fmt、全工作区 Clippy/测试、前端构建/类型/格式检查和 Linux desktop debug/release 构建通过。

实际隔离 Tauri 通过界面点击开关：默认开启；关闭后重启仍关闭，原生关闭窗口导致进程退出；开启后原生关闭窗口仅隐藏，进程继续存活。取得不同名称的真实本机服务端清单后，首页仍显示本地“蒹葭服务器整合包”。[原生验证数据](evidence/client-dedicated-native-verification.json)、[设置页面截图](evidence/client-dedicated-settings.png)。测试仅操作临时整合包，关闭行为在空闲阶段验证，未强制模拟原生安装/保存阶段；Windows 未实机验收。所有本轮隔离客户端及其调试接口已停止。


## 编译内置配置与后台颜色（2026-10-15）

专用配置源移至项目 config/client.json，通过 include_str! 编译嵌入；桌面和命令行不读取整包 config.json。核心显式配置入口用于独立装配和测试夹具，生产入口没有文件/环境配置覆盖。状态、后台偏好和任务记录仍在整包 .updater 中。删除设置页“文件处理”区块及旁置配置说明，侧栏状态点在后台开启时绿色、关闭时红色。

客户端测试通过，包含缺失/无效旁置配置不影响内置参数的回归，以及现有下载/安装/重试测试；全工作区 Clippy、fmt、前端类型/构建/格式及 Linux desktop debug/release 构建通过。隔离原生 Tauri 加载 index-BWht-El5.js / index-BNZIEUpV.css，在无效旁置配置存在时仍显示正确内置参数。实际开关对应绿色 rgb(97,213,162) 和红色 rgb(241,138,144)，用户偏好正常保存。

[原生验证数据](evidence/client-embedded-native-verification.json)、[开启后台](evidence/client-embedded-enabled.png)、[关闭后台](evidence/client-embedded-disabled.png)。当前项目配置保留示例 ID/域名，实际组包前必须填写并重新构建；本轮未验证真实服务地址可用性或 Windows，未执行下载/安装。临时客户端与调试接口已停止。早期依赖旁置配置的文档和截图属于历史版本，以本节及当前 CONFIG 契约为准。


## 三平台配置与独立构建（2026-10-15）

配置拆为 config/client.windows.json、client.linux.json、client.macos.json，由 Rust target_os 编译选择。新增 Node 标准库构建脚本和 GitHub Actions 三个平台原生任务，归档按系统/架构独立命名，不复制覆盖公共配置，不包含运行时可修改的组包配置。

客户端完整测试、fmt、全工作区 Clippy、构建脚本 3 个回归测试及脚本/工作流 Prettier 检查通过；工作流 YAML 经过解析，但未运行 actionlint 或远程 CI。Linux 实际执行完整 npm desktop:build，前端类型/生产构建和目标 release 构建通过。[产物验证数据](evidence/platform-build-linux.json) 确认归档中的 ELF、执行权限 0755、内嵌 Linux 配置及双 SHA-256 一致。

Windows/macOS 构建和运行未在本机验证，新增远程 CI 未推送/执行；当前 CI Mac 为 ARM64。Linux 产物不代表其他系统验收。三份配置依然包含待组包填写的示例服务地址和 pack_id。


## 统一构建与服务端部署包（2026-10-15）

npm run release:build 实际完成 Linux 客户端与服务端原生发布构建及打包；服务端包包含后端、后台静态资源、空 JWT 的环境变量模板、运行说明和双摘要，不带生产数据。CI 更名 Packy builds / .github/workflows/build.yml，新增独立 Linux 服务端任务，单次全量工作流对应三端客户端加一个服务端；远程 CI 尚未执行。

构建脚本 5 个 Node 测试、服务端回归测试、脚本/工作流格式检查通过。真实服务端归档解压后，在临时目录验证缺少 JWT 拒绝启动、首次管理员初始化、HTTP healthz、后台页面与静态文件、登录以及数据库创建/迁移全部通过。[实际包验证](evidence/server-package-verification.json)。临时服务已停止，未操作生产数据和原有服务。部署无需 Rust/Node/独立数据库服务，但需匹配系统运行库；后端不自动读取 env 文件，后台资源当前不在二进制内部。Windows/macOS 与桌面图形未在本轮重测。
