//! 系统设置实体

use rbatis::crud;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 系统设置 key-value 实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSetting {
    pub id: Option<i64>,
    pub setting_key: String,
    pub setting_value: Value,
    pub description: String,
    pub created_at: Option<rbdc::DateTime>,
    pub updated_at: Option<rbdc::DateTime>,
}

crud!(AdminSetting {}, "auth_admin_settings");
