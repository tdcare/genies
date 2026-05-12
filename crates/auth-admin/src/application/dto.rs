//! 请求/响应 DTO 定义

use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

/// 分页查询参数
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct PageQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub keyword: Option<String>,
}

/// 分页结果
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PageResult<T: Serialize> {
    pub total: u64,
    pub page: u64,
    pub size: u64,
    pub list: Vec<T>,
}

/// 登录请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: usize,
    pub username: String,
    pub display_name: String,
}

/// 修改密码请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// 添加部门成员请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddMemberRequest {
    pub user_id: i64,
}
