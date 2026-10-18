> 历史记录：2026-10-15 已切换 Rust + Tauri，Go/Wails 代码已清理。以下完成状态和命令仅适用于旧实现，不能用于当前工程验收。

# P1 工程初始化与契约冻结

更新时间：2026-10-15。
状态：工程与契约完成；Windows 实机运行待验收；停在 P1，不自动进入 P2。

## 授权

用户授权初始化 Go 工程、Wails v3 + Vue 3 骨架，确定接口、数据库和权限模型。业务功能、Git 提交/远程操作均不在本轮范围。

## 完成内容

- 模块名 packy，Go 1.27.0；Wails v3.0.0-beta.29，modernc.org/sqlite v1.60.1。
- Node 22.23.3，Vue 3.5.44/Vite 8.3.4/TypeScript 5.9.3/Tailwind 4.3.3，锁文件已生成。
- 服务端健康探针、Vue/Wails Go 服务绑定探针、开发与构建任务。
- internal/protocol DTO、internal/rbac 权限编码；docs/contracts 记录 API、数据库和配置。
- SQL 结构、权限/来源种子、首个超管命名参数 SQL；ApplySchema 仅作初始化工具，无业务库创建。

## 实际验证

前端类型检查与构建、Go 测试/竞态/vet、模块校验、临时 SQLite 结构约束通过。服务端实际健康响应、Linux gtk3 构建/启动与 dev 链路通过。Windows x64 交叉编译成功。浏览器探针按钮成功调用 Go；证据 docs/P1-VERIFICATION.md 与 docs/evidence/p1-browser-probe.png。

## 边界

Linux 已有 GTK3/WebKitGTK 4.1，缺少默认 WebKitGTK 6.0，因此 CLI 与本机桌面使用 gtk3 标签；不安装系统库。临时工具放 /tmp/packy-p1-tools，验证 Node 经 npm exec 使用，不改全局版本。

Windows 实际运行/绑定未执行，不能标为完整 Windows P1 验收。Linux 原生窗口仅验证进程启动与开发服务连接，按钮通过浏览器 server,gtk3 模式验证。系统主题 color-mix 警告未处理，不属于本项目范围。

JWT、注册开关、RBAC 中间件、版本发布、下载/安装均仅有契约，没有实现。测试未触碰真实整包或生产数据库。P2 未开始。

## 下一步

Windows 运行 bin/Packy.exe 并检查 Go 绑定。用户明确指示后再开始 P2；先读当前契约和本任务状态。Go/Node/Wails 构建命令见 README。
