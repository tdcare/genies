//! 部门实体

use rbatis::crud;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminDepartment {
    pub id: Option<i64>,
    pub name: String,
    pub parent_id: Option<i64>,
    pub sort_order: i32,
    pub description: Option<String>,
    pub status: i8,
    pub created_at: Option<rbdc::DateTime>,
    pub updated_at: Option<rbdc::DateTime>,
    /// 部门成员数量（不映射到数据库列，仅在查询时填充）
    #[serde(default)]
    pub member_count: Option<i64>,
}

crud!(AdminDepartment{}, "auth_admin_departments");
