# 服务端与三端客户端 YAML 配置

2026-10-15。用户授权迁移配置读取、客户端嵌入、构建脚本和部署说明，未授权提交或远程操作。

## 约定

- 服务端逐项采用环境变量 > YAML > 既有默认值，不区分开发与生产
- 默认工作目录下 config/server.yaml；PACKY_CONFIG 显式指定的文件必须可读，默认文件不存在时允许纯环境变量启动
- 日志由 RUST_LOG > log_filter > 既有默认过滤器决定；空值或错误配置直接报错，不静默回退
- JWT 不提供公共默认密钥；init-admin 使用同一数据库配置，不要求 JWT
- 三端客户端配置改为 config/client.<platform>.yaml，保持 target_os 选择和构建嵌入，不改变运行时状态文件格式
- 配置模板不含真实密钥；本地 config/server.yaml 忽略版本管理，分发包仅包含 YAML 模板

## 状态

已完成读取逻辑、三端嵌入配置、构建/打包脚本、部署模板与当前文档迁移。历史验收记录保持原样。

- 服务端一次加载 Settings，初始化命令与 HTTP 服务共用合并配置；配置错误不输出 YAML 原文
- 三端 YAML 保留原有字段值；状态、任务和用户偏好继续使用 JSON 契约
- 构建脚本使用 yaml，Rust server/client 使用 serde_yaml_ng，两个锁文件已更新
- 本地配置已原样迁移至 config/server.yaml，保留已有密钥、完整内容及 600 权限，admin_dir 仍指向 web/admin/dist
- RustRover 工作目录设为项目根目录后可直接启动；已有环境变量仍会覆盖 YAML

## 实际验证

- cargo test --locked：通过，含客户端边界、本地 HTTP 下载/安装/原记录重试、服务端认证和数据库等既有回归
- 新增服务端进程测试：仅 YAML、纯环境变量、逐项覆盖、指定 YAML 的无 JWT 初始化、文件缺失、未知字段/错误类型/语法错误、环境空值和错误值不回退，通过
- 三端 YAML 的 schema/UUID/URL/原参数验证、内置配置不接受旁置覆盖，通过
- cargo fmt --all --check、cargo clippy --locked --workspace --all-targets -- -D warnings：通过
- npm run test:build：5 个测试通过；脚本、三端 YAML、模板及前端 Prettier 检查通过；联调夹具 Python AST 解析通过
- 前端类型生成、桌面及后台 TypeScript/生产构建，通过；Linux desktop debug custom-protocol 构建通过
- 全量 npm run build 曾因 xlings 的嵌套 npm 包装器切回 Node 18 失败；随后通过显式 Node 22.23.3/npm CLI 分别执行两个 workspace 构建通过，未修改全局工具链
- npm run release:build -- --only server：通过显式 Node 22/npm CLI 执行，重建 Linux 服务端包
- 解包后在临时目录仅靠 YAML 完成管理员初始化、服务启动、healthz、后台 HTML、JWT 登录和数据库路径检查，通过后终止进程；包内包含 YAML 模板，不包含真实密钥、本地 config/server.yaml 或旧 env 模板
- 证据：docs/evidence/yaml-config-verification.json

## 边界

未修改业务数据库或真实玩家文件，未执行 Git 写操作或远程发布。未重复原生 GUI 交互验收；Windows/macOS 原生构建及运行仍需对应环境验证。用户仍需初始化自己本地数据库的管理员，测试临时账号不进入本地数据库。

## 统一 config 目录（2026-10-15）

按用户要求，默认文件路径、忽略规则改为 config/server.yaml，模板移至 config/server.yaml.example，源码与分发包使用一致目录。迁移前后已核对本地配置内容摘要和权限一致，未生成或替换密钥。PACKY_CONFIG 仍可指定其他文件；所有相对数据路径保持以工作目录解析。

打包脚本只复制并归档 config/server.yaml.example，不复制本地实际配置或整个 config 目录，并清理输出目录中已知的旧版根目录模板。README、部署文档、配置契约、AGENTS 和分发包说明已同步。

本次验证：服务端全部测试、服务端 all-targets Clippy、cargo fmt、5 个构建脚本测试及打包脚本 Prettier 通过。Linux 服务端包已重建；临时解包后，不指定环境变量即可从 config/server.yaml 初始化管理员、启动并完成 healthz、后台 HTML 和 JWT 登录，数据库和后台目录仍按工作目录解析。证据见 docs/evidence/config-directory-verification.json。未重复客户端构建或 Windows/macOS 运行验收。
