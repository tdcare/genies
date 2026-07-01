//! 角色实体与角色-权限关联实体

use rbatis::crud;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

// ============================================================================
// 角色
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminRole {
    pub id: Option<i64>,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub parent_id: Option<i64>,
    pub status: i8,
    pub created_at: Option<rbdc::DateTime>,
    pub updated_at: Option<rbdc::DateTime>,
}

crud!(AdminRole {}, "auth_admin_roles");

// ============================================================================
// 角色-权限关联
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePermission {
    pub id: Option<i64>,
    pub role_id: i64,
    pub permission_id: i64,
}

crud!(RolePermission {}, "auth_admin_role_permissions");
