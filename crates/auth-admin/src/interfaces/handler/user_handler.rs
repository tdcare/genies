//! 用户管理 Handler — 完整 CRUD + 角色分配

use salvo::prelude::*;
use salvo::oapi::extract::*;

use genies::core::RespVO;

use crate::application::dto::PageQuery;
use crate::application::service::{UserAppService, UserDepartmentAppService};
use crate::domain::entity::{AdminUser, AdminRole, AdminPermission, AdminDepartment};

/// 用户管理路由
pub fn routes() -> Router {
    Router::with_path("/users")
        .get(list_users)
        .post(create_user)
        .push(
            Router::with_path("{id}")
                .get(get_user)
                .put(update_user)
                .delete(delete_user)
        )
        .push(
            Router::with_path("{id}/status").put(update_status)
        )
        .push(
            Router::with_path("{id}/reset-password").put(reset_password)
        )
        .push(
            Router::with_path("{id}/roles")
                .get(get_user_roles)
                .post(assign_user_role)
                .push(
                    Router::with_path("{role_id}").delete(revoke_user_role)
                )
        )
        .push(
            Router::with_path("{id}/permissions").get(get_user_permissions)
        )
        .push(
            Router::with_path("{id}/departments")
                .get(get_user_departments)
                .post(assign_user_departments)
        )
        .push(
            Router::with_path("batch-delete").post(batch_delete_users)
        )
}

/// GET /users — 分页用户列表
#[endpoint(tags("users"), summary = "分页用户列表")]
pub async fn list_users(
    page: QueryParam<u64, false>,
    size: QueryParam<u64, false>,
    keyword: QueryParam<String, false>,
) -> Json<RespVO<serde_json::Value>> {
    let query = PageQuery {
        page: page.into_inner(),
        size: size.into_inner(),
        keyword: keyword.into_inner(),
    };

    match UserAppService::list(&query).await {
        Ok(data) => Json(RespVO::from(&data)),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// POST /users — 创建用户
#[endpoint(tags("users"), summary = "创建用户")]
pub async fn create_user(body: JsonBody<serde_json::Value>) -> Json<RespVO<serde_json::Value>> {
    let input = body.into_inner();
    match UserAppService::create(&input).await {
        Ok(data) => Json(RespVO::from(&data)),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /users/{id} — 用户详情
#[endpoint(tags("users"), summary = "获取用户详情")]
pub async fn get_user(id: PathParam<i64>) -> Json<RespVO<AdminUser>> {
    match UserAppService::get_by_id(id.into_inner()).await {
        Ok(user) => Json(RespVO::from(&user)),
        Err(msg) => Json(RespVO::<AdminUser>::from_error_info("-1", &msg)),
    }
}

/// PUT /users/{id} — 更新用户
#[endpoint(tags("users"), summary = "更新用户")]
pub async fn update_user(id: PathParam<i64>, body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    match UserAppService::update(id.into_inner(), &body.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// DELETE /users/{id} — 删除用户
#[endpoint(tags("users"), summary = "删除用户")]
pub async fn delete_user(id: PathParam<i64>) -> Json<RespVO<()>> {
    match UserAppService::delete(id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// POST /users/batch-delete — 批量删除
#[endpoint(tags("users"), summary = "批量删除用户")]
pub async fn batch_delete_users(body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    let input = body.into_inner();
    let ids: Vec<i64> = match input["ids"].as_array() {
        Some(arr) => arr.iter().filter_map(|v| v.as_i64()).collect(),
        None => {
            return Json(RespVO::from_error_info("-1", "缺少ids参数"));
        }
    };

    match UserAppService::batch_delete(&ids).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// PUT /users/{id}/status — 启用/禁用
#[endpoint(tags("users"), summary = "更新用户状态")]
pub async fn update_status(id: PathParam<i64>, body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    let input = body.into_inner();
    let status: i8 = input["status"].as_i64().unwrap_or(1) as i8;
    match UserAppService::update_status(id.into_inner(), status).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// PUT /users/{id}/reset-password — 重置密码
#[endpoint(tags("users"), summary = "重置用户密码")]
pub async fn reset_password(id: PathParam<i64>, body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    let input = body.into_inner();
    let new_password = input["password"].as_str().unwrap_or("123456");
    match UserAppService::reset_password(id.into_inner(), new_password).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /users/{id}/roles — 获取用户角色
#[endpoint(tags("users"), summary = "获取用户角色列表")]
pub async fn get_user_roles(id: PathParam<i64>) -> Json<RespVO<Vec<AdminRole>>> {
    match UserAppService::get_user_roles(id.into_inner()).await {
        Ok(roles) => Json(RespVO::from(&roles)),
        Err(msg) => Json(RespVO::<Vec<AdminRole>>::from_error_info("-1", &msg)),
    }
}

/// POST /users/{id}/roles — 分配角色
#[endpoint(tags("users"), summary = "分配用户角色")]
pub async fn assign_user_role(id: PathParam<i64>, body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    let input = body.into_inner();
    let role_id: i64 = match input["role_id"].as_i64() {
        Some(id) => id,
        None => {
            return Json(RespVO::from_error_info("-1", "缺少role_id"));
        }
    };

    match UserAppService::assign_role(id.into_inner(), role_id).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// DELETE /users/{id}/roles/{role_id} — 移除角色
#[endpoint(tags("users"), summary = "移除用户角色")]
pub async fn revoke_user_role(id: PathParam<i64>, role_id: PathParam<i64>) -> Json<RespVO<()>> {
    match UserAppService::revoke_role(id.into_inner(), role_id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /users/{id}/permissions — 用户有效权限（合并所有角色）
#[endpoint(tags("users"), summary = "获取用户有效权限")]
pub async fn get_user_permissions(id: PathParam<i64>) -> Json<RespVO<Vec<AdminPermission>>> {
    match UserAppService::get_user_permissions(id.into_inner()).await {
        Ok(perms) => Json(RespVO::from(&perms)),
        Err(msg) => Json(RespVO::<Vec<AdminPermission>>::from_error_info("-1", &msg)),
    }
}

/// GET /users/{id}/departments — 获取用户部门列表
#[endpoint(tags("users"), summary = "获取用户部门列表")]
pub async fn get_user_departments(id: PathParam<i64>) -> Json<RespVO<Vec<AdminDepartment>>> {
    match UserDepartmentAppService::get_user_departments(id.into_inner()).await {
        Ok(depts) => Json(RespVO::from(&depts)),
        Err(msg) => Json(RespVO::<Vec<AdminDepartment>>::from_error_info("-1", &msg)),
    }
}

/// POST /users/{id}/departments — 分配用户部门
#[endpoint(tags("users"), summary = "分配用户部门")]
pub async fn assign_user_departments(id: PathParam<i64>, body: JsonBody<Vec<i64>>) -> Json<RespVO<()>> {
    match UserDepartmentAppService::assign_departments(id.into_inner(), body.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}
