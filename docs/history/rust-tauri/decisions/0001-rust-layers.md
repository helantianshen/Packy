# Rust 模块化分层与目录规范

日期：2026-10-15。用户已授权完整 app/server 的分阶段实现与每阶段自审，不再限于 P1。

采用模块化分层（MVC 的接口、应用、持久化职责划分），暂不采用完整 DDD。当前核心是账号权限、不可变发布快照与文件更新流程，事务和状态机有明确边界；领域事件、聚合仓储抽象会增加跨层转换，尚无相应业务需要。纯规则保留在 domain，避免散落在控制器。

## 目录和依赖

- crates/common：协议 DTO、错误码、路径/哈希校验。不得依赖 server、client、Tauri 或数据库。
- crates/sources：DownloadProvider 接口和四种下载来源适配；Modrinth 元数据适配独立。依赖 common，不持有管理员 JWT，不向客户端发送平台密钥。
- crates/server/src/api：路由、提取器、认证入口、HTTP 错误映射。不得执行 SQL 或编排文件安装。
- crates/server/src/application：账号、权限、字典、发布、文件等用例和事务边界。
- crates/server/src/domain：发布差异等纯业务规则。
- crates/server/src/infrastructure：SQLite 仓储、初始化、配置、文件存储。
- crates/client/src/application：更新状态机、用例、取消/重试和进度。
- crates/client/src/domain：本地任务与安装计划规则。
- crates/client/src/infrastructure：HTTP、下载、文件安全、原子状态保存、进程锁和启动器。
- apps/desktop/src-tauri：Tauri commands、状态注入、事件桥接，调用 client，不包含更新算法。
- apps/desktop/src：Vue 页面、组件、组合式状态和样式，不直接操作文件或启动任意进程。
- web/admin/src：管理页面、API 客户端和权限展示；服务端授权为最终依据。
- migrations：可执行 SQLite 结构与种子；examples 为无密钥的部署/整包配置示例。

按实际功能建立文件，不创建空层级占位。模块按功能拆分，不堆入 utils 或单个控制器；只有真正跨 client/server 的逻辑进入 common。具体仓储先用具体类型；下载源存在四种实现，使用 trait 作为扩展边界。

## 错误与安全边界

可预期错误使用带稳定 code 的 Result；HTTP/Tauri 只返回安全信息，内部错误带 request_id 记录。生产代码禁止 unwrap/expect/panic，不吞 I/O、数据库或 JSON 错误。阻塞密码哈希和文件处理不在 UI 线程执行。Rust 严格检查请求与外部响应，不信任 TypeScript 类型。

路径验证按 Windows 目标约束处理；只访问本次名单中的路径，拒绝越界和链接逃逸。客户端已经取得的计划保持不变，重试不重新请求最新版本。事务保护注册开关和账号创建，账号状态/权限每次从数据库读取。

## 来源适配

DownloadProvider 将已确认的 DownloadSource 转成安全 HTTP 下载请求，各来源保留独立类型，统一 HTTP/HTTPS、超时、重定向、响应校验和错误映射。Modrinth 搜索/项目信息属于服务端元数据能力，不在下载时解析新链接；CurseForge/GitHub 继续采用已解析直链，不扩成平台同步系统。

参考 ferinth 2.13.1 文档后选择直接使用 reqwest 封装已有需求的五类 Modrinth API：文档仍列搜索为缺失能力，且本项目要求返回上游 JSON。保留准确 User-Agent、参数编码、限流/超时和错误边界，不维护无关 API 全集。

依据：[ferinth 文档](https://docs.rs/ferinth/2.13.1/ferinth/)、[Modrinth API](https://docs.modrinth.com/api/)、[Tauri IPC](https://v2.tauri.app/develop/calling-rust/)。

## 阶段审查

每阶段检查目录依赖、注释、错误处理、输入校验与权限；执行匹配范围的 fmt、Clippy、测试和前端构建。关键行为通过接口/文件结果验证，视觉通过实际渲染检查。模拟来源、真实来源、Linux、Windows 证据分别记录。没有对应实测的项目保持未完成，不用构建代替验收。

## 注释与桌面生命周期补充

代码精简以人类可读和可审查为前提。公开类型/方法、重要字段、事务边界、状态转换、错误语义和平台限制写完整中文注释；不省略关键约束，不用压缩表达式代替清晰流程。

首版只提供随整合包放置的可执行程序，不制作安装包，不提供更新器自更新。Tauri 常驻系统托盘；关闭窗口隐藏而非结束进程，后台任务继续。托盘提供打开窗口与退出，安装/保存版本阶段拒绝退出，下载阶段退出须取消并保留可重试任务。更新成功且版本保存完成后发送系统通知；通知权限拒绝或通知发送失败不得把已完成安装改为失败，仍可在窗口看到结果。

## HTTP/HTTPS 部署与联调补充（2026-10-15）

用户确认客户端支持 HTTP/HTTPS 服务地址和下载链接。Rust 服务端只监听 HTTP，不配置 TLS；自部署由 Nginx 终止 HTTPS 并转发 HTTP。PACKY_PUBLIC_URL 使用客户端实际可访问的 HTTP/HTTPS 外部地址，不能使用内部反代地址生成下载链接。本地联调用 HTTP。HTTPS 链接的重定向不允许降级为 HTTP；初始 HTTP 链接可正常使用。完成客户端和服务端后执行实际联调，检查计划、下载、校验、安装、失败重试与版本落盘，不能仅以单元测试代替。
