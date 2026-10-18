# Packy

<!-- impeccable:product-schema 1 -->

## Platform

web

Electron 桌面客户端，支持 Windows、Linux、macOS；本文只记录已由用户和当前功能契约确定的事实。

## Users

蒹葭服务器整合包玩家；管理员使用独立的 Web 后台。

## Product Purpose

玩家检查整合包更新、确认变更、下载校验并安装服务端发布的指定 Mod 清单，然后打开已有游戏启动器。

## Stack

客户端 Electron + Vue + TypeScript，Go Sidecar 执行更新核心；服务端 Go/Gin/GORM/Viper。

## Capabilities and Constraints

客户端绑定一个整合包，不是游戏商店或多实例管理器。展示版本、更新说明、增加/替换/移除清单、进度、失败和原任务重试。仅修改名单文件，不扫描或整理其他 Mod。下载可取消，安装与保存阶段不可取消。支持后台运行、托盘及完成通知。服务地址与路径配置构建内嵌，不能设计为玩家随意编辑的表单。后台沿用官方 Arco 模板，不属于当前客户端视觉重设计范围。

## Current Design Request

2026-10-18 用户要求先搜索网络，提供多种客户端 GUI 原型图供审查，每种包含明暗主题。用户已选定 B「游戏氛围」方向，批准明暗全流程原型及组件尺度方案，并授权生产 GUI 重构；客户端现已实现，验证范围见 docs/client-gui-verification.md。原型版本号、更新数量和说明为演示数据，不代表实际服务器状态。

## Evidence on Hand

README.md、docs/contracts/API.md、web/desktop/src/App.vue、docs/PRD-v0.2.md。历史实现不作为新平台验收证据。
