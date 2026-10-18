# 客户端下载设置

2026-10-18，已实现。默认配置在 config/client.<platform>.yaml 的 download 块内，通过 Go embed 随 Sidecar 构建。玩家在“配置与说明 → 下载设置”修改，保存到整合包根目录 .updater/preferences.json 的 download 字段。

## 字段

| 字段 | 默认 | 范围 |
| --- | --- | --- |
| max_concurrent_downloads | 4 | 整数1–16 |
| check_timeout_seconds | 30 | 整数5–300，检查更新总超时 |
| download_timeout_seconds | 600 | 整数30–3600，每次下载含限速等待的总超时 |
| max_attempts | 3 | 整数1–10，包含首次下载 |
| speed_limit_kib | 0 | 整数0–1048576 KiB/s，0不限速；1024 KiB/s=1 MiB/s |
| proxies.modrinth | 空 | Modrinth 渠道代理 |
| proxies.curseforge | 空 | CurseForge 渠道代理 |
| proxies.github | 空 | GitHub 渠道代理 |
| proxies.server | 空 | 服务端托管下载及检查更新代理 |

代理只区分渠道，不再分HTTP/HTTPS字段；同一地址用于该渠道的HTTP和HTTPS请求。地址支持无账号密码的 http:// 或 https:// 代理端点，禁止路径、查询、片段和用户名/密码。不支持SOCKS/PAC或认证代理。HTTPS下载可以使用HTTP代理的CONNECT隧道，保持目标证书验证，不增加忽略证书选项。

空值强制直连，不继承HTTP_PROXY/HTTPS_PROXY/NO_PROXY环境配置。代理失败不静默绕过代理直连。按发布清单source.type选择渠道，跳转CDN仍用原渠道代理，换源时更换代理。既有禁止HTTPS降级和移除认证头约束保留。此设置不改变后台Go服务访问Modrinth的网络行为。

## 继承、保存与任务

逐字段优先级为玩家明确覆盖 > 内嵌默认 > 程序兼容默认。旧YAML无download块、旧preferences只有background_enabled时继续可用。字段缺失表示继承，代理空字符串和限速0是有意义的覆盖。恢复单项/全部默认会删除相应覆盖，点击保存后生效；未修改字段跟随以后内嵌默认值的变化。取消编辑不改文件。

Go侧校验数值和代理后原子写入，失败不更新有效设置；后台运行开关与下载偏好互不覆盖。整合包身份/地址/路径仍不可修改。非法本地配置拒绝加载并报错，维护者可删除download覆盖或恢复备份，不能静默忽略用户设置。

每次检查或点击开始/重试时冻结一份设置。保存新设置不影响运行中任务；失败后可调整代理并重试原计划，原发布、URL和哈希不刷新。HTTP客户端按渠道复用，每次执行结束关闭闲置连接。

限速使用任务共用golang.org/x/time/rate令牌桶，所有并发文件合计；允许64KiB令牌突发和有限的读取缓冲，不是网卡级整形。等待支持取消并计入单次下载总超时；低限速与短超时组合可能导致重试，界面提示增加超时。并发单位是文件，不是分片连接。

## UI与协议

保留B森林主题和原有设置，下载表单独占一行；显示默认值、自定义状态、保存/取消/单项与全部恢复默认。窄窗单列，不整体缩放；输入校验同时由UI和Sidecar执行。

新增JSON-RPC命令set_download_settings，请求体为可选字段的DownloadOverrides，空对象恢复全部默认；返回Snapshot。Snapshot新增download（有效设置）和download_defaults，preferences.download为用户覆盖。Main和preload只放行该固定命令。

## 验证入口

- config/download_test.go：内嵌默认、空值覆盖、恢复继承、代理输入约束
- internal/client/application/network_test.go：本地HTTP代理、HTTPS CONNECT、跳转、直连环境隔离、尝试次数、限速共享/取消、执行期间设置冻结、实际并发上限、下载超时、偏好写入
- scripts/smoke.mjs：真实HTTP/Sidecar链路增加设置RPC和偏好落盘，再验证下载、替换、审计
- web/desktop/tests/gui-download-settings.cjs：Playwright浏览器fixture验证保存、刷新、恢复、非法输入、失败提示、明暗和390px溢出
- web/desktop/tests/download-settings-native.cjs：Linux Electron真实IPC/Sidecar保存、程序重启读取与恢复默认

本轮不包含Windows/macOS实机验收或公网代理性能验证。程序仍非自动回滚更新器；代理设置不改变名单文件保护与原任务恢复约束。
