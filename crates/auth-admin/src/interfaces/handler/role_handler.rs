//! 角色管理 Handler — 完整 CRUD + 权限分配

use salvo::prelude::*;
use salvo::oapi::extract::*;

use genies::core::RespVO;

use crate::application::service::RoleAppService;
use crate::domain::entity::{AdminRole, AdminUser, AdminPermission};

/// 角色管理路由
pub fn routes() -> Router {
    Router::with_path("/auth-admin/roles")
        .get(list_roles)
        .post(create_role)
        .push(
            Router::with_path("{id}")
                .get(get_role)
                .put(update_role)
                .delete(delete_role)
        )
        .push(
            Router::with_path("{id}/users").get(get_role_users)
        )
        .push(
            Router::with_path("{id}/permissions")
                .get(get_role_permissions)
                .post(assign_role_permission)
                .push(
                    Router::with_path("{perm_id}").delete(revoke_role_permission)
                )
        )
}

/// GET /auth-admin/roles — 角色列表
#[endpoint(tags("roles"), summary = "获取角色列表")]
pub async fn list_roles() -> Json<RespVO<Vec<AdminRole>>> {
    match RoleAppService::list_all().await {
        Ok(roles) => Json(RespVO::from(&roles)),
        Err(msg) => Json(RespVO::<Vec<AdminRole>>::from_error_info("-1", &msg)),
    }
}

/// POST /auth-admin/roles — 创建角色
#[endpoint(tags("roles"), summary = "创建角色")]
pub async fn create_role(body: JsonBody<serde_json::Value>) -> Json<RespVO<serde_json::Value>> {
    let input = body.into_inner();
    match RoleAppService::create(&input).await {
        Ok(data) => Json(RespVO::from(&data)),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /auth-admin/roles/{id} — 角色详情
#[endpoint(tags("roles"), summary = "获取角色详情")]
pub async fn get_role(id: PathParam<i64>) -> Json<RespVO<AdminRole>> {
    match RoleAppService::get_by_id(id.into_inner()).await {
        Ok(role) => Json(RespVO::from(&role)),
        Err(msg) => Json(RespVO::<AdminRole>::from_error_info("-1", &msg)),
    }
}

/// PUT /auth-admin/roles/{id} — 更新角色
#[endpoint(tags("roles"), summary = "更新角色")]
pub async fn update_role(id: PathParam<i64>, body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    match RoleAppService::update(id.into_inner(), &body.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// DELETE /auth-admin/roles/{id} — 删除角色
#[endpoint(tags("roles"), summary = "删除角色")]
pub async fn delete_role(id: PathParam<i64>) -> Json<RespVO<()>> {
    match RoleAppService::delete(id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /auth-admin/roles/{id}/users — 角色下用户列表
#[endpoint(tags("roles"), summary = "获取角色下用户列表")]
pub async fn get_role_users(id: PathParam<i64>) -> Json<RespVO<Vec<AdminUser>>> {
    match RoleAppService::get_role_users(id.into_inner()).await {
        Ok(users) => Json(RespVO::from(&users)),
        Err(msg) => Json(RespVO::<Vec<AdminUser>>::from_error_info("-1", &msg)),
    }
}

/// GET /auth-admin/roles/{id}/permissions — 角色权限列表
#[endpoint(tags("roles"), summary = "获取角色权限列表")]
pub async fn get_role_permissions(id: PathParam<i64>) -> Json<RespVO<Vec<AdminPermission>>> {
    match RoleAppService::get_role_permissions(id.into_inner()).await {
        Ok(perms) => Json(RespVO::from(&perms)),
        Err(msg) => Json(RespVO::<Vec<AdminPermission>>::from_error_info("-1", &msg)),
    }
}

/// POST /auth-admin/roles/{id}/permissions — 授予权限
#[endpoint(tags("roles"), summary = "给角色授予权限")]
pub async fn assign_role_permission(id: PathParam<i64>, body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    let input = body.into_inner();
    let perm_id: i64 = match input["permission_id"].as_i64() {
        Some(id) => id,
        None => {
            return Json(RespVO::from_error_info("-1", "缺少permission_id"));
        }
    };

    match RoleAppService::assign_permission(id.into_inner(), perm_id).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// DELETE /auth-admin/roles/{id}/permissions/{perm_id} — 撤销权限
#[endpoint(tags("roles"), summary = "撤销角色权限")]
pub async fn revoke_role_permission(id: PathParam<i64>, perm_id: PathParam<i64>) -> Json<RespVO<()>> {
    match RoleAppService::revoke_permission(id.into_inner(), perm_id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}
