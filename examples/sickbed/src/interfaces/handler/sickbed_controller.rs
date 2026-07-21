//! 床位管理控制器
//!
//! 对应 Java: SickbedController - 25+ 个端点

#![allow(non_snake_case)]

use salvo::oapi::extract::{JsonBody, PathParam, QueryParam};
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domain::aggregate::SickbedEntity;
use crate::domain::command::sickbed_commands::*;
use crate::model::vo::DeptCount;
use crate::domain::service::sickbed_service::SickbedService;
use crate::model::vo::{DoorwayScreenVo, PatientDoorwayScreenVo, UserSickbedListVo};

// ============ POST 端点 ============

/// POST /add - 添加床位
#[endpoint]
pub async fn add_sickbed(body: JsonBody<SickbedEntity>) -> Json<genies::core::ResultDTO<String>> {
    let model = body.into_inner();
    let result = SickbedService::add_sickbed(model).await;
    Json(result)
}

/// POST /update - 更新床位
#[endpoint]
pub async fn update_sickbed(body: JsonBody<SickbedEntity>) -> Json<genies::core::ResultDTO<String>> {
    let model = body.into_inner();
    let result = SickbedService::update_sickbed(model).await;
    Json(result)
}

/// POST /delete - 删除床位
#[endpoint]
pub async fn delete_sickbed(body: JsonBody<Vec<String>>) -> Json<genies::core::ResultDTO<String>> {
    let ids = body.into_inner();
    let result = SickbedService::batch_delete(ids).await;
    Json(result)
}

/// POST /batchupdate - 批量更新
#[endpoint]
pub async fn batch_update(body: JsonBody<Vec<SickbedEntity>>) -> Json<genies::core::ResultDTO<String>> {
    let models = body.into_inner();
    let result = SickbedService::batch_update(models).await;
    Json(result)
}

/// POST /arrangement - 安排床位
#[endpoint]
pub async fn arrangement_sickbed(body: JsonBody<ArrangementSickbedCommand>) -> Json<genies::core::ResultDTO<String>> {
    let cmd = body.into_inner();
    let result = SickbedService::arrangement_sickbed(cmd).await;
    Json(result)
}

/// POST /change - 换床
#[endpoint]
pub async fn change_sickbed(body: JsonBody<ChangeSickbedCommand>) -> Json<genies::core::ResultDTO<String>> {
    let cmd = body.into_inner();
    let result = SickbedService::change_sickbed(cmd).await;
    Json(result)
}

/// POST /empty - 清空床位
#[endpoint]
pub async fn empty_sickbed(body: JsonBody<EmptySickbedCommand>) -> Json<genies::core::ResultDTO<String>> {
    let cmd = body.into_inner();
    let result = SickbedService::empty_sickbed(cmd).await;
    Json(result)
}

/// POST /testarrangement - 测试安排床位
#[endpoint]
pub async fn test_arrangement_sickbed(body: JsonBody<TestArrangementSickbedCommand>) -> Json<genies::core::ResultDTO<String>> {
    let cmd = body.into_inner();
    let result = SickbedService::test_arrangement_sickbed(cmd).await;
    Json(result)
}

/// POST /forceemptyall - 强制清空科室所有床位
#[endpoint]
pub async fn force_empty_all(departmentCode: QueryParam<String, false>) -> Json<genies::core::ResultDTO<String>> {
    let department_id = departmentCode.into_inner().unwrap_or_default();
    let result = SickbedService::empty_sickbeds_by_dept(&department_id).await;
    Json(result)
}

/// POST /forceemptyone - 强制清空单个床位（按科室编码+床位号）
#[endpoint]
pub async fn force_empty_one(
    departmentCode: QueryParam<String, false>,
    sickbedNo: QueryParam<String, false>,
) -> Json<genies::core::ResultDTO<String>> {
    let department_code = departmentCode.into_inner().unwrap_or_default();
    let sickbed_no = sickbedNo.into_inner().unwrap_or_default();
    let result = SickbedService::empty_one(&department_code, &sickbed_no).await;
    Json(result)
}

