//! 应用实例 Handler — 注册/心跳/注销 + 管理查询

use salvo::prelude::*;
use salvo::oapi::extract::*;

use genies::core::RespVO;
use genies::context::CONTEXT;

use crate::application::dto::PageResult;
use crate::domain::entity::app_instance_entity::AppInstanceEntity;
use crate::domain::entity::application_entity::ApplicationEntity;
use crate::domain::service::AppInstanceDomainService;
use crate::interfaces::dto::instance_dto::{
    DeregisterRequest, HeartbeatRequest, InstanceVO, RegisterInstanceRequest,
};

// ── 内部路由（服务间调用，仅 JWT 签名验证） ──

/// 内部路由：注册/心跳/注销
pub fn internal_instance_routes() -> Router {
    Router::with_path("/auth-admin/internal/instances")
        .push(Router::with_path("register").post(register_instance))
        .push(Router::with_path("heartbeat").post(heartbeat))
        .push(Router::with_path("deregister").post(deregister_instance))
}

/// POST /auth-admin/internal/instances/register — 注册或更新实例
#[endpoint(tags("instances"), summary = "注册或更新实例")]
pub async fn register_instance(body: JsonBody<RegisterInstanceRequest>) -> Json<RespVO<String>> {
    let req = body.into_inner();
    let entity = AppInstanceEntity {
        id: None,
        app_name: Some(req.app_name),
        instance_id: Some(req.instance_id),
        base_url: Some(req.base_url),
        version: req.version,
        status: None,
        last_heartbeat_at: None,
        registered_at: None,
        metadata: None,
    };

    match AppInstanceDomainService::register_or_update(&entity).await {
        Ok(()) => Json(RespVO::from(&"ok".to_string())),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// POST /auth-admin/internal/instances/heartbeat — 心跳
#[endpoint(tags("instances"), summary = "实例心跳")]
pub async fn heartbeat(body: JsonBody<HeartbeatRequest>) -> Json<RespVO<String>> {
    let req = body.into_inner();
    match AppInstanceDomainService::heartbeat(req.instance_id).await {
        Ok(_) => Json(RespVO::from(&"ok".to_string())),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// POST /auth-admin/internal/instances/deregister — 注销实例
#[endpoint(tags("instances"), summary = "注销实例")]
pub async fn deregister_instance(body: JsonBody<DeregisterRequest>) -> Json<RespVO<String>> {
    let req = body.into_inner();
    match AppInstanceDomainService::deregister(req.instance_id).await {
        Ok(()) => Json(RespVO::from(&"ok".to_string())),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

// ── 受保护路由（JWT + Casbin） ──

/// 受保护路由：实例管理查询
pub fn protected_instance_routes() -> Router {
    Router::new()
        .push(
            Router::with_path("/auth-admin/apps/{app_id}/instances")
                .get(list_app_instances),
        )
        .push(
            Router::with_path("/auth-admin/instances")
                .get(list_all_instances),
        )
}

/// GET /auth-admin/apps/{app_id}/instances — 查询指定应用的实例列表
#[endpoint(tags("instances"), summary = "查询应用实例列表")]
pub async fn list_app_instances(app_id: PathParam<i64>) -> Json<RespVO<Vec<InstanceVO>>> {
    let rb = &CONTEXT.rbatis;
    let app_id = app_id.into_inner();

    // 先查 auth_applications 获取 app_name
    let app = match ApplicationEntity::find_by_id(rb, &app_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return Json(RespVO::from_error_info("-1", "应用不存在")),
        Err(e) => return Json(RespVO::from_error_info("-1", &e.to_string())),
    };

    let app_name = app.app_name.unwrap_or_default();

    match AppInstanceEntity::select_by_app_name(rb, &app_name).await {
        Ok(list) => {
            let vos: Vec<InstanceVO> = list.into_iter().map(|e| e.into()).collect();
            Json(RespVO::from(&vos))
        }
        Err(e) => Json(RespVO::from_error_info("-1", &e.to_string())),
    }
}

/// GET /auth-admin/instances — 分页查询所有实例
#[endpoint(tags("instances"), summary = "分页查询所有实例")]
pub async fn list_all_instances(
    page: QueryParam<u64, false>,
    size: QueryParam<u64, false>,
    keyword: QueryParam<String, false>,
) -> Json<RespVO<PageResult<InstanceVO>>> {
    let rb = &CONTEXT.rbatis;
    let page = page.into_inner().unwrap_or(1).max(1);
    let size = size.into_inner().unwrap_or(10).min(100);
    let keyword = keyword.into_inner().unwrap_or_default();
    let offset = (page - 1) * size;

    let total = match AppInstanceEntity::count_by_app_name(rb, &keyword).await {
        Ok(t) => t,
        Err(e) => return Json(RespVO::from_error_info("-1", &e.to_string())),
    };

    match AppInstanceEntity::select_all_instances(rb, &keyword, &offset, &size).await {
        Ok(list) => {
            let vos: Vec<InstanceVO> = list.into_iter().map(|e| e.into()).collect();
            Json(RespVO::from(&PageResult {
                total,
                page,
                size,
                list: vos,
            }))
        }
        Err(e) => Json(RespVO::from_error_info("-1", &e.to_string())),
    }
}
