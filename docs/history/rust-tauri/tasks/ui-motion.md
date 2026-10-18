# 客户端与管理后台过渡体验

日期：2026-10-15。范围：改善客户端和管理后台的页面切换、明暗主题、加载反馈；不修改 Rust 更新业务。用户明确客户端也必须包含。

## 实现

- 共享 CSS 动效变量、颜色渐变、淡入/淡出、轻微位移、按钮按压与旋转加载标记
- Vue Transition 用于客户端首页/配置页、初始化骨架与内容、进度/说明面板，以及管理后台页面和登录界面
- 客户端首次读取配置显示骨架，请求标记展示实际阶段；更新期间保留稳定的主操作反馈，进度宽度平滑变化；原生确认弹窗增加入场动画
- 共用 LoadingFeedback 以真实 busy 状态显示加载提示，保留行高度，不添加人为等待；管理后台五个页面与登录入口使用
- 客户端主内容顶部固定，避免不同高度页面随垂直居中跳动
- prefers-reduced-motion 关闭过渡、旋转与骨架动画；不引入动画库

## 验证

前端类型检查、生产构建和 Prettier 检查通过；Linux 桌面 debug custom-protocol 构建通过。最后一轮调整后的 release no-bundle 构建也已通过。

实际浏览器检查：管理后台登录中显示连接提示，进入 Mod 目录显示加载提示；页面切换读取到 view-leave-active 和 110ms 淡出。客户端首页/配置页切换、连续切换最终状态正确；主题切换读取到 280ms 颜色过渡及中间色。浏览器模拟 prefers-reduced-motion 后卡片 transition 为 0s，测试后恢复默认偏好；客户端控制台未见 warn/error。

浅色/深色客户端截图在 docs/evidence/ui-motion-client-*.png。仅作为浏览器视觉预览，浏览器中的更新按钮禁用，不伪造 IPC 或更新成功。原生进程仍运行之前的构建，新动画和首次读取骨架尚未经原生人工验收；之前托盘/通知人工通过不能替代本轮动效验收。

## 使用

新 debug 和 release 程序嵌入最新前端。现有窗口需从托盘退出后重新启动才加载新构建，单纯关闭窗口只会隐藏。未擅自停止用户当前测试窗口。预览临时 HTTP 服务位于 127.0.0.1:1422，本轮检查结束后已停止。

## 客户端布局顿挫回归修正

2026-10-15 用户反馈首次加载交接顿挫，检查更新时说明框下移再回弹。定位到骨架/整卡的 out-in 卸载交接，以及 checking 共用 active 导致临时挂载下载进度卡。

仅调整 apps/desktop/src/App.vue：骨架覆盖在始终存在的卡片内，通过透明度交接；初始化时卡片 inert，不能误点透明按钮。版本行、状态文案预留高度，整合包长名称单行省略并保留 title。下载进度卡要求存在 task_id，检查请求只使用原加载反馈行。实际文件任务继续显示进度。

隔离浏览器布局测试使用真实 App.vue，.tools/ui-layout 中临时 composable 提供受控状态，不触碰 Rust IPC、真实配置和文件；不得把它当成原生或下载联调。测得普通窗口检查前/中/后说明顶部均 584.59375px，760×560 窗口均 563.59375px；骨架前后卡片高均 267.59375px。模拟文件任务仍显示进度条。尺寸覆盖已恢复默认。

前端类型、构建、格式检查和 Linux desktop debug/release no-bundle 构建通过。结果见 docs/evidence/client-layout-regression.json。原生 WebKit 的实际帧率和用户手感尚未复测，不能把坐标一致等同于完整性能验收。

本轮布局测试页已关闭，127.0.0.1:1423 的临时 Vite 服务已停止；复现入口保留在忽略目录 .tools/ui-layout，可用临时 Node 22 运行 server.mjs，不进入生产构建。

## 客户端导航动画结束闪动

2026-10-15 用户明确导航切页结束仍闪动。此前只处理骨架交接，导航外层仍使用 Vue out-in 卸载/挂载。当前只修改 App.vue 的客户端导航：两个固定页常驻在同一 grid 单元，共享较高页面的占位；只切换 opacity/visibility，不在导航动画结束时移除节点或整页 transform。隐藏页通过 inert、aria-hidden 和 pointer-events 隔离；两页内部状态保留。较短页面会保留共同占位的空白，页脚和滚动条不会随导航收缩。

前端类型/生产构建/格式检查及 Linux desktop debug/release no-bundle 构建通过。独立受控浏览器页对前向、反向导航每帧采样：两页根节点始终 2 个，没有 childList 变化，内容高度、页脚位置与滚动区域宽度不变；两个页面的最大透明度始终大于 0，没有同时全透明的采样帧。隐藏页 inert=true/aria-hidden=true/visibility=hidden，活动页恢复交互。最小窗口连续导航和减少动态效果检查另记录到导航证据。

