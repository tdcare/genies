//! 床位相关 VO
//!
//! 从 domain/model/sickbed_model.rs 迁移至此

use genies_derive::casbin;
use serde::{Deserialize, Serialize};

/// 科室有效床位数统计
#[casbin]
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeptCount {
    pub department_id: Option<String>,
    pub count: Option<i64>,
}

/// 科室信息（DISTINCT departmentId, departmentName）
#[casbin]
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeptInfo {
    pub department_id: Option<String>,
    pub department_name: Option<String>,
}

/// 有效床位简要信息
#[casbin]
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SickbedBriefInfo {
    #[serde(rename = "Id", alias = "id")]
    pub id: Option<String>,
    pub sickbed_no: Option<String>,
    pub ward_id: Option<String>,
    pub ward_name: Option<String>,
}

/// 辅助结构体 - 仅返回 id 字段
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IdOnly {
    pub id: Option<String>,
}

/// 辅助结构体 - 仅返回 patientId 字段
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatientIdOnly {
    pub patient_id: Option<String>,
}
