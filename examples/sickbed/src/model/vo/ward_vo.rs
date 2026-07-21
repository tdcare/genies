//! 病房相关 VO
//!
//! 对应 Java: me.tdcarefor.tdnis.sickbed.model.vo.WardVo extends IdModel

use genies_derive::casbin;
use serde::{Deserialize, Serialize};

/// 病房简要 VO
#[casbin]
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WardBriefVo {
    pub id: Option<String>,
    pub ward_name: Option<String>,
}

/// 病房-床位联表查询结果
#[casbin]
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WardSickbedJoin {
    pub ward_id: Option<String>,
    pub ward_name: Option<String>,
    pub ward_no: Option<String>,
    pub sickbed_id: Option<String>,
    pub sickbed_no: Option<String>,
    pub patient_id: Option<String>,
    pub status: Option<i32>,
    pub order_id: Option<i32>,
}
