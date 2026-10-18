# Go / Electron 人工协议 v1

HTTP JSON 使用 snake_case，不包裹 `{code,data}`。错误为 `{code,message,request_id}`，响应头带 X-Request-ID。400 参数、401 登录失效、404 不存在、409 冲突、429 限流、502 上游、503 无可用来源、500 内部错误。JSON 最大 1 MiB，拒绝重复键与未知字段。列表 `{items,total,page,page_size}`，page 从 1 开始，page_size 默认 20、最大 100。

## HTTP

公开：GET `/healthz`；POST `/api/v1/auth/login`（username/password，响应 access_token/token_type/expires_in）；POST `/api/v1/packs/:pack/update-plan`（installed_release_id/client_version）；GET/HEAD `/api/v1/files/:id/download`（支持 Range）。其余接口用 Bearer JWT。GET `/api/v1/auth/me` 返回 id/username；POST `/api/v1/auth/logout` 返回 204 并废止该管理员所有现有 Token。JWT 为 HS256，issuer=packy，audience=packy-admin，1800 秒。

下表路径均加 `/api/v1/admin`。只有一个管理员，无注册、角色、租户或用户管理。

| 方法 | 路径 | 内容 |
|---|---|---|
| GET/POST | /packs | 分页列表 / name,minecraft_version,loader |
| GET/PUT | /packs/:pack | 详情 / 同创建字段 |
| GET/POST | /mods | keyword 分页 / modid,name,modrinth_project_id,curseforge_project_id |
| PATCH | /mods/:id | 完整提交 Mod 可编辑字段，可空平台 ID 清空关联 |
| GET/POST | /artifacts | mod_id 分页 / mod_id,version,filename,size,sha512,hosted_file_id,sources |
| POST | /files | multipart 的 file，JAR 上限 512 MiB；响应 id,filename,size,sha512,url |
| GET | /packs/:pack/releases | 分页发布列表，包含 items |
| GET | /packs/:pack/releases/:id | 完整快照 |
| GET | /packs/:pack/releases/:id/diff | 与该快照冻结前序版本比较的差异 |
| POST | /packs/:pack/drafts | version,changelog,base_release_id（可空，默认当前正式版） |
| PUT | /packs/:pack/drafts/:id | revision,version,changelog,items；item 为 mod_id,artifact_id,target_path |
| POST | /packs/:pack/drafts/:id/preview | revision，返回三类差异 |
| POST | /packs/:pack/drafts/:id/publish | revision，返回冻结快照 |
| GET | /system/download-source-types | 固定 modrinth/curseforge/github/server |
| PUT | /system/download-source-types/:code | name,enabled,sort_order |
| GET | /audit-logs | 分页；pack_id/action/result/resource_id/from/to 筛选，result=success/failed |
| GET | /audit-logs/:id | 字段 old/new、主体、时间、资源、request_id、结果 |

Modrinth：GET `/modrinth/search`，GET `/modrinth/projects/:id`，GET `/modrinth/projects/:id/versions`，GET `/modrinth/versions/:id`，POST `/modrinth/version-files`。返回上游 JSON；搜索支持 query/offset/limit/facets；版本支持 game_versions/loaders 的 JSON 数组字符串；哈希请求 `{hashes,algorithm:"sha512"}`，最多 100 个。CurseForge/GitHub 只录入实际下载 URL，不增加平台全量同步。

来源 `{type,url,priority}`。计划 `{pack_id,pack_name,target_release_id,target_version,changelog,operations}`。每个操作 `{action,mod_id,old_path?,target?}`：ADD 仅 target、REPLACE 两者、REMOVE 仅 old_path。target `{path,size,sha512,sources}`，size 可 null；SHA-512 为 128 位小写十六进制。path 必须是 mods/ 下 JAR，相对路径统一 `/`。不同文件不能有大小写/Unicode 归一化别名或父子冲突。名单外文件不扫描、不删除。空差异保留客户端已安装版本 ID；未知/跨包版本拒绝。预览和发布检查所有历史正式版本的直接升级路径，大小写/Unicode 别名改名冲突会提前拒绝；客户端计划接口同样执行协议校验。

