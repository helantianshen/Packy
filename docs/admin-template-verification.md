# Admin 官方模板扩展核查

日期：2026-10-18。范围为已有 Arco Design Pro Vue 模板的普通业务扩展与局部响应式修复，不建立新的视觉方向。

## 模板基线

来源见 [TEMPLATE.md](../web/admin/TEMPLATE.md)：Arco Design Pro Vue 官方 `arco-design-pro-vite`，固定提交 `27020819af653243b7b1a3e8e2481d6b3d798749`。

实际代码继续使用 `web/admin/src/layout/default-layout.vue` 的顶部导航、侧栏、移动端菜单抽屉和内容布局，以及 `web/admin/src/assets/style/global.less` 的背景、表格与通用卡片样式。`web/admin/src/views/packy` 使用 Arco 表格、表单、按钮和抽屉承载业务内容。颜色、字体层级与组件状态沿用模板，不重新搭建后台或增加视觉 token。

本轮检查未发现项目 DESIGN.md 或 PRODUCT.md，不创建替代文件，不把局部修复提升为新设计系统。

## 必要修复

Mod 文件版本抽屉的来源优先级增加可见标签“优先级（数字越小越优先）”，对应唯一 P2 问题；使用已有 Arco 表单项承载标签。来源行允许换行，URL 输入框设置最大宽度约束，抽屉宽度不超过视口，避免手机宽度下 URL 输入框溢出。未调整模板风格。

## 已查看的渲染证据

以下均为本轮实际运行截图，文档核查已逐张查看：

| 证据 | 可确认的范围 |
| --- | --- |
| [desktop.png](../.impeccable/review/desktop.png) | 1440 × 1000 审计列表；模板顶部导航、侧栏、面包屑、筛选与表格布局 |
| [mobile.png](../.impeccable/review/mobile.png) | 390 × 844 审计列表；侧栏收起、筛选换行、表格在自身区域展示，不能据此声称全部列同时可见 |
| [mods-drawer.png](../.impeccable/review/mods-drawer.png) | 1440 × 1000 Mod 文件版本抽屉；优先级语义标签可见，来源行对齐 |
| [mods-mobile.png](../.impeccable/review/mods-mobile.png) | 390 × 844 Mod 文件版本抽屉；URL 输入框位于视口内；下方优先级与保存操作不在此截图范围内 |

## 验收边界

最终 reviewer disposition 为 `ship`，仅表示本轮唯一 P2 优先级标签问题已修复，以及手机 URL 输入框不溢出的局部结果可接受。该结论不代表所有后台页面、完整业务流程、全部交互或动效均已验收，也不授权提交或发布。

本文件记录源码对照与截图复核；没有单独重新运行浏览器、构建或接口测试。浏览器截图不能替代 Electron 原生窗口、托盘、通知、Windows 实机或其他平台验收。
