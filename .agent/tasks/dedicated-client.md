# 蒹葭服务器专用客户端配置

2026-10-15。用户要求整合包名称固定为蒹葭服务器整合包并保留组包配置；游戏/启动器使用相对路径；后台运行提供开关。用户已确认服务端地址在配置中填写完整 HTTP/HTTPS URL。用户后续要求移除手动刷新本地状态入口，状态和进度保留自动刷新。

实施：config.json 增加 pack_name，旧配置默认蒹葭服务器整合包；新增 preferences.json 保存 background_enabled，默认开启。客户端开关立即保存，关闭窗口按偏好隐藏或走原安全退出流程，安装/保存阶段仍禁止退出。配置文档、示例、生成类型和实际原生验证均已完成。


## 已完成与验证

- 组包配置增加 pack_name，示例 examples/updater-config.json 使用蒹葭服务器整合包；server_url 保持完整 URL，game_dir 和 launcher_path 继续安全相对路径。旧配置未填写名称时使用默认名称，无须改写旧文件
- 用户偏好独立 .updater/preferences.json，默认开启后台，写入成功后更新内存，保存失败保留原值；不改写管理员配置。开关在界面保存，关闭窗口共用现有 prepare_exit 安全规则
- 首页与设置页读取配置名称，不使用清单中的 pack_name；窗口标题/托盘说明使用专用名称。背景提示随开关变化。按后续要求移除“刷新本地状态”按钮及附带说明，保留初始化快照、操作后刷新及状态事件订阅
- AGENTS.md、PRD-v0.2、配置/API 契约和 README 同步专用客户端及开关语义，未改写原始 PRD-v0.1

cargo fmt --all --check、cargo clippy --locked --workspace --all-targets -- -D warnings、cargo test --locked、npm run build、npm run format:check 通过；Linux desktop debug/release custom-protocol 构建通过。

新增核心边界测试证明旧配置名称默认值、偏好持久化/重启/移动整合包保留、管理员配置字节不变，以及模拟写入失败时不改变内存偏好；现有真实 HTTP 安装/重试、并发/取消、哈希和路径测试通过。

实际隔离 Tauri 目录 /tmp/packy-dedicated-native-mp050r63/game；WebKit 调试确认资产 index-BlXjM2Fd.js / index-D_0GAOX8.css。实际取得服务端“绿野旅途 · 联调整合包”清单后，首页名称仍为“蒹葭服务器整合包”。界面点击关闭后台后文件写入 false，重启后开关仍关闭；通过 niri 对指定 PID 窗口发送原生关闭请求，进程退出。另一轮开启后台后发送同样关闭请求，窗口消失而进程仍存活。未对真实玩家整包操作，未执行下载/安装；退出保护沿用核心规则，未在原生 GUI 强制构造安装/保存阶段。Windows 尚未实机验证。

证据 docs/evidence/client-dedicated-native-verification.json、client-dedicated-settings.png。本轮全部隔离客户端已停止，调试端口随进程关闭；已有服务和用户窗口未停止。

2026-10-15 后续小调整：删除手动刷新按钮及说明，useUpdater 自动订阅、初始化读取和终态刷新均未修改。客户端类型/生产构建、格式检查及 Linux debug custom-protocol 构建通过；本次纯入口删除未重复原生交互验收，之前截图仍为历史版本。


## 编译期配置与侧栏状态（2026-10-15，已完成）

用户要求删除整个“文件处理”区块；组包参数移入项目并构建嵌入，玩家旁置配置不能修改；侧栏状态点按后台开关绿/红切换。已删除区块，增加关闭时红色样式。config/client.json 为唯一分发构建配置，Client::open 使用 include_str!；核心 open_with_config 仅供明确装配和隔离夹具使用，桌面与命令行不调用文件配置。旧 examples/updater-config.json 移至 config/client.json，文档已更新。运行时 state/preferences/task 继续保存在整包中。新增无旁置配置和无效旁置配置均不影响内置参数的测试。客户端完整测试、全工作区 Clippy、fmt、前端构建/类型/格式，以及桌面 debug/release custom-protocol 构建通过。


实际隔离 Tauri /tmp/packy-embedded-native-7122amdz/game 放置了无效的玩家旁置 config.json，仍成功读取内置名称、1111 示例 pack_id、HTTPS example.invalid、.minecraft 和 HMCL.exe。这些值仍为待组包填写的占位参数，不能据此宣称实际更新可用。原生确认“文件处理”区块和旁置配置路径说明均消失；切换后台开关，状态点从 rgb(97,213,162) 变为 rgb(241,138,144)，并显示关闭窗口时安全退出。测试过程中出现过示例服务地址不可连接的提示，重载隔离界面清除局部错误后完成控件验收；未修改地址冒充联调成功，没有下载或安装。证据 client-embedded-native-verification.json、client-embedded-enabled.png、client-embedded-disabled.png。本轮隔离进程与调试端口已停止，用户窗口和已有服务未停止；Windows 未实机验证。

联调脚本只生成临时 client-build-config.json 作为构建参数参考，不再生成玩家端 config.json；README 与部署文档已说明先将参数填入项目配置再构建。脚本仅作 Python AST 语法检查，本轮未重新启动其 HTTP 夹具。
