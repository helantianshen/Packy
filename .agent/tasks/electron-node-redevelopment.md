# Packy Electron / Node.js 技术路线重建

更新时间：2026-10-17。状态：已被用户后续架构决定替代；未开始生产实现。

当前任务见 `go-electron-redevelopment.md`，当前方案见 `docs/specs/go-electron-redevelopment.md`。下文保留上一轮历史，不再作为 Node 服务端或更新核心的实施依据。

## 用户目标与范围

功能需求暂时不变，改用 Electron + TypeScript + Node.js，对每项功能设计和依赖进行分析，重点处理路径、原生调用和跨平台行为。本轮交付技术方案，不推导 Git 写操作、删除旧工程或真实数据迁移授权。

## 本轮交付

- 主方案：`docs/specs/electron-node-redevelopment.md`
- 已核对 PRD v0.2、API/CONFIG/DATABASE、实际 Rust DTO/路径/存储/安装流程和前端依赖
- 设计覆盖 21 组服务端能力、18 组客户端能力、后台页面、状态机、文件提交、分发和平台验收
- 官方资料确认 Electron 分发结构、通知条件、Node 路径/API 差异、原生 addon 边界及候选依赖

## 当前事实与建议

当前项目仍为 Rust/Tauri；原实现及旧架构文档未修改。建议 Electron Main + preload + utilityProcess 更新核心，Node server + SQLite，沿用 Vue/Arco。新 Node 24 工具链和 Electron 44.9.0 是候选，未安装验证；当前终端 Node 18.20.8 与项目 Node 22.23.3 约束不同，没有修改默认运行时。

## 未决事项

主方案 D1–D6：目录分发还是单 EXE、首批平台架构、原生锁/Windows 路径安全边界、Windows 通知、旧数据迁移、SQLite 驱动。P0 特别验证 Windows reparse/占用/锁、macOS bundle 根与首次运行、原生 addon 在 Electron worker 中加载。fs-native-extensions 只是候选，不是已证实可交付的依赖。

## 实际验证及边界

本轮仅文档与源码调查，未安装依赖或执行新栈测试；没有把 Linux 或旧 Rust 证据当 Electron 跨平台验收。文档本地引用存在性和 Git diff whitespace 检查完成。未进行 commit/push/PR/release。

## 下一步

用户审查方案并确定实质取舍后，在临时整包和数据库副本开展 P0。若决定 UI 重大改版，按项目规则先原型视觉确认，再实现和 Playwright/原生验收。真实数据库切换、Git 写操作和发布另需明确授权。
