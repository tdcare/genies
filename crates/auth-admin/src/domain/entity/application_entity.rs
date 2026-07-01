//! 应用实体

use rbatis::crud;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApplicationEntity {
    pub id: Option<i64>,
    pub app_name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub base_url: Option<String>,
    pub status: Option<i8>,
    pub created_at: Option<rbdc::DateTime>,
    pub updated_at: Option<rbdc::DateTime>,
}

crud!(ApplicationEntity {}, "auth_applications");
