# JSON 字段契约

从原 P1 DTO 提取，作为 Rust 重建时的字段基线；不代表接口已实现。nullable 表示允许 null，optional 表示字段可省略。


## Pack

| JSON 字段 | 类型 |
|---|---|
| id | string |
| name | string |
| minecraft_version | string |
| loader | string |
| current_release_id | nullable string |

## PackInput

| JSON 字段 | 类型 |
|---|---|
| name | string |
| minecraft_version | string |
| loader | string |

## Artifact

| JSON 字段 | 类型 |
|---|---|
| id | string |
| mod_id | string |
| version | string |
| filename | string |
| size | nullable integer (64-bit) |
| sha512 | string |
| hosted_file_id | nullable string |
| sources | array of DownloadSource |

## ArtifactInput

| JSON 字段 | 类型 |
|---|---|
| mod_id | string |
| version | string |
| filename | string |
| size | nullable integer (64-bit) |
| sha512 | string |
| hosted_file_id | nullable string |
| sources | array of DownloadSource |

## Release

| JSON 字段 | 类型 |
|---|---|
| id | string |
| pack_id | string |
| version | string |
| changelog | string |
| minecraft_version | string |
| loader | string |
| status | string |
| revision | int |
| base_release_id | nullable string |
| published_at | nullable RFC3339 UTC |
| items | array of ReleaseItem |

## ReleaseItem

| JSON 字段 | 类型 |
|---|---|
| mod_id | string |
| artifact_id | string |
| modid_snapshot | string |
| name_snapshot | string |
| target | TargetFile |

## CreateDraftRequest

| JSON 字段 | 类型 |
|---|---|
| version | string |
| changelog | string |
| base_release_id | nullable string |

## DraftItemInput

| JSON 字段 | 类型 |
|---|---|
| mod_id | string |
| artifact_id | string |
| target_path | string |

## UpdateDraftRequest

| JSON 字段 | 类型 |
|---|---|
| revision | int |
| version | string |
| changelog | string |
| items | array of DraftItemInput |

## RevisionRequest

| JSON 字段 | 类型 |
|---|---|
| revision | int |

## UpdateAccountRequest

| JSON 字段 | 类型 |
|---|---|
| enabled | boolean |
| roles | array of string |

## Role

| JSON 字段 | 类型 |
|---|---|
| code | string |
| name | string |
| permissions | array of string |

## Permission

| JSON 字段 | 类型 |
|---|---|
| code | string |
| name | string |

## UpdateRolePermissionsRequest

| JSON 字段 | 类型 |
|---|---|
| permissions | array of string |

## HostedFile

| JSON 字段 | 类型 |
|---|---|
| id | string |
| filename | string |
| size | integer (64-bit) |
| sha512 | string |
| url | string |

## Page

| JSON 字段 | 类型 |
|---|---|
| items | array of T |
| total | integer (64-bit) |
| page | int |
| page_size | int |

## DownloadSource

| JSON 字段 | 类型 |
|---|---|
| type | SourceType |
| url | string |
| priority | int |

## TargetFile

| JSON 字段 | 类型 |
|---|---|
| path | string |
| size | nullable integer (64-bit) |
| sha512 | string |
| sources | array of DownloadSource |

## Operation

| JSON 字段 | 类型 |
|---|---|
| action | Action |
| mod_id | string |
| old_path | string，optional |
| target | nullable TargetFile，optional |

## UpdatePlanRequest

| JSON 字段 | 类型 |
|---|---|
| installed_release_id | string |
| client_version | string |

## UpdatePlan

| JSON 字段 | 类型 |
|---|---|
| pack_id | string |
| pack_name | string |
| target_release_id | string |
| target_version | string |
| changelog | string |
| operations | array of Operation |

## APIError

| JSON 字段 | 类型 |
|---|---|
| code | string |
| message | string |
| request_id | string |

## Credentials

| JSON 字段 | 类型 |
|---|---|
| username | string |
| password | string |

## LoginResponse

| JSON 字段 | 类型 |
|---|---|
| access_token | string |
| token_type | string |
| expires_in | int |

## Account

| JSON 字段 | 类型 |
|---|---|
| id | string |
| username | string |
| enabled | boolean |
| roles | array of string |
| permissions | array of string |

## SystemSettings

| JSON 字段 | 类型 |
|---|---|
| registration_enabled | boolean |

## SourceDictionaryItem

| JSON 字段 | 类型 |
|---|---|
| code | SourceType |
| name | string |
| enabled | boolean |
| sort_order | int |

## Mod

| JSON 字段 | 类型 |
|---|---|
| id | string |
| modid | string |
| name | string |
| modrinth_project_id | nullable string |
| curseforge_project_id | nullable string |

## ModInput

| JSON 字段 | 类型 |
|---|---|
| modid | string |
| name | string |
| modrinth_project_id | nullable string |
| curseforge_project_id | nullable string |

## LocalConfig

| JSON 字段 | 类型 |
|---|---|
| pack_id | string |
| server_url | string |
| game_dir | string |
| launcher_path | string |

## LocalState

| JSON 字段 | 类型 |
|---|---|
| installed_release_id | string |

## TaskRecord

| JSON 字段 | 类型 |
|---|---|
| schema_version | int |
| task_id | string |
| installed_release_id | string |
| plan | UpdatePlan |
| phase | string |

## TaskStatus

| JSON 字段 | 类型 |
|---|---|
| task_id | string |
| sequence | uinteger (64-bit) |
| phase | string |
| completed_files | int |
| total_files | int |
| downloaded_bytes | integer (64-bit) |
| total_bytes | nullable integer (64-bit) |
| bytes_per_second | integer (64-bit) |
| error | nullable APIError，optional |

## Health

| JSON 字段 | 类型 |
|---|---|
| status | string |
| version | string |
| phase | string |

## ProbeInfo

| JSON 字段 | 类型 |
|---|---|
| name | string |
| version | string |
| phase | string |
| business_enabled | boolean |
| checked_at | RFC3339 UTC |
