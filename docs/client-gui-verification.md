# 客户端 B GUI 实现与验证

2026-10-18，Linux x86_64。用户批准 B 游戏氛围及流程、尺寸方案后实现；未提交或推送。此记录仅覆盖本次 GUI，不替代 Go 服务端与更新核心验收。

## 完成内容

- 顶部导航、明暗森林包头、更新摘要、变更清单、确认框、检查/下载/校验/安装/完成/失败状态、配置说明页
- Vue 接收真实 Sidecar 协议；安装/保存不能取消，失败重试保持原任务，启动器由 can_launch 决定
- 明确区分语义版本与本地发布 ID；只展示协议实际提供的信息
- 正文/导航 14px、按钮最小高 36/40px、独立字号层级；窄窗重排、长路径换行，不整体缩放
- 文字区实色背景，维护者替换高反差背景也不穿透标题；尊重减少动态效果
- 本次新增 image-size 2.0.2 用于栅格头尺寸校验；不增加原生 Node 扩展

## 构建时默认资源

默认 logo 和明暗背景在 `web/desktop/public/branding/` 中维护，经 Vite 构建放入 Electron app.asar；默认原生图标 `web/desktop/assets/icon.png` 随包复制到 resources/assets，Windows EXE 图标来自 icon.ico。修改后执行 `npm run desktop:build`，分发完整便携目录即可，无需玩家补放默认图片。配置、固定参数和路径完整清单见 [CONFIG.md](contracts/CONFIG.md)。

## 品牌资源替换

在**整合包根目录**创建 `.updater/branding/`，按需放入以下同名文件，重启客户端生效：

| 文件 | 用途 |
| --- | --- |
| logo.png | 导航品牌图与运行时窗口图标；推荐透明正方形 512×512 |
| background-light.webp | 浅色包头背景，推荐至少 1600×600 |
| background-dark.webp | 深色包头背景，推荐至少 1600×600 |
| tray.png、tray@2x.png | Windows/Linux 彩色托盘图 |
| trayTemplate.png、trayTemplate@2x.png | macOS 模板托盘图 |

文件均可单独替换，缺失/读取失败/解码失败时使用内置资源。背景按 cover 裁切，不拉伸，不在素材中加入文字。单文件最多 8 MiB，单边不超过 8192，像素总数不超过 3200 万；只接受对应 PNG/WebP 格式，不读取 SVG 或远程图片。拒绝目录、普通符号链接和非白名单名称。

Main 使用 Node path 处理系统路径，受限 IPC 仅返回固定图片 data URL，Renderer 没有任意路径读取能力。浏览器解码成功才启用覆盖资源；不使用 nativeImage 解码 WebP，以免平台支持差异导致有效背景被拒绝。托盘/窗口 PNG 使用 Electron nativeImage。

这套替换不更改服务地址、整合包 ID、游戏目录等内嵌配置。操作系统文件管理器中的 EXE/APP 图标属于打包资源，替换它仍需重新打包；运行时 logo 不会修改可执行文件元数据。

默认背景是独立生成的体素森林素材，未裁切原型 UI。来源与原始提示词见 `web/desktop/public/branding/provenance.json` 及图片旁的 JSON。默认 logo 沿用项目现有 P 标志。

## 已执行

- `npm --prefix web/desktop run build`：Vue/TypeScript 检查、Vite、Electron TS 编译通过
- `node --test web/desktop/tests/brand-files.test.cjs`：2 项通过，覆盖格式/尺寸/大小/固定名称/文件类型/Unix 符号链接
- Playwright CLI 执行 `web/desktop/tests/gui-browser.cjs` 和 `gui-edge-cases.cjs`：通过。浏览器注入协议 fixture，不在生产界面加入 Mock。覆盖确认、下载取消、原任务重试、安装态禁止取消、启动器可用性、未知大小、键盘/Escape、减少动态效果
- 浏览器宽度 390/720/1060/1440：无横向溢出，正文与导航均保持14px。720×560 长标题/长路径/长说明和明暗高反差背景已实际截图并查看
- Linux Electron + 真实 Go Sidecar：临时中文/空格根目录，logo/浅色背景覆盖、损坏深色背景回退通过；后台开启时关闭窗口隐藏到托盘
- Electron 主动缩放 100/125/150/200%：实际逻辑宽度936/749/624/468，均无横向溢出；200%使用 BrowserWindow.capturePage 截图查看
- `npm run desktop:build`：Linux x64 便携目录打包成功，位于 `artifacts/desktop/Packy-linux-x64`
- 独立视觉审查提出两项问题（标题可读性、ID标注），修复后两项均获 resolved；不是额外跨平台验收

