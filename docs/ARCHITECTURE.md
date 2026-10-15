# Packy 项目架构修订稿

日期：2026-10-15。Rust/Tauri 实现与当前目录一致；运行证据见 [验证记录](VERIFICATION.md)。
需求入口：[PRD v0.2](PRD-v0.2.md)。计划：[开发计划](DEVELOPMENT-PLAN.md)。

## 1. 技术与结构

采用 Rust 服务端、Rust 客户端核心、Tauri 2 + Vue 3 桌面、Arco Design Vue 管理后台，数据库保持 SQLite。单仓库 Cargo workspace；服务端和更新核心不依赖桌面框架，桌面通过 commands 调用核心，通过事件展示进度，不启动本地 HTTP API。

采用 Axum、Tokio、SQLx SQLite、Reqwest rustls 和 ts-rs，精确版本由锁文件固定。业务 HTTP/JSON、权限编码、数据库约束及本地配置格式继续沿用既有契约。

```text
管理端 Vue → HTTP/HTTPS API → Rust 服务端 → SQLite
                          ├─ Modrinth 元数据 API
                          ├─ 下载源字典 / 系统设置 / JWT / RBAC
                          └─ 已托管文件下载服务
桌面 Vue → Tauri commands → Rust 更新核心
          ← 状态与进度事件 ←  ├─ 获取一次完整更新记录
                              ├─ 直接从四类来源下载
                              └─ 按名单操作文件、保存版本、打开启动器
```

服务端不调度客户端任务，不维护执行进度。客户端取得记录后独立执行和重试，不查询最新发布。server 来源只是已托管文件服务，不转发所有第三方下载。

## 2. 目录与职责

实际分层与依赖方向见 [架构决策](decisions/0001-rust-layers.md)。

```text
Packy/
  Cargo.toml
  Cargo.lock
  crates/
    common/                  # JSON 协议、类型生成及纯校验
    sources/                 # 下载来源适配与 Modrinth 元数据
    server/                  # Rust HTTP 服务、认证、SQL、发布
    client/                  # 独立 Rust 更新核心
  apps/desktop/
    src/                     # Vue 3、TypeScript、Vite、共享 CSS 主题
    src-tauri/               # Cargo workspace 成员：commands、状态、事件
  web/admin/                 # Arco 管理后台
  migrations/                # 可执行 SQL 与初始化种子
  docs/
```

client 不依赖 Tauri、服务端数据库或元数据接口；不提前建立插件系统、ORM 抽象或多层仓储。服务端和核心可独立测试，不要求 GUI 系统库。

Tauri 层只暴露用途明确的 commands，校验参数与任务状态；前端不获得通用文件删除或任意 shell 执行接口。限制窗口 capabilities 和插件权限，不向远程页面开放 IPC。capabilities 不替代 Rust 业务校验；下载和阻塞文件操作不得阻塞 UI。ts-rs 从共享 Rust 类型生成 TypeScript，前端构建先重新生成，不手改生成文件。

## 3. Mod 身份与发布快照

业务 modid 在 Mod 主表唯一，Mod 名称可重复，不建立名称唯一索引，也不依赖名称推断相同 Mod。使用稳定内部 ID 关联 release_item；平台 Project ID 使用独立字段。编辑原记录修改 modid 时验证新值未被其他记录使用，保持内部 ID 不变；历史 release_item 保存当时的 modid、名称和文件信息，不被新值改写。若创建一条新记录而不是编辑旧记录，不自动按同名合并。

服务端从两个完整快照计算最终差异。REPLACE 包含旧路径与目标完整文件信息，客户端不猜文件名、不解析 JAR 来识别 Mod。保留 REMOVE，删除必须在名单中明确列出；不存在全目录清理。

发布事务包含草稿校验、固化快照、发布时间和当前指针更新；版本号按包唯一，单快照不允许重复目标路径。网络取元数据和文件录入不占发布事务。已发布 artifact、文件和快照不可覆盖。首个草稿允许空起点，整包分发必须附有效的已安装 release ID。

不增加不同发布 ID 且空差异的专门流程，也不以此新增发布限制。正常“本地已最新”仍返回空操作列表。

## 4. 来源字典与文件准备

下载源类型字典固定编码 modrinth、curseforge、github、server；后台可维护名称、启用状态和排序，不扩展成任意来源代码执行器。具体 artifact 的 URL/优先级另存，不在字典放密钥或文件数据。

