# Rust/Tauri 架构调整与代码清理

日期：2026-10-15。状态：代码清理和文档同步完成；新 P1 尚未初始化。

用户要求保留当前需求、清空代码、改回 Rust + Tauri。沿用 Vue 3、SQLite 和管理后台方案。清理 apps、cmd、internal、migrations、bin、go.mod/go.sum、旧 Node 版本约束；保留 .idea、原始 PRD、业务契约、配置示例与历史证据。DTO 字段和 SQL 内容迁为文档参考，避免需求丢失。

代码及清理前文档备份：/home/helan/.codex/backups/packy-before-rust-tauri-20261015-135333.tar.gz。备份不包含可重新生成的 node_modules、dist、bindings 和 bin 产物。没有 Git 仓库，未执行 Git 写操作。

环境：Linux、bash、systemd-detect-virt=none；Rust/Cargo 1.99.0，Node 18.20.8。未安装依赖或改动系统环境。当前只有设计文件，未执行新架构构建或运行；旧 P1 验收不延用。下一步按开发计划重建 Rust/Tauri P1，业务阶段尚未开始。

验证通过：原始 PRD 与附件逐字节一致；全部相对文档链接有效；配置示例 JSON 可解析；旧代码及产物路径已移除；SQL 参考保留备份中的完整 SQL；所有原 DTO JSON 字段已收录。新工程不存在，未运行构建或业务测试。
