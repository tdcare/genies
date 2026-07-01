//! 权限实体

use rbatis::crud;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminPermission {
    pub id: Option<i64>,
    pub name: String,
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
    pub status: i8,
    pub created_at: Option<rbdc::DateTime>,
    pub updated_at: Option<rbdc::DateTime>,
}

crud!(AdminPermission{}, "auth_admin_permissions");
