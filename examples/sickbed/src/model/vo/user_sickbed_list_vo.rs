//! UserSickbedListVO - 用户床位列表视图
//!
//! 对应 Java: me.tdcarefor.tdnis.sickbed.model.vo.UserSickbedListVO extends SickbedModel

use genies_derive::casbin;
use serde::{Deserialize, Serialize};
use crate::domain::aggregate::sickbed_entity::SickbedEntity;

/// 用户床位列表 VO
///
/// 扩展了 SickbedEntity，增加 managed 是否管理标记
#[casbin]
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserSickbedListVo {
    #[serde(flatten)]
    pub sickbed: SickbedEntity,
    /// 是否为当前用户管理的床位
    pub managed: Option<bool>,
}
