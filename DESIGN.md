---
name: Packy 客户端 B 游戏氛围
description: 森林绿色与体素森林包头，稳定尺度的桌面更新界面
colors:
  light-canvas: "#f5f6f0"
  light-surface: "#fffefa"
  light-subtle: "#edf3e9"
  light-text: "#1b2921"
  light-muted: "#53645a"
  light-line: "#dce4da"
  light-accent: "#18713b"
  light-accent-hover: "#105c2e"
  light-on-accent: "#ffffff"
  light-accent-soft: "#e4f3e6"
  light-warning: "#854c0e"
  light-warning-bg: "#fff6e7"
  light-danger: "#ac302e"
  light-blue: "#2861a4"
  light-focus: "#276fce"
  dark-canvas: "#0c1b17"
  dark-surface: "#12261f"
  dark-subtle: "#1a3027"
  dark-text: "#edf5ee"
  dark-muted: "#adbcaf"
  dark-line: "#30473b"
  dark-accent: "#56db85"
  dark-accent-hover: "#75e79b"
  dark-on-accent: "#092d18"
  dark-accent-soft: "#1b3c29"
  dark-warning: "#f3c477"
  dark-warning-bg: "#332b1d"
  dark-danger: "#ffaaa4"
  dark-blue: "#9fc8ff"
  dark-focus: "#a0c7ff"
typography:
  headline:
    fontFamily: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Microsoft YaHei', 'PingFang SC', 'Noto Sans CJK SC', sans-serif"
    fontSize: "1.5rem"
    fontWeight: 700
    lineHeight: 1.34
  headline-compact:
    fontFamily: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Microsoft YaHei', 'PingFang SC', 'Noto Sans CJK SC', sans-serif"
    fontSize: "1.375rem"
    fontWeight: 700
    lineHeight: 1.34
  title:
    fontFamily: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Microsoft YaHei', 'PingFang SC', 'Noto Sans CJK SC', sans-serif"
    fontSize: "1rem"
    fontWeight: 650
    lineHeight: 1.5
  body:
    fontFamily: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Microsoft YaHei', 'PingFang SC', 'Noto Sans CJK SC', sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.57
  label:
    fontFamily: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Microsoft YaHei', 'PingFang SC', 'Noto Sans CJK SC', sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 600
    lineHeight: 1.57
  number:
    fontFamily: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Microsoft YaHei', 'PingFang SC', 'Noto Sans CJK SC', sans-serif"
    fontSize: "1.25rem"
    fontWeight: 700
    lineHeight: 1.4
rounded:
  badge: "5px"
  file-icon: "6px"
  control: "8px"
  summary: "10px"
  panel: "12px"
  dialog: "14px"
  status: "20px"
spacing:
  step-4: "4px"
  step-6: "6px"
  step-8: "8px"
  step-10: "10px"
  step-12: "12px"
  step-16: "16px"
  step-20: "20px"
  step-24: "24px"
  step-28: "28px"
  step-32: "32px"
components:
  button-primary:
    backgroundColor: "{colors.light-accent}"
    textColor: "{colors.light-on-accent}"
    rounded: "{rounded.control}"
    padding: "7px 16px"
  button-primary-dark:
    backgroundColor: "{colors.dark-accent}"
    textColor: "{colors.dark-on-accent}"
    rounded: "{rounded.control}"
    padding: "7px 16px"
  button-secondary:
    backgroundColor: "{colors.light-surface}"
    textColor: "{colors.light-text}"
    rounded: "{rounded.control}"
    padding: "7px 16px"
  status:
    backgroundColor: "{colors.light-accent-soft}"
    textColor: "{colors.light-accent}"
    rounded: "{rounded.status}"
    padding: "3px 10px"
  panel:
    backgroundColor: "{colors.light-surface}"
    textColor: "{colors.light-text}"
    rounded: "{rounded.panel}"
  switch:
    rounded: "{rounded.status}"
    width: "38px"
    height: "22px"
  navigation:
    textColor: "{colors.light-accent}"
    padding: "6px 12px"
  progress:
    backgroundColor: "{colors.light-accent}"
    height: "10px"
    rounded: "{rounded.summary}"
---
# Design System: Packy 客户端

## Overview

**Creative North Star：「森林中的整合包出发站」**。这是对用户已批准的 B「游戏氛围」方向的归纳：森林绿、体素森林头图与清楚的更新流程。插画营造游戏环境，正文保持工具界面的稳定、可读与克制。

**作用域仅为 `web/desktop`。** Web 管理后台继续使用官方 Arco Design Pro Vue 模板的布局、主题与组件模式，不继承本文配色、圆角或导航。本文从 `web/desktop/src/shared/theme.css` 与 `App.vue` 提取当前生产实现；原型中的初始尺寸不覆盖实际代码。实施与验证状态见 `.agent/tasks/client-gui-b.md` 和 `docs/client-gui-verification.md`。

- 明暗主题分别使用语义颜色与独立背景素材。
- 顶部导航、森林包头、更新内容与底部操作保持稳定秩序。
- 字号和控件尺寸按角色确定；窗口变化通过断点重排。

## Colors

前置 tokens 的 `light-*` 与 `dark-*` 分别对应 `:root` 和 `:root[data-theme='dark']` 的同名 CSS 变量；组件 tokens 的浅色值是浅色示例，运行时所有组件随语义变量切换。

森林绿 `accent` 用于主操作、当前导航、完成和新增；`accent-soft` 提供低强度状态底。`canvas` 是页面背景，`surface` 是内容表面，`subtle` 区分说明和文件图标。`text`、`muted`、`line` 分担正文、辅助文字与边界。