/// POST /emptynurse - 清空护士绑定
#[endpoint]
pub async fn empty_nurse(
    departmentId: QueryParam<String, false>,
    userId: QueryParam<String, false>,
) -> Json<genies::core::ResultDTO<String>> {
    let department_id = departmentId.into_inner().unwrap_or_default();
    let user_id = userId.into_inner().unwrap_or_default();
    let result = SickbedService::empty_nurse(&department_id, &user_id).await;
    Json(result)
}

/// POST /effectivenesscountbylist - 按多科室统计有效床位数
#[endpoint]
pub async fn effectiveness_count_by_list(body: JsonBody<Vec<String>>) -> Json<genies::core::ResultDTO<serde_json::Value>> {
    let dept_ids = body.into_inner();
    let counts: Vec<DeptCount> =
        SickbedService::count_idle_sickbeds_by_list(&dept_ids, 1).await;
    let map: serde_json::Map<String, serde_json::Value> = counts
        .into_iter()
        .filter_map(|dc| {
            let dept_id = dc.department_id?;
            let count = dc.count.unwrap_or(0);
            Some((dept_id, serde_json::Value::Number(serde_json::Number::from(count))))
        })
        .collect();
    let result = genies::core::ResultDTO::success("查询成功", serde_json::Value::Object(map));
    Json(result)
}

/// 床位简要信息 VO（getAllBed 返回）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SickbedBriefVo {
    pub id: Option<String>,
    pub bed_no: Option<String>,
    pub state: Option<i32>,
    pub department_id: Option<String>,
    pub responsible_person_id: Option<String>,
    pub order_no: Option<i32>,
}

/// POST /getAllBed - 获取所有床位
#[endpoint]
pub async fn get_all_bed() -> Json<Vec<SickbedBriefVo>> {
    let result = SickbedService::get_all_bed().await;
    Json(result)
}

// ============ GET 端点 ============

/// GET /id/{id} - 按ID查找
#[endpoint]
pub async fn find_by_id(id: PathParam<String>) -> Json<serde_json::Value> {
    let id = id.into_inner();
    let result = SickbedService::find_by_id(&id).await;
    Json(serde_json::json!(result))
}

/// GET /effectiveness - 有效床位列表
#[endpoint]
pub async fn effectiveness(departmentId: QueryParam<String, false>) -> Json<Vec<SickbedEntity>> {
    let department_id = departmentId.into_inner().unwrap_or_default();
    let result = SickbedService::active_sickbeds(&department_id).await;
    Json(result)
}

/// GET /effectivenessForSearch - 分页搜索床位
#[endpoint]
pub async fn effectiveness_for_search(
    departmentId: QueryParam<String, false>,
    wardId: QueryParam<String, false>,
    sickbedNo: QueryParam<String, false>,
    pageIndex: QueryParam<u64, false>,
    pageSize: QueryParam<u64, false>,
) -> Json<genies::core::ResultDTO<genies::core::page::SpringPage<SickbedEntity>>> {
    let department_id = departmentId.into_inner().unwrap_or_default();
    let ward_id = wardId.into_inner().unwrap_or_default();
    let sickbed_no = sickbedNo.into_inner().unwrap_or_default();
    let page_index = pageIndex.into_inner().unwrap_or(0);
    let page_size = pageSize.into_inner().unwrap_or(10);

    let department_ids: Vec<String> = if department_id.is_empty() {
        vec![]
    } else {
        vec![department_id]
    };

    let page = SickbedService::effectiveness_for_search(
        &department_ids, &ward_id, &sickbed_no, page_index, page_size,
    ).await;

    let result = genies::core::ResultDTO::success("操作完成", page);
    Json(result)
}

