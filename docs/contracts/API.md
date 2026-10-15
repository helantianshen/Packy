# 接口契约

核验日期：2026-10-15。以下接口已由 Rust 实现。当前字段以 crates/common/src/models 与生成的 web/shared/generated 为准；[旧 JSON 字段](FIELDS.md) 仅供迁移追溯。运行验证范围见 [验证记录](../VERIFICATION.md)。

## 1. 公共规则

HTTP JSON 使用 snake_case、UTF-8；成功体直接返回资源，不包 data。列表为 Page<T>：items、total、page、page_size；page 默认 1，page_size 默认 20、最大 100。集合必须返回 []，可空标量返回 null。未知请求字段返回 400；注册体仅 username/password，不接收角色字段。

资源 ID 为服务端生成的 UUID 字符串，业务 modid 大小写敏感且唯一，名称不唯一；ModID/mod_id 是稳定内部 ID。日期 RFC3339 UTC。文件大小为非负 int64 或 null，SHA-512 为 128 位小写十六进制，来源优先级为非负整数（小值先用）；文件来源类型仅四个内置编码。JSON 路径用 /，相对游戏实例目录；合法性检查必须兼容 Windows，不能只拼路径。

API 基址 /api/v1。管理请求 Authorization: Bearer <JWT>。通用错误体 APIError：code、message、request_id；内部栈与凭据不返回。

| 状态 | code |
|---|---|
| 400 | INVALID_REQUEST |
| 401 | UNAUTHENTICATED |
| 403 | FORBIDDEN、REGISTRATION_CLOSED |
| 404 | NOT_FOUND |
| 409 | CONFLICT、NO_PUBLISHED_RELEASE |
| 429 | RATE_LIMITED |
| 502 / 503 | UPSTREAM_UNAVAILABLE、SOURCE_UNAVAILABLE |
| 500 | INTERNAL_ERROR |

/healthz 表示进程存活，不代表数据库就绪或业务可用。

## 2. 公开与认证接口

| 方法与路径 | 请求 | 成功响应 | 授权 |
|---|---|---|---|
| GET /healthz | 无 | 200 Health | 公开 |
| POST /api/v1/packs/{pack_id}/update-plan | UpdatePlanRequest | 200 UpdatePlan | 公开 |
| GET /api/v1/files/{file_id}/download | 无 | 200/206 JAR 二进制 | 公开，仅已有托管文件标识 |
| GET /api/v1/auth/registration-status | 无 | 200 SystemSettings | 公开，只返回开关 |
| POST /api/v1/auth/register | Credentials | 201 Account | 公开；事务内检查开关，只创建 user |
| POST /api/v1/auth/login | Credentials | 200 LoginResponse | 公开 |
| POST /api/v1/auth/logout | 无 | 204 | 已登录；增加账号 token_version |
| GET /api/v1/auth/me | 无 | 200 Account | 已登录，返回当前 RBAC |

账号 username：3—32 个 ASCII 字母、数字、下划线或短横线，唯一性不区分 ASCII 大小写；password 为 8—72 UTF-8 字节。账号 ID 不由注册请求指定。JWT 采用 HS256，至少 32 随机字节密钥，issuer=packy、audience=packy-admin，TTL=1800 秒，无 refresh token。声明 sub、iss、aud、iat、exp、jti、token_version；只接受 HS256，校验账号启用与 token_version。登录失败统一 401，不暴露账号是否存在。注销使该账号已签发 Token 全部失效；权限从数据库实时读取，不能从旧 Token 角色字段授权。

更新请求不能缺已安装 release ID；未知或跨 pack 的 ID 返回 NOT_FOUND，无正式版返回 NO_PUBLISHED_RELEASE。本地已最新版返回 operations: []。客户端不向服务端上报执行进度，也不在重试期间刷新计划。

文件下载支持 Range 是文件服务行为，不表示客户端采用分段并发；客户端使用文件级并发，不使用 Range 分段下载。上传请求为 multipart/form-data，字段 file，仅 JAR，上限 512 MiB；服务端流式计算 SHA-512。普通用户不需要管理员 JWT 下载整合包文件。

## 3. 管理接口与权限映射

下列路径均以 /api/v1/admin 为前缀。所有 ID 从 URL 取得，不通过 body 覆盖身份；PUT 表示完整替换所列配置，PATCH /mods 表示完整提交 ModInput 的可编辑字段，未提供可空平台 ID 即清空。

| 方法 / 相对路径 | 请求 DTO | 成功响应 | 权限 |
|---|---|---|---|
| GET /packs | 分页 | 200 Page<Pack> | pack:read |
| POST /packs | PackInput | 201 Pack | pack:write |
| GET /packs/{id} | 无 | 200 Pack | pack:read |
| PUT /packs/{id} | PackInput | 200 Pack | pack:write |
| GET /mods | 分页、keyword | 200 Page<Mod> | mod:read |
| POST /mods | ModInput | 201 Mod | mod:write |
| PATCH /mods/{id} | ModInput | 200 Mod | mod:write |
| GET /artifacts | 分页、mod_id | 200 Page<Artifact> | mod:read |
| POST /artifacts | ArtifactInput | 201 Artifact | mod:write |
| POST /files | multipart file | 201 HostedFile | file:write |
| GET /packs/{id}/releases | 分页 | 200 Page<Release>，列表 items=[] | release:read |
| GET /packs/{id}/releases/{release_id} | 无 | 200 Release 完整 items | release:read |
| POST /packs/{id}/drafts | CreateDraftRequest | 201 Release | release:write |
| PUT /packs/{id}/drafts/{release_id} | UpdateDraftRequest | 200 Release，新 revision | release:write |
| POST /packs/{id}/drafts/{release_id}/preview | RevisionRequest | 200 UpdatePlan | release:read |
| POST /packs/{id}/drafts/{release_id}/publish | RevisionRequest | 200 Release | release:publish |
| GET /system/download-source-types | 无 | 200 SourceDictionaryItem[] | source:read |
| PUT /system/download-source-types/{code} | name、enabled、sort_order | 200 SourceDictionaryItem | source:write |
| GET /system/settings | 无 | 200 SystemSettings | settings:read |
| PUT /system/settings | SystemSettings | 200 SystemSettings | settings:write |
| GET /system/users | 分页 | 200 Page<Account> | user:read |
| PATCH /system/users/{id} | UpdateAccountRequest | 200 Account | user:write |
| GET /system/roles | 无 | 200 Role[] | permission:read |
| GET /system/permissions | 无 | 200 Permission[] | permission:read |
| PUT /system/roles/{code}/permissions | UpdateRolePermissionsRequest | 200 Role | permission:write |

