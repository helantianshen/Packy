# 客户端与服务端统一构建

2026-10-15。用户要求统一构建三个客户端和服务端，并询问单二进制与配置依赖。受 SDK 限制，本地入口构建本机客户端和服务端，完整 CI 单次运行四个并行 job：三个客户端和一个 Linux 服务端。只写本地工作流，不推送或调度远程任务。

服务端当前依赖进程环境变量，JWT 密钥必填，管理后台是外部静态目录；SQLite 与迁移内置，数据库首次自动初始化，上传目录按需创建。脚本打包程序、后台产物、无密钥环境变量示例和运行说明，不改动后端配置读取逻辑。实现和本地验证已完成。


## 实现

- scripts/build-release.mjs / npm run release:build 默认顺序构建本机桌面与服务端，复用原桌面脚本；--only desktop/server 可分开，--platform 与 --check 支持同一套本机限制。无 SDK 的主机不伪造其他系统产物
- 服务端输出 artifacts/server/packy-server-<platform>-<target>.tar.gz，包括程序、admin、server.env.example、README.txt、build-info.json，不含真实密钥、账号、数据库或上传数据；不改后端读取配置的业务代码
- 原桌面 workflow 更名 .github/workflows/build.yml / Packy builds，三个桌面任务及独立 Linux 服务端任务组成一次全量构建，各自上传，未发布/推送/调度远程 CI
- README 与部署文档说明后端可执行文件无需 Rust/Node/独立 DB 服务，但当前后台静态文件未嵌入、JWT 环境变量必填、不自动读 env 文件、运行库须匹配、持久化路径和 Nginx TLS 边界

## 验证

实际运行 npm run release:build，Linux x86_64 桌面、服务端与后台生产构建全部通过并输出两个原生包。Node 构建脚本 5 个回归测试、服务端差异/认证/RBAC/数据库约束测试、脚本与 YAML 的 Prettier 检查通过。桌面包摘要按本次重建结果刷新。

从真实服务端 tar.gz 解压到 /tmp/packy-server-package-9pbq80pw，使用随机临时 JWT/管理员密码、随机本机端口和临时数据库。验证程序/后台摘要、无生产密钥/数据、缺少 JWT 拒绝启动、init-admin、healthz、后台 HTML/静态 asset、登录和 SQLite 内置迁移落盘均通过。没有打印密钥、密码或 Token。测试服务已终止，未接触生产数据或已有服务。ldd 确认本机服务端链接 libc、libm、libgcc_s 与 Linux 动态加载器，不需 SQLite 守护进程，不能宣称任意系统静态单文件通用。

证据 docs/evidence/server-package-verification.json、platform-build-linux.json。Windows/macOS 与远程 CI 未执行。此次没有重复桌面图形验收，因为只调整构建/打包入口；先前客户端原生验收是历史事实。
