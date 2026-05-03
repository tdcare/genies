//! 组织架构（部门）管理 Handler — CRUD + 移动

use salvo::prelude::*;
use salvo::oapi::extract::*;

use genies::core::RespVO;

use crate::application::service::{DepartmentAppService, UserDepartmentAppService};
use crate::domain::entity::{AdminDepartment, AdminUser};

/// 部门管理路由
pub fn routes() -> Router {
    Router::with_path("/auth-admin/departments")
        .get(list_departments)
        .post(create_department)
        .push(
            Router::with_path("{id}")
                .get(get_department)
                .put(update_department)
                .delete(delete_department)
        )
        .push(
            Router::with_path("{id}/move/{parent_id}").put(move_department)
        )
        .push(
            Router::with_path("{id}/users").get(get_department_users)
        )
}

/// GET /auth-admin/departments — 部门列表
#[endpoint(tags("departments"), summary = "获取部门列表")]
pub async fn list_departments() -> Json<RespVO<Vec<AdminDepartment>>> {
    match DepartmentAppService::list_all().await {
        Ok(depts) => Json(RespVO::from(&depts)),
        Err(msg) => Json(RespVO::<Vec<AdminDepartment>>::from_error_info("-1", &msg)),
    }
}

/// POST /auth-admin/departments — 创建部门
#[endpoint(tags("departments"), summary = "创建部门")]
pub async fn create_department(body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    let input = body.into_inner();
    match DepartmentAppService::create(&input).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /auth-admin/departments/{id} — 部门详情
#[endpoint(tags("departments"), summary = "获取部门详情")]
pub async fn get_department(id: PathParam<i64>) -> Json<RespVO<AdminDepartment>> {
    match DepartmentAppService::get_by_id(id.into_inner()).await {
        Ok(dept) => Json(RespVO::from(&dept)),
        Err(msg) => Json(RespVO::<AdminDepartment>::from_error_info("-1", &msg)),
    }
}

/// PUT /auth-admin/departments/{id} — 更新部门
#[endpoint(tags("departments"), summary = "更新部门")]
pub async fn update_department(id: PathParam<i64>, body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    match DepartmentAppService::update(id.into_inner(), &body.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// DELETE /auth-admin/departments/{id} — 删除部门
#[endpoint(tags("departments"), summary = "删除部门")]
pub async fn delete_department(id: PathParam<i64>) -> Json<RespVO<()>> {
    match DepartmentAppService::delete(id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// PUT /auth-admin/departments/{id}/move/{parent_id} — 移动部门
#[endpoint(tags("departments"), summary = "移动部门到新的父部门")]
pub async fn move_department(id: PathParam<i64>, parent_id: PathParam<i64>) -> Json<RespVO<()>> {
    match DepartmentAppService::move_dept(id.into_inner(), parent_id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /auth-admin/departments/{id}/users — 部门成员列表
#[endpoint(tags("departments"), summary = "获取部门成员列表")]
pub async fn get_department_users(id: PathParam<i64>) -> Json<RespVO<Vec<AdminUser>>> {
    match UserDepartmentAppService::get_department_users(id.into_inner()).await {
        Ok(users) => Json(RespVO::from(&users)),
        Err(msg) => Json(RespVO::<Vec<AdminUser>>::from_error_info("-1", &msg)),
    }
}
