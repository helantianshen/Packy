Packy Go 服务端
使用步骤见根目录 README.md。先在 config/server.yaml 中配置随机 JWT 密钥和新数据库路径，再执行 packy-server migrate，手动填写并执行 sql/init.sql。
正常启动：packy-server
管理后台独立部署：web/admin 执行 npm run dev（5173）或 npm run build；此服务仅提供 API
不要把旧 Rust 数据库原地交给 AutoMigrate；从副本导入新库。
