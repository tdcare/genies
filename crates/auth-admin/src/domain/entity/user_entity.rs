//! 用户实体与用户-角色关联实体

use rbatis::crud;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

// ============================================================================
// 用户
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminUser {
    pub id: Option<i64>,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub status: i8,
    pub last_login_at: Option<rbdc::DateTime>,
    pub created_at: Option<rbdc::DateTime>,
    pub updated_at: Option<rbdc::DateTime>,
}

crud!(AdminUser {}, "auth_admin_users");

// ============================================================================
// 用户-角色关联
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRole {
    pub id: Option<i64>,
    pub user_id: i64,
    pub role_id: i64,
}

crud!(UserRole {}, "auth_admin_user_roles");

// ============================================================================
// 用户-角色映射（用于 sync 导出）
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserRoleMapping {
    pub ptype: String,
    pub v0: String,
    pub v1: String,
}
