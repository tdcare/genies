//! 床位领域事件
//!
//! 对应 Java 事件类:
//! - SickbedDomainEvent: 领域事件 trait
//! - SickbedUpdatedEvent: 床位更新事件 (extends SickbedModel)
//! - SickbedArrangementedEvent: 安排床位事件
//! - SickbedChangedEvent: 换床事件
//! - SickbedEmptyedEvent: 清空患者事件
//! - SickbedTestArrangementedEvent: 测试安排床位事件
//! - SickbedTestEmptyedEvent: 测试清空患者事件

use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

/// 床位安排事件
///
/// 对应 Java: SickbedArrangementedEvent extends BaseCommandModel implements SickbedDomainEvent
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.tdnis.sickbed.domain.aggregate.SickbedEntity")]
#[event_type("me.tdcarefor.tdnis.sickbed.event.SickbedArrangementedEvent")]
#[serde(rename_all = "camelCase")]
pub struct SickbedArrangementedEvent {
    /// 病人id
    pub patient_id: Option<String>,
    /// 病床编号
    pub sickbed_no: Option<String>,
    /// 病床排序号
    pub sickbed_order_id: Option<i32>,
    /// 责任医生id
    pub doctor_user_id: Option<String>,
    /// 责任医生姓名
    pub doctor_user_name: Option<String>,
}

/// 换床事件
///
/// 对应 Java: SickbedChangedEvent implements SickbedDomainEvent
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.tdnis.sickbed.domain.aggregate.SickbedEntity")]
#[event_type("me.tdcarefor.tdnis.sickbed.event.SickbedChangedEvent")]
#[serde(rename_all = "camelCase")]
pub struct SickbedChangedEvent {
    pub patient_id: Option<String>,
    pub from_sickbed_id: Option<String>,
    pub to_sickbed_id: Option<String>,
    pub change_date: Option<rbdc::DateTime>,
    /// 病床编号
    pub sickbed_no: Option<String>,
    /// 病床排序号
    pub sickbed_order_id: Option<i32>,
    /// 责任医生id
    pub doctor_user_id: Option<String>,
    /// 责任医生姓名
    pub doctor_user_name: Option<String>,
}

/// 床位更新事件
///
/// 对应 Java: SickbedUpdatedEvent extends SickbedModel implements SickbedDomainEvent
/// 包含完整的 SickbedModel 字段
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.tdnis.sickbed.domain.aggregate.SickbedEntity")]
#[event_type("me.tdcarefor.tdnis.sickbed.event.SickbedUpdatedEvent")]
#[serde(rename_all = "camelCase")]
pub struct SickbedUpdatedEvent {
    pub id: Option<String>,
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub department_code: Option<String>,
    pub department_abstract: Option<String>,
    pub patient_id: Option<String>,
    pub his_id: Option<String>,
    pub sickbed_no: Option<String>,
    pub ward_id: Option<String>,
    pub ward_name: Option<String>,
    pub status: Option<i32>,
    pub effectiveness: Option<i32>,
    pub bed_level: Option<i32>,
    pub nurse_user_id: Option<String>,
    pub doctor_user_id: Option<String>,
    pub doctor_user_name: Option<String>,
    pub response_user_id: Option<String>,
    pub enter_user_id: Option<String>,
    pub packet_bed_name: Option<String>,
    pub create_date: Option<rbdc::DateTime>,
    pub ward_code: Option<String>,
    pub approved_type: Option<String>,
    pub sex_limit: Option<i32>,
    pub sex_type: Option<String>,
    pub bed_class: Option<String>,
    pub order_id: Option<i32>,
    pub nurse_user_name: Option<String>,
}

impl SickbedUpdatedEvent {
    pub fn from_entity(entity: &crate::domain::aggregate::SickbedEntity) -> Self {
        genies::copy!(entity, SickbedUpdatedEvent)
    }
}

/// 清空患者事件
///
/// 对应 Java: SickbedEmptyedEvent extends BaseCommandModel implements SickbedDomainEvent
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.tdnis.sickbed.domain.aggregate.SickbedEntity")]
#[event_type("me.tdcarefor.tdnis.sickbed.event.SickbedEmptyedEvent")]
#[serde(rename_all = "camelCase")]
pub struct SickbedEmptyedEvent {
    /// 床位ID
    pub id: Option<String>,
    /// 病人id
    pub patient_id: Option<String>,
}

/// 测试安排床位事件
///
/// 对应 Java: SickbedTestArrangementedEvent extends BaseCommandModel implements SickbedDomainEvent
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.tdnis.sickbed.domain.aggregate.SickbedEntity")]
#[event_type("me.tdcarefor.tdnis.sickbed.event.SickbedTestArrangementedEvent")]
#[serde(rename_all = "camelCase")]
pub struct SickbedTestArrangementedEvent {
    /// 床位ID
    pub id: Option<String>,
    /// 病人id
    pub patient_id: Option<String>,
    /// 病床编号
    pub sickbed_no: Option<String>,
    /// 病床排序号
    pub sickbed_order_id: Option<i32>,
    /// 责任医生id
    pub doctor_user_id: Option<String>,
    /// 责任医生姓名
    pub doctor_user_name: Option<String>,
}

/// 测试清空患者事件
///
/// 对应 Java: SickbedTestEmptyedEvent extends BaseCommandModel implements SickbedDomainEvent
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.tdnis.sickbed.domain.aggregate.SickbedEntity")]
#[event_type("me.tdcarefor.tdnis.sickbed.event.SickbedTestEmptyedEvent")]
#[serde(rename_all = "camelCase")]
pub struct SickbedTestEmptyedEvent {
    /// 床位ID
    pub id: Option<String>,
    /// 病人id
    pub patient_id: Option<String>,
}