`blue` 表示替换，`danger` 表示移除，`warning` 与 `warning-bg` 表示失败或取消提示。颜色必须与文字或图标共同表达状态。`focus` 专用于键盘焦点。

## Typography

所有内容使用系统字体栈，中文依次回退至微软雅黑、苹方和 Noto Sans CJK SC；没有额外装饰字体。根字号固定 16px，正文 0.875rem，行高 1.57；标题、辅助文字和数字遵循前置角色 tokens。主标题在窄窗口切为 1.375rem，不连续随窗口缩放。

包名、路径、版本和错误允许换行，使用 `overflow-wrap:anywhere`；数字使用 `tabular-nums`。辅助信息通常为 0.8125rem，标签页计数为 0.75rem。版本行使用 1.125rem，摘要与进度数值使用 1.25rem。

## Layout

顶部应用导航最小高 56px，品牌图形 28px；它不是操作系统标题栏。工作区最大外宽 1176px，内边距 24px 28px，正常内容宽 1120px。包头最小高 168px、内边距 24px；图片 `object-fit:cover`，焦点位置 `center 48%`。

配置页使用 `minmax(240px, 0.8fr) minmax(0, 1.4fr)` 两栏，间距 20px；摘要四列。宽度不超过 799px 时工作区内边距变为 16px、配置单栏、摘要两列、操作按钮另起一行。宽度不超过 480px 时导航可换行、信息字段单列、按钮弹性分配宽度。高度不超过 620px 时包头最小高 112px。所有断点针对 CSS viewport，而非 Electron 外框。

底部操作区 `position:sticky; bottom:0`，保留正常文档占位并使用实色页面背景。弹窗宽 `min(560px, calc(100% - 32px))`，最大高 `calc(100% - 32px)`，清单最大高 40vh 且可滚动。不使用根容器 scale、zoom 或动态根字号模拟响应式。

## Elevation & Depth

主要靠语义表面和 1px 边界分区，内容面板辅以轻量阴影。浅色面板阴影为 `0 6px 20px #142f1910`，深色为 `0 6px 20px #00000018`；弹窗使用 `0 20px 64px #0004` 与 `#081b1680` 遮罩。按钮按下使用 `inset 0 1px 3px #0002`。不要把普通内容区域改成层层浮起的卡片。

## Shapes

控件 8px、面板与包头 12px、弹窗 14px 圆角；状态标签与开关 20px，文件图标 6px，文件动作徽标 5px。圆形步骤编号为 28px。体素感由插画提供，正文控件保持当前清晰的圆角工具形态。

## Components

### 按钮与焦点

普通按钮最小高 36px、内边距 7px 16px、字重 550；操作栏与弹窗按钮最小高 40px。图标按钮宽 36px，常用图形 20px。主按钮用 `accent/on-accent`，悬停用 `accent-hover`；次按钮用 `surface/text`，悬停用 `subtle`。禁用透明度 0.55。交互元素焦点为 `2px solid var(--focus)`，偏移 3px。

### 顶部导航与标签页

当前主导航用绿色文字及 3px 底线，更新内容标签页使用 48px 最小高及 3px 底线。状态属性分别为 `aria-current="page"` 与 `aria-pressed`，不只靠颜色传递选中状态。

### 森林包头与品牌素材

独立浅色、深色森林背景不承载业务文字。包名实际文字区使用不透明 `surface` 背景，外缘渐变用于视觉衔接；不能仅依赖整张背景的渐变保护长名称。文字区域正常最大宽 74%，窄窗口 85%，极窄窗口 100%。替换图片不得改变文字尺寸或业务布局。界面品牌素材与操作系统应用图标是不同层级；素材加载失败保留默认资源。

本地只能读取发布 ID 时明确写“本地发布 ID”，显示简写并通过 title 提供完整 ID；确认目标发布 ID 与本地相同时才使用对应版本号。不得把未知版本伪装成语义版本。

### 清单、状态与进度

文件行最小高 52px、上下内边距 12px，32px 文件图标底；长路径自然换行。状态标签为 13px、3px 10px 内边距，错误文本独立展示可操作的提示，技术详情写入客户端日志。进度条高 10px；大小未知或安装校验阶段采用不定进度，不编造百分比。

### 配置与开关

客户端提供下载设置表单，允许玩家覆盖渠道代理、并发、超时、尝试次数和总限速；默认值仍来自内嵌配置。后台运行使用 38×22px 开关、16px 圆形滑块；配置与说明展示只读地址、路径等信息。主题以明暗按钮选择，不能扩展成玩家编辑服务地址或游戏路径的表单。

### 动效

按钮背景、边界和阴影过渡 140ms；开关背景与滑块过渡 140ms；进度宽度过渡 200ms。不定进度使用 1.8s ease-in-out 往返，加载图标 1.4s linear 旋转。`prefers-reduced-motion:reduce` 禁用动画与过渡，状态仍通过文字和图形可见。

## Do's and Don'ts

### Do

- 保持主题变量、文字语义和交互状态一致；每种主题独立检查。
- 窗口缩窄时重排、换行和滚动；保留用户字体缩放与系统 DPI。
- 安装和保存阶段明确标注不可取消；失败重试沿用原任务。
- 验证长名称、长路径、替换背景、未知下载大小和键盘焦点。

### Don't

- 不把本设计系统应用到 Arco 管理后台。
- 不缩放整个应用来适配窗口，不把重要文字压成装饰小字。
- 不让替换背景侵入不透明文字区，不在素材中嵌入业务文案。
- 不把浏览器截图、文档提取或 Linux 结果当作 Windows/macOS 原生托盘与窗口验收。
