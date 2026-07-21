#![allow(non_snake_case)]

use genies_derive::topic;
use rbs::value;
use crate::domain::event::patient_events::*;
use crate::domain::aggregate::sickbed_entity::SickbedEntity;
use crate::remote::patient_service;

/// 患者入院事件 → 更新床位医生信息
#[topic(
    name = "me.tdcarefor.tdnis.patient.domain.aggregate.PatientEntity",
    pubsub = "messagebus"
)]
pub async fn on_patient_in_hospitaled(tx: &mut dyn Executor, event: PatientInHospitaledEvent) -> anyhow::Result<u64> {
    let sickbed_id = match &event.sickbed_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            log::error!("无效入院更新床位事件->患者：{:?}", event.patient_id);
            return Ok(0);
        }
    };
    let entities = SickbedEntity::select_by_map(tx, value!{"id": &sickbed_id}).await?;
    if let Some(mut entity) = entities.into_iter().next() {
        let changed = entity.doctor_user_id != event.doctor_user_id;
        if changed {
            entity.doctor_user_id = event.doctor_user_id;
            entity.doctor_user_name = event.doctor_user_name;
            SickbedEntity::update_by_map(tx, &entity, value!{"id": &sickbed_id}).await?;
            log::info!("更新管床医生事件完成，床位: {}", sickbed_id);
        }
    }
    Ok(0)
}

/// 患者转科事件 → 清空原床位、占用新床位
#[topic(
    name = "me.tdcarefor.tdnis.patient.domain.aggregate.PatientEntity",
    pubsub = "messagebus"
)]
pub async fn on_patient_transfer(tx: &mut dyn Executor, event: PatientTransferEvent) -> anyhow::Result<u64> {
    // 清空原科室床位
    if let Some(from_sickbed_id) = &event.from_sickbed_id {
        if !from_sickbed_id.is_empty() {
            let entities = SickbedEntity::select_by_map(tx, value!{"id": from_sickbed_id}).await?;
            if let Some(mut entity) = entities.into_iter().next() {
                entity.status = Some(0);
                entity.patient_id = None;
                entity.doctor_user_id = None;
                entity.doctor_user_name = None;
                SickbedEntity::update_by_map(tx, &entity, value!{"id": from_sickbed_id}).await?;
            }
        }
    }
    // 占用新床位
    if let Some(sickbed_id) = &event.sickbed_id {
        if !sickbed_id.is_empty() {
            let entities = SickbedEntity::select_by_map(tx, value!{"id": sickbed_id}).await?;
            if let Some(mut entity) = entities.into_iter().next() {
                entity.status = Some(1);
                entity.patient_id = event.patient_id.clone();
                // 从患者信息服务获取医生信息
                if let Some(patient_id) = &event.patient_id {
                    match patient_service::find_in_patient_by_id(patient_id).await {
                        Ok(patient) => {
                            entity.doctor_user_id = patient.doctor_user_id;
                            entity.doctor_user_name = patient.doctor_user_name;
                        }
                        Err(e) => {
                            log::error!("转科事件获取患者医生信息失败, patient_id: {}, error: {}", patient_id, e);
                        }
                    }
                }
                SickbedEntity::update_by_map(tx, &entity, value!{"id": sickbed_id}).await?;
            }
        }
    }
    Ok(0)
}

/// 患者出院事件 → 清空所有该患者的床位
#[topic(
    name = "me.tdcarefor.tdnis.patient.domain.aggregate.PatientEntity",
    pubsub = "messagebus"
)]
pub async fn on_patient_leaved_hospital(tx: &mut dyn Executor, event: PatientLeavedHospitalEvent) -> anyhow::Result<u64> {
    let patient_id = match &event.patient_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => return Ok(0),
    };
    let entities = SickbedEntity::select_by_map(tx, value!{"patientId": &patient_id}).await?;
    for mut entity in entities {
        let entity_id = entity.id.clone();
        entity.status = Some(0);
        entity.patient_id = None;
        entity.doctor_user_id = None;
        entity.doctor_user_name = None;
        SickbedEntity::update_by_map(tx, &entity, value!{"id": entity_id}).await?;
    }
    Ok(0)
}

/// 床位医生变更事件
#[topic(
    name = "me.tdcarefor.tdnis.patient.domain.aggregate.PatientEntity",
    pubsub = "messagebus"
)]
pub async fn on_sickbed_doctor_changed(tx: &mut dyn Executor, event: SickbedDoctorChangedEvent) -> anyhow::Result<u64> {
    let sickbed_id = match &event.sickbed_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            log::error!("无效患者更新床位事件");
            return Ok(0);
        }
    };
    let entities = SickbedEntity::select_by_map(tx, value!{"id": &sickbed_id}).await?;
    if let Some(mut entity) = entities.into_iter().next() {
        let changed = entity.doctor_user_id != event.doctor_user_id;
        if changed {
            entity.doctor_user_id = event.doctor_user_id;
            entity.doctor_user_name = event.doctor_user_name;
            SickbedEntity::update_by_map(tx, &entity, value!{"id": &sickbed_id}).await?;
            log::info!("更新管床医生事件完成，床位: {}", sickbed_id);
        }
    }
    Ok(0)
}