## JAR 安装命名

后台 Mod 的 `name` 是显示名称；管理员填写中文名后，草稿保存时冻结为 `name_snapshot`。安装文件名由服务端统一生成为 `[name_snapshot]Artifact.filename`，例如 `mods/[钠]sodium-fabric.jar`。原始文件名、下载 URL 和哈希保持不变；`target_path` 仍需为合法 mods/ 下 JAR 路径，其目录保留，文件名由服务端覆盖。后台保存后显示响应中的最终路径。

名称先做 NFC 归一化、去除首尾空白，控制字符及 `\ / : < > " | ? * [ ]` 替换为 `_`；不自动翻译名称。文件名超过 255 个 UTF-8 字节或 UTF-16 码元时拒绝，避免截断造成混淆。最终路径按大小写不敏感及 Unicode 归一化检查重复和父子冲突。重复保存始终从原始 Artifact.filename 生成，不叠加前缀。

从既有发布创建草稿时沿用名称快照；保存草稿时刷新为后台当前名称。预览与发布使用相同格式化规则，兼容修改前创建的草稿。已发布快照和已有客户端任务不改写；下一次发布的路径变化产生 REPLACE，只删除清单记录的旧路径，不扫描玩家其他 JAR。

## Electron / Sidecar

stdio JSON-RPC 2.0，Content-Length framing，消息最大 8 MiB，header 最大 4 KiB；stdout 仅协议，stderr 诊断。Go sourcegraph/jsonrpc2 与 TS vscode-jsonrpc 负责消息关联。

1. Sidecar 通知 hello：protocol_version=1，version，os，arch。
2. Main 请求 initialize：`{pack_root:绝对路径,protocol_version:1}`，返回 Snapshot。
3. get_status、check_update 无参数；start_update 无参数，返回 task_id。
4. retry_update、cancel_update：`{task_id}`，检查期间 task_id 为空；set_background_enabled：`{enabled}`；launch_game 无参数；shutdown 无参数。

Snapshot 包含 config/preferences/state/status/plan/can_launch。status 包含 instance_id/task_id/sequence/phase/completed_files/total_files/downloaded_bytes/total_bytes/bytes_per_second/error。Electron 每 250ms 取快照并发给渲染页；instance_id 与 sequence 防止旧状态覆盖。Main 仅开放固定命令，校验发送者；隔离 context、sandbox、禁止页面 Node、导航和新窗口。

状态：idle → checking → available/ready/failed；available → downloading → verifying → installing → saving_state → completed；下载和校验可取消；安装保存不可取消。失败/取消保留原计划，retry 不访问最新计划。先全部下载校验，再应用名单，最后原子保存 state；只读 state 已等于目标而完成标记未写时恢复完成，不重装。Main 退出先 shutdown 再关 stdin；意外断管时 Go 取消下载或等安装完成后释放文件锁。

本地 `.updater/state.json` 为 `{installed_release_id}`；preferences 为 `{background_enabled}`；task 为 `{schema_version:2,task_id,installed_release_id,plan,phase}`，兼容旧 schema 1。不存在有效版本记录时拒绝猜测。没有协议生成器。

## 下载设置 RPC 补充

`set_download_settings` 请求体为可选字段：max_concurrent_downloads、check_timeout_seconds、download_timeout_seconds、max_attempts、speed_limit_kib及proxies映射（modrinth/curseforge/github/server）。空对象恢复默认，显式空代理强制直连。保存整个覆盖对象，缺失字段继承内嵌默认；返回Snapshot。Snapshot新增download和download_defaults，preferences新增download覆盖。活动任务保持启动时设置，重试仍使用原计划。字段含义和边界见CONFIG.md及下载设置方案。
