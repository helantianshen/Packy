# 官方模板来源

Arco Design Pro Vue，https://github.com/arco-design/arco-design-pro-vue

固定提交：27020819af653243b7b1a3e8e2481d6b3d798749；导入目录：arco-design-pro-vite；日期：2026-10-18。

保留官方布局、主题、组件和页面范式。Packy 仅替换 API、业务路由与必要页面内容；关闭 mock、角色和注册演示，不重新设计视觉。LICENSE 保留上游 MIT 许可。移除安装时自动执行 husky 的 prepare，避免安装依赖修改 Git hooks。

模板接入移除示例页面、mock、演示权限和未使用的图表/工具依赖；保留布局、导航、登录页和 Arco 组件样式。兼容升级锁定在 package-lock.json，Vue 类型检查器固定为 vue-tsc 3.1.0；没有执行 npm audit fix --force。