实际表为 download_source_types(code, name, enabled, sort_order) 与 download_sources(artifact_id, source_code, url, priority 等)，完整结构见 migrations。引用中的编码不能删除或变更含义。字典禁用不移除已发布 artifact；生成新记录时过滤禁用来源，若需要下载的目标没有任何启用来源则明确失败，不下发不能执行的计划。客户端已拿到的 sources 不受字典后续变更影响。

Modrinth 保留原搜索和版本 API 接入；CurseForge、GitHub 可先由管理员录入准确链接，不把支持下载源扩大为两套平台全量管理系统。需要 CurseForge API 解析时由服务端持有 API Key，客户端不得携带平台密钥或后台 JWT。获取不到可直接用的链接则禁止发布该来源，不要求客户端安装中再回服务端换链接。

所有来源沿用 SHA-512。CurseForge 官方 FileHash 的算法为 SHA-1/MD5，不能直接复用为 SHA-512；由管理员提供实际文件 SHA-512，或在受控录入时计算。同一 artifact 的备用链接必须提供相同字节。短期、需要后台登录或需要客户端取新令牌的链接不符合本版执行约定；已取得链接失效时正常报下载失败，不新增自动刷新协议。

Rust 服务端来源支持管理员上传 JAR → 计算哈希 → 保存不可变文件 → 返回文件 ID 和下载 URL。文件下载按 ID 映射，禁止任意路径读取；上传接口限制 512 MiB，失败不生成可发布 artifact。由 Nginx 提供 HTTPS 并反代 Rust 文件端点，托管目录与数据库及静态页面分离。文件来源标为 server 不代表其可以自动代理第三方受限文件。

## 5. JWT、账号与实时注册开关

管理员登录后取得 JWT；管理请求使用 Authorization: Bearer，校验固定允许的签名算法、签名、issuer/audience、有效期及账号权限。密钥只在服务端保存，密码用专用密码哈希，不记录密码和 Token。采用基础 RBAC，初始只有 super_admin 和 user 两种角色；注册固定 user，拒绝注册请求中的角色/权限字段，不允许提升权限。

使用 1800 秒 access token，前端只在内存持有，过期重新登录；不在本版自行增加 refresh token/单点登录系统。账号 token_version 与账号状态在每次管理请求检查，注销或禁用立即使旧 Token 失效。

account、role、permission、account_role、role_permission 表实现 RBAC。SQL 初始化角色、权限、关联及首个超管账号；初始化命令读取部署者提供的密码并生成 bcrypt 哈希，不内置通用明文默认密码。初始化应可重复执行而不覆盖已有账号密码或管理员配置。超管管理用户启用状态、角色分配和角色权限；普通用户默认无管理写权限，后端逐接口校验有效权限。第一版不增加自定义角色创建、组织树、字段级权限等功能。

JWT 保存身份及必要的有效性声明，管理请求以数据库中当前账号状态及 RBAC 关系为授权依据，避免角色变更后继续信任过期角色声明。前端菜单和按钮只做展示控制，不代替后端鉴权。

system_settings 持久化 registration_enabled，初始化为关闭，部署者按需求决定是否开放。系统管理写设置要求管理员权限，注册入口读取设置用于展示，后端注册接口必须重新检查，不能信任页面状态或 JWT 内的旧开关值。

“实时生效”定义：设置更新事务提交后，随后判定的注册请求使用新值，无须重启或等待 Token 过期。注册检查和账号创建放在同一个 SQLite 写事务，设置更新与注册创建按事务顺序串行化；先提交成功的注册不因随后关开关被撤销。当前单服务进程直接读数据库，不引入 TTL 缓存、配置文件轮询、Redis 或广播。

注册开关不影响已有用户登录。注册固定普通用户；用户和权限管理是本版交付内容，不再使用固定单管理员假设。超管承担是否开放注册及后续授权的管理责任。

## 6. API 与 Tauri 接口

公开计划接口仍为 POST /api/v1/packs/{pack_id}/update-plan，请求 installed_release_id/client_version，返回目标 ID、版本、说明、操作和完整文件信息、pack_name。每个目标包含相对 path、可空 size、SHA-512、来源编码及 URL 列表，客户端不额外查询字典。

管理 API 分组：auth/login、auth/register、公开注册状态读取、system/settings、system/download-source-types、system/users、system/roles 与角色权限配置、artifacts 与托管文件上传、packs/releases/drafts，以及 Modrinth 元数据。公开文件下载不要求玩家具备后台 JWT。所有管理写接口检查权限，未登录 401、无权限或注册关闭 403、并发/唯一冲突 409；精确 DTO 见 common 模型与生成类型。

