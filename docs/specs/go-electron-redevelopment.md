# Packy Go + Electron 重建方案

日期：2026-10-18。状态：技术路线与后台模板策略已由用户确定；详细实施方案待落地验证。

本文件取代 [Electron / Node 业务核心方案](electron-node-redevelopment.md) 的架构、目录、实现依赖、原生模块和实施顺序。旧稿的功能编号仅作追踪；2026-10-18 用户明确取消注册/RBAC/多租户/协议生成/Goose，并增加单管理员、多整合包和业务审计要求，以本文为准。客户端文件恢复语义与适用的平台测试场景继续保留；不得继续按旧稿安装 Node server / Node updater 依赖。2026-10-18 已开始 Go/Electron 实施；实际完成与验收边界见 `.agent/tasks/go-electron-redevelopment.md` 和 `docs/VERIFICATION-go-electron.md`。

## 1. 已确定的架构和职责

采用用户指定的单 Go module 结构，项目根目录继续使用现有 Packy，不为示意名称移动仓库：

```text
Packy/
├── cmd/
│   ├── server/                 # Go 服务端入口
│   └── client/                 # Go 客户端 Sidecar 入口
├── internal/
│   ├── server/                 # 服务端业务、HTTP、SQL、文件托管
│   └── client/                 # 更新状态机、下载、文件操作、锁、启动器
├── pkg/
│   └── protocol/               # 双端共享 DTO、协议校验和错误码
├── web/
│   ├── desktop/                # Electron + Vue；main/preload/renderer
│   └── admin/                  # Arco Design Pro Vue 官方完整模板
├── config/                     # 分平台构建配置、服务端模板
├── sql/init.sql                # 首次手动执行的管理员和基础数据初始化
├── scripts/                    # 构建和打包；不含协议生成或部署脚本
├── docs/
├── go.mod
└── go.sum
```

`cmd` 只处理参数、配置、装配和启动/退出。`internal/server` 按实际功能拆 api/application/domain/infrastructure；`internal/client` 按 application/domain/infrastructure 拆分，不提前建空包。server 的控制器不写 SQL；纯快照差异在 domain。`pkg/protocol` 不依赖 internal、SQLite、Electron 或 UI；不把业务服务塞进共享包。下载来源共用安全 HTTP 下载器，仅元数据行为不同的部分独立适配。

| 部分 | 拥有的职责 | 不承担的职责 |
|---|---|---|
| Go server | 单管理员认证、多整合包、Mod、来源字典、发布快照、计划、上传下载、配置、数据库和审计 | 不调度客户端任务，不持有玩家本地状态 |
| Go client Sidecar | 读取内嵌配置/版本、持锁、请求计划、下载/校验/安装、任务恢复、启动器执行 | 不管理窗口/托盘，不信任 Renderer 提交的文件列表 |
| Electron Main | Sidecar 拉起/通信、窗口、托盘、通知、用户退出协调 | 不复制下载算法、不写游戏文件、不自行判断更新成功 |
| Preload | 固定方法桥接，参数/消息校验、受控订阅 | 不暴露 Node、任意 IPC 或任意命令 |
| Desktop Vue | 状态展示和用户动作 | 不取得 fs/子进程权限，不决定是否允许启动 |
| Admin Vue | 在官方模板内接入登录、业务操作和审计查询 | 不重新搭建 layout/主题/组件系统，不用 mock 登录验收真实权限 |

Node.js 仍用于 Electron Main/preload 和前端构建，**不作为服务端或更新业务运行时**。不再需要 utilityProcess 执行 JS 更新核心、better-sqlite3、bcryptjs、Fastify、Node 文件锁 addon 或自制 Node-API 路径模块。服务器部署不需要 Electron/Node；桌面用户不需要安装 Go/Node。

### 1.1 依赖选型原则（用户已确定）

服务端 HTTP 框架采用 **Gin**，数据库访问采用 **GORM**（数据库仍是 SQLite），配置管理采用 **Viper**。不再把依赖数量最少、只用标准库或纯 Go/零 CGO 当成选型目标。

其余通用能力同样优先采用成熟生态：RPC、迁移、鉴权、验证、HTTP 请求与重试、限流、日志、CLI、文件锁先调查已有库，再按维护状态、功能契合度、文档、测试、许可证和目标平台兼容性选型。自研限于项目特有规则和现有库确实不覆盖的接口适配，不重复实现成熟框架已有能力。标准库可以作为正常底层能力使用，但不以“标准库能写”否决成熟库。

第三方库的默认行为必须与既有契约核对，例如零值更新、环境变量空值、重试次数、事务模式和代理信任；这属于集成验证，不是拒绝使用生态的理由。以下除 Gin/GORM/Viper 外为方案推荐或候选，精确版本与兼容性仍由 P0 验证，不声称已安装。

## 2. Electron 与 Go Sidecar 通信

### 2.1 通道选择

默认采用 Electron Main `child_process.spawn` 创建 Go 子进程，使用私有 stdin/stdout 双向通信，stderr 为日志。`shell:false`，程序路径取打包目录中的固定文件，参数逐项传递；不从 PATH 查找同名程序，不拼 shell 命令。

