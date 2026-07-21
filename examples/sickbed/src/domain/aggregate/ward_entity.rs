//! WardEntity 聚合根 - 病房管理
//!
//! 对应 Java: me.tdcarefor.tdnis.sickbed.domain.aggregate.WardEntity
//! 字段展平自: IdModel → IdAndDepartmentModel → IdAndDepartmentAndHisModel → WardModel

use genies_derive::casbin;
use genies_derive::Aggregate;
use rbatis::crud;
use serde::{Deserialize, Serialize};

/// 自定义反序列化器：接受 int 或 string，都转为 Option<String>
///
/// DB 中 wardType/predictLevel 为 int 类型，Java JPA 映射为 String，
/// rbatis 从 MySQL 读 int 列时 rbs::Value 为数字类型。
/// 此反序列化器兼容两种格式，统一输出 String 以匹配 Java 行为。
mod i32_as_string {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) => serializer.serialize_str(v),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        match v {
            serde_json::Value::Number(n) => Ok(Some(n.to_string())),
            serde_json::Value::String(s) => Ok(Some(s)),
            serde_json::Value::Null => Ok(None),
            _ => Ok(None),
        }
    }
}

/// 病房聚合根
///
/// 表名保持 Java JPA 一致: WardEntity (PascalCase)
#[casbin]
#[derive(Aggregate, Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[aggregate_type("me.tdcarefor.tdnis.sickbed.domain.aggregate.WardEntity")]
#[serde(rename_all = "camelCase")]
pub struct WardEntity {
    // === IdModel ===
    pub id: Option<String>,

    // === IdAndDepartmentModel ===
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub department_code: Option<String>,
    pub department_abstract: Option<String>,

    // === 医生科室信息 ===
    /// 医生科室id
    pub doctor_dept_id: Option<String>,
    /// 医生科室名称
    pub doctor_dept_name: Option<String>,
    /// 医生科室简称
    pub doctor_dept_abstract: Option<String>,
    /// 医生科室编码
    pub doctor_dept_code: Option<String>,

    // === IdAndDepartmentAndHisModel ===
    pub his_id: Option<String>,

    // === WardModel ===
    /// 病房编号
    pub ward_no: Option<String>,
    /// 病房名称
    pub ward_name: Option<String>,
    /// 病床数
    pub sickbed_count: Option<i32>,
    /// 病房等级
    #[serde(default, serialize_with = "i32_as_string::serialize", deserialize_with = "i32_as_string::deserialize")]
    pub predict_level: Option<String>,
    /// 地址
    pub address: Option<String>,
    /// 有效性
    pub effectiveness: Option<i32>,
    /// 负责人id
    pub response_user_id: Option<String>,
    /// 负责人姓名
    pub response_user_name: Option<String>,
    /// 录入人id
    pub enter_user_id: Option<String>,
    /// 录入时间
    #[salvo(schema(value_type = Option<String>))]
    #[serde(
        default,
        serialize_with = "crate::model::date_format::serialize_option_datetime",
        deserialize_with = "crate::model::date_format::deserialize_option_datetime"
    )]
    pub create_date: Option<rbdc::DateTime>,
    /// 病房描述
    pub description: Option<String>,
    /// 病房类型
    #[serde(default, serialize_with = "i32_as_string::serialize", deserialize_with = "i32_as_string::deserialize")]
    pub ward_type: Option<String>,
    /// 病房排序Id
    pub order_id: Option<i32>,
}

crud!(WardEntity {}, "WardEntity");
