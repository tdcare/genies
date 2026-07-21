//! 门口屏相关 VO

use genies_derive::casbin;
use serde::{Deserialize, Serialize};

/// 门口屏基础数据
#[casbin]
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoorwayScreenVo {
    /// 病区id
    pub ward_id: Option<String>,
    /// 病区名称
    pub ward_name: Option<String>,
    /// 科室id
    pub department_id: Option<String>,
    /// 科室主任(责任人)名字
    pub response_people_name: Option<String>,
    /// 科室主任图片地址
    pub response_people_address: Option<String>,
    /// 护士长姓名
    pub headurse_name: Option<String>,
    /// 护士长图片
    pub headurse_address: Option<String>,
    /// 科室名
    pub department_name: Option<String>,
    /// 科室简称
    pub department_abstract: Option<String>,
    /// 科室编码
    pub department_code: Option<String>,
    /// 患者门口屏列表
    pub patient_doorway_screen_vo_list: Option<Vec<PatientDoorwayScreenVo>>,
}

/// 患者门口屏信息
#[casbin]
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatientDoorwayScreenVo {
    /// 患者ID
    pub patient_id: Option<String>,
    /// 姓名
    pub name: Option<String>,
    /// 病床id
    pub sickbed_id: Option<String>,
    /// 病床编号
    pub sickbed_no: Option<String>,
    /// 排序Id
    pub sickbed_order_id: Option<i32>,
    /// 住院号
    pub patient_no: Option<String>,
    /// 住院次数
    pub visit_id: Option<i32>,
    /// 责任护士id
    pub nurse_user_id: Option<String>,
    /// 责任护士姓名
    pub nurse_user_name: Option<String>,
    /// 责任护士图片
    pub nurse_user_name_image_address: Option<String>,
    /// 责任医生id
    pub doctor_user_id: Option<String>,
    /// 责任医生图片
    pub doctor_image_address: Option<String>,
    /// 责任医生姓名
    pub doctor_user_name: Option<String>,
}