/// GET /bedside/effectiveness - 床旁有效床位简要信息
#[endpoint]
pub async fn bedside_effectiveness(departmentId: QueryParam<String, false>) -> Json<genies::core::ResultDTO<Vec<crate::model::vo::SickbedBriefInfo>>> {
    let department_id = departmentId.into_inner().unwrap_or_default();
    let rb = &genies::context::CONTEXT.rbatis;
    let briefs = SickbedEntity::query_effectiveness_sickbeds(rb, &department_id)
        .await
        .unwrap_or_default();
    let result = genies::core::ResultDTO::success("查询成功", briefs);
    Json(result)
}

/// GET /effectiveness/empty - 有效且空闲床位
#[endpoint]
pub async fn effectiveness_empty(departmentId: QueryParam<String, false>) -> Json<Vec<SickbedEntity>> {
    let department_id = departmentId.into_inner().unwrap_or_default();
    let result = SickbedService::active_and_empty_sickbeds(&department_id).await;
    Json(result)
}

/// GET /effectiveness/usersickbed - 用户床位列表
#[endpoint]
pub async fn effectiveness_usersickbed(
    departmentId: QueryParam<String, false>,
    userId: QueryParam<String, false>,
) -> Json<Vec<UserSickbedListVo>> {
    let department_id = departmentId.into_inner().unwrap_or_default();
    let user_id = userId.into_inner().unwrap_or_default();
    // 通过 departmentId 获取该科室有效床位的ID列表
    let sickbeds = SickbedService::active_sickbeds(&department_id).await;
    let ids: Vec<String> = sickbeds.into_iter().filter_map(|s| s.id).collect();
    let result: Vec<UserSickbedListVo> = SickbedService::find_by_sickbed_list(ids, &user_id).await;
    Json(result)
}

/// GET /effectivenessedepartmentcount - 包含有效床位的科室列表
/// Java 端返回 `[["id","name"], ...]` 二元数组格式，此处对齐
#[endpoint]
pub async fn effectivenesse_department_count() -> Json<serde_json::Value> {
    use crate::model::vo::DeptInfo;
    let rb = &genies::context::CONTEXT.rbatis;
    let rows: Vec<DeptInfo> = SickbedEntity::effectiveness_sickbeds_count(rb)
        .await
        .unwrap_or_default();
    let result: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|d| serde_json::json!([
            d.department_id,
            d.department_name,
        ]))
        .collect();
    Json(serde_json::Value::Array(result))
}

/// GET /effectivenesscount - 科室空闲床位计数
#[endpoint]
pub async fn effectiveness_count(departmentId: QueryParam<String, false>) -> Json<i64> {
    let department_id = departmentId.into_inner().unwrap_or_default();
    let result = SickbedService::count_idle_sickbeds(&department_id).await;
    Json(result)
}

/// GET /findbyscanno - 按扫描号查找
#[endpoint]
pub async fn find_by_scan_no(scanNo: QueryParam<String, false>) -> Json<genies::core::ResultDTO<Vec<SickbedEntity>>> {
    let scan_no = scanNo.into_inner().unwrap_or_default();
    let result = SickbedService::find_by_scan_no(&scan_no).await;
    Json(result)
}

/// GET /findbywardId - 按病房ID查找床位（返回 PatientDoorwayScreenVo 列表）
/// Java 端透传 patientInfo.findInHostbySickbedIds 的 ResultDTO
#[endpoint]
pub async fn find_by_ward_id(wardId: QueryParam<String, false>) -> Json<genies::core::ResultDTO<Vec<PatientDoorwayScreenVo>>> {
    let ward_id = wardId.into_inner().unwrap_or_default();
    let dto = SickbedService::find_by_ward_id(&ward_id).await;
    Json(dto)
}

