# 三平台独立构建与嵌入配置

2026-10-15。Windows/Linux/macOS 分别配置、分别构建及输出产物。现有项目没有 CI，新增本地脚本和 GitHub Actions 工作流文件，不进行远程操作。

三份 config/client.{windows,linux,macos}.json 根据 Rust target_os 编译选择。脚本仅支持本机平台构建，输出按系统和目标架构隔离，CI 使用对应原生 runner。配置仍含组包占位参数，需按各端填写。实施与本地验证已完成；不能将 Linux 构建当成 Windows/macOS 或远程 CI 通过。


## 实现

- 三份独立 JSON，由 crates/client/src/infrastructure/config.rs 的 target_os 条件选择；桌面/命令行共用，仍不接受旁置配置覆盖。Linux/macOS 示例启动器为 launcher.sh，Windows 为 HMCL.exe，实际组包需分别填写
- npm run desktop:build 使用 Node 标准库脚本；默认本机平台，可指定 --platform 和 --check。拒绝跨系统/目标误用，显式传递原生 Rust target 与 --locked，经 Tauri CLI 构建，不覆盖配置文件
- 输出 artifacts/desktop/packy-desktop-<platform>-<target>.tar.gz，独立目录和归档，只包含程序与 build-info.json 的配置/二进制 SHA-256；tar 保留执行权限。构建期间配置变化会失败，避免摘要与实际嵌入不同
- GitHub Actions 三个独立任务：Windows 2022/x86_64 MSVC、Ubuntu 24.04/x86_64 GNU、macOS 15/ARM64，分别执行配置检查、脚本测试、客户端回归、构建和上传归档。只读仓库权限，不发布 Release 或推送。Runner 标签及 Actions v7 依据 GitHub 官方文档核对
- README、配置/部署契约、PRD、AGENTS 和联调脚本说明同步三端配置，不改写原始 PRD-v0.1。历史单配置原生验收仍为历史证据

## 验证与边界

客户端完整测试、fmt、全工作区 Clippy 通过。修改目标选择断言后，额外执行了 bundled_configuration_ignores_pack_local_override 测试，通过；断言从当前目标系统独立配置文件读取期望值，而非复用被测选择函数。构建脚本 3 个 Node 内置测试通过，包含本机配置/产物命名、跨系统拒绝和未知参数拒绝。脚本/工作流 Prettier 检查通过，YAML 已由其解析器成功解析，未安装 actionlint 或实际运行 GitHub Actions。

实际运行 npm run desktop:build -- --platform linux，Tauri --no-bundle / --target x86_64-unknown-linux-gnu / --locked，前端类型/生产构建和 Linux release 构建通过。解包验证 ELF、Linux 编译配置原始字节存在、配置/程序摘要匹配、程序 mode=0755，归档只有程序与摘要。证据 docs/evidence/platform-build-linux.json。仅本机构建验证，没有再次运行图形窗口；Windows/macOS 编译、运行和远程 CI 未执行。

配置仍含 example.invalid 和示例 pack_id，组包前必须替换；CI 目前只覆盖 Mac ARM64，Intel Mac 可在 Intel 主机原生输出独立目标归档，未验证。macOS 签名/公证、安装器和自更新不在本次范围。没有提交、推送或远程工作流调度。