这些是浏览器和构建证据，原生 WebKit 的最终视觉结果仍需用户检查。不得将其表述为原生 FPS 或显卡问题已排除。数据见 docs/evidence/client-navigation-regression.json。

导航回归补充结果：普通窗口正反方向及最小窗口快速切换的逐帧断言全部通过，减少动态效果为 0s；浏览器尺寸/媒体偏好已恢复，测试页关闭，临时 Vite 服务停止。用户原生窗口未被停止。

## 原生 WebKit 暗帧复现与第一轮修正

用户要求必须原生验证，并显式授权本机 WebKit 调试与窗口录屏，仅使用隔离副本。原有用户进程 PID 562289 未被停止；测试原生窗口单独进程、单独游戏目录 `/tmp/packy-native-flash-apjr5u33/game`。没有安装新软件或修改系统配置。

真实窗口加载的 JS/CSS 与 dist 及二进制资产匹配，排除当次旧前端。实际 niri 合成窗口截图复现近空白帧：原代码第 24 帧内容差异约为稳定基线 5.3%。will-change:opacity 对照仍闪，未写入产品；取消 visibility:hidden 的结束切换后原生正反向对照通过。

产品仅修改 App.vue 的导航 CSS，保留 opacity 淡化，移除 visibility 与重复 transition 设置，继续 inert/aria-hidden/pointer-events 隔离。新 debug 二进制重新启动后通过原生调试确认资源 index-DrbC-B-H.js / index-CbvBXiaq.css，无临时注入，will-change=auto，两页 CSS visibility=visible、隐藏页 opacity=0/inert=true。两轮正反切页，共 400 张实际窗口截图，没有近空白帧。前端格式/类型/构建和桌面 debug/release no-bundle 均通过。

证据和慢放已保存 docs/evidence/client-native-*，详情见 docs/VERIFICATION.md。慢放按 30fps 保留截图序列，不是实际 FPS。浏览器先前的通过结果不能代替本次原生结论。

README 新增 RustRover Cargo 配置、Before launch 前端构建外部工具和临时 Node 22 命令。纯 Cargo 不执行 beforeBuildCommand，Tauri CLI dev 才启动 Vite；不需要每次重新编译 Tauri 框架依赖。未擅自改写 .idea 配置。

最终补充：原生隐藏页 inert=true，尝试聚焦其按钮被阻止；临时注入不存在。窗口视频均保留 100 帧。原生测试进程已停止，临时 WebKit 调试端口随进程关闭；用户原有窗口未停止。

## 原生旧页回显与文字重影修正

2026-10-15 用户提供截图，显示配置页上叠有首页旧标题和图标。进一步检查上轮真实窗口截图，发现旧标题区域在首次清空后又于第 28/29 帧出现；反向切换的旧配置内容于第 35/36 帧回显。上轮近空白帧检测并不能覆盖旧页回显，不能将“无暗帧”写成“所有闪动消失”。

当前产品仅调整 App.vue 导航 CSS：非当前页 animation/transition 均 none、opacity=0；只有新当前页执行 180ms、opacity 0.86→1 的轻度入场，避免同时显示两页，也避免新页从全透明起步。继续保留双页占位、inert/aria-hidden/pointer-events 隔离，不重新引入 visibility 切换。

重新构建并运行实际 Tauri debug 隔离窗口（PID 608587，niri 窗口 102），确认加载 index-C5LF-pwx.js、index-BlV5D5ha.css，无临时样式注入。两轮正反切页共 400 张 compositor 原生窗口截图；逐帧透明度样本每次均只有当前页可见，旧页 opacity=0/inert=true。旧标题专用区域和旧配置专用区域一旦清空，在剩余截图中均不再出现旧内容。无近空白帧。

前端格式/类型/生产构建及桌面 debug/release no-bundle 构建通过。原生录制与 .tools/native-ghost-check.py 检测没有使用浏览器预览，也没有修改 Rust 更新业务。证据 docs/evidence/client-native-ghost-*；视频保留所有截图、以 30fps 慢放，不能推算实际 FPS。原始截图在 /tmp/packy-native-flash-apjr5u33/native-no-ghost-*。Windows 尚未复测。

本轮原生测试窗口与其临时 WebKit 调试服务已停止，用户客户端未停止。


## Motion for Vue 接管客户端切页

2026-10-15 用户授权只用 motion-v 改造客户端切页，清理历史动画补丁，保留样式和业务逻辑，并要求实际 Tauri 验证。实施过程中用户明确取消切页晃动，因此最终版本取消整页水平位移，也不执行整页淡入淡出。