Tauri commands 暴露 get_status、check_update、start_update、retry_update、cancel_update、launch_game。前端只能引用 Rust 核心已持有的任务，不得传入任意文件删除命令。后台任务返回 task_id，事件含阶段、计数、字节、速度和错误；进度合并至每秒最多 10 次，完成/失败立即推送。UI 重载通过 get_status 恢复显示。

## 7. 客户端按名单执行

1. 以可执行文件所在目录定位整包，读取配置和 state；未知/损坏版本不猜测。
2. 取得一次完整记录并交给 Rust 核心持有；本地 task.json 可保存相同记录和任务阶段，供失败重试或重启继续。无需服务端注册任务。
3. 玩家确认后，仅下载本次 ADD/REPLACE 所需文件；默认并发 4，每文件初次加 2 次重试，源切换计入总次数。
4. 所有下载完成并通过 SHA-512 校验后，才修改记录列明的文件。失败保留已安装 release ID。
5. 新增/替换按目标路径处理；改名替换只处理明确列出的旧路径和新路径。不扫描目录、不推断玩家 Mod、不自动清理未列明文件。名单目标同名文件按指定操作处理，不新增“玩家来源冲突”分支。
6. 本任务重试始终使用原记录，不请求新版、不自动转入另一个任务。按名单重新下载校验、重做所需操作即可，不建立逐文件认领/回滚系统。
7. 全部文件操作完成后保存本地版本，再将任务标为完成；保存失败仍报错。未完成安装时不允许离线启动。完成后下一次普通检查可取得新记录，不属于失败重试协调。

安装步骤需可重复执行：旧路径已不存在不能导致重复删除永久失败；目标安装成功后再次执行替换也须得到同样结果。操作涉及的目标与其他操作旧路径存在交叉时先完成全计划校验与排序，必要时在发布预览拒绝歧义计划，不保留上一稿“不同 Mod 永久不得复用历史路径”的限制。

仅对指定路径做安全校验，不等于扫描全部 Mod：拒绝越界、绝对路径、Windows 非法名称和符号链接/联接逃逸；范围仍为 mods/ 下 JAR，不擅自限制必须直接子级。同一整包保持进程内/跨进程单写入者。文件占用、权限或跨卷操作失败明确报错，不能把 rename 当成跨卷复制。

下载/校验阶段可取消；安装阶段不提供普通取消。断电原子性和复杂自动恢复不作承诺；本地任务损坏时提示恢复整包，不向服务端猜测新计划。名单外文件保持原样是验收要求，名单中的目标路径不区分原文件来源。

## 8. 范围与验证边界

用户已确认业务 modid 唯一、名称可重复，保留显式 REMOVE，注册普通用户及基础 RBAC。已替换上一稿 D1—D6 讨论，不再将 Session、v2、空差异推进或玩家 Mod 管理列为候选。具体权限编码、JWT 有效期、工具链和初始化参数见 contracts。

Rust 已实现服务端、客户端核心与桌面，旧 SQL 仅作设计追溯。契约见 [API 契约](contracts/API.md)、[数据库模型](contracts/DATABASE.md)、[配置与版本](contracts/CONFIG.md)。旧 Go 构建证据不能用于 Rust/Tauri 验收。当前完整实现已获授权；Windows 与原生交互验收仍独立记录。

## 9. 官方依据

核查日期：2026-10-15。能力依据不代表 Packy 已通过运行验证。

- [Tauri 2 环境依赖](https://v2.tauri.app/start/prerequisites/)：Linux 使用 WebKitGTK 4.1，Windows 开发需对应 C++ 工具链和 WebView2；旧 Go 交叉编译命令不适用。
- [前端调用 Rust commands](https://v2.tauri.app/develop/calling-rust/)。
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)。
- [CurseForge API](https://docs.curseforge.com/rest-api/)。

## HTTP/HTTPS 部署与联调补充（2026-10-15）

用户确认客户端支持 HTTP/HTTPS 服务地址和下载链接。Rust 服务端只监听 HTTP，不配置 TLS；自部署由 Nginx 终止 HTTPS 并转发 HTTP。PACKY_PUBLIC_URL 使用客户端实际可访问的 HTTP/HTTPS 外部地址，不能使用内部反代地址生成下载链接。本地联调用 HTTP。HTTPS 链接的重定向不允许降级为 HTTP；初始 HTTP 链接可正常使用。完成客户端和服务端后执行实际联调，检查计划、下载、校验、安装、失败重试与版本落盘，不能仅以单元测试代替。