实际渲染截图在 `.impeccable/review/client-b/`，包括 home-light/home-dark、confirm、各更新状态、settings、窄屏及原生截图。全页截图里的 sticky 操作栏保持捕获时视口位置，不表示内容无法滚动。设计规范在 `DESIGN.md` 与 `.impeccable/design.json`；作用域仅桌面客户端，后台模板不变。

## 复现入口

浏览器先启动 `npm --prefix web/desktop run dev -- --port 5174`，再用 Playwright CLI 打开页面，以 `run-code --filename` 依次运行两个浏览器脚本；它们共享 fixture 状态，顺序不可倒置。

Linux 原生脚本为 `web/desktop/tests/gui-native.cjs`。先构建桌面与 `web/desktop/sidecar/packy-client`，再设置 `PACKY_PLAYWRIGHT_MODULE` 为本机 Playwright Core 的模块绝对路径后用 Node 执行。该工具仅为验证依赖，不加入生产包；脚本自动创建/清理临时根目录，需要有效 GUI 会话。截图输出位于 `.impeccable/review/client-b/`。

格式检查 `npm run format:check` 已通过。

## 未执行与边界

- 未在 Windows/macOS 实机验证标题栏、原生托盘图、通知、文件锁、系统 DPI或签名
- Electron zoom 是用户缩放测试，不能替代系统高 DPI 实机测试
- 本轮未连接真实服务执行更新，也未启动真实游戏；浏览器阶段来自 fixture，原生测试只验证上述外壳行为
- 本轮未改 Go 业务核心，未重跑服务端完整测试；原有证据见 VERIFICATION-go-electron.md

## 客户端动效补充（2026-10-18）

页面与详情标签切换使用 180ms 可中断动画；导航指示线渐变，按钮按压使用 120ms 的 1px 位移与颜色反馈。原生确认框及遮罩使用 140–180ms 进入/退出过渡，不延迟业务方法或改写焦点机制。减少动态效果偏好下禁用位移，切换偏好时取消正在运行的导航动画。

桌面构建、类型与格式检查通过。Playwright 协议 fixture 回归通过，另验证连续 8 次导航、标签切换、Escape/取消关闭及再次打开弹窗；减少动态效果时导航动画数为 0。查看了桌面过渡帧、确认框和390px窄屏截图。本轮没有重新执行原生平台或真实更新验收。

## 错误提示与日志（2026-10-18）

导航按钮按压不再位移，选中下划线继续过渡；品牌标志仅展示，不接受导航点击。界面保留可理解的错误摘要与恢复按钮，不展示 Electron IPC 前缀或裸错误码。404 提示核对整合包/发布配置；技术信息写入控制台和日志。

Electron Main 使用 electron-log，日志固定为 `app.getPath('appData')/Packy/logs/client.log`：Linux 通常是 `~/.config/Packy/logs/client.log`（遵循 XDG_CONFIG_HOME），Windows 是 `%APPDATA%/Packy/logs/client.log`，macOS 是 `~/Library/Application Support/Packy/logs/client.log`。达到 5 MiB 后由库轮转到 client.old.log。反馈时可附上这两个文件。

记录主进程未捕获异常、命令失败、任务错误、Sidecar stderr、渲染器警告/错误及异常退出；不记录命令参数或完整状态快照。URL 用户信息、查询和片段会移除，Bearer 凭据隐藏；日志可能包含本地路径，反馈前可自行检查。文件目录由库自动创建，写入失败时仍保留控制台诊断。

本轮构建及浏览器流程回归通过；实际检查桌面/窄屏错误提示、导航按压无位移、标志点击不切页。Linux Electron 临时环境实测日志写入与 URL/Bearer 脱敏通过；Windows/macOS 路径依据 Electron API，未实机验证。
