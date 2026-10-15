Packy 服务端运行说明

本包包含后端程序与管理后台静态资源，不包含生产密钥、账号、数据库或上传文件。
运行机器需要与产物匹配的操作系统/架构及系统运行库，不需要 Rust、Node 或独立数据库服务。

1. 在解压目录运行，复制 config/server.yaml.example 为 config/server.yaml 并填写实际参数。
   jwt_secret 必须设置为至少 32 字节的随机密钥，可用 openssl rand -hex 32 生成。
   public_url 填写玩家可访问的完整 HTTP/HTTPS 地址。
   admin_dir: admin 对应本包的静态后台目录。
2. 开发和生产使用同一套读取逻辑：环境变量 > YAML 字段 > 内置默认值。
   默认读取工作目录下的 config/server.yaml，PACKY_CONFIG 可指定其他 YAML 路径。
   默认文件不存在时允许全部通过环境变量配置；显式指定文件缺失、文件格式错误或配置值无效会报错。
   现有 PACKY_* 环境变量仍逐项覆盖 YAML，RUST_LOG 覆盖 log_filter；空值不触发回退。
   相对文件路径以工作目录解析；程序不自动加载 .env 或 server.env。
3. 首次执行 ./packy-server init-admin admin，按标准输入提供密码。
   Windows 将程序名改为 .\packy-server.exe。
   初始化使用同一份数据库配置，不要求 JWT 密钥。
4. 执行 ./packy-server serve，访问 /healthz 和 /admin/。
   SQLite 在首次初始化/启动时创建并执行内置迁移，上传目录按需创建。
   data 目录必须持久化且允许服务账号写入；保管数据库与上传文件的一致备份。
5. 程序只监听 HTTP。外部 HTTPS 由 Nginx 等反向代理终止 TLS。

不要把生产 JWT 密钥或管理员密码提交到项目或加入构建产物。
