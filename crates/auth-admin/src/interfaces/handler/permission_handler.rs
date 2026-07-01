//! 权限管理 Handler — CRUD

use salvo::prelude::*;
use salvo::oapi::extract::*;

use genies::core::RespVO;

use crate::application::service::PermissionAppService;
use crate::domain::entity::AdminPermission;

/// 权限管理路由
pub fn routes() -> Router {
    Router::with_path("/permissions")
        .get(list_permissions)
        .post(create_permission)
        .push(
            Router::with_path("{id}")
                .get(get_permission)
                .put(update_permission)
                .delete(delete_permission)
        )
}

/// GET /permissions — 权限列表
#[endpoint(tags("permissions"), summary = "获取权限列表")]
pub async fn list_permissions() -> Json<RespVO<Vec<AdminPermission>>> {
    match PermissionAppService::list_all().await {
        Ok(perms) => Json(RespVO::from(&perms)),
        Err(msg) => Json(RespVO::<Vec<AdminPermission>>::from_error_info("-1", &msg)),
    }
}

/// POST /permissions — 创建权限
#[endpoint(tags("permissions"), summary = "创建权限")]
pub async fn create_permission(body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    let input = body.into_inner();
    match PermissionAppService::create(&input).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /permissions/{id} — 权限详情
#[endpoint(tags("permissions"), summary = "获取权限详情")]
pub async fn get_permission(id: PathParam<i64>) -> Json<RespVO<AdminPermission>> {
    match PermissionAppService::get_by_id(id.into_inner()).await {
        Ok(perm) => Json(RespVO::from(&perm)),
        Err(msg) => Json(RespVO::<AdminPermission>::from_error_info("-1", &msg)),
    }
}

/// PUT /permissions/{id} — 更新权限
#[endpoint(tags("permissions"), summary = "更新权限")]
pub async fn update_permission(id: PathParam<i64>, body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    match PermissionAppService::update(id.into_inner(), &body.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// DELETE /permissions/{id} — 删除权限
#[endpoint(tags("permissions"), summary = "删除权限")]
pub async fn delete_permission(id: PathParam<i64>) -> Json<RespVO<()>> {
    match PermissionAppService::delete(id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}
