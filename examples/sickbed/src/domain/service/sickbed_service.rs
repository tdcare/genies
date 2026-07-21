use genies::context::CONTEXT;
use genies::core::ResultDTO;
use genies::ddd::DomainEventPublisher::publish;
use genies::pool;
use rbs::value;
use crate::domain::aggregate::{SickbedEntity, WardEntity};
use crate::domain::command::sickbed_commands::*;
use crate::model::vo::DeptCount;
use crate::model::vo::UserSickbedListVo;
use crate::model::vo::{DoorwayScreenVo, PatientDoorwayScreenVo};
use crate::interfaces::handler::sickbed_controller::SickbedBriefVo;

use crate::remote::baseinfo_service::get_user_manage_beds;
use crate::remote::patient_service::find_in_host_by_sickbed_ids;

/// 床位域服务
pub struct SickbedService;

#[allow(unused_must_use)]
impl SickbedService {
    /// 添加床位
    pub async fn add_sickbed(model: SickbedEntity) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;

        // 仅当请求提供了 hisId 时才做重复检查
        if let Some(ref his_id) = model.his_id {
            let finds = SickbedEntity::select_by_map(rb, value!{"hisId": his_id})
                .await
                .unwrap_or_default();
            if !finds.is_empty() {
                let id = finds[0].id.clone().unwrap_or_default();
                return ResultDTO::success("成功添加床位", id);
            }
        }

        let mut entity = SickbedEntity::add_sickbed(&model);
        // Java 不自动设置 groupNo / sickbedType，保持 null
        entity.group_no = None;
        entity.sickbed_type = None;
        let id = entity.id.clone().unwrap_or_default();
        SickbedEntity::insert(rb, &entity).await.unwrap();

        // 复现 Java Bug：Java 第108行传入的是 entity.getId()（sickbedId），而不是 entity.getWardId()
        // findByWardIdOrderByOrderIdAsc(sickbedId) → wardId 字段不等于 sickbedId → 结果为空 → sickbedCount = 0
        Self::update_ward_sickbed_count(rb, entity.ward_id.as_deref(), entity.id.as_deref()).await;

