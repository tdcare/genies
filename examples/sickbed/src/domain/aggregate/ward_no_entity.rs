//! WardNoEntity - 病房号实体
//!
//! 对应 Java: WardNoEntity (自增ID)

use rbatis::crud;
use serde::{Deserialize, Serialize};

/// 病房号实体
///
/// 表名: WardNoEntity, 自增 INT 主键
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WardNoEntity {
    /// 自增主键
    pub id: Option<i32>,
    /// 名称
    pub name: Option<String>,
    /// 病房编号
    pub ward_no: Option<String>,
    /// 类型 (type 是 Rust 关键字, 用 type_field 代替)
    #[serde(rename = "type")]
    pub type_field: Option<String>,
}

crud!(WardNoEntity {}, "WardNoEntity");
