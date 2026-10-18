# Packy Electron / TypeScript / Node.js 重建方案

日期：2026-10-17。状态：已被后续用户决定替代，仅保留业务清单和分析历史。

当前目标见 [Go + Electron 重建方案](go-electron-redevelopment.md)：Go 服务端、Go 客户端 Sidecar、共享 Go protocol、Electron + Vue，以及 Arco Design Pro Vue 完整模板。本文 Node server / Node 更新核心、utilityProcess 和 Node 原生 addon 依赖选型均不再执行。

本轮用户决定：功能需求暂时不变，技术路线改为 Electron + TypeScript + Node.js。本文件覆盖技术替换设计、逐功能依赖、平台差异、迁移和验收。它不表示候选依赖已经安装或跨平台测试已经通过，不授权删除旧实现、修改真实数据、提交或发布。

## 1. 需求基线与当前证据

基线是 [PRD v0.2](../PRD-v0.2.md)、[API](../contracts/API.md)、[配置](../contracts/CONFIG.md)、[数据库](../contracts/DATABASE.md)，并以当前 `crates/common/src/models` 为字段事实来源；FIELDS.md 含历史 DTO，不直接作为新代码生成输入。

当前仓库为 Rust server/client + Tauri 2 + Vue 3，后台为 Arco Design Vue。新技术路线替代 Rust/Tauri 的实现方式，保留业务规则与外部协议。旧文档中的 Rust、Tauri、include_str!、WebView2 验收条件将在实施切换时逐项替换；本轮不把旧实现文档改写为已经完成的 Electron 状态。

本轮检查：项目根目录和工作目录均为 `/home/helan/own/Packy`；Linux x86_64，执行 Shell 为 Bash，登录 Shell 为 fish；虚拟化探测返回 none，未发现 Docker 标志。当前终端 Node v18.20.8、npm 10.8.2，而项目 `.node-version` 为 22.23.3；没有切换全局运行时。初始 Git 工作区干净。本轮只读源码和官方资料，新增设计与任务记录。

必须保持的业务不变量：

- 稳定内部 Mod ID 与业务 modid 分离；名称可重复，modid 唯一；历史快照不可变。
- 服务端计算完整快照间的 ADD / REPLACE / REMOVE；客户端不推导历史差异。
- 只处理本次计划的目标和旧路径；不扫描、识别、整理玩家额外 Mod。
- 所有目标文件下载并校验成功后才修改游戏文件；安装或版本保存失败不报告成功、不启动游戏。
- 重试与重启恢复使用原任务，不查询新计划；不新增服务端调度、自动回滚、完整事务日志或分段下载。
- 四种来源编码保持 `modrinth / curseforge / github / server`；server 的展示文案去掉 Rust 技术名称。
- JWT、数据库实时 RBAC、注册事务、两种角色、最后一个超管保护保持不变。
- 平台配置构建时固定，玩家不能通过旁置配置切换服务器；后台偏好单独保存。
- 随整包便携分发、不做安装器、不做自更新；HTTP/HTTPS 客户端，HTTP server + Nginx TLS。
- 不引入不同 release ID 但空 operations 的版本推进流程；遇到这种响应明确为不支持的契约情况，不悄悄修改 state。

## 2. 建议架构与可选方案

建议继续 npm workspaces、Vue 3、Arco 和现有主题，重写后端及桌面桥接。Electron 不要求换成 React；UI 框架重写不解决路径或原生 API 问题。

```text
桌面 Vue Renderer（无 Node 权限）
  → 窄接口 Preload（contextBridge）
    → Electron Main（窗口、IPC、托盘、通知、生命周期、启动器）
      → utilityProcess（独立 Node 更新核心、整包锁、下载、安装、任务记录）
        → packages/contracts + packages/update-core

Web Admin（Vue + Arco）→ HTTP API
  → Node Server：api → application → domain / infrastructure
    → SQLite、托管文件、Modrinth API

contracts：运行时 schema、DTO、错误码、纯协议校验
update-core：不导入 electron，不依赖 server 或浏览器
```

| 选择 | 实现方式 | 优点 | 代价与结论 |
|---|---|---|---|
| Main 直接运行核心 | Main 调异步 fs/network | 进程少，调试简单 | 哈希、异常与文件编排共用桌面主事件循环；可做最小探针，不建议正式方案 |
| Worker thread | Main 创建 Node worker | 隔离计算、传输成本较低 | 与 Main 同进程，原生崩溃隔离较弱；可选 |
| utilityProcess | Main fork 固定核心入口，消息协议交互 | 桌面事件与安装隔离，进程异常可识别 | 多一次消息边界和生命周期协调；建议采用 |

