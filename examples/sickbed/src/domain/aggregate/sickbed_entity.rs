//! SickbedEntity 聚合根 - 床位管理
//!
//! 对应 Java: me.tdcarefor.tdnis.sickbed.domain.aggregate.SickbedEntity
//! 字段展平自: IdModel → IdAndDepartmentModel → IdAndDepartmentAndPatientModel
//!            → IdAndDepartmentAndPatientAndHisModel → SickbedModel → SickbedEntity

use genies_derive::casbin;
use genies_derive::Aggregate;
use rbatis::crud;
use serde::{Deserialize, Serialize};

use crate::domain::event::sickbed_events::SickbedUpdatedEvent;

/// 自定义反序列化器：接受 null、数字、字符串数字 → `Option<i32>`
///
/// Java 端 `patientSickbedOrderId` 为 String 类型，数据库列为 VARCHAR，
/// 但语义上是整数排序字段。此反序列化器兼容两种格式：
/// - null / 缺失 → None
/// - 数字 123 → Some(123)
/// - 字符串 "123" → Some(123)
/// - 空字符串 "" → None
mod string_or_i32 {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<i32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) => serializer.serialize_i32(*v),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrInt {
            Int(i64),
            Str(String),
        }

        let opt = Option::<StringOrInt>::deserialize(deserializer)?;
        match opt {
            None => Ok(None),
            Some(StringOrInt::Int(n)) => Ok(Some(n as i32)),
            Some(StringOrInt::Str(s)) => {
                let s = s.trim();
                if s.is_empty() {
                    Ok(None)
                } else {
                    s.parse::<i32>()
                        .map(Some)
                        .map_err(serde::de::Error::custom)
                }
            }
        }
    }
}

/// 床位聚合根
///
/// 表名保持 Java JPA 一致: SickbedEntity (PascalCase)
/// 数据库列名为 camelCase, Rust 字段为 snake_case, serde rename_all 处理映射
#[casbin]
#[derive(Aggregate, Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[aggregate_type("me.tdcarefor.tdnis.sickbed.domain.aggregate.SickbedEntity")]
#[id_field(id)]
#[serde(rename_all = "camelCase")]
pub struct SickbedEntity {
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

    // === IdAndDepartmentAndPatientModel ===
    pub patient_id: Option<String>,
    /// 患者姓名
    pub patient_name: Option<String>,
    /// 住院号
    pub patient_no: Option<String>,
    /// 患者病床编号
    pub patient_sickbed_no: Option<String>,
    /// 患者病床id
    pub patient_sickbed_id: Option<String>,
    /// 患者年龄
    pub patient_age: Option<String>,
    /// 患者病床排序id
    #[serde(
        default,
        serialize_with = "string_or_i32::serialize",
        deserialize_with = "string_or_i32::deserialize"
    )]
    pub patient_sickbed_order_id: Option<i32>,

    // === IdAndDepartmentAndPatientAndHisModel ===
    pub his_id: Option<String>,

    // === SickbedModel ===
    /// 病床别名
    pub sickbed_no_alias: Option<String>,
    /// 病床编号
    pub sickbed_no: Option<String>,
    /// 病房id
    pub ward_id: Option<String>,
    /// 病房名字
    pub ward_name: Option<String>,
    /// 状态 (0=空床, 1=有人)
    pub status: Option<i32>,
    /// 有效性
    pub effectiveness: Option<i32>,
    /// 病床等级
    pub bed_level: Option<i32>,
    /// 责任护士编号
    pub nurse_user_id: Option<String>,
    /// 责任医生id
    pub doctor_user_id: Option<String>,
    /// 责任医生姓名
    pub doctor_user_name: Option<String>,
    /// 负责人id
    pub response_user_id: Option<String>,
    /// 录入人id
    pub enter_user_id: Option<String>,
    /// 组号
    pub packet_bed_name: Option<String>,
    /// 录入时间
    #[salvo(schema(value_type = Option<String>))]
    #[serde(
        default,
        serialize_with = "crate::model::date_format::serialize_option_datetime",
        deserialize_with = "crate::model::date_format::deserialize_option_datetime"
    )]
    pub create_date: Option<rbdc::DateTime>,
    /// 护理单元编码
    pub ward_code: Option<String>,
    /// 核定类型
    pub approved_type: Option<String>,
    /// 性别要求 1男,2女,>=3其他
    pub sex_limit: Option<i32>,
    /// 冗余字段
    pub sex_type: Option<String>,
    /// 床位分类
    pub bed_class: Option<String>,
    /// 排序ID
    pub order_id: Option<i32>,
    /// 责任护士Name
    pub nurse_user_name: Option<String>,
    /// 备注
    pub remark: Option<String>,
    /// 床位类型（Java: SickbedModel.sickbedType）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sickbed_type: Option<String>,
    /// 组号（Java: SickbedModel.groupNo）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_no: Option<String>,
}

