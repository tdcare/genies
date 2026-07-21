//! 病房管理控制器
//!
//! 对应 Java: WardController - 11 个端点

#![allow(non_snake_case)]

use salvo::oapi::extract::{JsonBody, PathParam, QueryParam};
use salvo::prelude::*;

use crate::domain::aggregate::WardEntity;
use crate::domain::service::ward_service::WardService;
use crate::model::vo::WardBriefVo;
use crate::model::vo::WardRelevanceSickbedVo;

// ============ POST 端点 ============

/// POST /ward/add - 添加病房
#[endpoint]
pub async fn add_ward(body: JsonBody<WardRelevanceSickbedVo>) -> Json<genies::core::ResultDTO<String>> {
    let vo = body.into_inner();
    // 从 vo 中获取 departmentId，如果没有则使用空字符串
    let department_id = vo.ward.department_id.clone().unwrap_or_default();
    let result = WardService::add(vo, &department_id).await;
    Json(result)
}

/// POST /ward/addByDeptId - 按科室ID添加病房
#[endpoint]
pub async fn add_by_dept_id(
    body: JsonBody<WardRelevanceSickbedVo>,
    departmentId: QueryParam<String, false>,
) -> Json<genies::core::ResultDTO<String>> {
    let vo = body.into_inner();
    let department_id = departmentId.into_inner().unwrap_or_default();
    let result = WardService::add(vo, &department_id).await;
    Json(result)
}

/// POST /ward/update - 更新病房
#[endpoint]
pub async fn update_ward(body: JsonBody<WardRelevanceSickbedVo>) -> Json<genies::core::ResultDTO<String>> {
    let vo = body.into_inner();
    let result = WardService::update(vo).await;
    Json(result)
}

/// POST /ward/delete - 删除病房
#[endpoint]
pub async fn delete_ward(id: QueryParam<String, false>) -> Json<genies::core::ResultDTO<String>> {
    let id = id.into_inner().unwrap_or_default();
    let result = WardService::delete(&id).await;
    Json(result)
}

/// POST /ward/findbywardId - 条件搜索病房
#[endpoint]
pub async fn find_by_ward_id_post(body: JsonBody<serde_json::Value>) -> Json<genies::core::ResultDTO<Vec<WardEntity>>> {
    let body = body.into_inner();
    let department_id = body.get("departmentId").and_then(|v| v.as_str()).unwrap_or("");
    let ward_name = body.get("wardName").and_then(|v| v.as_str()).unwrap_or("");
    let ward_type = body.get("wardType").and_then(|v| v.as_str()).unwrap_or("");
    let result = WardService::find_all_by_search(department_id, ward_name, ward_type).await;
    let dto = genies::core::ResultDTO::success("", result);
    Json(dto)
}

// ============ GET 端点 ============

/// GET /ward/id/{id} - 按ID查找病房
#[endpoint]
pub async fn find_by_id(id: PathParam<String>) -> Json<serde_json::Value> {
    let id = id.into_inner();
    let result = WardService::find_by_id(&id).await;
    Json(serde_json::json!(result))
}

/// GET /ward/findWardRelevnceSickbe - 查询病房+关联床位
#[endpoint]
pub async fn find_ward_relevance_sickbed(id: QueryParam<String, false>) -> Json<Option<WardRelevanceSickbedVo>> {
    let id = id.into_inner().unwrap_or_default();
    let result = WardService::find_ward_relevance_sickbed(&id).await;
    Json(result)
}

/// GET /ward/effectiveness - 有效病房列表
#[endpoint]
pub async fn effectiveness_wards(departmentId: QueryParam<String, false>) -> Json<Vec<WardEntity>> {
    let department_id = departmentId.into_inner().unwrap_or_default();
    let result = WardService::effectiveness_wards(&department_id).await;
    Json(result)
}

/// GET /ward/effectivenessForSearch - 分页搜索病房
#[endpoint]
pub async fn effectiveness_for_search(
    departmentId: QueryParam<String, false>,
    wardName: QueryParam<String, false>,
    wardType: QueryParam<String, false>,
    pageIndex: QueryParam<u64, false>,
    pageSize: QueryParam<u64, false>,
) -> Json<genies::core::ResultDTO<genies::core::page::SpringPage<WardEntity>>> {
    let department_id = departmentId.into_inner().unwrap_or_default();
    let ward_name = wardName.into_inner().unwrap_or_default();
    let ward_type = wardType.into_inner().unwrap_or_default();
    let page_index = pageIndex.into_inner().unwrap_or(0);
    let page_size = pageSize.into_inner().unwrap_or(10);

    let page = WardService::effectiveness_for_search(
        &department_id, &ward_name, &ward_type, page_index, page_size,
    ).await;

    let result = genies::core::ResultDTO::success("操作完成", page);
    Json(result)
}

/// GET /ward/effectivenessWardVo - 病房简要列表（门口屏绑定用）
#[endpoint]
pub async fn effectiveness_ward_vo(departmentId: QueryParam<String, false>) -> Json<Vec<WardBriefVo>> {
    let department_id = departmentId.into_inner().unwrap_or_default();
    let result: Vec<WardBriefVo> = WardService::effectiveness_ward_vo(&department_id).await;
    Json(result)
}

/// 组装病房路由
pub fn ward_router() -> Router {
    Router::with_path("ward")
        // POST 端点
        .push(Router::with_path("add").post(add_ward))
        .push(Router::with_path("addByDeptId").post(add_by_dept_id))
        .push(Router::with_path("update").post(update_ward))
        .push(Router::with_path("delete").post(delete_ward))
        .push(Router::with_path("findbywardId").post(find_by_ward_id_post))
        // GET 端点
        .push(Router::with_path("id/{id}").get(find_by_id))
        .push(Router::with_path("findWardRelevnceSickbe").get(find_ward_relevance_sickbed))
        .push(Router::with_path("effectiveness").get(effectiveness_wards))
        .push(Router::with_path("effectivenessForSearch").get(effectiveness_for_search))
        .push(Router::with_path("effectivenessWardVo").get(effectiveness_ward_vo))
}
