//! 应用管理 Handler — CRUD

use std::sync::Arc;

use salvo::prelude::*;
use salvo::oapi::extract::*;

use genies::core::RespVO;

use crate::application::app_service::ApplicationAppService;
use crate::application::dto::PageResult;
use crate::interfaces::dto::application_dto::{
    ApplicationVO, CreateApplicationRequest, UpdateApplicationRequest,
};

/// 应用管理路由
pub fn routes() -> Router {
    Router::with_path("/auth-admin/apps")
        .get(list_apps)
        .post(create_app)
        .push(
            Router::with_path("{id}")
                .get(get_app)
                .put(update_app)
                .delete(delete_app)
        )
}

/// 从 Depot 中获取 casbin enforcer 和 subject，对 JSON Value 进行字段级权限过滤
fn apply_casbin_filter(depot: &Depot, value: &mut serde_json::Value) {
    let enforcer = depot
        .get::<Arc<casbin::Enforcer>>("casbin_enforcer")
        .ok()
        .cloned();
    let subject = depot
        .get::<String>("casbin_subject")
        .ok()
        .cloned();

    if let (Some(ref e), Some(ref s)) = (enforcer, subject) {
        // 分页列表中的 list 数组
        if let Some(arr) = value.pointer_mut("/list") {
            if let serde_json::Value::Array(items) = arr {
                for item in items.iter_mut() {
                    ApplicationVO::casbin_filter(item, e.as_ref(), s.as_str());
                }
            }
            return;
        }
        // 单个对象
        ApplicationVO::casbin_filter(value, e.as_ref(), s.as_str());
    }
}

/// GET /auth-admin/apps — 分页应用列表
#[endpoint(tags("apps"), summary = "分页应用列表")]
pub async fn list_apps(
    page: QueryParam<u64, false>,
    size: QueryParam<u64, false>,
    keyword: QueryParam<String, false>,
    depot: &mut Depot,
) -> Json<RespVO<PageResult<ApplicationVO>>> {
    let page = page.into_inner().unwrap_or(1);
    let size = size.into_inner().unwrap_or(10);
    let keyword = keyword.into_inner().unwrap_or_default();

    match ApplicationAppService::list_apps(page, size, &keyword).await {
        Ok(data) => {
            let mut value = serde_json::to_value(&data).unwrap_or_default();
            apply_casbin_filter(depot, &mut value);
            let filtered: PageResult<ApplicationVO> =
                serde_json::from_value(value).unwrap_or(data);
            Json(RespVO::from(&filtered))
        }
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /auth-admin/apps/{id} — 应用详情
#[endpoint(tags("apps"), summary = "获取应用详情")]
pub async fn get_app(id: PathParam<i64>, depot: &mut Depot) -> Json<RespVO<ApplicationVO>> {
    match ApplicationAppService::get_app(id.into_inner()).await {
        Ok(app) => {
            let vo: ApplicationVO = app.into();
            let mut value = serde_json::to_value(&vo).unwrap_or_default();
            apply_casbin_filter(depot, &mut value);
            let filtered: ApplicationVO =
                serde_json::from_value(value).unwrap_or(vo);
            Json(RespVO::from(&filtered))
        }
        Err(msg) => Json(RespVO::<ApplicationVO>::from_error_info("-1", &msg)),
    }
}

/// POST /auth-admin/apps — 创建应用
#[endpoint(tags("apps"), summary = "创建应用")]
pub async fn create_app(body: JsonBody<CreateApplicationRequest>) -> Json<RespVO<serde_json::Value>> {
    let req = body.into_inner();
    match ApplicationAppService::create_app(
        &req.app_name,
        req.display_name.as_deref().unwrap_or(""),
        req.description.as_deref().unwrap_or(""),
        &req.base_url,
        req.status.unwrap_or(1),
    ).await {
        Ok(data) => Json(RespVO::from(&data)),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// PUT /auth-admin/apps/{id} — 更新应用
#[endpoint(tags("apps"), summary = "更新应用")]
pub async fn update_app(id: PathParam<i64>, body: JsonBody<UpdateApplicationRequest>) -> Json<RespVO<()>> {
    let req = body.into_inner();
    let app_id = id.into_inner();

    // 获取已有数据以合并
    let existing = match ApplicationAppService::get_app(app_id).await {
        Ok(a) => a,
        Err(msg) => return Json(RespVO::from_error_info("-1", &msg)),
    };

    let app_name = req.app_name.unwrap_or_else(|| existing.app_name.unwrap_or_default());
    let display_name = req.display_name.unwrap_or_else(|| existing.display_name.unwrap_or_default());
    let description = req.description.unwrap_or_else(|| existing.description.unwrap_or_default());
    let base_url = req.base_url.unwrap_or_else(|| existing.base_url.unwrap_or_default());
    let status = req.status.unwrap_or_else(|| existing.status.unwrap_or(1));

    match ApplicationAppService::update_app(app_id, &app_name, &display_name, &description, &base_url, status).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// DELETE /auth-admin/apps/{id} — 删除应用
#[endpoint(tags("apps"), summary = "删除应用")]
pub async fn delete_app(id: PathParam<i64>) -> Json<RespVO<()>> {
    match ApplicationAppService::delete_app(id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}
