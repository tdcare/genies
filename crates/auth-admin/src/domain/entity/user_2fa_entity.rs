//! 用户双因素认证实体

use rbatis::crud;
use serde::{Deserialize, Serialize};

/// 用户 2FA 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTwoFactor {
    pub id: Option<i64>,
    pub user_id: i64,
    pub method: String,
    pub enabled: i8,
    pub secret: String,
    pub phone: String,
    pub backup_codes: Option<String>,
    pub created_at: Option<rbdc::DateTime>,
    pub updated_at: Option<rbdc::DateTime>,
}

crud!(UserTwoFactor {}, "auth_admin_user_2fa");
