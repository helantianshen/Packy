# CI 补齐（2026-10-18）

用户授权修复 CI、补齐六种客户端及服务端/后台产物、接入回归；禁止提交和推送。

实现：verify 前置门禁；六个平台/架构原生 runner；Linux x64 服务端和独立后台；tar.gz 保留权限/链接；缺失产物报错；并发取消过时任务、缓存和超时。Playwright Test 固定版本及锁文件，乱序请求夹具改为显式延迟 A 到 B 显示后返回。Linux 原生 Electron 启动、设置持久化与品牌文件测试接入；其他平台运行 Go 客户端核心和品牌文件测试。

实现与本机验证完成。已通过 actionlint 1.7.12、Go race/vet、CGO_ENABLED=0 客户端测试、全前端构建/格式、烟测、Linux 原生 Electron、品牌测试、Linux 打包、标准 Playwright 浏览器入口以及三类归档内容/权限检查。根 npm ci 与 audit（0）通过。细节见 docs/VERIFICATION-go-electron.md。没有触发远端 CI，不以本机验证冒充六平台运行；Ubuntu Xvfb 环境及其余五个平台仍待远端实际运行。

## GitHub 跟进（2026-10-18）

用户现已明确授权提交、推送并跟进本次 CI，失败继续修复。正在提交当前 Go/Electron 重构及 CI 完整实现；真实数据库、本地服务配置备份、构建产物和依赖目录不纳入提交。最终以本次提交的八个远端产物及归档内容验证为准。
