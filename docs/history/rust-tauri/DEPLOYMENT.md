# 构建、部署和整合包分发

## 服务端

服务端为单进程 Rust HTTP 服务，SQLite 和托管目录需持久化。结构和种子编译进程序；首个超管由 `init-admin <用户名>` 在写事务内参数化创建。密码来自标准输入，用 bcrypt cost=12 哈希；不内置默认生产账号或密码。重复初始化不修改已有超管或角色权限。

部署目录建议：服务程序和后台产物只读；SQLite 与 files 目录对服务账号可写。服务不要以 root 身份运行。开发和生产统一按环境变量 > YAML > 默认值读取，默认配置文件是工作目录下 `config/server.yaml`，`PACKY_CONFIG` 可显式指定其他路径。完整字段和环境变量映射见[配置契约](contracts/CONFIG.md#服务端)。

源码和分发包均将 `config/server.yaml.example` 复制为 `config/server.yaml`，填写随机 `jwt_secret`、实际 `public_url` 和持久化路径。源码运行使用 `admin_dir: web/admin/dist`，解压包运行保留 `admin_dir: admin`。所有相对路径以工作目录解析。默认文件不存在允许仅用环境变量；显式文件缺失、YAML 错误和配置值无效会拒绝启动。空环境变量不会回退，不自动读取 `.env/server.env`。

`PACKY_PUBLIC_URL` 用于生成托管文件链接，必须设置为客户端实际可达的地址。已发布记录含固定 URL，更换域名时须保持旧地址可下载；不要直接修改已发布文件或快照。后台默认部署于 `/admin/`；前缀部署需对应调整前端 Vite base 与 Nginx 路由。

注册默认关闭，在后台系统管理中打开后立即生效。注册只创建普通用户，超管可授予业务权限；系统用户、权限和注册设置始终要求超管角色。管理员应自行决定是否开放公网注册。

## Nginx

TLS 由 Nginx 负责，Rust 服务不读取证书、不监听 HTTPS。以下为部署配置示例，需替换域名和证书路径；当前机器未运行 Nginx 配置验收。

```nginx
server {
    listen 443 ssl;
    server_name packy.example.com;
    ssl_certificate /etc/letsencrypt/live/packy.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/packy.example.com/privkey.pem;
    client_max_body_size 513m;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_request_buffering off;
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
    }
}
```

此部署的 `PACKY_PUBLIC_URL=https://packy.example.com`。原始 HTTP 地址也受支持，适合内网或本地联调；HTTP 不提供传输加密。HTTPS 下载重定向不允许降级为 HTTP。无需为客户端添加管理员 JWT，公开下载按不可变文件 ID 映射。

## 数据维护

备份 SQLite 应使用 SQLite 在线备份或在停服后复制数据库及其必要 WAL 状态，不能仅随意复制活跃主文件。托管目录与数据库需一致备份。已应用迁移不可直接改写，后续结构变更新增迁移版本。不要删除仍被正式快照引用的文件。

本版不提供物理删除正式发布、托管文件清理、用户密码重置和自定义角色创建接口。

## Windows 整合包

在具备 Rust、Node 22、Microsoft C++ 工具链和 WebView2 的 Windows 10/11 x64 上构建，执行 `npm ci`、`npm run build`、`npm run desktop:build`。脚本生成 `artifacts/desktop/packy-desktop-windows-x86_64-pc-windows-msvc.tar.gz`，解压取得其中的 `packy-desktop.exe`，可由整包管理员重命名为 `Start.exe`；不生成安装包或自更新组件。WebView2 是运行前提，本版不提供自动安装器。

```text
MyModpack/
  Start.exe
  HMCL.exe
  .minecraft/mods/
  .updater/state.json
```

构建前编辑项目 `config/client.<platform>.yaml`，填写专用名称、整合包 ID、完整 HTTP/HTTPS 地址和相对路径，再构建程序。配置通过 include_str! 嵌入，不随整包分发；程序不会读取 `.updater/config.json` 覆盖内置值。后台开关保存在可选 `.updater/preferences.json` 中。state 中的 UUID 是该完整整包实际对应的已发布 release ID。不要在未知文件状态下猜测版本。移动整个整合包后路径仍从 Start.exe 目录解析。

只处理本次计划中的 mods 子树 JAR，支持合法嵌套路径、中文和空格。两个文件路径不能互为父目录；符号链接/联接、设备名和越界路径会拒绝。文件占用、权限或版本保存失败时保留原任务重试，不提示成功。完整自动回滚和断电事务恢复不在首版承诺内。

Windows 实机的 WebView2、托盘通知、中文/空格路径、文件占用与真实 HMCL/PCL2 启动仍需对应环境验收；Linux 的结果不能替代这些项目。


## 三端独立构建

组包配置分别为 config/client.windows.yaml、config/client.linux.yaml、config/client.macos.yaml，各自填写服务地址、发布所属整合包 ID、名称和相对路径。选择由 Rust 编译目标决定，修改后重新构建。生产程序不读取整包配置覆盖。

在对应系统运行 npm run desktop:build，可加 -- --platform windows/linux/macos 指明平台；跨系统会失败，不会只换配置输出假产物。产物按系统及 Rust target triple 放入 artifacts/desktop 并生成 tar.gz，携带 build-info.json 核对配置和二进制摘要。脚本使用 tar 保留可执行权限，不生成安装包。

GitHub Actions 三个独立原生任务当前覆盖 Windows x64、Linux x64 和 macOS ARM64；本机 Intel Mac 通过相同脚本可输出独立 x86_64 产物。macOS 不制作安装器，不包含签名/公证；Linux 构建和 CI 配置文件不能代替 macOS/Windows 实际运行验收。


CI runner 与依赖依据：[GitHub 托管 runner](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)、[Tauri Linux 系统依赖](https://v2.tauri.app/start/prerequisites/)。Actions 用法依据各官方仓库的 README：checkout、setup-node、upload-artifact；配置使用 v7。


## 完整构建与服务端分发包

运行 npm run release:build 构建本机客户端和服务端；加 -- --only server 只构建服务端。完整三端客户端与 Linux 服务端由 Packy builds CI 的四个原生任务同时构建，不在单一主机上伪装交叉编译。CI 文件只在项目中准备，本地修改不等于已运行 CI。

服务端归档 artifacts/server/packy-server-<platform>-<target>.tar.gz 包含：

- packy-server（Windows 为 .exe）：Rust HTTP 后端
- admin/：编译后的后台静态资源，运行时 PACKY_ADMIN_DIR=admin
- config/server.yaml.example：无生产密钥的 YAML 模板，复制为 config/server.yaml 后自动读取
- README.txt 和 build-info.json：运行说明和产物摘要

后端二进制内置 SQLite 接入及迁移 SQL，首次初始化/启动创建数据库；上传存储目录按需创建，不依赖外部数据库服务。管理后台静态文件未嵌入二进制，仅部署后端程序不能显示管理页面。程序逐项优先使用环境变量，再读取 YAML，最后使用默认值；JWT 密钥为空会拒绝启动，模板须复制并填写后使用。

在解压目录复制 config/server.yaml.example 为 config/server.yaml，填写实际 JWT 密钥和对外地址，再执行 init-admin 与 serve；服务管理器设置工作目录，环境变量可覆盖文件字段。数据文件不进入构建包，升级时保留持久化数据，不能用新包覆盖数据库。部署系统的运行库和架构必须与二进制匹配；不承诺一个文件可在任意操作系统运行。
