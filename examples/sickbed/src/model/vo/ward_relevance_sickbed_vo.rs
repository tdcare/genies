//! WardRelevanceSickbedVo - 病房及其关联床位视图
//!
//! 对应 Java: me.tdcarefor.tdnis.sickbed.model.vo.WardRelevanceSickbedVo extends WardModel

use genies_derive::casbin;
use serde::{Deserialize, Serialize};
use crate::domain::aggregate::sickbed_entity::SickbedEntity;
use crate::domain::aggregate::ward_entity::WardEntity;

/// 病房关联床位 VO
///
/// ward 作为嵌套对象，与 Java 端 JSON 结构对齐: {"ward": {...}, "sickbedModelList": [...]}
#[casbin]
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WardRelevanceSickbedVo {
    pub ward: WardEntity,
    /// 新增床位数量
    pub sickbed_count_add: Option<i32>,
    /// 关联的床位列表
    pub sickbed_model_list: Option<Vec<SickbedEntity>>,
}