客户端精确依赖 motion-v 2.7.0，AnimatePresence mode="wait" 管理两个互斥 motion.div 的退场和挂载，initial=false、exit opacity=1/duration=0。删除双页常驻的 Grid 叠放、is-current、隐藏页透明度/inert 和 client-page-enter 关键帧。页面底色保持不透明。业务 composable 仍在 App 层，Rust 核心、IPC、按钮动作、共享主题和局部加载动画未修改。标准组件完成退出即移除，客户端没有残留的隐藏旧页。

最终版本前端生产构建、类型检查、格式检查，以及 Linux desktop debug/release custom-protocol 构建通过。最终嵌入资产 index-ClNO6d_y.js、index-BFJvmnSp.css，实际隔离 Tauri 窗口通过 WebKit 调试接口确认加载这些资产，未注入临时产品样式。

隔离目录 /tmp/packy-motion-native-hqliesp6/game；仅复制既有临时测试整合包。第一组无计划状态 6 个场景，第二组通过实际 check_update 取得清单后 4 个场景；第二组仅检查更新，没有下载、安装或启动游戏。单次正反切换共 6 轮，快速连续切页 4 轮，每轮 25 次点击、间隔 30ms，覆盖最终选择两个不同页面。10 轮共 1800 张 niri 合成窗口截图、1118 个原生 RAF 样本。每个样本仅一个页面、opacity=1、transform=none，同页布局起点不变，连续点击最终状态正确。

主内容截图按每 6 像素网格与切换前后稳定画面对比，全部采样像素匹配其中一个稳定画面，没有混合残影、空白中间帧；普通切页没有目标出现后旧页再次回显。导航选中颜色过渡和页脚不参与该像素断言。已人工查看截图联系表，原始 PNG、时间记录和调试采样在临时目录保留，工具位于忽略目录 .tools/native-motion-*。证据 docs/evidence/client-motion-native-verification.json、client-motion-navigation.png/mp4、client-motion-rapid.mp4。视频保留每张截图，30fps 慢放，不代表实际帧率。采样不能覆盖每次显示刷新；Windows 尚未实机验证。

本轮隔离客户端和 WebKit 调试端口已随测试进程停止关闭；既有测试服务和用户窗口未停止。


## 检查更新时组件闪动（2026-10-15）

实际原生基线复现：checking 被共用 active 当成更新任务，主按钮切换 v-if 分支、启动器按钮移除后重新挂载，顶部说明和页脚也切换为后台任务；disabled 全局透明度 0.5 与恢复动画导致按钮闪暗。原生采样确认主/启动器按钮身份变化，主按钮最小 opacity=0.5、60 条 childList 变更。此次不是导航动画，也没有重新更换动画库。

只修改 apps/desktop/src/App.vue：展示层区分 checking 与实际 updating；主操作使用单一按钮，保留原检查、确认、重试操作分派，检查期间启动器保留但禁用，按钮不变暗。检查持续 150ms 后才显示状态与加载反馈，快速请求不显示短暂提示，不人为延迟请求或返回结果。加载图标与操作图标均 16px，防止相邻按钮出现 1px 位移。顶部说明和页脚仅对实际更新任务切换，清理未使用 spinning 样式。Rust、IPC、useUpdater、服务端和更新名单规则未修改。

最终资源 index-DOl_b1pV.js / index-CBrS1Wu0.css。类型/前端构建/格式检查和 Linux desktop debug/release custom-protocol 构建通过。隔离原生窗口 PID 689714，整合包 /tmp/packy-check-flash-shgnu_0s/game，来自既有测试副本。临时本机 HTTP 转发代理只用于给真实本机服务端响应增加 400ms 延迟；没有伪造 Tauri IPC、更新清单或界面状态，没有下载、安装或启动游戏。

最终原生验证为两次快速响应和一次 400ms 延迟响应，540 张 niri 窗口截图、411 个 RAF 样本。卡片/主按钮/启动器节点始终保留，按钮透明度始终 1，位置与尺寸固定。快速响应下无 childList 变化，无短暂加载提示；主内容每 8 像素网格采样与起始画面完全一致。慢响应下加载提示在 338ms 的采样中首次可见，操作在请求期间保持禁用；加载反馈插入/退出有 8 条 childList 变更，操作按钮不重建。

证据 docs/evidence/client-check-native-verification.json、client-check-before.mp4、client-check-after.mp4、client-check-slow.mp4。视频保留每张截图，30fps 慢放，不代表实际 FPS；窗口截图不覆盖每次显示刷新，Windows 未实机验证。原始截图、采样和可重跑工具在临时目录及忽略目录 .tools/native-check-* 保留。

本轮隔离客户端、WebKit 调试接口及临时转发代理均已停止；原有本机服务端和用户窗口未停止。
