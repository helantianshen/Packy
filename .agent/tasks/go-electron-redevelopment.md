# Go / Electron 重构执行状态

更新：2026-10-18。用户授权分阶段重构并优先清理旧产物/依赖；禁止提交、推送。

## 当前状态

- 已清理 Rust/Tauri 源码、target、旧 node_modules/artifacts/迁移/构建脚本；历史文档在 docs/history/rust-tauri
- Go Gin/GORM/Viper 服务端、Go 更新 Sidecar、Electron/Vue 桌面、官方 Arco Pro Vue 后台均已落地；当前依赖与产物为新栈重新生成
- 单管理员、多整合包、手动 SQL 初始化、AutoMigrate、持久化审计和人工协议已实现；无 RBAC/多租户/Goose；已补充仅首次使用的 Linux 部署脚本
- Go race 测试、vet、两前端 build/typecheck、格式、隔离端到端、合成旧库导入、Linux 打包通过
- Linux 原生 Electron/Sidecar 开发入口及最终便携包启动与关闭行为通过；Windows amd64/macOS arm64 Sidecar 交叉编译通过，但未做对应原生验收
- 后台模板截图已查看，reviewer 对指定修复给出 ship；范围见 docs/admin-template-verification.md
- 旧阶段依赖审计结果仅代表当时状态，最新扫描以全仓修复验证记录为准
- 未修改真实 data、本地 config/server.yaml 或玩家目录；没有 Git 写操作

## 尚未完成的发布验收

Windows/macOS 原生功能、真实游戏启动器/通知、真实网络来源/反代、真实旧库副本与整包验收、远端 CI。详见 docs/VERIFICATION-go-electron.md。不要把“实现已落地”记为“三平台完整验收通过”。

## 后续入口

README.md 包含安装、手动初始化、打包和迁移命令；docs/contracts 是当前契约。下方设计记录保留历史时间点，不代表当前实现状态。

---

## 历史设计过程

# Packy Go + Electron 重建

更新时间：2026-10-18。状态：架构、模板策略及 Gin/GORM/Viper 已由用户确定；设计已更新，生产实现未开始。

## 用户决定

采用 cmd/server、cmd/client、internal/server、internal/client、pkg/protocol、web/desktop、web/admin 和单 go.mod 结构。Go 服务端，Go 客户端 Sidecar，Electron + Vue 桌面，Arco Design Pro Vue 后台。后台直接采用官方完整模板，不自行重搭 layout/UI；风格后期调整。

## 当前方案

`docs/specs/go-electron-redevelopment.md` 是当前目标方案；上一稿 `docs/specs/electron-node-redevelopment.md` 已标记替代，保留业务编号及历史分析。AGENTS.md 已追加目标架构与模板约束，没有把旧实现写成已替换。

建议 Main spawn Go Sidecar，通过成熟库承载 JSON-RPC 2.0 stdio 通信，不开本地 HTTP 端口；Go 拥有更新状态与文件锁，Main 拥有原生桌面能力。方案包含父子进程退出、背压、断管、恢复、Go 路径/文件替换和打包语义。服务端已确定 Gin + GORM + Viper；SQLite 驱动默认候选为 GORM 官方 CGO 驱动，使用 GORM AutoMigrate，基础数据手动执行 init.sql 初始化。其余通用能力优先成熟库，不追求依赖最小化或零 CGO；精确版本待 P0 验证。

## 模板核查

核查官方 arco-design/arco-design-pro-vue 的 arco-design-pro-vite 模板、package.json 和 API interceptor。必须适配 Packy 直接响应体、移除生产 mock、使用单管理员真实认证；保留模板视觉和组件模式。尚未导入模板、固定 commit、安装依赖或运行浏览器。

## 验证与边界

完成文档本地链接、代码围栏、尾随空白和 git diff 检查；未新建 Go 工程、未执行新栈构建/运行、未修改数据库或玩家目录、未删除 Rust/Tauri、未做 Git 写操作和发布。上一轮 Node 原生锁和 SQLite 驱动选项不再适用。

## 下一步

依据当前方案开展 P0：固定模板基线与运行时，验证 stdio 生命周期、Go 锁/Windows reparse 与文件替换、macOS 资源/根定位，以及官方模板原样渲染。然后按 P1–P5 实施与验收。首批平台/架构、最终分发形态、真实迁移窗口仍需在相应阶段落实。

## 依赖策略补充

用户明确不希望为减少依赖而手写成熟通用能力。方案已重审 RPC、HTTP 请求/重试、配置、ORM、迁移、CLI、校验、权限、并发、日志及测试选型；采用成熟库，保留必要业务适配。GORM 零值更新/业务与审计同事务、Viper 空环境值和客户端内嵌源、Resty 总重试预算等列为集成验证点。尚未安装依赖或运行对应代码。

