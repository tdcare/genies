//! 组织架构（部门）管理 Handler — CRUD + 移动

use salvo::prelude::*;
use salvo::oapi::extract::*;

use genies::core::RespVO;

use crate::application::dto::AddMemberRequest;
use crate::application::service::{DepartmentAppService, UserDepartmentAppService};
use crate::domain::entity::{AdminDepartment, AdminUser};

/// 部门管理路由
pub fn routes() -> Router {
    Router::with_path("/departments")
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
            Router::with_path("{id}/users")
                .get(get_department_users)
                .post(add_department_user)
                .push(Router::with_path("{user_id}").delete(remove_department_user))
        )
}

/// GET /departments — 部门列表
#[endpoint(tags("departments"), summary = "获取部门列表")]
pub async fn list_departments() -> Json<RespVO<Vec<AdminDepartment>>> {
    match DepartmentAppService::list_all().await {
        Ok(depts) => Json(RespVO::from(&depts)),
        Err(msg) => Json(RespVO::<Vec<AdminDepartment>>::from_error_info("-1", &msg)),
    }
}

/// POST /departments — 创建部门
#[endpoint(tags("departments"), summary = "创建部门")]
pub async fn create_department(body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    let input = body.into_inner();
    match DepartmentAppService::create(&input).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /departments/{id} — 部门详情
#[endpoint(tags("departments"), summary = "获取部门详情")]
pub async fn get_department(id: PathParam<i64>) -> Json<RespVO<AdminDepartment>> {
    match DepartmentAppService::get_by_id(id.into_inner()).await {
        Ok(dept) => Json(RespVO::from(&dept)),
        Err(msg) => Json(RespVO::<AdminDepartment>::from_error_info("-1", &msg)),
    }
}

/// PUT /departments/{id} — 更新部门
#[endpoint(tags("departments"), summary = "更新部门")]
pub async fn update_department(id: PathParam<i64>, body: JsonBody<serde_json::Value>) -> Json<RespVO<()>> {
    match DepartmentAppService::update(id.into_inner(), &body.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// DELETE /departments/{id} — 删除部门
#[endpoint(tags("departments"), summary = "删除部门")]
pub async fn delete_department(id: PathParam<i64>) -> Json<RespVO<()>> {
    match DepartmentAppService::delete(id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// PUT /departments/{id}/move/{parent_id} — 移动部门
#[endpoint(tags("departments"), summary = "移动部门到新的父部门")]
pub async fn move_department(id: PathParam<i64>, parent_id: PathParam<i64>) -> Json<RespVO<()>> {
    match DepartmentAppService::move_dept(id.into_inner(), parent_id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /departments/{id}/users — 部门成员列表
#[endpoint(tags("departments"), summary = "获取部门成员列表")]
pub async fn get_department_users(id: PathParam<i64>) -> Json<RespVO<Vec<AdminUser>>> {
    match UserDepartmentAppService::get_department_users(id.into_inner()).await {
        Ok(users) => Json(RespVO::from(&users)),
        Err(msg) => Json(RespVO::<Vec<AdminUser>>::from_error_info("-1", &msg)),
    }
}

/// POST /departments/{id}/users — 添加成员到部门
#[endpoint(tags("departments"), summary = "添加部门成员")]
pub async fn add_department_user(id: PathParam<i64>, body: JsonBody<AddMemberRequest>) -> Json<RespVO<()>> {
    let req = body.into_inner();
    match UserDepartmentAppService::add_user_to_department(id.into_inner(), req.user_id).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// DELETE /departments/{id}/users/{user_id} — 从部门移除成员
#[endpoint(tags("departments"), summary = "移除部门成员")]
pub async fn remove_department_user(id: PathParam<i64>, user_id: PathParam<i64>) -> Json<RespVO<()>> {
    match UserDepartmentAppService::remove_user_from_department(id.into_inner(), user_id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}
