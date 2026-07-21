//! BaseInfo (User) 微服务远程调用
//!
//! 对应 Java: me.tdcarefor.tdnis.user.remote.UserInfo (FeignClient)
//! 服务路由前缀: /user

use genies_derive::remote;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// User/BaseInfo 微服务基础路径
pub static USER: Lazy<String> = genies::config_gateway!("/user");

// ============================================================
// 返回结构体
// ============================================================

/// 用户管床模型（远程调用返回）
///
/// 对应 Java: me.tdcarefor.tdnis.user.model.UserManageBedsModel
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserManageBedsModel {
    pub id: Option<String>,
    pub user_id: Option<String>,
    pub sickbed_id: Option<String>,
    pub sickbed_no: Option<String>,
}

// ============================================================
// 远程调用函数
// ============================================================

// --- 获取用户管理的床位列表 ---
// 对应 Java: UserInfo.getUserManageBedsEntities(userId)
// GET /usermanagebeds/findbyuser?user_id=xxx

/// 获取用户管理的床位列表（自动 token 管理）
#[remote]
#[get(url = USER, path = "/usermanagebeds/findbyuser")]
pub async fn get_user_manage_beds(
    #[query] user_id: &str,
) -> feignhttp::Result<Vec<UserManageBedsModel>> { impled!() }