Electron 官方提供 Node utility process 与消息通信；本项目将其作为受信任的核心执行进程，不将其视为可运行不可信脚本的安全沙箱。[官方 API](https://www.electronjs.org/docs/latest/api/utility-process)

建议目录如下，只在对应功能实施时创建：

```text
apps/desktop/{src,main,preload,worker}
apps/server/src/{api,application,domain,infrastructure}
packages/contracts/src
packages/update-core/src/{application,domain,infrastructure}
web/admin/src
web/shared
migrations
config
scripts
```

核心通过具体模块组合，不建立单实现仓储接口或依赖注入容器。四源的下载协议相同，采用受限类型联合和统一下载器；只有真正不同的元数据行为独立适配，不机械重建四个内容相同的类。

### 2.1 运行时与版本基线

建议新 server/build 工具链使用 Node 24 LTS；当前 Node 22 约束仅在批准并开始新工程时修改。桌面使用 Electron 自带 Node，不能通过系统 Node 升级桌面运行时。截至本次检索，官方稳定发布页列出 Electron 44.9.0 / Node 24.22.0，可作为 P0 候选，不能据此跳过安装和平台验证。[Electron 版本表](https://releases.electronjs.org/)；[Node 生命周期](https://nodejs.org/en/about/previous-releases)

依赖版本全部精确锁定；实施时确认 npm 实际可获取版本、engines、peerDependencies、平台二进制与许可证，再提交 lockfile。候选版本不是永久兼容承诺。构建记录 Node/Electron/Chromium、OS/arch、依赖树、内嵌配置摘要和产物哈希。

### 2.2 类型与模块系统

- Node 侧 TypeScript 编译后运行，不依赖生产环境直接执行 `.ts`；Node 服务端采用 ESM / NodeNext。
- Main、worker 使用明确构建入口；沙箱 preload 生成一个自包含 `.cjs`，不在 preload 动态加载工作区包。
- 共享包禁止导入 Electron、fs、数据库；Renderer 的依赖树不得包含任何原生包或服务端凭据。
- HTTP DTO 继续 snake_case；严格运行时解析，未知字段拒绝。联合类型必须验证 action 与 old_path/target 的组合。
- Rust int64/u64 不能直接等同 JS number。现有 JSON 数字保持兼容，但全部校验 `Number.isSafeInteger`、非负与累计溢出；超范围明确拒绝，不静默舍入。若真实数据需要更大范围，单独升级协议为十进制字符串，不在重写中暗改字段。
- `null`、字段缺失、空数组维持原语义；日期 RFC3339 UTC、UUID、128 位小写 SHA-512 均做边界校验。

## 3. 每项功能设计与依赖

表中“依赖”同时包含 npm/API 和前置业务能力；完整候选分析见第 8 节。

### 3.1 服务端和后台

| ID / 功能 | 输入、处理与输出设计 | 依赖与验证重点 |
|---|---|---|
| S01 配置与启动 | YAML + 逐字段环境覆盖；显式空值不回退；默认文件缺失与显式文件缺失分开；相对路径以 server cwd 为准 | yaml、Zod、node:path；未知字段/错误配置拒绝启动且不打印密钥 |
| S02 数据库迁移 | 复用经核对的 SQL 结构、外键、不可变触发器；schema_migrations 幂等；迁移整个事务提交 | SQLite 驱动、migrations；临时旧库验证，不自动碰真实库 |
| S03 初始化超管 | CLI 从标准输入接收密码，生成 bcrypt cost 12；绑定参数写账户和角色；已有超管不覆盖 | bcryptjs、crypto.randomUUID、数据库事务；保留 SQL 初始化语义和无默认密码 |
| S04 注册 | 请求仅账号密码；密码哈希先做，随后 BEGIN IMMEDIATE 内读取当前开关、查重、建 user 账号 | Zod、bcrypt worker、SQLite；与关闭注册并发时按写事务顺序判定 |
| S05 登录/退出/身份 | HS256、iss=packy、aud=packy-admin、TTL 1800 秒；校验时间和 token_version；logout 增版本 | jose、bcryptjs；密码 8–72 UTF-8 字节；统一登录失败，拒绝旧 Token |
| S06 实时授权 | 每次请求读取账号启用、角色、权限；系统管理还要求 super_admin | Fastify hook + 具体查询；不缓存 JWT 中角色来代替数据库 |
| S07 用户与权限管理 | 仅两角色；启停账号、分配角色、改授权；最后启用超管及其管理权限受事务保护 | S06、SQLite；所有管理 API 的拒绝路径回归 |
| S08 注册开关 | DB 单行设置；提交后新注册请求立即读取新值；公开接口只暴露开关 | S04、S06；前端隐藏入口不能代替后端授权 |
| S09 来源字典 | 固定四编码，名称/启用/排序可改；禁用不删除文件，也不改已发计划 | schema、SQLite；新计划过滤当前字典，原任务来源保持原样 |
| S10 整合包概览 | pack CRUD、分页、当前版/数量/时间和历史 | Fastify、SQLite、Vue/Arco；同包当前发布指针校验 |
| S11 Mod 管理 | 内部 UUID 稳定；修改业务 modid 查重，名称允许重复；平台项目 ID 独立 | contracts、SQL 唯一约束；修改后历史快照不变 |
| S12 Modrinth 检索 | 原有五组 API、参数编码、可识别 User-Agent、超时、限流映射；返回上游 JSON | Node fetch、AbortController；仅服务端请求元数据，不实现全平台 SDK |
| S13 Artifact/手动来源 | 保存版本、目标文件、真实 SHA-512、可空大小与排序链接；所有镜像必须是同一文件 | URL/schema；GitHub/CurseForge 直链不自动扩为搜索/同步平台 |
| S14 托管上传 | 管理员 multipart 单 file，流式限制 512 MiB，临时文件计算 SHA-512 后转不可变 ID 路径，再入库 | @fastify/multipart、fs/stream/crypto；异常清理仅此次临时文件；入库失败可能遗留孤立文件，记录待维护，不自动扫删 |
| S15 托管下载 | file_id 查元数据后打开固定文件；200/206、Content-Length、单段 Range/416、HEAD；不接受磁盘路径 | Fastify、fs.createReadStream；无 JWT 下载、不可变内容、禁目录列表 |
| S16 创建/编辑草稿 | 从指定或当前正式版复制；无正式版则空草稿；全量条目编辑 + revision 乐观并发 | SQLite 事务；跨包基础版拒绝、过期 revision 返回 409 |
| S17 预览/发布 | 校验目标路径集合、来源、哈希、版本号；事务内重查 revision，冻结快照并切当前版 | domain 差异、路径规则、SQL 触发器；发布基础版本记录实际前驱 |
| S18 发布历史 | 展示不可变快照及与前驱的差异；无正式版改删 API | S17、只读查询；不引用当前 Mod 名称覆盖快照 |
| S19 更新计划 | 请求 installed_release_id；验证属于该 pack；稳定内部 ID 比较旧/新快照，路径或哈希改变即 REPLACE | 纯 domain + S09；未知版拒绝、无发布明确错误；不增加客户端扫描 |
| S20 静态资源/部署 | server 提供后台静态文件；API 404 不落入 SPA fallback；只监听 HTTP，Nginx TLS | @fastify/static、Fastify；反代信任仅配置的代理，public_url 支持子路径 |
| S21 观测与限制 | 保持 APIError/code/request_id；结构日志脱敏；JSON 1 MiB，登录/注册有界限流；healthz 只表示进程存活 | Fastify/Pino、@fastify/rate-limit；避免同步密码计算阻塞探针 |

Modrinth 使用官方实际 URL 与 SHA-512。CurseForge 获取下载 URL 可能需要 API Key，且文件 hash 类型不等同 SHA-512；本版维持人工提供有效直链和实际文件 SHA-512，密钥不发客户端。[Modrinth](https://docs.modrinth.com/api/)；[CurseForge](https://docs.curseforge.com/rest-api/)

### 3.2 桌面与更新核心

| ID / 功能 | 设计 | 依赖与验证重点 |
|---|---|---|
| C01 内嵌配置 | 按目标平台读取现有 YAML，构建时验证后生成 TS 常量，打入 Main/core；生产不接受 env/旁置文件覆盖 | yaml 仅构建期、Zod；平台/配置摘要写 build-info |
| C02 定位与版本 | Main 从受支持分发布局确定整包根；core 校验 game_dir/state；缺失或损坏版本拒绝猜测 | node:path/fs、平台定位适配；不使用 cwd、app.asar 路径作为整包根 |
| C03 单实例与单写者 | Electron 单实例用于窗口恢复，核心持有整包文件锁到全部 I/O 结束 | Electron app + 原生锁候选；重复点击、不同整包、崩溃/重启均测试 |
| C04 获取状态/检查 | get_status 为权威；检查返回完整计划及摘要，锁定当前展示计划；有未完成任务先进入恢复 | contracts、HTTP 客户端；过期请求不能覆盖较新状态 |
| C05 确认和启动任务 | start_update 不接收 UI 的文件或 URL；使用核心已保存的计划，生成 task_id，持久化后下载 | 状态机、原子 JSON；重复调用幂等拒绝或返回已有 task_id |
| C06 四源下载 | 共用 HTTP/HTTPS 流式下载器；最多 4 文件并发，单文件不分段，按原顺序切源 | fetch、stream.pipeline、AbortController；每文件总尝试最多 3 次，包含所有来源尝试 |
| C07 下载校验 | 下载写任务 staging；流式计数/hash，关闭落盘后最终再校验；所有新文件合格后才进入安装 | crypto.createHash('sha512')、fs；未知大小不伪造百分比 |
| C08 进度 | task_id + 实例标识 + sequence；最多 10Hz、终态立即；速度使用短时窗口，重试不重复累计完成量 | 消息桥、单调时钟；UI 重载从 snapshot 恢复，旧实例事件丢弃 |
| C09 取消 | checking/downloading/verifying 可取消；Abort 并等待全部流和句柄结束；保留原任务供重试 | AbortController + Promise.allSettled；不能取消后立刻放开写锁 |
| C10 安装 | 全名单安全预检；先处理不属于最终目标集合的显式旧路径，再以同目录临时文件替换最终目标 | fs/path/native safety；交叉换名按最终目标集合处理，不能晚删刚写的新文件 |
| C11 保存完成 | 目标全部安装后保存 state，之后标记任务 completed；两文件间崩溃按明确规则恢复 | 原子 JSON、任务状态机；持久化失败不提前显示成功 |
| C12 重试恢复 | task_id/原计划不变；重验 staging，新文件可重新下载；重放操作幂等；不重新请求最新计划 | 本地 task schema；安装开始后失败不可离线启动 |
| C13 离线启动 | 仅检查网络失败、版本有效且无未完成安装时允许；明确提示无法确认最新版本 | 状态守卫；未完成/损坏任务拒绝启动 |
| C14 启动器 | 核心确认可启动，Main 只调用嵌入的已验证启动描述；exe/JAR/sh/app 分平台处理 | child_process、Electron shell（仅 app）；细节见第 6 节 |
| C15 后台偏好 | preferences.json 默认 true；原子保存成功后才变更内存/窗口行为 | fs；失败保持旧偏好，不能覆盖管理员配置 |
| C16 托盘/关闭/退出 | 默认关闭隐藏；关闭后台则安全退出；installing/saving_state 禁止用户退出；下载退出先取消并持久化 | Tray/Menu/BrowserWindow/app；worker 异常时禁止启动、显示恢复入口 |
| C17 完成通知 | 仅在版本与完成状态提交后发送；失败保留窗口结果，不撤销安装 | Notification；Windows 便携条件、Linux 通知服务、macOS 权限分别验收 |
| C18 主题与交互 | 保留专用整包名称、明暗主题、键盘焦点、明确错误重试和有意义过渡 | Vue、现有 CSS/motion-v；减少动态效果偏好，长文本/未知大小/原生缩放验证 |

### 3.3 界面功能对应

本轮采用 Impeccable 的操作型界面原则：玩家完成检查、更新、启动；管理员完成录入、发布和授权。现有 desktop/admin 均未发现 PRODUCT.md/DESIGN.md，不据此推定需要重新设计视觉。本轮不生成新的视觉规范或页面。

桌面首页保留本地版/目标版/更新说明和一个主要动作；确认态展示三类操作；下载态显示速度、文件计数及可取消操作；安装/保存态给出禁止关闭原因；失败态区分网络、哈希、占用、权限、状态损坏，按钮按核心能力显示；成功态显示启动器入口。前端禁用按钮不能代替核心状态守卫。

后台逐页保留登录/注册、概览、Mod 搜索与手工录入、Artifact 来源维护、上传、草稿编辑/预览/发布、历史详情、来源字典、注册开关、用户、角色权限。Token 只在管理 Web 边界使用，优先内存保存，刷新后重新登录；若既有会话体验要求持久化，单独确认存储与 XSS 风险，不在 Electron 玩家端存后台 Token。版本冲突提示刷新并保留用户可审查的未提交内容，不自动覆盖服务器草稿。

本轮无 UI 实现，不声称浏览器或原生视觉验收。后续小范围桥接沿用当前视觉；若决定重大改版，按项目规则先高保真原型并由用户确认，再实现，使用 Playwright CLI 检查桌面/移动端后台截图及交互，另做真实 Electron 窗口验收。

## 4. 更新状态机与文件提交语义

### 4.1 状态和权限

```text
idle → checking → ready / available
available → downloading → verifying → installing → saving_state → completed
checking / downloading / verifying → failed / cancelled
installing / saving_state → failed（保留原任务，只能恢复，不可启动游戏）
failed / cancelled → 原任务 downloading / verifying / 后续阶段
```

核心串行处理控制命令；网络下载内部最多 4 并发。进入 installing 前必须同时完成取消检查、全文件校验、全路径检查以及 installing 检查点落盘。先建立不可取消守卫再持久化阶段；落盘失败恢复失败态且不修改游戏文件。UI 按钮、托盘、窗口关闭、应用退出和启动器共用同一个守卫。

记录建议升级为 schema_version=2，增加原计划摘要、执行实例标识、安装是否开始等恢复元数据；不是完整逐文件事务日志。原计划正文保持不可变，仅阶段字段变化。显式维护 old_release_id、target_release_id 的核对关系，不凭当前版本号猜任务身份。

### 4.2 安装顺序

1. 任务目录由 task UUID 和序号命名，不从远端文件名拼 staging 路径。所有目标 JAR 在 staging 可读且最终校验通过。
2. 检查所有 old_path、target.path 及祖先、安全根、目标集合唯一性、文件类型；不遍历 mods 的其他条目。
3. 构造最终目标集合 T 和显式旧路径集合 O。删除 O−T 中存在的普通文件；不存在视为该删除已完成。O∩T 不先删除，由目标替换覆盖，避免同路径替换先丢旧文件。
4. 每个目标用 `open('wx')` 创建同目录随机临时文件；从 staging 流式复制，检查复制内容/大小/hash，sync、close，再 rename 到目标。临时名进入本任务跟踪；失败只清理自己创建的临时文件。
5. Windows 占用导致 EACCES/EPERM/EBUSY 时做有限短延时重试，再提示关闭启动器/Java 后重试原任务。不得先 unlink 目标绕过 rename 失败。
6. 所有操作完成才保存 state；再保存任务 completed；之后通知并允许启动。

这是单文件替换与可重复安装，不是跨文件原子事务。中途失败可能已有部分目标更新，旧版本记录不推进；使用原任务恢复。不同卷仅影响 staging→目标临时文件的复制，不对跨卷路径直接 rename。不自动回滚、不扫描旧 Mod；是否有足够空间的预检只是提示，实际 ENOSPC 仍必须处理。

Node fs 提供 realpath、lstat、文件句柄 sync 和 rename，但不同文件系统/权限/占用行为需要实测，不能把一个 rename 宣称为整次安装原子。[Node 文件 API](https://nodejs.org/download/release/latest-jod/docs/api/fs.html)

### 4.3 状态保存与恢复表

JSON 写入使用同目录独占临时文件、完整写入、文件 sync、close、rename；POSIX 在适用时 sync 父目录。Windows 不能假设同样的目录 fsync 能力；状态替换与断电持久性须区分，最低保证可检测损坏并停止，不能承诺所有磁盘断电无损。

| 重启时证据 | 行为 |
|---|---|
| 无 task、state 有效 | 正常检查 |
| 下载/校验未完成、state=旧版本 | 恢复原任务；可重下，不获取新计划 |
| installing/saving_state、state=旧版本 | 禁止启动；按原计划重做安全幂等操作 |
| state=目标版本、任务仍 saving_state/installing | 视为 state 已提交而最终标记未完成的窗口；只验证记录关系并补写完成标记，不重放删除/安装、不扫描旧 Mod；完成前不启动 |
| state 与任务两端 ID 均不匹配、任务 schema 未知、JSON 损坏 | 明确恢复错误，保留证据；不清空任务、不请求最新版掩盖问题 |
| completed、state=目标版本 | 展示完成；暂存清理失败只记录，不撤销成功 |

清理不是完成条件；只能清理本任务已知文件及空任务目录，不递归清理任意 staging，不删除锁文件。外部手工篡改记录不在可信恢复保证内，仍做结构、包 ID、摘要和路径校验。

### 4.4 网络语义

- 保留完整源列表及顺序；每文件最多 3 次总尝试，网络/5xx/429 可短退避，Retry-After 有上限；哈希失败可切下一原来源，不解析新地址。
- 设连接、响应头、空闲读取与整体任务的合理有界超时；`fetch` 单一总超时不足以表达全部策略，P0 验证内置 Undici 行为，必要时只给下载器引入显式 undici，不全局替换网络栈。
- 使用手动重定向，建议最多 5 跳；每跳验证 HTTP/HTTPS、无用户凭据，禁止 HTTPS→HTTP，拒绝循环。跨域不转发 Authorization/Cookie；玩家下载本来不持有后台 Token。
- `200` 成功下载；不自行请求 Range。Content-Length 是提示，实际字节与计划大小另查；未知大小按文件数呈现。请求 identity 编码，不能用压缩传输长度假装文件大小。
- 首个终止失败取消其他流并等待结束；校验失败不触碰 mods。错误消息不回显带签名查询参数的完整 URL。
- HTTP 保留兼容，但 SHA-512 只能证明文件匹配所获计划，不能认证被明文篡改的计划；公网仍使用 HTTPS。此限制不以增加本版不要求的签名系统解决。
- Node fetch 与 Electron net 的系统代理/证书行为不能视为一致；建议核心统一 Node 网络栈，P0 测 Windows/macOS 系统代理与企业 CA。额外代理支持范围待确认，禁止关闭 TLS 校验作为修复。

## 5. 路径、文件系统和进程锁

### 5.1 三种路径严格分开

| 种类 | 格式和根 | 禁止事项 |
|---|---|---|
| API / task 的 Mod 路径 | `/` 分段，相对 game_root，仅 mods 下 JAR | 禁绝对路径、反斜杠、`.`/`..`、空段、NUL、盘符、UNC、设备路径和 ADS |
| 构建配置相对路径 | `/` 分段，相对 pack_root；game_dir 可为 `.` | 不通过 cwd 解释，不自动搜索硬盘，不从运行参数覆盖 |
| 本机绝对路径 | 验证后按本机 node:path 生成 | 不用于 JSON 协议或下载 URL；不拼接 `file://` 字符串 |

必须先验证原输入，再拼接；不能先 normalize 抹掉 `..` 证据。使用 path.relative(root, candidate) 检查非绝对且没有 `..` 段，不用字符串 startsWith 判断包含关系。跨系统纯校验使用显式 posix/win32 规则，本机实际文件访问使用宿主路径规则。URL 使用 WHATWG URL，file URL 使用 pathToFileURL/fileURLToPath，不能对路径进行 URL decode。[Node path](https://nodejs.org/api/path.html)

Windows 约束在所有平台发布时应用：拒绝 `<>:"|?*`、控制字符、尾随点/空格、CON/PRN/AUX/NUL/COM/LPT 等保留设备名及扩展形式。不能认为 Linux 上创建成功就能在 Windows 使用。Unicode 大小写和规范化不是 SQLite NOCASE 能完全覆盖的；应用层建立保守的 Unicode 规范化/大小写冲突键，保留原始合法路径用于显示与访问，不自动改名。碰撞键仅用于拒绝歧义，不宣称精确模拟 NTFS/APFS；目标平台创建/替换测试是最终门槛。[Windows 文件命名](https://learn.microsoft.com/en-us/windows/desktop/fileio/naming-a-file)

长路径不能仅靠添加 `\\?\` 解决；构建程序、操作系统策略、解压工具和启动器都要支持。P0 测绝对路径大于 260 字符的边界；发布前确定并记录实测支持长度，超出时明确让用户移到较短目录，不修改注册表或截断路径。[Windows 长路径](https://learn.microsoft.com/windows/win32/fileio/maximum-file-path-limitation)

### 5.2 安全根、链接与重解析点

- pack_root、game_root、`.updater`、staging、目标和旧路径分别验证；逐段 lstat，只查名单路径和必要祖先，不遍历 mods。
- 既有祖先 realpath 必须位于准许根内；目标尚不存在时验证最近存在祖先，创建后再验证。拒绝符号链接、junction；目录不能作为 JAR 被删改。
- 目标若为硬链接，禁止就地 truncate/write；用同目录新文件 rename 替换这个目录项，不能改变外部硬链接内容。锁文件的替换或删除也必须避免。
- Windows `Stats.isSymbolicLink()` 不等价于“所有 reparse point”。原 Rust 实现检查了属性位 `0x400`；新实现若只用 lstat，会有保证缺口。必须在 P0 验证 Win32 属性读取和重解析标签，必要时增加仅提供固定路径检查操作的 Node-API 小模块；不通过任意 PowerShell/FFI 命令字符串绕过。
- 对本进程的重复操作由串行状态机和锁保证；对另一个恶意本地进程在检查后替换祖先目录的 TOCTOU，纯路径 lstat/realpath 不能作绝对保证。若要求抵抗此类并发攻击，需 handle-relative/no-follow 原生安全访问层，各平台单独实现和审计；不能宣称现有 Rust 或新 JS 已具备完整保证。

建议维持“拒绝链接逃逸、拒绝 Windows 重解析点；正常用户拥有的本地整包目录”的边界。网络盘、同步盘占位文件、被其他程序并发重排目录都不能无条件承诺支持。此处是显式平台约束候选，需审查确认，不能悄悄缩减功能。[Microsoft 重解析点说明](https://learn.microsoft.com/en-us/windows/win32/fileio/reparse-points)

### 5.3 单写者方案比较

| 方案 | 能力 | 成本与缺口 |
|---|---|---|
| 仅 requestSingleInstanceLock | 重复启动唤醒已有 Electron 窗口 | 不作为游戏目录的通用 OS 文件锁，也不能证明与旧客户端互斥 |
| mkdir/exclusive-create 锁文件 | 不加原生依赖，可拒绝第二进程 | 崩溃遗留锁；仅凭超时/PID 自动抢占会有休眠、进程复用和事件循环停顿竞态 |
| 原生文件锁 | 句柄生命周期关联占用，进程结束后由 OS 释放 | 引入本地二进制与 ABI/平台验证；建议方向 |

候选 `fs-native-extensions` 提供 tryLock/unlock，仓库有 Windows 实现；本轮只核查文档与源码入口，没有验证 Electron 预编译资产或与旧 fs2 锁的互操作。因此它是 P0 候选，不是已批准的生产依赖。若不可用，选择受控最小 Node-API 锁模块，或经用户接受采用“无自动抢占的残留锁人工恢复”，不退回无锁更新。[候选 API](https://github.com/holepunchto/fs-native-extensions)

核心独占 `.updater/packy.lock`，持有同一文件句柄；不在退出时 unlink 锁文件，避免另一实例锁到不同 inode。Electron 单实例仅负责用户体验：按整包真实身份隔离 profile，第二实例显示窗口；不同整包可独立运行。最终写权限始终由核心 OS 锁决定，不能依赖路径哈希完全识别 Windows 路径别名。

锁在 worker 中持有，Main 异常不应使另一核心与尚在写入的 worker 并行。父进程断联：下载时停止并保存；安装阶段按已进入的不可取消临界阶段处理，禁止新任务，最终释放。进程被强杀/断电无法被“禁止退出”拦截，重启必须走原任务恢复。

## 6. 原生 API、启动器与分发

### 6.1 分发结构与根目录

Electron 官方打包结构包含可执行程序、资源和运行时；ASAR 是归档，不是单文件原生应用格式，也不是防篡改或加密机制。[应用分发](https://www.electronjs.org/docs/latest/tutorial/application-distribution)；[ASAR](https://www.electronjs.org/docs/latest/tutorial/asar-archives)

建议完整目录随整包分发，不安装、不自更新：

```text
Windows / Linux
MyModpack/
  Start.exe 或 Start
  resources/app.asar
  其余 Electron 运行库、资源、许可文件
  HMCL.exe 或 launcher.sh
  .minecraft/
  .updater/{state.json,preferences.json,task.json,staging/}

macOS
MyModpack/
  Packy.app/Contents/{MacOS,Resources,Frameworks,...}
  launcher.sh 或 HMCL.app
  .minecraft/
  .updater/...
```

Windows/Linux：以发布布局中 `dirname(process.execPath)` 为 pack_root。macOS：识别当前应用的 `.app` bundle 边界，使用其父目录；不能使用 `Contents/MacOS` 或 `process.resourcesPath` 作为整包根。Main 解析后传入核心，worker 的 process.execPath 指向 helper/runtime，不能自行推导根。开发模式必须显式传隔离 fixture 根，只在未打包开发入口允许。

只读介质/压缩包未解压/不可写目录应在任务前报错；不自动提权。运行中不支持移动整包；停止程序后移动整个目录再启动应正常。Electron 缓存/log 可按 root 身份放 OS userData，安装 state/task/preferences 始终在整包；需要严格“零外部写入”时另作评估。

macOS 需要验证隔离属性、Gatekeeper/App Translocation 对 `.app` 外部游戏目录定位的影响；不能依赖搜索原路径或自动移除 quarantine。建议正式发行签名、公证并用标准解压后真实首次启动测试；如无法稳定定位，应停止并提示正确放置整个包。签名/公证是平台分发能力，不是 Mod 计划签名系统。[Apple 分发签名注意事项](https://developer.apple.com/library/archive/documentation/Security/Conceptual/CodeSigningGuide/Procedures/Procedures.html)

单个 portable 自解压 EXE 可隐藏目录，但实际仍要解压运行时，新增临时目录、杀软、启动延迟和原始目录定位问题；不是本轮默认方案。若用户坚持物理上单文件且没有旁置运行时，必须单独评估，不能声称普通 Electron 打包满足。

### 6.2 启动器调用设计

当前配置仅 launcher_path；建议在各平台构建配置增加受限启动描述 `launcher_kind`、可选固定 `launcher_args`、JAR 模式的整包内 `java_path`。这些是管理员构建输入，不是 Renderer 可传的任意命令。旧配置按平台模板迁移，不运行时猜测脚本内容。

| 平台 / 目标 | 调用 | 必要检查 |
|---|---|---|
| Windows HMCL/PCL2 `.exe` | spawn(绝对 exe, 固定 args, {cwd:packRoot,shell:false}) | 路径存在、普通文件、参数独立；成功 spawn 不等于游戏已启动 |
| Linux/macOS `.sh` | 固定 `/bin/sh` + 脚本绝对路径参数，shell:false | 仅为脚本声明兼容 sh 时使用；bash 专用脚本须显式声明可用解释器；不要拼 `sh -c` |
| 跨平台 HMCL `.jar` | spawn(构建配置中的 Java 路径, ['-jar', jarPath, ...args]) | Java 必须已随整包提供或配置允许的运行时可用；Java 版本按真实启动器/MC 要求验证，不自动下载 |
| macOS `.app` | Electron shell.openPath(已校验 app 路径) | 处理返回错误字符串；工作目录可能不按预期，依赖 cwd 的启动器使用明确 wrapper |
| Windows `.cmd/.bat` | 不自动当 exe 运行 | 需要 cmd.exe 的引用与转义，首版模板建议使用 exe；若实际包需要脚本另行实现及测试 |

Node 官方特别区分 Windows `.bat/.cmd` 与可执行文件；不可把 `shell:true` 当通用兼容开关。中文、空格、单引号、`&`、括号等都作为参数数据传递；不手工加双引号，不拼整条命令。[Node 子进程](https://nodejs.org/api/child_process.html)

launcher_path 经过独立相对路径与链接检查。启动后监听 error/spawn/退出，子进程 stdio 按策略关闭或重定向且不包含凭据；需要启动器独立存活时配置 detached/ignore/unref，并分别实测退出更新器后的行为。窗口显示“已打开启动器”，不承诺 Minecraft 成功运行。Linux/macOS 不承诺运行 PCL2 Windows 可执行文件，不默认引入 Wine。

### 6.3 窗口、托盘、通知和退出

| 能力 | Windows | Linux | macOS |
|---|---|---|---|
| 托盘 | .ico、多 DPI、右键菜单 | SNI/桌面环境差异；Wayland 与 X11 分测 | template 图标、菜单栏、Dock activate |
| 窗口关闭 | 默认隐藏；关闭后台按安全退出 | 无托盘宿主时避免隐藏后无法找回 | close 与 Cmd+Q 分开；Dock 恢复 |
| 通知 | AppUserModelID、快捷方式/Toast 注册条件 | 通知服务是否存在、是否支持点击 | 权限、应用身份、签名/首次运行 |
| 系统结束 | 用户退出可拦截，系统强制结束不可保证 | SIGTERM 尽量安全结束，SIGKILL 无法阻止 | 退出与系统关机区别处理 |
| 错误回退 | 保留完成窗口和托盘结果 | 保留窗口，不因创建 Tray 对象就断言有可见图标 | 通知拒绝不撤销安装 |

Electron 的 Linux Tray 使用 SNI 等平台机制，激活动作随桌面不同；必须测 KDE/GNOME 目标环境和当前 niri/SNI 宿主，不能把一次 click handler 当全部兼容。[Tray](https://www.electronjs.org/docs/latest/api/tray)

Windows 官方通知文档要求应用标识及开始菜单快捷方式等条件。无安装器并不保证每台 Windows 都能收到 toast。选项 A：保持严格便携，尽力通知+窗口回退，符合现有“通知不可用保留完成提示”；选项 B：首次运行经用户同意注册当前用户快捷方式，移动整包时修复，增加持久系统状态。建议 A；若“所有支持的 Windows 必须通知”是硬要求，需要选择 B 并实测。[Electron 通知](https://www.electronjs.org/docs/latest/tutorial/notifications)

安装期间只禁止 Packy 自己的取消/退出入口，不承诺阻止关机、任务管理器或拔盘。后台不可见时错误必须能通过再次打开入口查看，任务仍由核心快照恢复。

### 6.4 Electron 安全边界

- BrowserWindow 显式 `nodeIntegration:false`、`contextIsolation:true`、`sandbox:true`、`webSecurity:true`；只载本地打包 UI，开发 URL 严格限开发入口。
- preload 只暴露 get_status/check_update/start_update/retry_update/cancel_update/launch_game/set_background_enabled 和受控订阅；不暴露 ipcRenderer、fs、spawn、任意 channel/URL。
- 每个 IPC 验证调用 frame/origin、参数 schema、任务身份与状态；拒绝子 frame 和非主窗口。订阅返回取消函数，不向 Renderer 透传 Electron event 对象。
- Main→worker 消息也有请求 ID、schema、超时、明确成功/错误 envelope；退出/超时不自动再发安装命令。恢复前先读权威状态。
- CSP 限制脚本来源；禁止任意导航、新窗口、webview；更新说明按纯文本或安全渲染，不能直接 v-html 注入服务端字符串。
- 外链如保留，仅允许 http/https 且由主进程校验；不得把任意 IPC 字符串传给 shell.openExternal。
- 打包时评估 fuses 关闭 RunAsNode、NODE_OPTIONS/调试入口等；启用 ASAR 完整性相关能力需验证目标平台及工具链。不能为了测试而给正式包开放 Node 集成。

以上选择以 Electron 官方安全清单为依据；防止玩家通过配置切换服务器不等于能阻止拥有本地文件权限的人修改 JS/ASAR。[安全清单](https://www.electronjs.org/docs/latest/tutorial/security)；[Fuses](https://www.electronjs.org/docs/latest/tutorial/fuses)

## 7. 数据库、协议与旧版本迁移

建议采用原 SQL 表结构，不引入 ORM 自动生成第二套 schema。SQLite 单写连接、foreign_keys=ON、busy_timeout=5000、WAL；本地磁盘存储，不把 WAL 数据库放网络共享。同步驱动的 SQL 调用必须短小，不能在事务中 await 网络或密码哈希。若实测请求延迟显示 DB 阻塞明显，将数据库放单独 worker 并用粗粒度完整事务命令调用，不把事务拆成多条跨线程往返。

bcrypt cost 12、72 UTF-8 字节上限及既有 `$2b$` 等实际哈希兼容须用 fixture 验证。密码计算使用有界 worker 池（起步 1–2 worker，队列有上限），队列满明确返回忙，不能为每次请求无限开线程。新注册在算完 hash 后才持有写事务并再次读开关。

迁移建议是“保留业务数据和协议、替换实现”，不是默认清库：

1. 保存当前 API/JSON/SQL/错误码固定 fixture；抽取全部公开及管理路由清单，逐条做差分回归。
2. 用临时旧库副本启动新 server，验证迁移幂等、账户登录、发布快照、触发器、托管文件 ID/URL、范围下载。路径字段与日期/布尔/整数转换逐项核对。
3. 新旧 server 不同时写同一真实数据库；正式切换需停写并做 SQLite 一致性备份，不能在运行时只复制 `.db` 忽略 WAL。真实迁移与回退另获明确授权。
4. 桌面 state/preferences 保持兼容；v1 task 必须先验证、备份，再无损转 v2，保留原计划和 task_id。中间状态存在疑义时停止，不丢任务换最新版。首次迁移写入前必须拿整包锁。
5. 旧 Rust 客户端与新 Node 客户端不允许混跑；P0 测旧 fs2 与新锁的实际互斥。未证明前，分发时要求旧程序退出并替换入口，不能只靠两套锁文件名相同。
6. 旧 UI 可以复用功能组件，Tauri invoke/listen 全部替换为 preload bridge；Rust 导出类型由新 schema 推导替换，避免双来源。
7. 新套件和平台验收通过后，才清理 Rust/Tauri、Cargo 构建脚本和旧 workflow。历史证据保留标明技术路线，不挪用为 Electron 验收。

本轮不修改真实库、旧 task、配置、包管理器、运行时或现有 CI。

## 8. 详细依赖分析

### 8.1 直接依赖候选

“建议”表示方案中的首选，“门槛”表示必须先取得兼容证据。所有新增依赖都需锁定确切版本；本表不是已安装清单。

| 依赖 | 层 / 目的 | 原生与传递依赖风险 | 取舍与门槛 |
|---|---|---|---|
| electron | 桌面运行时；Main/preload/worker | Chromium/Node 和各平台系统库；大体积、安全更新责任 | 候选 44.9.0；三平台真实启动；不安装 WebView2/WebKitGTK 来代替它 |
| typescript、@types/node、vue-tsc | 开发期类型检查 | 类型版本不能超出运行时 API 能力 | 沿用 TS 5.9 系列候选；Node 类型对齐新 Node 24；strict + noUncheckedIndexedAccess |
| Vue 3、Arco、@lucide/vue、motion-v | 玩家/后台 UI | Renderer JS/CSS；不应引入 Node 模块 | 优先保留当前锁定版，验证 Electron Chromium、Vite peer 范围；不新增第二组件库 |
| Vite、@vitejs/plugin-vue | Renderer/admin 构建 | 构建工具平台二进制及 Node engines | 保留既有工具；新 Node 工具链跑真实 build，不能只看版本号 |
| Zod 4 | contracts 运行时 schema 和推导类型 | 纯 TS/JS；进入客户端和服务端 | strict object；URL/path 仍需业务规则；避免同时维护手写 DTO 与重复校验 |
| Fastify 5 | server 路由、hooks、错误处理 | Ajv/Pino 等传递依赖、插件 major 匹配 | 比裸 http 更少样板；不用 Nest 容器；Zod 显式 parse，避免请求被默认 coercion/removeAdditional 悄悄改写 |
| @fastify/multipart | 上传流和限制 | 流解析器传递依赖 | 锁兼容 Fastify 5 版本；截断/异常连接/边界限制测试 |
| @fastify/static | admin 静态文件 | MIME/send 相关依赖 | 静态根隔离，禁上传目录暴露；下载接口自行按 ID 处理 |
| @fastify/rate-limit | 登录/注册及受控上游限流 | 内存计数、代理 IP 语义 | 单 server 进程满足首版；信任代理须明确，不引入 Redis |
| better-sqlite3 | 建议 server SQLite 驱动 | C++ addon、SQLite、预编译安装；无匹配时需编译工具链 | 只进 server，不进桌面；按 Node ABI/OS/arch 构建和验证 |
| jose | JWT 签名与校验 | ESM/WebCrypto 运行时要求 | 显式算法/issuer/audience/时间/token_version；不引入另一 JWT 库 |
| bcryptjs | 兼容原 bcrypt 密码 | 纯 JS，省原生 bcrypt 构建但有 CPU 成本 | 放 worker、有界队列；验证旧哈希和 UTF-8 字节边界 |
| yaml | server 配置 + desktop 构建期配置 | 解析别名、资源耗尽、类型转换 | 已有依赖；限制输入大小/别名，拒绝重复键/未知字段；玩家运行时不读配置 YAML |
| fs-native-extensions | core 进程文件锁候选 | native addon，require-addon/which-runtime；预编译覆盖未验证 | P0 必须在真实 Electron worker 加载、崩溃释放、跨进程和旧版互斥；失败则另选，不能掩盖 |
| 最小 Node-API 路径检查模块（条件项） | 补 Windows 全重解析点检测缺口 | 自维护 C/C++、SDK、签名/ABI | 只在现成可信接口不能覆盖时引入；不是通用 FFI；详细接口审查后实施 |
| @electron-forge/cli、maker-zip | package + 普通归档 | packager、签名/重建相关传递依赖 | package/make，不配置 installer/publish；Linux 可用保留权限的 tar.gz 脚本 |
| @electron/rebuild（若需要） | Electron 原生 addon ABI 对齐 | Python/编译器/平台 headers | 先看候选包构建系统；不是所有自定义 addon 都能直接 rebuild |
| @electron/fuses | 构建安全开关 | 修改平台产物，可能影响签名顺序 | 打包后、签名前配置；与 E2E 启动方式兼容测试 |
| Prettier、ESLint/typescript-eslint | 格式/静态检查 | 开发期依赖 | 沿用 Prettier；重点未处理 Promise、跨层 import；不复刻 Rust lint 名称 |
| node:test、node:assert | domain/server/core 集成测试 | Node 内置 | 编译 TS 后测试，不同时加 Jest/Vitest 两套框架 |
| Playwright | Web 与 Electron 自动化 | 浏览器下载；Electron 驱动能力需版本验证 | Web 用 CLI 工作流；Electron 自动化另跑，不能替代托盘/通知/OS 对话框人工验收 |

Zod、Fastify、SQLite、JWT、bcrypt 的能力依据分别来自维护者资料：[Zod](https://zod.dev/)、[Fastify LTS](https://fastify.dev/docs/latest/Reference/LTS/)、[better-sqlite3](https://github.com/WiseLibs/better-sqlite3)、[jose](https://github.com/panva/jose)、[bcryptjs](https://github.com/dcodeIO/bcrypt.js)。以上是本项目使用设计，不是这些库自动保证所有业务规则。

### 8.2 关键替代方案

| 决策 | 方案 A | 方案 B | 建议 |
|---|---|---|---|
| SQLite | better-sqlite3：成熟直接 SQL，但需原生编译/预构建 | node:sqlite：无额外 addon，但本次 Node 24 文档标为 Release candidate | A；若愿意接受 RC API 并固定 Node 可选 B，不并用两驱动 |
| 打包 | Forge package + 普通 zip/tar，无安装器 | electron-builder dir/portable，生态成熟但 portable 改根目录语义 | 普通目录用 Forge；强制单 EXE 时再选 builder portable 并单独测试 |
| Vite 集成 | Vite build + tsc 明确构建，再 Forge 打包 | Forge Vite plugin 自动集成 | A；官方当前仍将 plugin 标为 experimental，不为了少几个脚本默认接受变动 |
| Node 网络 | 内置 fetch + stream | 显式 undici 控制 dispatcher/timeout/proxy | 先 A，P0 如遇明确能力缺口仅对下载器选 B |
| 通用工具 | fs/path/crypto/URL/AbortController | fs-extra/axios/uuid/p-limit 等 | 当前需求标准库足够，不默认增加这些依赖 |

Node 24 的 SQLite 页面仍标注 1.2 Release candidate；不把“内置”当成稳定 API 的同义词。[Node 24 SQLite](https://nodejs.org/download/release/latest-v24.x/docs/api/sqlite.html)。Forge 文档区分 package 与 make，Vite plugin 有实验性声明。[构建生命周期](https://www.electronforge.io/core-concepts/build-lifecycle)、[Vite plugin](https://www.electronforge.io/config/plugins/vite)

### 8.3 原生依赖交付规则

Electron addon 与系统 Node addon 不能互换。server 的 better-sqlite3 按 server Node 构建，desktop 锁模块按 Electron 实际运行时加载验证；工作区 hoist 不得把同一二进制误打进两个产物。原生 `.node` 放 ASAR 外的解包目录，并携带必要 loader/动态库，不能只把 JS 打包成功就算通过。[Electron 原生模块](https://www.electronjs.org/docs/latest/tutorial/using-native-node-modules)

Windows 构建可能需要 MSVC/Windows SDK/Python；macOS 需要 Xcode 命令行工具及签名凭据；Linux 需要编译器、glibc 和 Electron 运行库。具体最低 OS、glibc、SDK 以选定版本实际安装和发布产物为准，不在本方案虚构兼容下限。无预编译资产的组合必须在 CI 原生编译并实际运行，否则不列为支持平台。

在干净机器上验证压缩包离线启动：桌面不要求用户安装 Node/npm；server 可选“匹配 Node 已安装 + JS/node_modules”或“携带同平台 Node 的完整目录”。建议后一种用于延续独立服务包体验，附许可证和运行时来源/校验；不默认采用 Node SEA/第三方单文件封装。服务包不包含管理员密码、JWT 密钥、真实 DB 或上传文件。

依赖审计输出：精确树、lockfile integrity、原生资产清单、licenses/NOTICE、漏洞检查结果、已知无法修复项。npm audit 为零不证明路径安全、原生 ABI 正确或供应链无风险。不执行远程发布或下载任意二进制来绕过失败。

## 9. 跨平台验收矩阵与证据标准

建议首批目标：Windows 10/11 x64、Linux x64、macOS arm64；macOS x64 是否同批交付待确认。已有 macOS 配置代表设计意图，不代表兼容证据。Windows ARM64、Linux ARM64、网络盘/UNC、Wine 不默认宣称支持；若用户需要则补相应 runner 和验收，不仅增加 target 字符串。

| 验证域 | Windows | Linux | macOS | 通过依据 |
|---|---|---|---|---|
| 原生启动/依赖 | 10/11 干净 VM，普通用户，无 Node | 选定 glibc 基线 + 当前 CachyOS，Wayland/X11 | arm64；x64 独立测试 | 解压产物真实启动，不以源代码 dev 代替 |
| 路径 | 中文/空格/emoji、盘符切换、大小写/保留名/ADS/长路径 | 大小写敏感、中文、权限、符号链接 | APFS 默认/大小写敏感、Unicode NFC/NFD | 合法正常处理，非法在任何 mods 写入前拒绝 |
| 链接安全 | junction、符号链接、其他 reparse tag | symlink、硬链接 | symlink、硬链接 | 明确拒绝逃逸；外部文件哈希不变 |
| 安装/失败 | Java/杀软占用、只读、磁盘满、跨卷 staging | chmod 权限、磁盘满、跨挂载复制 | 权限、磁盘满、跨卷 | 不误写版本；关闭占用后按原任务重试 |
| 单写者 | 双实例、不同整包、旧/新客户端锁 | 双进程、暂停/崩溃、旧 fs2 互斥 | helper 崩溃、双实例 | 任意时刻一个 writer；锁释放后可恢复 |
| 状态提交 | 在阶段切换/rename/state 写后强杀 | 同左 | 同左 | 重启禁止错误启动；原任务恢复或明确报损坏 |
| 生命周期 | 关闭/托盘退出/Alt+F4/系统结束 | 无托盘宿主、SNI 恢复、SIGTERM | Cmd+Q/Dock/窗口关闭/退出登录 | 可取消阶段安全结束，临界阶段用户退出被拒绝 |
| 通知 | 便携无快捷方式与有标识环境 | 通知服务存在/缺失 | 允许/拒绝通知 | 成功语义不受通知失败影响；记录系统行为 |
| 启动器 | 实际 HMCL/PCL2 exe、特殊字符路径 | 实际 HMCL 与 Java/sh | 实际 HMCL jar/app、Gatekeeper | 启动器打开与游戏启动分别记录 |
| 网络 | HTTP、HTTPS、代理/系统 CA 约定 | 同左 | 同左 | 四源、重定向降级拒绝、错误 hash、限速/断网 |
| 原生 UI | 缩放 100/150/200%、窗口最小尺寸 | Wayland/X11、字体渲染 | Retina、系统菜单 | 查看实际截图，键盘操作/焦点/进度不卡顿 |

通用自动化必须覆盖：

- S01–S21 每个功能的成功与权限/输入错误路径；所有 API 路由与旧契约响应比对；参数绑定、草稿并发、注册开关竞争、不可变快照。
- ADD/REPLACE/REMOVE、跨多版本、同路径替换、路径交换和大小写冲突；玩家额外文件在 fixture 中预置，测试后比对所有哨兵文件字节（测试夹具可全量检查，生产程序不可扫描）。
- 下载并发上限 4、总尝试上限、镜像切换、未知大小、慢速/截断/压缩响应、重定向环、取消等待流关闭、计划接口离线后原任务继续。
- 每个文件操作和状态提交点注入失败；分清“下载失败未改目录”“安装失败已有部分变化”“state 已提交但标记未完成”。
- Renderer 注入恶意 IPC、加载非可信页面、未知 channel、任意路径/命令参数等均被拒绝；无 Node globals 暴露。
- 上传超限、断开、Range、路径探测；托管已发 URL 内容稳定；不能下载 DB/config/admin 源文件。

每个平台证据包含 commit/工作树摘要、OS/arch/文件系统、Node/Electron、产物哈希、内嵌配置哈希、执行命令、日志、目录前后结果、截图和手工验收人。CI 构建、受控 mock 来源、真实四源、真实启动器、原生托盘/通知、真实 Nginx 分开记录。没有对应设备时列“未验证”，不能把测试矩阵本身当通过结果。

## 10. 实施阶段和完成门槛

| 阶段 | 交付 | 才能进入下一阶段的条件 |
|---|---|---|
| P0 平台探针 | 最小 Electron 包 + utilityProcess + 原生锁/路径检查 + launcher fixture；临时 Node server/SQLite 探针 | Windows 文件占用/锁/reparse、macOS 根定位、三平台打包加载证据；确定第 11 节选择 |
| P1 契约和骨架 | TS workspace、schema、API fixture、工具链/锁文件、构建配置生成 | 类型/格式/lint、纯规则、协议差分通过；先保留旧工程可运行 |
| P2 Node server | S01–S21、SQL 迁移、后台真实接口 | 临时旧库/新库、RBAC、并发、上传下载、草稿发布端到端通过 |
| P3 Node 核心 | C01–C13、原任务恢复、路径/锁/故障注入 | 隔离整包完整更新；名单外不变，临界失败可恢复 |
| P4 Electron/后台接入 | C14–C18、preload、安全/原生生命周期；替换 Tauri bridge | Web Playwright + 原生窗口、托盘、通知、启动器分平台通过 |
| P5 兼容与分发 | v1 数据/任务迁移、完整服务包、三平台便携包、Nginx 验证 | 干净系统解压运行、真实来源/真实整包、审核发布证据 |
| P6 切换与清理 | 更新 AGENTS/PRD 技术章节/ARCHITECTURE/PLAN/contracts/README/CI；移除已替代 Rust 路径 | 用户确认方案和真实迁移窗口；不混淆旧验收结果；Git/发布单独授权 |

P0 不修改真实玩家目录、服务器或全局 Node。所有脚本通过 Node API/path 执行，避免 Unix 专用 `rm -rf/cp/chmod` 作为跨平台构建逻辑；平台特有签名/权限动作显式分支。测试、类型检查、格式、lint 和真正的打包启动各自作为门槛，不用一个 build 命令替代全部。

跨平台风险应先用 P0 暴露，再写大量业务代码；不承诺未经测量的性能、包体积或工期。Electron 安全更新通过管理员重新构建并随整包分发，维持不做自更新的业务边界。

## 11. 待用户决定的实质取舍

| 决策 | 建议 | 可选项与代价 |
|---|---|---|
| D1 分发形态 | 完整 Electron 目录/`.app` 随整包，不制作安装器 | 单 EXE 需 portable 自解压，增加体积/启动/临时目录定位与杀软验收；常规 Electron 不是单文件 |
| D2 首批支持平台 | Win10/11 x64 + Linux x64 + macOS arm64；有 x64 Mac 再纳入同批 | 只优先 Windows 可缩短平台准备，但其余不能标完成；全架构需要设备和单独产物 |
| D3 原生与安全边界 | 接受经过验证的最小原生锁/Windows 属性检查依赖，限制受支持本地文件系统 | 坚持零 addon 则残留锁恢复和全 reparse 检测需降低保证；强对抗 TOCTOU 则需更大的原生访问层 |
| D4 Windows 通知 | 便携、尽力 toast、窗口回退 | 若必须可靠注册通知，需要用户级快捷方式/标识注册及移动目录修复 |
| D5 旧数据 | 保留旧 SQLite/托管文件/版本状态，先在副本迁移验证 | 明确全新部署可减少迁移工作，但不能默认丢旧数据；仍要决定已有玩家 task 兼容 |
| D6 SQLite 驱动 | better-sqlite3，仅服务端 | node:sqlite 减少 addon，但需接受当前 RC 稳定级别；两者都可保留现有 SQL |

以上建议不是替用户作最终决定；未决定不影响本轮设计交付。批准实施前至少确认 D1/D2/D3；其余可明确接受建议或指定替代。完成这些选择后才能准确锁定原生依赖、CI runner 和交付范围。

## 12. 本轮验证与未完成项

已完成：当前 PRD、API/配置/数据库契约、Rust 路径/任务/安装/密码逻辑和现有前端依赖核对；官方 Electron/Node/依赖维护者文档检索；逐功能设计、风险和验收矩阵；新增文档内部链接和差异检查。

未执行：安装新依赖、Node 工具链切换、新工程编码、跨平台探针、生产数据库迁移、真实 Electron 运行、UI 浏览器验收、Git 写操作和远程发布。因此本轮结论是“形成可实施、可验证的审查稿”，不是“已确保所有平台运行正常”。
