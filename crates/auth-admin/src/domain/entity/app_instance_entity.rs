//! 应用实例实体

use rbatis::crud;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AppInstanceEntity {
    pub id: Option<i64>,
    pub app_name: Option<String>,
    pub instance_id: Option<i64>,
    pub base_url: Option<String>,
    pub version: Option<String>,
    pub status: Option<i8>,
    pub last_heartbeat_at: Option<rbdc::DateTime>,
    pub registered_at: Option<rbdc::DateTime>,
    pub metadata: Option<String>,
}

crud!(AppInstanceEntity {}, "auth_app_instances");
