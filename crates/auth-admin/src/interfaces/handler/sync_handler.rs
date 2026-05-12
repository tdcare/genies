//! 数据同步 Handler — 供其他微服务启动时拉取数据

use salvo::prelude::*;

use genies::core::RespVO;

use crate::application::service::SyncAppService;
use crate::domain::entity::UserRoleMapping;

/// 内部路由（服务间调用）
pub fn internal_routes() -> Router {
    Router::new()
        .push(Router::with_path("/sync/user-roles").get(list_user_roles))
}

/// GET /sync/user-roles — 导出所有启用状态的用户-角色映射（casbin 'g' 规则）
#[endpoint(tags("sync"), summary = "导出用户-角色映射")]
pub async fn list_user_roles() -> Json<RespVO<Vec<UserRoleMapping>>> {
    match SyncAppService::list_active_user_roles().await {
        Ok(data) => Json(RespVO::from(&data)),
        Err(msg) => Json(RespVO::<Vec<UserRoleMapping>>::from_error_info("-1", &msg)),
    }
}
