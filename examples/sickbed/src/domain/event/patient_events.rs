//! 外部消费事件 - 来自 patient 模块
//!
//! 对应 Java 事件类:
//! - PatientInHospitaledEvent: 患者入院事件
//! - PatientTransferEvent: 患者转科/换床/借出事件
//! - PatientLeavedHospitalEvent: 患者出院事件
//! - SickbedDoctorChangedEvent: 管床医生变更事件

use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

/// 患者入院事件
///
/// 对应 Java: PatientInHospitaledEvent extends PatientModel
#[derive(DomainEvent, Clone, Debug, Serialize, Deserialize, Default)]
#[event_type("me.tdcarefor.tdnis.patient.event.PatientInHospitaledEvent")]
#[event_source("me.tdcarefor.tdnis.patient.domain.aggregate.PatientEntity")]
#[serde(rename_all = "camelCase")]
pub struct PatientInHospitaledEvent {
    pub id: Option<String>,
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub department_abstract: Option<String>,
    pub department_code: Option<String>,
    pub patient_id: Option<String>,
    pub patient_name: Option<String>,
    pub patient_no: Option<String>,
    pub patient_sickbed_no: Option<String>,
    pub patient_sickbed_id: Option<String>,
    pub patient_age: Option<String>,
    pub patient_sickbed_order_id: Option<i32>,
    pub last_menstruation: Option<String>,
    pub expected_of_childbirth: Option<String>,
    pub pelvis: Option<f32>,
    pub fetal_weight: Option<f32>,
    pub fetal_azimuth: Option<f32>,
    pub productivity: Option<f32>,
    pub head_score: Option<f32>,
}

/// 患者转科/换床/借出事件
///
/// 对应 Java: PatientTransferEvent extends IdAndDepartmentModel
#[derive(DomainEvent, Clone, Debug, Serialize, Deserialize, Default)]
#[event_type("me.tdcarefor.tdnis.patient.event.PatientTransferEvent")]
#[event_source("me.tdcarefor.tdnis.patient.domain.aggregate.PatientEntity")]
#[serde(rename_all = "camelCase")]
pub struct PatientTransferEvent {
    pub id: Option<String>,
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub department_abstract: Option<String>,
    pub department_code: Option<String>,
    pub doctor_dept_id: Option<String>,
    pub doctor_dept_name: Option<String>,
    pub doctor_dept_abstract: Option<String>,
    pub doctor_dept_code: Option<String>,
    /// 0:换床 1:转科 2:转科并换床 3:借出
    pub transfer_flag: Option<i32>,
    pub handler_time: Option<rbdc::DateTime>,
    pub from_department_id: Option<String>,
    pub from_department_name: Option<String>,
    pub from_department_abstract: Option<String>,
    pub from_department_code: Option<String>,
    pub from_doctor_dept_id: Option<String>,
    pub from_doctor_dept_name: Option<String>,
    pub from_doctor_dept_abstract: Option<String>,
    pub from_doctor_dept_code: Option<String>,
    pub from_sickbed_no: Option<String>,
    pub from_sickbed_id: Option<String>,
    pub from_sickbed_order_id: Option<i32>,
    pub sickbed_no: Option<String>,
    pub sickbed_id: Option<String>,
    pub sickbed_order_id: Option<i32>,
    pub state: Option<i32>,
}

/// 患者出院事件
///
/// 对应 Java: PatientLeavedHospitalEvent extends EventIdModel
#[derive(DomainEvent, Clone, Debug, Serialize, Deserialize, Default)]
#[event_type("me.tdcarefor.tdnis.patient.event.PatientLeavedHospitalEvent")]
#[event_source("me.tdcarefor.tdnis.patient.domain.aggregate.PatientEntity")]
#[serde(rename_all = "camelCase")]
pub struct PatientLeavedHospitalEvent {
    pub id: Option<String>,
    pub out_hospital_date: Option<rbdc::DateTime>,
    pub out_hospital_type_string: Option<String>,
    pub department_id: Option<String>,
    pub patient_nurse_id: Option<String>,
    pub patient_nurse_name: Option<String>,
    pub doctor_dept_id: Option<String>,
    pub patient_id: Option<String>,
    pub patient_name: Option<String>,
    pub patient_sickbed_no: Option<String>,
    pub patient_sickbed_id: Option<String>,
    pub patient_sickbed_order_id: Option<i32>,
}

/// 管床医生变更事件
///
/// 对应 Java: SickbedDoctorChangedEvent extends EventIdModel
#[derive(DomainEvent, Clone, Debug, Serialize, Deserialize, Default)]
#[event_type("me.tdcarefor.tdnis.patient.event.SickbedDoctorChangedEvent")]
#[event_source("me.tdcarefor.tdnis.patient.domain.aggregate.PatientEntity")]
#[serde(rename_all = "camelCase")]
pub struct SickbedDoctorChangedEvent {
    pub id: Option<String>,
    /// 责任医生id
    pub doctor_user_id: Option<String>,
    /// 责任医生姓名
    pub doctor_user_name: Option<String>,
    /// 病床id
    pub sickbed_id: Option<String>,
}
