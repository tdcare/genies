//! 患者床头屏 VO

use genies_derive::casbin;
use serde::{Deserialize, Serialize};

/// 患者床头屏信息
#[casbin]
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatientBedsideScreenVo {
    /// ID
    pub id: Option<String>,
    /// 姓名
    pub name: Option<String>,
    /// 住院号
    pub patient_no: Option<String>,
    /// 住院次数
    pub visit_id: Option<i32>,
    /// 性别（男1女2）
    pub sex: Option<i32>,
    /// 责任护士id
    pub nurse_user_id: Option<String>,
    /// 责任护士职称
    pub nurse_user_title: Option<String>,
    /// 责任护士姓名
    pub nurse_user_name: Option<String>,
    /// 责任护士图片
    pub nurse_user_name_image_address: Option<String>,
    /// 责任医生id
    pub doctor_user_id: Option<String>,
    /// 责任医生职称
    pub doctor_user_title: Option<String>,
    /// 责任医生图片
    pub doctor_image_address: Option<String>,
    /// 责任医生姓名
    pub doctor_user_name: Option<String>,
    /// 住院开始时间
    pub hospitalization_start_time: Option<String>,
    /// 护理等级
    pub execute_level: Option<String>,
    /// 病床id
    pub sickbed_id: Option<String>,
    /// 病床编号
    pub sickbed_no: Option<String>,
    /// 科室id
    pub department_id: Option<String>,
    /// 科室名
    pub department_name: Option<String>,
    /// 科室主任(责任人)
    pub response_people: Option<String>,
    /// 护士长姓名
    pub headurse_name: Option<String>,
}