## 2026-10-18 用户简化决定

协议仅维护文档，不生成 Go/TS 类型；Go pkg/protocol 保留实际共享 DTO。取消 Goose，结构用 GORM AutoMigrate；建议显式 migrate 命令先准备结构，然后维护者手动执行 init.sql 创建唯一管理员及四源字典，正常启动不自动补种子或建号，不编写部署脚本。

取消注册、注册开关、RBAC/Casbin、角色权限与用户管理、多租户。保留单管理员登录并管理多个整合包，各包快照独立。新增持久化业务审计与模板审计列表/详情，记录字段前后变化，成功审计和业务修改同事务；敏感字段不记录值。审计失败/上传文件与 DB 非原子边界已写入方案。

本轮更新设计及项目约定，未执行 AutoMigrate、未创建或运行初始化 SQL、未修改真实数据库，未实现审计代码。旧 API/PRD 中相关内容为历史基线，新目标以最新方案为准，实施时同步替换。

## 管理后台独立运行（2026-10-18）

用户要求前后端分离，客户端保持不变。已移除 Go 的 /admin 静态托管和 admin_dir 配置、server:build 的前端构建与拷贝；本地 config/server.yaml 只删除 admin_dir。后台根路径改为 /，dev 默认5173代理8080，build独立输出dist，preview使用4173；认证失效跳转同步改为前端根路径。

验证：后台类型检查/构建、Go服务端测试、隔离smoke通过（含旧/admin/返回JSON404）；浏览器访问独立登录页并验证Vite代理返回401；Linux服务端打包成功且无admin资源。客户端文件未修改，无提交推送。此前“服务端包含后台”的记录已被本节覆盖。

## JAR 中文名称格式化（2026-10-18）

安装文件名统一由服务端生成 `[名称快照]原始文件名.jar`，保留提交路径的目录部分，后台保存草稿后回填最终路径。创建草稿、保存、预览与发布覆盖旧草稿兼容；已发布快照及运行中的客户端任务保持不变。名称中的跨平台非法字符替换为下划线，最终路径检查冲突与文件名长度；超长拒绝，不截断。重复保存不叠加前缀，名称变化通过 REPLACE 精确替换旧路径。

验证：Go 全套测试、后台类型检查与生产构建通过；隔离真实 HTTP + Sidecar smoke 验证中文名落盘、名称含冒号时的清洗、再次发布后旧文件删除以及名单外文件保留。Go 服务端/客户端 application 与协议 race 测试通过。本轮没有重新进行浏览器视觉验收或 Windows/macOS 实机验证；未操作真实数据库和玩家目录，无提交推送。

## 配置与资源文档核对（2026-10-18）

更新 README、docs/contracts/CONFIG.md 和客户端 GUI 文档，列出客户端内嵌字段、服务端Viper字段、环境变量、静态资源源文件/产物位置、运行路径及固定参数索引。明确默认客户端配置/资源已经随构建携带，外部branding覆盖可选；部署密钥与数据库/玩家状态不固化进包，后台保持独立dist。未改变打包机制或运行行为。

重新完成Linux x64桌面打包、服务端打包和后台构建，并检查asar及资源字节、二进制内三平台YAML、Sidecar及生产依赖、服务端样例与SQL存在。未做本轮GUI或Windows/macOS实机验收，未提交推送。

## 玩家下载设置（2026-10-18）

用户确认代理只区分渠道，并加入最大并发、检查/下载超时、最大尝试次数与合计限速。已实现三平台YAML内嵌默认、逐字段本地覆盖、显式空代理直连、恢复默认；保留原任务计划，运行中设置冻结，重试使用新网络设置。代理支持无认证HTTP/HTTPS端点，不支持认证/SOCKS/PAC。限速为共享令牌桶，64KiB突发。设置页保持既有明暗风格。

验证：Go全套race、vet、桌面类型检查/构建、格式检查通过；本地代理HTTP/HTTPS CONNECT/重定向、直连、尝试次数、并发上限、超时、共享限速/取消通过。真实HTTP+Sidecar smoke包含设置保存后更新和替换通过。Playwright浏览器交互及明暗/390px截图实际查看；发现表单边距/布局后修正并重验。原生测试发现preload白名单遗漏，补齐后Linux Electron真实保存、程序重启读取与恢复默认通过。没有Windows/macOS原生验证，未提交推送。

入口：docs/specs/client-download-settings.md；测试：config/download_test.go、internal/client/application/network_test.go、web/desktop/tests/gui-download-settings.cjs、download-settings-native.cjs。CLI run-code读取浏览器测试时须去除Prettier添加的末尾分号。截图位于.impeccable/review/client-b/settings-download-{light,dark,mobile}.png。
