//! 用户-部门关联实体

use rbatis::crud;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserDepartment {
    pub id: Option<i64>,
    pub user_id: i64,
    pub department_id: i64,
    pub created_at: Option<rbdc::DateTime>,
}

crud!(UserDepartment {}, "auth_admin_user_departments");
