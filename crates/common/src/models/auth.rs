use super::*;

/// 账号启用状态与完整角色列表，仅超管可提交
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct UpdateAccountRequest {
    pub enabled: bool,
    pub roles: Vec<String>,
}

/// 内置角色和当前有效权限编码
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Role {
    pub code: String,
    pub name: String,
    pub permissions: Vec<String>,
}

/// 可授予角色的固定权限及其中文显示名
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Permission {
    pub code: String,
    pub name: String,
}

/// 替换角色的完整权限列表
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct UpdateRolePermissionsRequest {
    pub permissions: Vec<String>,
}

/// 密码仅用于认证，不派生 Debug 以防误写日志
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    pub username: String,
    /// 按 UTF-8 字节限制长度，禁止记录或返回
    pub password: String,
}

/// 短期访问令牌，前端仅在内存保存
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    #[ts(type = "number")]
    pub expires_in: i64,
}

/// 实时账号和权限视图，不包含密码哈希与 Token 撤销版本
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub enabled: bool,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// 持久化系统设置，注册判断仍需在写事务中读取
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SystemSettings {
    /// 在数据库写事务内判定是否允许创建普通账号
    pub registration_enabled: bool,
}