crud!(SickbedEntity {}, "SickbedEntity");

impl SickbedEntity {
    /// 添加床位 - 工厂方法
    pub fn add_sickbed(model: &SickbedEntity) -> Self {
        let mut entity: SickbedEntity = genies::copy!(model, SickbedEntity);
        if entity.id.is_none() {
            entity.id = Some(genies::next_id());
        }
        if entity.effectiveness.is_none() {
            entity.effectiveness = Some(1);
        }
        entity
    }

    /// 更新床位 - 返回 (self, 事件列表)
    pub fn update_sickbed(
        &mut self,
        model: &SickbedEntity,
    ) -> (SickbedEntity, Vec<SickbedUpdatedEvent>) {
        let updated: SickbedEntity = genies::copy!(model, SickbedEntity);
        let id = self.id.clone();
        *self = updated;
        self.id = id;
        // Java update 不更新 groupNo / sickbedType，保持 null
        self.group_no = None;
        self.sickbed_type = None;
        let event = SickbedUpdatedEvent::from_entity(self);
        (self.clone(), vec![event])
    }

    /// 安排床位 - 返回 (self, 事件列表)
    pub fn arrangement_sickbed(
        &mut self,
        cmd: &crate::domain::command::sickbed_commands::ArrangementSickbedCommand,
    ) -> (
        SickbedEntity,
        Vec<crate::domain::event::sickbed_events::SickbedArrangementedEvent>,
    ) {
        self.patient_id = cmd.patient_id.clone();
        self.status = Some(1);
        let event = crate::domain::event::sickbed_events::SickbedArrangementedEvent {
            patient_id: cmd.patient_id.clone(),
            sickbed_no: self.sickbed_no.clone(),
            sickbed_order_id: self.order_id,
            doctor_user_id: self.doctor_user_id.clone(),
            doctor_user_name: self.doctor_user_name.clone(),
        };
        (self.clone(), vec![event])
    }

    /// 换床 - 返回 (self, 事件列表)
    pub fn change_sickbed(
        &mut self,
        cmd: &crate::domain::command::sickbed_commands::ChangeSickbedCommand,
    ) -> (
        SickbedEntity,
        Vec<crate::domain::event::sickbed_events::SickbedChangedEvent>,
    ) {
        self.patient_id = cmd.patient_id.clone();
        self.status = Some(1);
        let event = crate::domain::event::sickbed_events::SickbedChangedEvent {
            patient_id: cmd.patient_id.clone(),
            from_sickbed_id: cmd.source_sickbed_id.clone(),
            to_sickbed_id: cmd.target_sickbed_id.clone(),
            change_date: cmd.change_date.clone(),
            sickbed_no: self.sickbed_no.clone(),
            sickbed_order_id: self.order_id,
            doctor_user_id: self.doctor_user_id.clone(),
            doctor_user_name: self.doctor_user_name.clone(),
        };
        (self.clone(), vec![event])
    }

    /// 清空床位 - 返回 (self, 事件列表)
    pub fn empty_sickbed(
        &mut self,
        _cmd: &crate::domain::command::sickbed_commands::EmptySickbedCommand,
    ) -> (
        SickbedEntity,
        Vec<crate::domain::event::sickbed_events::SickbedEmptyedEvent>,
    ) {
        let old_patient_id = self.patient_id.clone();
        self.patient_id = None;
        self.status = Some(0);
        let event = crate::domain::event::sickbed_events::SickbedEmptyedEvent {
            id: self.id.clone(),
            patient_id: old_patient_id,
        };
        (self.clone(), vec![event])
    }

    /// 测试安排床位（不回写HIS）- 返回 (self, 事件列表)
    pub fn test_arrangement_sickbed(
        &mut self,
        cmd: &crate::domain::command::sickbed_commands::TestArrangementSickbedCommand,
    ) -> (
        SickbedEntity,
        Vec<crate::domain::event::sickbed_events::SickbedTestArrangementedEvent>,
    ) {
        self.patient_id = cmd.patient_id.clone();
        self.status = Some(1);
        let event = crate::domain::event::sickbed_events::SickbedTestArrangementedEvent {
            id: cmd.sickbed_id.clone(),
            patient_id: cmd.patient_id.clone(),
            sickbed_no: self.sickbed_no.clone(),
            sickbed_order_id: self.order_id,
            doctor_user_id: self.doctor_user_id.clone(),
            doctor_user_name: self.doctor_user_name.clone(),
        };
        (self.clone(), vec![event])
    }
}