| 方案 | 优点 | 代价 | 结论 |
|---|---|---|---|
| stdio + 成熟 JSON-RPC 库 | 无监听端口、跨平台；复用请求关联、错误、通知与流解析 | 需验证两端库的 framing/背压/关闭语义 | 首选 |
| 本机 HTTP/WebSocket | Gin 等生态可直接使用，调试工具多 | 需要 loopback 身份密钥/Origin/端口/关闭管理 | 若后续需要多客户端连接再评估 |
| Unix socket / Windows named pipe | 可独立连接、操作系统访问控制 | 两套地址/权限语义 | 需要独立连接时采用已有库适配 |

采用 **JSON-RPC 2.0**，不自行定义 RPC 请求/响应格式与 pending-request 管理。Go 侧 `sourcegraph/jsonrpc2`、TS 侧 `vscode-jsonrpc` 作为配对候选，P0 核查锁定版本的维护状态与兼容性；选用 Content-Length framing 并做互通测试。若候选不适配，换维护良好的现成实现，不回退成手写完整 RPC 框架。[Go 实现](https://github.com/sourcegraph/jsonrpc2)、[TS 实现](https://github.com/microsoft/vscode-languageserver-node/tree/main/jsonrpc)

```text
request  {jsonrpc:"2.0", id, method, params}
response {jsonrpc:"2.0", id, result} 或 {jsonrpc:"2.0", id, error}
notification {jsonrpc:"2.0", method:"update_status", params:{instance_id,task_id,sequence,...}}
```

- 协议/构建版本在 initialize 握手协商；id 用字符串。JSON-RPC error.code 为数字，已有 APIError 放 error.data，不能直接拿 HTTP 字符串 code 冒充 JSON-RPC 错误码。
- 帧长度按 UTF-8 字节计算，头与体均有限制，初始体上限建议 8 MiB；通过库的限制选项或有界 reader 适配，禁止先按不可信长度分配内存。测试拆包、粘包、超限和无效 UTF-8/JSON，不用默认 Scanner 64 KiB 假定可承载计划。
- stdout 只给 RPC，stderr 为日志；序列化/请求关联使用库。Main 持续消费两条管道；普通进度最多 10Hz，终态和响应优先，有界背压不能卡住安装提交。
- 事件含 instance_id/task_id/sequence，重启后丢弃旧实例事件；get_status 为权威，Renderer 重载不重启任务。
- 库提供的取消/超时不代替业务状态机：start/retry/shutdown 超时表示结果未知，不自动重发；先取状态，断连后确认进程/锁状态再恢复。
- hello 通过 JSON-RPC notification 传送；Sidecar 断管时保持既定安全退出/安装策略，不能把 RPC writer 的错误直接当成磁盘事务回滚。

### 2.2 握手与方法

Sidecar 启动后发 hello，包含协议版本、构建版本、目标 OS/arch、内嵌配置摘要和 instance_id。Main 核对配套版本/平台，完成初始化并获取状态后才开放操作。协议不匹配阻止更新并给出重新取得完整整包的提示，不尝试猜兼容。

| 方法 | 参数 | 核心行为 |
|---|---|---|
| initialize | Main 定位的 pack_root、协议握手信息 | 校验根与固定布局；读取 Sidecar 内嵌配置；持锁后读本地记录 |
| get_status | 无 | 返回权威 snapshot 和允许的操作 |
| check_update | 无 | 使用内嵌 pack/server；有未完成任务不刷新计划 |
| start_update | 无 | 仅执行核心当前展示的计划，返回 task_id |
| retry_update / cancel_update | task_id | 验证当前任务和可操作阶段 |
| set_background_enabled | enabled | Sidecar 原子保存偏好；成功后 Main 改变关闭行为 |
| launch_game | 无 | Sidecar 在同一状态守卫内验证并执行固定启动器描述 |
| shutdown | 无 | 按核心阶段准许退出、取消并等待，或拒绝；由最终响应确认 |

Renderer 不传 pack_root、下载 URL、路径、启动命令。pack_root 仅从 Main 的初始化边界进入，Sidecar 自己验证；这是限制 UI 权限，不声称可以防止本机用户手动修改程序或自行运行 Sidecar。

### 2.3 进程生命周期

1. Main 取得 Electron 单实例控制权，确定真实应用布局，然后 spawn Sidecar。Sidecar 持有整包 OS 文件锁，才允许进入更新逻辑；Electron 单实例不代替目录锁。
2. 窗口关闭且后台开启：隐藏窗口，两个进程继续。关闭后台或托盘退出：Main 发送 shutdown，不直接 kill。
3. 下载/校验阶段：Sidecar 取消 context、等待全部 I/O/goroutine 完成、保存原任务后响应并退出；Main 等待子进程 exit 后退出。
4. installing/saving_state：拒绝用户退出。Main 的 close/before-quit 共用守卫，防止重复退出事件递归；超时不得强杀安装进程。
5. Main 异常退出或 stdin EOF：Sidecar 停止接受新任务；可取消阶段安全终止；临界阶段完成当前原任务的受控安装/状态提交或记录失败后退出，持续持锁。避免通过进程组/Job Object 的无条件 kill-on-parent-exit 破坏这个策略。
6. Sidecar 崩溃：Main 禁止更新/启动，显示错误；确认旧 PID 退出后可重建连接并读取原任务，不自动重复 start。孤立 Sidecar 尚在安装时，新进程拿不到锁，显示“任务仍在处理中”而不是删锁抢占。
7. OS 强杀、断电、磁盘拔出无法被应用阻止，沿用原任务恢复表。测试必须覆盖父死子活、子死父活、双实例和安装时退出。

若子进程断管，错误写日志/通知不得中断已进入的文件提交；核心与 stdout writer 必须解耦，磁盘状态是恢复依据。完成通知由 Main 接收已提交完成态后触发；Main 不在场时下次打开显示结果，不让通知成为安装成功条件。

## 3. Go 功能与依赖映射

业务契约以 [PRD](../PRD-v0.2.md)、[API](../contracts/API.md)、[数据库](../contracts/DATABASE.md)、[配置](../contracts/CONFIG.md) 为基线。旧稿 S/C 编号保留，以下替换其实现技术。

| 功能编号 | Go 实现设计 | 依赖/重点 |
|---|---|---|
| S01 配置 | YAML、环境逐字段覆盖、严格字段检查、cwd 相对数据路径 | Viper + mapstructure/validator；显式环境绑定，空值和缺文件语义保持 |
| S02 数据库/迁移 | GORM AutoMigrate 管理结构；首次基础数据手动执行 init.sql；单连接、WAL/外键/busy_timeout | 不引入 Goose，正常启动不创建管理员/补种子；首次部署脚本可执行一次初始化 |
| S03–S05 初始化/登录 | 手动 SQL 建立唯一管理员；bcrypt + JWT 登录、注销、当前身份 | 删除注册及程序自动建号；无默认密码；认证仍必需 |
| S06–S08 旧功能撤销 | 删除用户/角色/权限管理、注册开关及相应表/API/UI | 无 Casbin/RBAC；所有管理接口统一要求有效管理员登录 |
| S09 来源字典 | 四编码、顺序、启停；新计划过滤，原任务不变 | protocol 联合枚举、SQL |
| S10–S11 包/Mod 管理 | 分页、唯一 modid、稳定 ID、名称可重复、历史快照独立 | Gin、GORM、google/uuid；复用验证与分页能力 |
| S12 Modrinth | 五组既有 API、User-Agent、超时/限流和参数编码 | Resty、context；保留上游 JSON；匹配现有五类 API 的维护良好 SDK 可直接复用 |
| S13 Artifact | 手工直链、128 位 SHA-512、可空大小、优先级 | protocol + URL 校验；不混用其他 hash |
| S14–S15 托管文件 | multipart 流式限长、计算 SHA-512、固定 ID 存储；ServeContent/Range | Gin 上传/下载接口 + 流式 I/O/SHA-512；强制实际 body 限长，拒绝任意路径 |
| S16–S18 草稿/发布/历史 | revision 比较、发布事务、不可变触发器、前驱快照 | domain 纯函数、GORM 事务；事务内不做网络访问 |
| S19 更新计划 | 稳定 Mod ID 对比两份快照，生成三类操作 | protocol + domain；未知版本拒绝，空差异不额外推进版本 |
| S20–S21 静态/部署/日志 | HTTP server、后台静态目录、Nginx TLS；healthz 语义不变 | Gin、Zap、成熟 Gin 限流中间件；API 404 不返回 SPA；请求大小/超时明确 |
| C01–C03 配置/定位/锁 | 各平台生成 Go 内嵌常量、验证根、读取 state、OS 独占锁 | os/filepath、gofrs/flock 或 x/sys；不同平台 build tags |
| C04–C05 状态/原任务 | 单一应用状态机，控制请求串行；落 task 后开始下载 | context、sync、encoding/json；重复点击不重复任务 |
| C06–C09 下载/校验/进度/取消 | 4 worker、每文件最多 3 总尝试、手动受限跳转、SHA-512、取消等待全部句柄 | Resty 流式响应、errgroup、SHA-512；统一总尝试预算，未知大小不伪造百分比 |
| C10–C12 安装/提交/恢复 | 名单路径预检、同目标目录临时文件、同步/替换、state 最后提交、原任务重试 | os.Root/平台文件 API；跨文件不是原子事务，Windows rename 另验 |
| C13–C14 离线/启动器 | 状态机检查后 Go exec.Command 参数数组启动固定 exe/JAR/脚本 | os/exec、平台适配；不接任意 shell 命令、不默认启动 Windows 程序于 Linux |
| C15–C18 偏好/托盘/通知/UI | 偏好由 Go 落盘；UI 原生操作由 Electron，完成依据由 Go 提供 | Electron 内置 API；明暗/动效沿用当前桌面，后台依官方模板 |

### 3.1 数据库结构、手动初始化与事务

使用 `gorm.io/gorm`；默认候选 `gorm.io/driver/sqlite`，其底层为 mattn/go-sqlite3，需要 CGO/C 工具链。服务端在目标平台 runner 构建并验证运行库需求，不为了追求零 CGO 改写 ORM 或自己做 SQLite adapter。若目标部署确需纯 Go，采用已维护的 GORM SQLite 驱动并作同一组兼容测试。SQLite 不因使用 GORM 而更换为其他数据库。[GORM SQLite](https://gorm.io/docs/connecting_to_the_database.html)

GORM 负责常规 CRUD、关联查询、事务和作用域；使用 WithContext、显式业务 model/TableName/列映射，不直接嵌入 gorm.Model 引入既有表没有的 ID/时间/软删除语义。更新 false、0、空字符串或 null 时使用明确字段 Select/Updates map，避免 struct Updates 默认跳过零值；revision 原子条件更新检查 RowsAffected。不要把 client/server DTO 直接作为 ORM model 或无限制写入对象。[GORM 更新行为](https://gorm.io/docs/update.html)

业务写入使用 `db.Transaction(func(tx *gorm.DB) error)`，修改与对应成功审计使用同一个 tx，任一写入失败整体回滚。发布的 revision 比较和当前版本切换也在事务内。删除注册开关竞争逻辑；SQLite 如需 IMMEDIATE 处理发布并发，使用驱动配置验证，不保留一个已经没有业务用途的“注册事务”设计。

结构采用 **GORM AutoMigrate**，不引入 Goose、迁移编号表或自研迁移框架。建议以显式 `server migrate` 命令调用 AutoMigrate，把结构准备与正常服务启动分开；这不是部署脚本。首次流程为：准备配置 → 手动运行结构命令 → 手动执行 `sql/init.sql` → 启动服务。后续模型变化由维护者备份后再次执行结构命令。

`init.sql` 只初始化唯一管理员和四类来源等必要基础数据，不重复维护一套 CREATE TABLE schema；密码填写预先生成的 bcrypt hash，不内置通用密码。可提供只从终端安全读取密码并输出 hash 的辅助命令，该命令不连接数据库、不建号。SQL 在单事务内执行，唯一管理员约束冲突明确失败；不使用 INSERT OR REPLACE 覆盖既有密码，不靠应用判断是否初始化后自动补写。

正常启动仅打开既有数据库并做只读可用性检查；SQLite 使用不自动创建文件的打开模式。缺表、缺管理员或缺必要基础数据时明确提示手动初始化，绝不创建账号或重置数据。“检查并报告错误”与“自行检测后初始化”分开。仅首次部署脚本可执行一次 init.sql，不在 HTTP 启动路径调用；部署脚本生成随机 JWT 密钥，已有安装时拒绝运行。

AutoMigrate 不自动删除废弃列，也不是数据转换工具；模型变更在副本上验证 SQLite 重建表、索引/外键和已有触发器的结果。发布不可变性由业务写入口和事务守卫保证；如保留既有 DB 触发器，必须验证其在 AutoMigrate 后仍有效，不能假定 ORM 会生成/保留所有自定义 DDL。旧 RBAC 表无需为本次设计擅自删除；旧数据导入/破坏性清理单独规划。[GORM AutoMigrate](https://gorm.io/docs/migration.html)

配置使用独立 `viper.New()` 实例，SetDefault、明确 BindEnv、AllowEmptyEnv(true)、ReadInConfig/ReadConfig、UnmarshalExact 与类型/业务校验；禁止全局 Viper 单例泄漏 server/client 设置。不手写 YAML/env 合并器。既有 env > YAML > default 优先级保持，Cobra flags 仅暴露既有或批准的覆盖项；显式空值覆盖文件值，随后必填验证失败而不是悄悄回退。AutomaticEnv 不等于所有字段都能正确 Unmarshal，需绑定并测试。[Viper](https://github.com/spf13/viper)

Viper 只装配受控配置源：server 读既定 YAML/env，client 运行时只读嵌入配置、不启用环境/磁盘覆盖；构建期可用 Viper 解析平台文件。注册和 registration_enabled 已取消；不因 Viper 支持 WatchConfig 就擅自增加热更新语义。重复键、未知键、类型错误和显式缺文件需验证解析/解码结果，不能声称默认设置自动满足全部契约。

server 保留 YAML/env 配置、无默认密码、普通 HTTP 监听。产物为 Go 可执行文件、admin dist、手动使用的 init.sql、配置样例、许可/构建信息；不嵌入并自动执行初始化 SQL，不把真实 DB/密钥打入包。GORM models 是结构定义来源，首次执行步骤必须在 README 写清。

Go 二进制允许跨编译，但交叉 build 成功不是目标平台运行证据。生产切换继续使用旧数据库副本兼容测试、停写一致性备份和独立迁移授权。

### 3.2 共享模型与校验

Go DTO 位于 pkg/protocol，明确 JSON tags、nullable/omitempty、UUID、错误码、HTTP/Sidecar 版本；路径校验是共享纯规则。HTTP/stdio 都限制输入、拒绝未知字段、确保只含一个 JSON 值；对权限敏感输入拒绝重复键，不依赖 encoding/json 默认“后值覆盖”行为。

**协议以人工维护的文档为契约，不做 Go→TS/OpenAPI 类型生成，也不建设生成流水线。** `pkg/protocol` 继续放 Go server/client 共同使用的实际 DTO；桌面与后台在各自 API/bridge 中手写必要 TS 类型。文档写清方法/路由、字段、可空性、错误、请求响应示例和状态语义，少量集成样例验证双方能正确读写即可，不增加重复的全协议框架。

Go/TS 仍保留各自入口的运行时校验。客户端涉及的时间字段统一文档约定：业务时间为服务端 UTC RFC3339 字符串，界面按本地时区显示；进度速度使用本地单调时钟，不能用 UI 时间推断安装成功。数字不得超出 JS 安全整数范围，HTTP snake_case 和不受本次简化影响的状态码保持。

### 3.3 成熟生态选型与集成边界

| 能力 | 选型 | 集成要求 |
|---|---|---|
| HTTP 框架 | **Gin（已确定）** | RouterGroup、binding/validator、中间件和统一错误映射；代理信任显式配置；Context 不跨异步任务误用 |
| ORM/SQLite | **GORM（已确定）** + gorm.io/driver/sqlite | model/关联/事务交给 ORM；驱动 CGO、零值更新、旧表映射和事务模式实测 |
| 配置 | **Viper（已确定）** + mapstructure/validator | 配置读取/覆盖/解码交给库；严格保持配置源与空值语义 |
| 数据库结构/初始化 | GORM AutoMigrate + 手动 init.sql | 显式准备结构，管理员手动执行种子 SQL；无 Goose/启动时自动建号；支持首次部署脚本 |
| 参数校验 | go-playground/validator（Gin 集成） | 通用字段用 tag/库，项目路径/计划不变量用自定义校验器；拒绝未知字段 |
| JWT/密码 | golang-jwt/jwt/v5、x/crypto/bcrypt | 复用签名、claims、密码算法；显式限制算法/时间及当前账号状态 |
| 出站 HTTP/下载 | Resty | 复用请求/超时/重定向/重试；下载流式处理，响应 body 关闭；镜像切换与库重试共用每文件最多 3 次预算，不能两层相乘 |
| 并发/取消 | x/sync/errgroup、semaphore（按实际需要） | 复用错误传播和并发限额；进入安装前等待全部下载结束 |
| Sidecar RPC | sourcegraph/jsonrpc2 + vscode-jsonrpc 配对候选 | 复用标准协议、请求关联、流读写；测试 Content-Length 限制/断管/背压 |
| CLI | Cobra + Viper | server/migrate/password-hash/version 等命令、帮助和 flags；不手写一套 CLI parser |
| 日志/限流 | Zap；Gin 兼容成熟限流中间件候选 | 结构化日志脱敏，stderr 与 RPC 隔离；按真实代理 IP 限流，不暴露密钥 |
| UUID/锁/平台 API | google/uuid、gofrs/flock、x/sys | 使用成熟 ID/锁实现；平台缺口才写受限适配，不自己实现心跳抢锁 |
| 测试 | Go test + Testify、httptest/Gin 测试、Playwright | 使用成熟断言/mock/浏览器工具；真实文件故障与原生平台证据仍必要 |
| 桌面/后台 | Electron/Forge、Vue；Arco Design Pro Vue 完整模板 | 保留模板生态，不为削减依赖重搭布局或替换已有组件能力 |

Gin 的验证/中间件、Resty 的 HTTP 能力、Cobra 的命令体系和 errgroup 的并发控制用于避免重复实现通用机制；版本维护状态、许可证和兼容性在安装前核查。[Gin](https://gin-gonic.com/en/docs/)、[Resty](https://resty.dev/)、[Cobra](https://github.com/spf13/cobra)、[errgroup](https://pkg.go.dev/golang.org/x/sync/errgroup)

仍由项目实现的部分是完整快照差异、只处理名单文件、原计划恢复、安装临界状态、单管理员认证和业务审计，以及平台路径/启动器配置契约。这些规则依托上述库完成，不自行再造 ORM、HTTP 框架、RPC 框架、配置系统或密码学实现。不设依赖数量预算，不以“官方库优先”排除成熟第三方实现。

## 4. Go 路径与原生适配

1. 协议路径统一 `/`，先拒绝绝对路径、反斜杠、空段、`.`/`..`、NUL、设备名/盘符/ADS 等，再 filepath.FromSlash；URL 只用 net/url，不用 filepath.Join。
2. Windows 文件名/大小写冲突检查跨平台执行；Unicode 规范化保守检测，不擅自改名。中文、空格、emoji、盘符切换和长路径需要各平台实测。
3. 使用所选 Go 稳定工具链的 os.Root 受根约束 API 检查能否覆盖 open/create/remove/rename 流程，按实际版本选择方法；不要以 EvalSymlinks 后普通 os.Open 作为抗竞态保证。Root 约束访问不等于“拒绝所有链接”，仍按原需求逐段拒绝链接/reparse。[Go 根约束文件 API](https://go.dev/blog/osroot)
4. Windows 使用 `_windows.go` + x/sys/windows 读取 FILE_ATTRIBUTE_REPARSE_POINT 等原生属性；Linux/macOS 用对应系统实现。平台函数有真实差异才分文件，不用 runtime.GOOS 分支污染所有业务代码。
5. Go `os.Rename` 在非 Unix 平台没有统一原子保证；Windows 状态/目标替换要验证 MoveFileEx/ReplaceFile 等适用语义与 flags、目标存在/不存在和共享占用情况。不得先删旧文件以伪装覆盖成功。[Go os 文档](https://pkg.go.dev/os)
6. 跨卷 staging 采用复制到目标同目录临时文件、校验、Sync/Close 后替换；安装仍只处理计划路径。保持旧稿最终目标集合 T/旧路径 O 和 O−T 删除顺序，保护交叉换名。
7. OS 锁由 Go 持有到最后一项 I/O 完成；锁文件不 unlink。测试同 Go 二进制多进程、旧 Rust fs2、休眠和崩溃释放；不以 PID 超时抢锁。
8. Windows exe/JAR/脚本、Linux/macOS sh/JAR、macOS app 由 Go 内部平台启动适配执行；macOS app 可固定调用 `/usr/bin/open` 并传独立参数。执行成功只表示启动器已打开。PCL2 不默认跨 OS 可用。

Go 工具链按实施时官方受支持稳定版锁定，验证 os.Root 所需 API 与依赖最低 Go 版本，不照抄旧 Go 历史任务的版本。切换栈解决 Node addon 的 ABI 包装负担，但不消除路径、文件占用、权限、断电或系统差异。

## 5. Sidecar 打包和跨平台分发

- Windows：`resources/bin/packy-client.exe`；Linux/macOS：`resources/bin/packy-client`，Mac 在 `.app/Contents/Resources/bin` 内。使用 Electron extraResources，禁止把 Sidecar 当 JS 放入 ASAR 后假定可执行。
- Main 用 process.resourcesPath 找配套 Sidecar；整包根按 Windows/Linux 可执行文件目录、macOS bundle 父目录确定，再传给 Go。Sidecar 的 os.Executable 所在目录是资源目录，不能用来当游戏根。
- Electron 与 Sidecar 必须同目标 OS/arch 成套；分别构建输出和配置。Mac arm64/x64 独立产物；不因 Electron universal 包自动认为 Go 二进制也是 universal。
- Linux/macOS 归档保留 executable bit；macOS Sidecar 先签名再签外层 app 并按需要公证；Linux 运行库、Windows 安全软件、Mac Gatekeeper/Translocation 都要测。
- Go client 内嵌对应平台配置；构建生成 Go 文件放可编译包内，不分发可覆盖 YAML。Main 与 Sidecar build-info 摘要一致；前端显示的包名来自 Sidecar 配置快照。
- Go server/admin 产物与桌面产物独立；server 构建不依赖 Electron，client 构建不引用 SQLite。用 `go list -deps` 验证依赖边界。
- 无安装器/无自更新保持不变。Electron 仍需完整运行时目录，增加一个 Go 可执行文件不会让它成为真正单文件程序。Windows 通知的快捷方式/标识和 Linux 托盘宿主限制仍保留。

## 6. Admin：完整模板优先，暂不调整视觉

**明确采用 Arco Design Pro Vue 官方完整项目模板，以其现有 layout、菜单、导航、登录、路由、状态管理、表格/表单/详情页模式和主题作为后台基础。禁止只装 `@arco-design/web-vue` 再自行搭一套后台。**

官方源码入口为 [arco-design/arco-design-pro-vue](https://github.com/arco-design/arco-design-pro-vue)，模板目录为 `arco-design-pro-vite`。模板具备路由/状态/页面示例，但不包含 Packy 的业务页面和服务端权限；需要按模板模式接入字段和接口，不能承诺所有业务“无需写 UI”。

### 6.1 导入与保留范围

1. 实施时固定官方 tag/commit，记录来源、许可证和模板版本；导入模板源码到 `web/admin`。先让原模板实际安装、类型检查、构建、浏览器渲染通过，保存基线截图。
2. 保留布局、设计 token、响应式、组件尺寸、表格/表单交互和默认主题，不重写全局 CSS、不改字体/间距/卡片视觉、不混入旧自建后台壳。
3. 去除或隐藏无业务用途的演示路由；保留业务页面需要的模板基础组件。图表/编辑器等依赖只有确认无引用时删除，不为“最少依赖”破坏模板。
4. 只作业务必要修改：产品文案、菜单、字段、校验、接口、权限和空/错/加载状态。新业务页面复制相应模板范式，不自行发明布局。
5. 视觉风格调整单独安排在后期，不在本轮启动设计探索或要求原型选风格。模板实际渲染有缺陷时做最小修复并留下证据。

### 6.2 模板与业务映射

| Packy 页面 | 基于模板的处理 |
|---|---|
| 登录 | 保留模板登录布局；唯一管理员 JWT 登录，无注册入口 |
| 整合包概览 | 工作台/概览范式，替换为当前版、Mod 数、最近发布与历史入口，不保留虚假统计 |
| Mod / Artifact | 标准搜索表格与表单；沿模板 drawer/modal 习惯录入、选版本、编辑来源 |
| Modrinth 搜索 | 搜索列表与详情选择模式；使用真实 loading/空结果/限流错误 |
| 文件上传 | 模板上传控件与反馈；接入 multipart、大小/后缀限制和服务端 hash 结果 |
| 草稿编辑/预览/发布 | 表单和表格范式；显示三类差异、revision 冲突与明确发布动作 |
| 发布历史/详情 | 列表与详情范式，展示不可变快照 |
| 来源字典 | 标准配置表格；删除注册设置页面 |
| 审计日志 | 模板搜索表格/详情抽屉；按整合包、动作、时间、结果筛选，展示字段前后差异；无用户/角色管理 |

### 6.3 必须处理的模板集成差异

- 当前官方模板 API 拦截器有自己的响应约定，不能直接套上现有“成功体直接返回资源、错误 APIError”的服务端。仅修改 `src/api` 适配及错误处理，保留 Packy API；不能为迁就模板把所有后端响应改为另一种 envelope。[官方拦截器](https://raw.githubusercontent.com/arco-design/arco-design-pro-vue/main/arco-design-pro-vite/src/api/interceptor.ts)
- 关闭生产 mock 和模拟登录；`/auth/me` 只返回管理员身份，不返回角色/权限矩阵。删除模板示例角色切换和动态权限配置；401、Token 失效与未登录写入拒绝用真实接口验证。
- 官方 package.json 存在与现有仓库不同的版本范围，不能直接强制 hoist 成旧后台 Vue/Vite 版本；先用模板兼容依赖建立可重复 lockfile，再只作必要安全/兼容升级。若共用 npm workspace 导致不兼容，可让 admin 保留独立依赖边界，不能为统一工具链重写模板。[模板依赖](https://raw.githubusercontent.com/arco-design/arco-design-pro-vue/main/arco-design-pro-vite/package.json)
- 审查模板 prepare/husky 脚本；导入模板不自动授权修改本仓库 Git hooks 或运行 Git 写操作。不需要全局安装 arco-cli，源码模板可按固定版本导入。
- 测试 `/admin/` 部署 base、静态路由刷新、资源 404、CSS 顺序/重复加载、暗色 class、图标和字体、表格宽度、弹窗/抽屉层级及控制台报错。
- Playwright CLI 对模板基线和接入业务后分别截图并实际查看；覆盖桌面和移动视口、真实登录、列表/表单、空/错/加载态。模板来源并不免除渲染验证。

## 7. 单管理员、多整合包与审计

### 7.1 管理边界

一个服务端只有一个管理员账号，管理该服务端的全部整合包。使用单行管理员表（固定 singleton ID 与数据库 CHECK/唯一约束），不建 roles、permissions、account_roles、role_permissions 或 tenant 表；不实现注册、邀请、用户列表和角色分配。保留登录/注销/当前身份及密码 hash/Token 失效检查，管理 API 仍要求认证，玩家检查/文件下载继续公开。

多个 pack 是业务资源，不是租户。release/draft/current_release 归属 pack_id，请求中的 pack/release 对应关系仍必须校验，防止把 A 包的版本写给 B 包。Mod/Artifact 可以维持服务端共享目录，不因多整合包复制租户模型；每个包通过 release_items 独立维护完整清单，修改共享元数据不改变已发布快照。桌面仍是构建时固定 pack_id 的专用客户端，后台提供包选择器。

旧契约的 register、registration-status、system/users、system/roles、system/permissions、注册开关等退出新实现；admin/me 不再包含角色权限。现有文档中的 RBAC 要求为历史约束，以本节最新需求为准。

### 7.2 审计记录模型

审计存 SQLite 的独立 `audit_logs` 表，持久化可查询；HTTP access log/Zap 诊断日志不能代替业务审计。建议字段：

| 字段 | 用途 |
|---|---|
| id、occurred_at | 唯一标识、服务端 UTC 操作时间 |
| actor_id、actor_name | 从已认证身份取得管理员及当时名称；失败登录没有可信身份时 actor_id=null |
| action、result | 稳定动作编码与 success/failed；区分发布、编辑、上传、登录等 |
| pack_id（可空） | 包内操作记录包 ID；共享 Mod/字典/认证操作为全局，不伪造归属 |
| resource_type、resource_id、resource_label | 操作对象及当时可读名称；删除后仍可理解，不级联删除审计 |
| request_id、client_ip | 请求追踪与来源；只信任已配置反代提供的真实 IP |
| changes | 按字段记录 old/new；新增 old=null，删除 new=null；无变化不冒充修改 |
| summary、error_code | 易读动作摘要和安全错误码，不保存内部堆栈或原始请求体 |

记录整合包/Mod/Artifact/来源变更、草稿新增编辑、发布、托管上传、登录/注销，以及失败写操作。发布记录旧/新 release ID 和三类操作明细；草稿条目用稳定 Mod ID 对比增删改，不能只记“请求成功”。只读浏览、搜索和公开玩家下载用访问日志即可，避免与业务变更审计混淆。多包引用的全局资源变更按资源查询，详情可关联相关包，不伪装成一次包内修改。

### 7.3 写入与敏感信息

- 在 application 用例层读取真实旧值、完成验证，使用同一 GORM 事务更新业务并插入成功审计；审计写入失败则整个数据库业务写入回滚，不出现业务成功却无对应审计。仅靠 Gin 路由日志或通用 GORM hook 无法准确描述发布/条目变化。
- 失败用例回滚后独立写 failed 审计，记录尝试操作/安全错误码而非假称已变更；失败审计本身不可写时发严重诊断并返回错误，不伪造日志已落盘。
- 上传文件不与 SQLite 原子提交：先完成临时文件与不可变文件落盘，再事务写 hosted_file 元数据和审计；失败可能遗留孤立文件，记录诊断，不报告上传成功、不扫描删除其他文件。
- 字段白名单与类型化变更构造，排除 password/password_hash、JWT、密钥、Authorization/Cookie、带凭据或签名参数的完整下载 URL。敏感字段仅记“已修改”；普通地址可记录去掉敏感参数后的前后值。不能把完整 GORM model/request marshal 后当审计内容。
- 审计 API 只读分页/详情/筛选，全部要求管理员登录；无编辑删除入口，无自动过期清理。时间+ID 稳定排序，限制 page_size；Pack 删除不删除审计。直接操作数据库的人仍可改记录，不宣称具备防篡改审计链。
- 手动 init.sql 属于服务启动前的运维操作，不声称应用自动捕获任意 SQL；初始化 SQL 可显式插入一条 actor 为 manual-init 的审计记录，后续外部 SQL 同样需操作者自行留证。

### 7.4 对应验收

空库先 AutoMigrate、再手动 SQL 建唯一管理员；重跑 init.sql 不覆盖密码；正常启动缺数据只报错、不补写。管理员登录后管理 A/B 两包，跨包 release 拒绝，客户端仍固定自身 pack。验证普通字段、false/null/空字符串、草稿增删和发布前后值；审计写失败业务回滚；业务失败只留下失败记录；上传失败不伪装成功；秘密不出现在审计/日志/错误体。后台使用模板表格和详情展示审计，实际浏览器验证筛选与长差异渲染。

## 8. 实施顺序和验收

| 阶段 | 工作 | 门槛 |
|---|---|---|
| P0 | Go/Electron Sidecar 通信与生命周期探针；三平台路径/锁/替换探针；原版 Admin 模板基线 | 安装阶段断管/退出安全；目标平台文件操作实测；模板无业务接入时可正常渲染 |
| P1 | 按用户目录建单 Go module、protocol、配套构建和人工协议文档/必要 TS 类型；固定模板来源/版本 | go vet/test、类型检查、协议示例互通、依赖边界、构建产物握手 |
| P2 | Go server 全功能、SQLite、Admin 真实 API 接入 | 手动初始化/认证/多整合包/审计/快照/发布/托管文件、数据库副本验证、模板业务页面实际渲染 |
| P3 | Go 更新核心、状态恢复、下载/安装、启动器 | 4 并发/3 次尝试、名单外不变、故障注入、原任务恢复 |
| P4 | Electron 安全 bridge、托盘/通知/退出、配套分发 | 各 OS 原生运行、父子进程失败、实际启动器、干净机运行 |
| P5 | 旧数据/任务迁移、真实整包、文档和 CI 切换 | 旧库副本、真实来源/反代、Windows/Linux/macOS 分项证据；旧实现已按用户后续明确指令清理，旧证据归档 |

原矩阵追加 Sidecar 专项：JSON-RPC 帧拆包/粘包/超长、损坏 JSON、协议版本不匹配、stdio 背压、stderr 堵塞、Main 崩溃、Sidecar 崩溃、孤立子进程、安装时父退出、超时后命令不重复执行、资源目录特殊字符、缺少 Sidecar/执行权限、错误架构。测试夹具不触碰真实玩家目录。

Go 执行 gofmt 检查、go vet、go test；支持的 runner 另跑 `go test -race`。server 使用默认 SQLite CGO 驱动时配置 C 工具链；client 可独立采用 CGO_ENABLED=0，但必须验证依赖图，不把零 CGO 设为全项目约束。前端按模板/桌面的脚本执行类型、lint、build；Electron 打包和原生验收单列。旧 Go/Wails、Rust/Tauri、浏览器截图都不能替代新栈原生验收。

## 9. 本轮结果与后续边界

本轮已确定：协议文档手工维护、AutoMigrate + 手动初始化 SQL、单管理员/多整合包/无 RBAC 与多租户、业务审计；Gin + GORM + Viper 和成熟生态优先原则保持；用户给定的 Go server + Go Sidecar + pkg/protocol + Electron/Vue + Arco Design Pro Vue 完整模板架构；后期再调整 UI 风格。已补充依赖、stdio 协议、进程控制、Go 路径/原生 API、打包和模板集成方案。

尚需在实际实施中验证/定稿：精确 Go/Electron/模板依赖版本、模板固定 commit、首批 OS/arch、便携包分发形态、旧数据迁移窗口。上一稿的 Node SQLite 驱动和 Node 原生 addon 选项已撤销，不再要求用户选择。

本轮仅修改设计/任务记录和目标架构约定；未安装模板或依赖，未创建 Go 工程，未删除现有实现，未执行新栈运行测试、真实迁移、Git 写操作或发布。文档链接/格式与差异检查不等于实现验收。