不提供新角色创建、用户密码重置、正式版编辑/删除接口。普通用户默认没有上表权限；超管初始拥有全部权限。系统用户、权限与注册设置管理仅 super_admin 可调用，不能仅通过给 user 增加同名管理权限绕过此角色边界。至少保留一名启用超管以及其系统管理权限；降低最后一个超管或破坏超管管理权限返回 CONFLICT。其余业务权限可通过 RBAC 配置给普通用户。

CreateDraftRequest.base_release_id=null 时从当前正式版复制，无正式版则从空清单创建；指定值必须属于同包。更新草稿携带旧 revision，服务端原子比较并递增，不匹配返回 CONFLICT。items 是完整清单，删去条目在发布差异中产生 REMOVE。发布时间及快照由服务端写入，客户端不提交。Artifact 来源与哈希发布后保持不可变。

## 4. Modrinth 接入接口

前缀 /api/v1/admin/modrinth，全部要求 mod:read：

- GET /search：query、minecraft_version、loader、offset（默认 0）、limit（默认 20、最大 100）。
- GET /projects/{project_id}。
- GET /projects/{project_id}/versions：minecraft_version、loader。
- GET /versions/{version_id}。
- POST /version-files：{"hashes":["sha512"],"algorithm":"sha512"}，最多 100 个哈希。

第一版返回对应上游 JSON，不把它与自己的业务 Mod DTO 混用；由服务端限制参数、隐藏 API 凭据和映射上游异常。CurseForge/GitHub 暂仅录入实际文件来源，不开放完整元数据同步 API。

## 5. 更新计划示例

```json
{
  "pack_id": "11111111-1111-4111-8111-111111111111",
  "pack_name": "Packy Demo",
  "target_release_id": "22222222-2222-4222-8222-222222222222",
  "target_version": "1.1.0",
  "changelog": "更新示例",
  "operations": [
    {
      "action": "REPLACE",
      "mod_id": "33333333-3333-4333-8333-333333333333",
      "old_path": "mods/demo-old.jar",
      "target": {
        "path": "mods/demo-new.jar",
        "size": null,
        "sha512": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "sources": [{"type":"github","url":"https://example.invalid/demo-new.jar","priority":0}]
      }
    }
  ]
}
```

示例仅说明结构，地址与哈希不可作为真实下载任务。ADD 必须有 target、不得有 old_path；REPLACE 两者都必须有；REMOVE 必须有 old_path、不得有 target。sources 至少一项，无可用来源返回 SOURCE_UNAVAILABLE。名单外路径不处理。

## 6. Tauri 2 commands 与事件

get_probe_info 返回 ProbeInfo 用于诊断；实际桌面通过以下 commands 调用 Rust 核心：

| 方法 | 参数 | 返回/效果 |
|---|---|---|
| get_status | 无 | ClientSnapshot，包含 config/preferences/state/status/plan |
| check_update | 无 | ClientSnapshot，包含检查结果；pack/server 来自已校验的本地配置 |
| start_update | 无 | task_id；执行核心当前持有且展示的计划 |
| retry_update | task_id | 同任务 task_id，不刷新计划 |
| cancel_update | task_id | 仅检查/下载/校验阶段可取消 |
| launch_game | 无 | 仅调用已校验的 launcher_path |
| set_background_enabled | enabled: boolean | 原子保存当前整合包的后台运行偏好，返回最新 ClientSnapshot；保存失败不改变原行为 |

事件 packy:update-status 使用 TaskStatus：sequence 在同一核心实例内单调递增，允许合并中间事件；普通进度最多 10 次/秒，终态立即发。阶段固定 idle/checking/available/ready/downloading/verifying/installing/saving_state/completed/failed/cancelled；不以事件到达证明业务已完成，get_status 为权威快照。前端不传文件路径、URL 或任意操作列表控制核心。

## HTTP/HTTPS 部署与联调补充（2026-10-15）

用户确认客户端支持 HTTP/HTTPS 服务地址和下载链接。Rust 服务端只监听 HTTP，不配置 TLS；自部署由 Nginx 终止 HTTPS 并转发 HTTP。PACKY_PUBLIC_URL 使用客户端实际可访问的 HTTP/HTTPS 外部地址，不能使用内部反代地址生成下载链接。本地联调用 HTTP。HTTPS 链接的重定向不允许降级为 HTTP；初始 HTTP 链接可正常使用。完成客户端和服务端后执行实际联调，检查计划、下载、校验、安装、失败重试与版本落盘，不能仅以单元测试代替。