/// GET /findSickbedInfoByWardId - 按病房ID查床位信息（返回 DoorwayScreenVo）
#[endpoint]
pub async fn find_sickbed_info_by_ward_id(wardId: QueryParam<String, false>) -> Json<genies::core::ResultDTO<DoorwayScreenVo>> {
    let ward_id = wardId.into_inner().unwrap_or_default();
    let result = SickbedService::find_sickbed_info_by_ward_id(&ward_id).await;
    let dto = genies::core::ResultDTO::success("获取病房科室数据成功", result);
    Json(dto)
}

/// GET /findWardSickbedIdBySickbedId - 按床位ID查同病房床位ID列表
#[endpoint]
pub async fn find_ward_sickbed_id_by_sickbed_id(sickbedId: QueryParam<String, false>) -> Json<genies::core::ResultDTO<Vec<String>>> {
    let sickbed_id = sickbedId.into_inner().unwrap_or_default();
    let result = match SickbedService::find_ward_sickbed_id_by_sickbed_id(&sickbed_id).await {
        Ok(ids) => ids,
        Err(_) => vec![sickbed_id],
    };
    let dto = genies::core::ResultDTO::success("操作完成", result);
    Json(dto)
}

/// GET /findPatientIdByWardId - 按病房ID查患者ID列表
#[endpoint]
pub async fn find_patient_id_by_ward_id(wardId: QueryParam<String, false>) -> Json<genies::core::ResultDTO<Vec<String>>> {
    let ward_id = wardId.into_inner().unwrap_or_default();
    let result = SickbedService::find_patient_id_by_ward_id(&ward_id).await;
    if result.is_empty() {
        let dto = genies::core::ResultDTO::from_code_message(0, "病房信息获取失败", &vec![]);
        Json(dto)
    } else {
        let dto = genies::core::ResultDTO::success("操作完成", result);
        Json(dto)
    }
}

/// 组装床位路由
pub fn sickbed_router() -> Router {
    Router::new()
        // POST 端点
        .push(Router::with_path("add").post(add_sickbed))
        .push(Router::with_path("update").post(update_sickbed))
        .push(Router::with_path("delete").post(delete_sickbed))
        .push(Router::with_path("batchupdate").post(batch_update))
        .push(Router::with_path("arrangement").post(arrangement_sickbed))
        .push(Router::with_path("change").post(change_sickbed))
        .push(Router::with_path("empty").post(empty_sickbed))
        .push(Router::with_path("testarrangement").post(test_arrangement_sickbed))
        .push(Router::with_path("forceemptyall").post(force_empty_all))
        .push(Router::with_path("forceemptyone").post(force_empty_one))
        .push(Router::with_path("emptynurse").post(empty_nurse))
        .push(Router::with_path("effectivenesscountbylist").post(effectiveness_count_by_list))
        .push(Router::with_path("getAllBed").post(get_all_bed))
        // GET 端点
        .push(Router::with_path("id/{id}").get(find_by_id))
        .push(Router::with_path("effectiveness").get(effectiveness))
        .push(Router::with_path("effectivenessForSearch").get(effectiveness_for_search))
        .push(Router::with_path("bedside/effectiveness").get(bedside_effectiveness))
        .push(Router::with_path("effectiveness/empty").get(effectiveness_empty))
        .push(Router::with_path("effectiveness/usersickbed").get(effectiveness_usersickbed))
        .push(Router::with_path("effectivenessedepartmentcount").get(effectivenesse_department_count))
        .push(Router::with_path("effectivenesscount").get(effectiveness_count))
        .push(Router::with_path("findbyscanno").get(find_by_scan_no))
        .push(Router::with_path("findbywardId").get(find_by_ward_id))
        .push(Router::with_path("findSickbedInfoByWardId").get(find_sickbed_info_by_ward_id))
        .push(Router::with_path("findWardSickbedIdBySickbedId").get(find_ward_sickbed_id_by_sickbed_id))
        .push(Router::with_path("findPatientIdByWardId").get(find_patient_id_by_ward_id))
}