        ResultDTO::success("成功添加床位", id)
    }

    /// 批量更新
    pub async fn batch_update(models: Vec<SickbedEntity>) -> ResultDTO<String> {
        for model in models {
            Self::update_sickbed(model).await;
        }
        ResultDTO::success("修改成功", String::new())
    }

    /// 批量删除
    pub async fn batch_delete(ids: Vec<String>) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        for id in &ids {
            SickbedEntity::delete_by_map(rb, value!{"id": id}).await.ok();
        }
        ResultDTO::success("删除成功", String::new())
    }

    /// 更新床位
    pub async fn update_sickbed(model: SickbedEntity) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        let id = match &model.id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => return ResultDTO::<String>::error("床位信息更新,传递id为空"),
        };

        let entities = SickbedEntity::select_by_map(rb, value!{"id": &id})
            .await
            .unwrap_or_default();
        let mut entity = match entities.into_iter().next() {
            Some(e) => e,
            None => return ResultDTO::from_code_message(2, "床位信息更新,根据id找不到对应床位", &id),
        };

        let mut tx = genies::tx_defer!();
        let (_updated, events) = entity.update_sickbed(&model);
        SickbedEntity::update_by_map(&mut tx, &entity, value!{"id": &id})
            .await
            .unwrap();
        for event in events {
            publish(&mut tx, &entity, Box::new(event)).await;
        }
        tx.commit().await.unwrap();

        ResultDTO::success("床位信息更新成功", String::new())
    }

    /// 安排床位
    pub async fn arrangement_sickbed(cmd: ArrangementSickbedCommand) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        let id = match &cmd.sickbed_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => return ResultDTO::<String>::error("床位信息更新,传递id为空"),
        };

        let entities = SickbedEntity::select_by_map(rb, value!{"id": &id})
            .await
            .unwrap_or_default();
        let mut entity = match entities.into_iter().next() {
            Some(e) => e,
            None => return ResultDTO::from_code_message(2, "床位信息更新,根据id找不到对应床位", &id),
        };

        // 幂等校验：床位已被占用（status=1 且 patient_id 不为空），直接返回成功（与 Java 行为一致）
        if entity.status == Some(1) {
            if let Some(ref pid) = entity.patient_id {
                if !pid.is_empty() {
                    return ResultDTO::success("安排床位完成", String::new());
                }
            }
        }

        let mut tx = genies::tx_defer!();
        let (_updated, events) = entity.arrangement_sickbed(&cmd);
        SickbedEntity::update_by_map(&mut tx, &entity, value!{"id": &id})
            .await
            .unwrap();
        for event in events {
            publish(&mut tx, &entity, Box::new(event)).await;
        }
        tx.commit().await.unwrap();

        ResultDTO::success("安排床位完成", String::new())
    }

    /// 换床（清空源床 + 安排目标床）
    pub async fn change_sickbed(cmd: ChangeSickbedCommand) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        let from_id = match &cmd.source_sickbed_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => return ResultDTO::<String>::error("换床操作,源床位id为空"),
        };
        let to_id = match &cmd.target_sickbed_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => return ResultDTO::<String>::error("换床操作,目标床位id为空"),
        };

        // 查询源床位
        let from_entities = SickbedEntity::select_by_map(rb, value!{"id": &from_id})
            .await
            .unwrap_or_default();
        let mut from_entity = match from_entities.into_iter().next() {
            Some(e) => e,
            None => return ResultDTO::from_code_message(2, "换床操作,找不到源床位", &from_id),
        };

        // 查询目标床位
        let to_entities = SickbedEntity::select_by_map(rb, value!{"id": &to_id})
            .await
            .unwrap_or_default();
        let mut to_entity = match to_entities.into_iter().next() {
            Some(e) => e,
            None => return ResultDTO::from_code_message(2, "换床操作,找不到目标床位", &to_id),
        };

        let mut tx = genies::tx_defer!();

        // 清空源床位
        let empty_cmd = EmptySickbedCommand {
            id: Some(from_id.clone()),
            sickbed_id: Some(from_id.clone()),
            patient_id: from_entity.patient_id.clone(),
        };
        let (_updated, empty_events) = from_entity.empty_sickbed(&empty_cmd);
        SickbedEntity::update_by_map(&mut tx, &from_entity, value!{"id": &from_id})
            .await
            .unwrap();
        for event in empty_events {
            publish(&mut tx, &from_entity, Box::new(event)).await;
        }

        // 安排目标床位
        let (_updated, change_events) = to_entity.change_sickbed(&cmd);
        SickbedEntity::update_by_map(&mut tx, &to_entity, value!{"id": &to_id})
            .await
            .unwrap();
        for event in change_events {
            publish(&mut tx, &to_entity, Box::new(event)).await;
        }

        tx.commit().await.unwrap();

        ResultDTO::success("换床完成", String::new())
    }

    /// 测试安排床位
    pub async fn test_arrangement_sickbed(cmd: TestArrangementSickbedCommand) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        let id = match &cmd.sickbed_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => return ResultDTO::<String>::error("床位信息更新,传递id为空"),
        };

        let entities = SickbedEntity::select_by_map(rb, value!{"id": &id})
            .await
            .unwrap_or_default();
        let mut entity = match entities.into_iter().next() {
            Some(e) => e,
            None => return ResultDTO::from_code_message(2, "床位信息更新,根据id找不到对应床位", &id),
        };

        // 前置校验：测试安排要求床位空闲（status=0），已占用时返回错误（与 Java 行为一致）
        if entity.status == Some(1) {
            return ResultDTO::<String>::error("床位当前已有患者，无法进行测试安排");
        }

        let mut tx = genies::tx_defer!();
        let (_updated, events) = entity.test_arrangement_sickbed(&cmd);
        SickbedEntity::update_by_map(&mut tx, &entity, value!{"id": &id})
            .await
            .unwrap();
        for event in events {
            publish(&mut tx, &entity, Box::new(event)).await;
        }
        tx.commit().await.unwrap();

        ResultDTO::success("测试安排床位完成", String::new())
    }

    /// 清空床位
    ///
    /// 与 Java 完全对齐：
    /// Java EmptySickbedCommand 继承 IdModel，getId() 读取 JSON 的 "id" 字段。
    /// 测试请求体传的是 "sickbedId" 字段（不是 "id"），所以 Java 的 cmd.getId() = null，
    /// 调用 findById(null) 抛出 IllegalArgumentException → HTTP 500，无 DB 变更。
    /// Rust 同样读 "id" 字段（cmd.id）；如果为空则不操作 DB。
    pub async fn empty_sickbed(cmd: EmptySickbedCommand) -> ResultDTO<String> {
        // Java: emptySickbedById(cmd.getId()) — 读取 JSON "id" 字段
        // 测试发送的是 "sickbedId"，Java 读到 null → findById(null) 抛异常 → 500
        // Rust 复现：使用 cmd.id（对应 JSON "id" 字段），为 null 则不做任何操作
        let id = match &cmd.id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => {
                // Java 行为：findById(null) 抛出 IllegalArgumentException，导致 HTTP 500
                // Rust 无法直接抛 500，但可以不操作 DB（与 Java 的 DB 变更结果一致：无变更）
                return ResultDTO::<String>::error("清空床位失败，id为空");
            }
        };

        Self::empty_sickbed_by_id(&id).await;

        ResultDTO::success("已清空床位", String::from("true"))
    }

    /// 按ID清空床位（内部辅助方法）
    async fn empty_sickbed_by_id(sickbed_id: &str) {
        let rb = &CONTEXT.rbatis;
        let entities = SickbedEntity::select_by_map(rb, value!{"id": sickbed_id})
            .await
            .unwrap_or_default();
        if let Some(mut entity) = entities.into_iter().next() {
            let cmd = EmptySickbedCommand {
                id: Some(sickbed_id.to_string()),
                sickbed_id: Some(sickbed_id.to_string()),
                patient_id: entity.patient_id.clone(),
            };
            let mut tx = genies::tx_defer!();
            let (_updated, events) = entity.empty_sickbed(&cmd);
            SickbedEntity::update_by_map(&mut tx, &entity, value!{"id": sickbed_id})
                .await
                .unwrap();
            for event in events {
                publish(&mut tx, &entity, Box::new(event)).await;
            }
            tx.commit().await.unwrap();
        }
    }

    /// 按 ID 查询
    pub async fn find_by_id(id: &str) -> Option<SickbedEntity> {
        let rb = &CONTEXT.rbatis;
        SickbedEntity::select_by_map(rb, value!{"id": id})
            .await
            .ok()?
            .into_iter()
            .next()
    }

    /// 有效床位列表
    pub async fn active_sickbeds(department_id: &str) -> Vec<SickbedEntity> {
        let rb = &CONTEXT.rbatis;
        SickbedEntity::effectiveness_sickbeds(rb, department_id)
            .await
            .unwrap_or_default()
    }

    /// 有效且空闲床位
    pub async fn active_and_empty_sickbeds(department_id: &str) -> Vec<SickbedEntity> {
        let rb = &CONTEXT.rbatis;
        SickbedEntity::effectiveness_and_empty_sickbeds(rb, department_id)
            .await
            .unwrap_or_default()
    }

    /// 所有空闲床位（有效且 status != 1）
    pub async fn all_idle_sickbeds(department_id: &str) -> Vec<SickbedEntity> {
        let rb = &CONTEXT.rbatis;
        SickbedEntity::effectiveness_and_empty_sickbeds(rb, department_id)
            .await
            .unwrap_or_default()
    }

    /// 统计科室有效床位数（effectiveness = 1，不过滤 status）
    pub async fn count_idle_sickbeds(department_id: &str) -> i64 {
        let beds = Self::active_sickbeds(department_id).await;
        beds.len() as i64
    }

    /// 按多个科室统计有效床位数
    pub async fn count_idle_sickbeds_by_list(department_ids: &[String], effectiveness: i32) -> Vec<DeptCount> {
        let rb = &CONTEXT.rbatis;
        if department_ids.is_empty() {
            return vec![];
        }
        SickbedEntity::count_by_department_ids_and_effectiveness(rb, department_ids, effectiveness)
            .await
            .unwrap_or_default()
    }

    /// 按科室清空所有占用的床位
    ///
    /// 对应 Java: emptySickbedsByDept — 按 departmentCode 查找所有床位，
    /// 设置 patientId=null, status=0，然后 JPA save（全字段更新），不检查 status。
    pub async fn empty_sickbeds_by_dept(department_id: &str) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        let sickbeds = SickbedEntity::find_by_department_code(rb, department_id)
            .await
            .unwrap_or_default();

        for mut entity in sickbeds {
            entity.patient_id = None;
            entity.status = Some(0);
            if let Some(id) = &entity.id {
                SickbedEntity::update_by_map(rb, &entity, value!{"id": id})
                    .await
                    .ok();
            }
        }

        ResultDTO::success("批量清空床位完成", String::new())
    }

    /// 清空单个床位（通过科室编码+床位号查找）
    pub async fn empty_one(department_code: &str, sickbed_no: &str) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        let sickbeds = SickbedEntity::find_by_department_code_and_sickbed_no(rb, department_code, sickbed_no)
            .await
            .unwrap_or_default();

        for mut entity in sickbeds {
            entity.patient_id = None;
            entity.status = Some(0);
            if let Some(id) = &entity.id {
                SickbedEntity::update_by_map(rb, &entity, value!{"id": id})
                    .await
                    .ok();
            }
        }

        ResultDTO::success("清空床位完成", String::new())
    }

    /// 通过扫描号查找（wardNo）
    /// Java 逻辑: 先用 scanNo (即 wardNo) 查询 WardEntity，获取 ward.id，
    /// 再查 SickbedEntity WHERE wardId = ward.id AND effectiveness = 1 ORDER BY orderId ASC
    /// 然后从患者服务获取患者姓名，脱敏后填充 patientName
    pub async fn find_by_scan_no(scan_no: &str) -> ResultDTO<Vec<SickbedEntity>> {
        let rb = &CONTEXT.rbatis;

        // 通过 wardNo 查询 WardEntity
        let wards = WardEntity::find_by_ward_no(rb, scan_no)
            .await
            .unwrap_or_default();
        let ward = match wards.into_iter().next() {
            Some(w) => w,
            None => {
                return ResultDTO::success("", vec![]);
            }
        };

        // 获取 wardId
        let ward_id = ward.id.unwrap_or_default();
        if ward_id.is_empty() {
            return ResultDTO::success("", vec![]);
        }

        // 查询 SickbedEntity WHERE wardId = ward.id ORDER BY orderId ASC（不过滤 effectiveness，与 Java 一致）
        let mut result = SickbedEntity::find_by_ward_id_order_by_order_id_asc(rb, &ward_id)
            .await
            .unwrap_or_default();

        // 与 Java 一致：对有 patientId 的床位，从患者服务获取姓名并脱敏填充 patientName
        let sickbed_ids: Vec<String> = result
            .iter()
            .filter(|s| s.patient_id.as_ref().map_or(false, |pid| !pid.is_empty()))
            .filter_map(|s| s.id.clone())
            .collect();

        if !sickbed_ids.is_empty() {
            if let Ok(patients) =
                crate::remote::patient_service::find_patients_by_sickbed_ids(&sickbed_ids).await
            {
                // 构建 sickbed_id → patient_name 映射
                let name_map: std::collections::HashMap<String, String> = patients
                    .into_iter()
                    .filter_map(|p| {
                        let sid = p.sickbed_id?;
                        let name = p.name?;
                        if name.is_empty() {
                            return None;
                        }
                        Some((sid, Self::mask_name_keep_edges(&name)))
                    })
                    .collect();

                for entity in result.iter_mut() {
                    if let Some(sid) = &entity.id {
                        if let Some(masked) = name_map.get(sid) {
                            entity.patient_name = Some(masked.clone());
                        }
                    }
                }
            }
        }

        ResultDTO::success("", result)
    }

    /// 按病房ID查询床位，返回 PatientDoorwayScreenVo 列表
    ///
    /// 对应 Java: SickbedService.findByWardId
    /// 1. 查询有效床位ID列表（按 orderId 排序）
    /// 2. 调用远程患者服务 findInHostbySickbedIds 获取门口屏 VO
    /// 3. 直接透传患者服务的 ResultDTO
    pub async fn find_by_ward_id(ward_id: &str) -> ResultDTO<Vec<PatientDoorwayScreenVo>> {
        let rb = &CONTEXT.rbatis;
        // Java: sickbedRep.findIdByWardIdOrderByOrderId(wardId)
        let sickbed_ids: Vec<String> = SickbedEntity::find_id_by_ward_id_order_by_order_id(rb, ward_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|id_only| id_only.id)
            .collect();

        if sickbed_ids.is_empty() {
            return ResultDTO::from_code_message(0, "病房信息获取失败", &vec![]);
        }

        // 调用远程患者服务获取门口屏信息（与 Java patientInfo.findInHostbySickbedIds 一致）
        match find_in_host_by_sickbed_ids(&sickbed_ids).await {
            Ok(result_dto) => result_dto,
            Err(e) => {
                log::warn!("find_by_ward_id: 远程调用患者服务失败: {}", e);
                // 降级：仅返回床位基础信息
                let sickbeds = SickbedEntity::find_by_ward_id_order_by_order_id_asc(rb, ward_id)
                    .await
                    .unwrap_or_default();
                let fallback: Vec<PatientDoorwayScreenVo> = sickbeds
                    .into_iter()
                    .map(|e| Self::sickbed_to_patient_doorway_screen_vo(&e, false))
                    .collect();
                ResultDTO::success("患者查询成功", fallback)
            }
        }
    }

    /// 按病房ID查询患者ID列表
    pub async fn find_patient_id_by_ward_id(ward_id: &str) -> Vec<String> {
        let rb = &CONTEXT.rbatis;
        SickbedEntity::find_patient_id_by_ward_id_order_by_order_id(rb, ward_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| p.patient_id)
            .collect()
    }

    /// 按病房ID查询床位信息，返回 DoorwayScreenVo
    ///
    /// 对应 Java: SickbedService.findSickbedInfoByWardId
    /// 1. 查询 WardEntity 获取病房/科室信息
    /// 2. 查询该 ward 下的 SickbedEntity，仅映射 sickbedId/sickbedNo/sickbedOrderId
    /// 3. 组装 DoorwayScreenVo
    pub async fn find_sickbed_info_by_ward_id(ward_id: &str) -> DoorwayScreenVo {
        let rb = &CONTEXT.rbatis;

        // 查询病房实体
        let mut vo = DoorwayScreenVo::default();
        if let Ok(wards) = WardEntity::select_by_map(rb, value!{"id": ward_id}).await {
            if let Some(ward) = wards.into_iter().next() {
                vo.ward_id = Some(ward_id.to_string());
                vo.ward_name = ward.ward_name;
                vo.department_id = ward.department_id;
                vo.department_name = ward.department_name;
                vo.department_abstract = ward.department_abstract;
                vo.department_code = ward.department_code;
                // response_people_name / headurse_name 等 Java 端也是从 WardEntity 拷贝
                // WardEntity 无这些字段，保持 None
            }
        }

        // 查询该 ward 下的床位，仅映射基础床位信息（Java 端此接口不返回 nurse/doctor）
        let sickbeds = SickbedEntity::find_by_ward_id_order_by_order_id_asc(rb, ward_id)
            .await
            .unwrap_or_default();
        let patient_list: Vec<PatientDoorwayScreenVo> = sickbeds
            .into_iter()
            .map(|e| Self::sickbed_to_patient_doorway_screen_vo(&e, false))
            .collect();
        vo.patient_doorway_screen_vo_list = Some(patient_list);

        vo
    }

    /// 查询同病房所有床位ID
    /// 返回 Result，由 controller 层处理 fallback 逻辑
    pub async fn find_ward_sickbed_id_by_sickbed_id(sickbed_id: &str) -> Result<Vec<String>, String> {
        let rb = &CONTEXT.rbatis;
        let entities = SickbedEntity::find_ward_sickbed_id_by_sickbed_id(rb, sickbed_id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(entities.into_iter().filter_map(|i| i.id).collect())
    }

    /// 获取所有床位（简要信息）
    pub async fn get_all_bed() -> Vec<SickbedBriefVo> {
        let rb = &CONTEXT.rbatis;
        let all = SickbedEntity::find_all(rb)
            .await
            .unwrap_or_default();
        all.into_iter()
            .map(|e| SickbedBriefVo {
                id: e.id,
                bed_no: e.sickbed_no,
                state: e.status,
                department_id: e.department_id,
                responsible_person_id: e.response_user_id,
                order_no: e.order_id,
            })
            .collect()
    }

    /// 清除科室中指定护士的绑定
    pub async fn empty_nurse(department_id: &str, user_id: &str) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        SickbedEntity::update_sickbed_entity_department_id(rb, department_id, user_id)
            .await
            .ok();
        ResultDTO::success("清除护士绑定完成", String::new())
    }

    /// 分页搜索床位（条件过滤）
    pub async fn effectiveness_for_search(
        department_ids: &[String],
        ward_id: &str,
        sickbed_no: &str,
        page_index: u64,
        page_size: u64,
    ) -> genies::core::page::SpringPage<SickbedEntity> {
        let rb = &CONTEXT.rbatis;
        let all = SickbedEntity::effectiveness_for_search(rb, department_ids, ward_id, sickbed_no)
            .await
            .unwrap_or_default();

        // 内存分页
        let total = all.len() as u64;
        let content: Vec<SickbedEntity> = all
            .into_iter()
            .skip((page_index * page_size) as usize)
            .take(page_size as usize)
            .collect();

        let page_size_i64 = page_size as i64;
        let total_i64 = total as i64;
        let total_pages = if page_size_i64 > 0 {
            (total_i64 + page_size_i64 - 1) / page_size_i64
        } else {
            0
        };
        let number = page_index as i64;
        let number_of_elements = content.len() as i64;
        let sort = genies::core::page::Sort::unsorted();

        genies::core::page::SpringPage {
            content,
            pageable: genies::core::page::Pageable {
                page_number: number,
                page_size: page_size_i64,
                sort: sort.clone(),
                offset: number * page_size_i64,
                paged: true,
                unpaged: false,
            },
            last: number >= total_pages - 1,
            total_pages,
            total_elements: total_i64,
            first: number == 0,
            size: page_size_i64,
            number,
            sort,
            number_of_elements,
            empty: number_of_elements == 0,
        }
    }

    /// 查询床位列表，并标记当前用户是否管理该床位
    pub async fn find_by_sickbed_list(ids: Vec<String>, user_id: &str) -> Vec<UserSickbedListVo> {
        let rb = &CONTEXT.rbatis;
        if ids.is_empty() {
            return vec![];
        }
        let sickbeds = SickbedEntity::find_by_id_in(rb, &ids)
            .await
            .unwrap_or_default();

        // 调用远程服务获取用户管理的床位列表
        let managed_bed_ids: Vec<String> = if !user_id.is_empty() {
            match get_user_manage_beds(user_id).await {
                Ok(manage_beds) => {
                    manage_beds
                        .into_iter()
                        .filter_map(|b| b.sickbed_id)
                        .collect()
                }
                Err(e) => {
                    log::warn!("find_by_sickbed_list: 获取用户管床信息失败: {}", e);
                    vec![]
                }
            }
        } else {
            vec![]
        };

        sickbeds
            .into_iter()
            .map(|bed| {
                // 根据用户管床列表判断当前床位是否被管理
                let is_managed = bed.id.as_ref()
                    .map(|id| managed_bed_ids.contains(id))
                    .unwrap_or(false);
                UserSickbedListVo {
                    sickbed: bed,
                    managed: Some(is_managed),
                }
            })
            .collect()
    }

    /// 转科处理 - 清空该患者在原科室的床位
    pub async fn out_dept_record_info(patient_id: &str) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        let sickbeds = SickbedEntity::find_by_patient_id(rb, patient_id)
            .await
            .unwrap_or_default();

        for bed in sickbeds {
            if let Some(id) = &bed.id {
                Self::empty_sickbed_by_id(id).await;
            }
        }

        ResultDTO::success("转科处理完成", String::new())
    }

    /// 出院处理 - 清空该患者的床位
    pub async fn leaved_hosp(patient_id: &str) -> ResultDTO<String> {
        Self::out_dept_record_info(patient_id).await
    }

    /// 更新床位医生信息
    pub async fn upd_sickbed_doctor_user_name(
        sickbed_id: &str,
        doctor_user_id: &str,
        doctor_user_name: &str,
    ) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        SickbedEntity::upd_sickbed_doctor_user_name(rb, sickbed_id, doctor_user_id, doctor_user_name)
            .await
            .ok();
        ResultDTO::success("更新医生信息完成", String::new())
    }

    /// 初始化床位信息到 Redis 缓存
    /// 查询所有有效床位（effectiveness=1），序列化为 JSON 后存入 Redis
    pub async fn init_sickbed_info() {
        log::info!("init_sickbed_info: 开始初始化床位缓存");
        let rb = &CONTEXT.rbatis;
        let beds = SickbedEntity::find_by_effectiveness(rb, 1)
            .await
            .unwrap_or_default();
        let cache = &CONTEXT.cache_service;
        let json = serde_json::to_string(&beds).unwrap_or_default();
        match cache.set_string("sickbed:all", &json).await {
            Ok(_) => log::info!("init_sickbed_info: 床位缓存初始化完成, 共 {} 条记录", beds.len()),
            Err(e) => log::error!("init_sickbed_info: 写入 Redis 缓存失败: {}", e),
        }
    }

    // ===== 内部辅助方法 =====

    /// 更新病房的床位数
    ///
    /// `ward_id`: 要更新的 WardEntity 的 ID
    /// `query_id`: 用于查询床位数的 ID（复现 Java Bug 时传 sickbedId，正常情况传 wardId）
    async fn update_ward_sickbed_count(
        rb: &rbatis::RBatis,
        ward_id: Option<&str>,
        query_id: Option<&str>,
    ) {
        if let Some(ward_id) = ward_id {
            if !ward_id.is_empty() {
                if let Ok(wards) = WardEntity::select_by_map(rb, value!{"id": ward_id}).await {
                    if let Some(mut ward) = wards.into_iter().next() {
                        // 用 query_id 查询床位数（Java Bug 复现：传 sickbedId 时结果为空，count = 0）
                        let effective_query_id = query_id.unwrap_or(ward_id);
                        let beds =
                            SickbedEntity::find_by_ward_id_order_by_order_id_asc(rb, effective_query_id)
                                .await
                                .unwrap_or_default();
                        ward.sickbed_count = Some(beds.len() as i32);
                        WardEntity::update_by_map(rb, &ward, value!{"id": ward_id}).await.ok();
                    }
                }
            }
        }
    }

    /// SickbedEntity → PatientDoorwayScreenVo 转换
    ///
    /// `with_patient_info`: true 时填充患者/护士/医生信息（findByWardId 场景），
    ///                       false 时仅填充床位基础信息（findSickbedInfoByWardId 场景）。
    fn sickbed_to_patient_doorway_screen_vo(
        entity: &SickbedEntity,
        with_patient_info: bool,
    ) -> PatientDoorwayScreenVo {
        let mut vo = PatientDoorwayScreenVo {
            sickbed_id: entity.id.clone(),
            sickbed_no: entity.sickbed_no.clone(),
            sickbed_order_id: entity.order_id,
            ..Default::default()
        };
        if with_patient_info {
            vo.patient_id = entity.patient_id.clone();
            vo.name = entity.patient_name.as_ref().map(|n| Self::mask_name(n));
            vo.patient_no = entity.patient_no.clone();
            vo.nurse_user_id = entity.nurse_user_id.clone();
            vo.nurse_user_name = entity.nurse_user_name.clone();
            vo.doctor_user_id = entity.doctor_user_id.clone();
            vo.doctor_user_name = entity.doctor_user_name.clone();
            // nurseUserNameImageAddress / doctorImageAddress 需要额外查询用户服务，暂留 None
        }
        vo
    }

    /// 姓名脱敏 — 与 Java 端逻辑一致
    ///
    /// 4 字及以上保留前 2 字，其余用 * 替代；
    /// 3 字及以下保留第 1 字，其余用 * 替代。
    fn mask_name(full_name: &str) -> String {
        if full_name.is_empty() {
            return String::new();
        }
        let chars: Vec<char> = full_name.chars().collect();
        let len = chars.len();
        let keep = if len >= 4 { 2 } else { 1 };
        let mut masked: String = chars[..keep].iter().collect();
        for _ in keep..len {
            masked.push('*');
        }
        masked
    }

    /// 姓名脱敏 — 保留首尾字，中间用 "x" 替代
    ///
    /// 与 Java findByScanNo 返回格式一致：
    /// - 1 字: 原样返回
    /// - 2 字: 首字 + "x"
    /// - 3 字及以上: 首字 + "x" + 尾字（如 "郑达衡" → "郑x衡"）
    fn mask_name_keep_edges(full_name: &str) -> String {
        if full_name.is_empty() {
            return String::new();
        }
        let chars: Vec<char> = full_name.chars().collect();
        let len = chars.len();
        match len {
            1 => chars[0].to_string(),
            2 => format!("{}x", chars[0]),
            _ => format!("{}x{}", chars[0], chars[len - 1]),
        }
    }
}
