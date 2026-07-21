use genies::context::CONTEXT;
use genies::core::ResultDTO;
use genies::core::page::SpringPage;
use rbs::value;
use crate::domain::aggregate::{SickbedEntity, WardEntity, WardNoEntity};
use crate::model::vo::WardBriefVo;
use crate::model::vo::WardRelevanceSickbedVo;

/// 病房域服务
pub struct WardService;

impl WardService {
    /// 添加病房（含自动生成病房号 + 关联床位）
    pub async fn add(vo: WardRelevanceSickbedVo, department_id: &str) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;

        // 生成病房号：先插入 WardNoEntity 获取自增 id，再拼接 ward_no
        let ward_no_entity = WardNoEntity {
            id: None,
            name: None,
            ward_no: None,
            type_field: Some("ward".to_string()),
        };
        if let Err(e) = WardNoEntity::insert(rb, &ward_no_entity).await {
            log::error!("插入 WardNoEntity 失败: {}", e);
            return ResultDTO::error(&format!("生成病房号失败: {}", e));
        }

        // 查出刚插入的记录（取最后一条）获取自增ID作为病房号
        let ward_no_list = WardNoEntity::select_by_map(rb, value!{"type": "ward"})
            .await
            .unwrap_or_default();
        let auto_no = ward_no_list
            .last()
            .and_then(|w| w.id)
            .unwrap_or(1);
        let ward_no = format!("W{:04}", auto_no);

        // 创建 WardEntity
        let ward_id = genies::next_id();
        let mut ward = vo.ward;
        ward.id = Some(ward_id.clone());
        ward.ward_no = Some(ward_no);
        ward.department_id = Some(department_id.to_string());
        if ward.effectiveness.is_none() {
            ward.effectiveness = Some(1);
        }
        ward.create_date = Some(rbdc::types::datetime::DateTime::now());

        // 计算关联床位数
        let sickbed_count = vo.sickbed_count_add.unwrap_or(0);
        ward.sickbed_count = Some(sickbed_count);

        if let Err(e) = WardEntity::insert(rb, &ward).await {
            log::error!("插入 WardEntity 失败: {}", e);
            return ResultDTO::error(&format!("添加病房失败: {}", e));
        }

        // 如果有关联床位列表则创建
        if let Some(sickbed_list) = vo.sickbed_model_list {
            for mut bed in sickbed_list {
                bed.ward_id = Some(ward_id.clone());
                bed.ward_name = ward.ward_name.clone();
                bed.department_id = Some(department_id.to_string());
                bed.department_name = ward.department_name.clone();
                bed.department_code = ward.department_code.clone();
                let entity = SickbedEntity::add_sickbed(&bed);
                SickbedEntity::insert(rb, &entity).await.ok();
            }
        } else if sickbed_count > 0 {
            // 自动创建床位
            for i in 1..=sickbed_count {
                let bed = SickbedEntity {
                    id: Some(genies::next_id()),
                    department_id: Some(department_id.to_string()),
                    department_name: ward.department_name.clone(),
                    department_code: ward.department_code.clone(),
                    ward_id: Some(ward_id.clone()),
                    ward_name: ward.ward_name.clone(),
                    sickbed_no: Some(format!("{:02}", i)),
                    status: Some(0),
                    effectiveness: Some(1),
                    order_id: Some(i),
                    ..Default::default()
                };
                SickbedEntity::insert(rb, &bed).await.ok();
            }
        }

        ResultDTO::success("成功添加病房", ward_id)
    }

    /// 更新病房
    pub async fn update(vo: WardRelevanceSickbedVo) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        let id = match &vo.ward.id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => return ResultDTO::<String>::error("病房id为空"),
        };

        let wards = WardEntity::select_by_map(rb, value!{"id": &id})
            .await
            .unwrap_or_default();
        let mut ward = match wards.into_iter().next() {
            Some(w) => w,
            None => return ResultDTO::from_code_message(2, "根据id找不到对应病房", &id),
        };

        // 更新病房基本信息
        let updated: WardEntity = genies::copy!(&vo.ward, WardEntity);
        let ward_id_saved = ward.id.clone();
        ward = updated;
        ward.id = ward_id_saved;

        // 更新床位数（仅更新 WardEntity 的 sickbed_count，不级联修改 SickbedEntity）
        let bed_count = SickbedEntity::find_by_ward_id_order_by_order_id_asc(rb, &id)
            .await
            .unwrap_or_default()
            .len() as i32;
        ward.sickbed_count = Some(bed_count);

        if let Err(e) = WardEntity::update_by_map(rb, &ward, value!{"id": &id}).await {
            log::error!("更新 WardEntity 失败: {}", e);
            return ResultDTO::error(&format!("更新病房失败: {}", e));
        }

        ResultDTO::success("病房信息更新成功", String::new())
    }

    /// 删除病房
    pub async fn delete(id: &str) -> ResultDTO<String> {
        let rb = &CONTEXT.rbatis;
        // 删除关联床位
        let beds = SickbedEntity::find_by_ward_id_order_by_order_id_asc(rb, id)
            .await
            .unwrap_or_default();
        for bed in &beds {
            if let Some(bed_id) = &bed.id {
                SickbedEntity::delete_by_map(rb, value!{"id": bed_id}).await.ok();
            }
        }

        // 删除病房
        WardEntity::delete_by_map(rb, value!{"id": id}).await.ok();

        ResultDTO::success("删除病房成功", String::new())
    }

    /// 按ID查询
    pub async fn find_by_id(id: &str) -> Option<WardEntity> {
        let rb = &CONTEXT.rbatis;
        WardEntity::select_by_map(rb, value!{"id": id})
            .await
            .ok()?
            .into_iter()
            .next()
    }

    /// 查询病房+关联床位
    pub async fn find_ward_relevance_sickbed(id: &str) -> Option<WardRelevanceSickbedVo> {
        let rb = &CONTEXT.rbatis;
        let ward = WardEntity::select_by_map(rb, value!{"id": id})
            .await
            .ok()?
            .into_iter()
            .next()?;

        let beds = SickbedEntity::find_by_ward_id_order_by_order_id_asc(rb, id)
            .await
            .unwrap_or_default();

        Some(WardRelevanceSickbedVo {
            ward,
            sickbed_count_add: None,
            sickbed_model_list: Some(beds),
        })
    }

    /// 有效病房列表
    pub async fn effectiveness_wards(department_id: &str) -> Vec<WardEntity> {
        let rb = &CONTEXT.rbatis;
        WardEntity::effectiveness_wards(rb, department_id)
            .await
            .unwrap_or_default()
    }

    /// 分页搜索病房
    pub async fn effectiveness_for_search(
        department_id: &str,
        ward_name: &str,
        ward_type: &str,
        page_index: u64,
        page_size: u64,
    ) -> SpringPage<WardEntity> {
        let rb = &CONTEXT.rbatis;
        let all = WardEntity::effectiveness_for_search(rb, department_id, ward_name, ward_type)
            .await
            .unwrap_or_default();

        // 内存分页
        let total = all.len() as u64;
        let content: Vec<WardEntity> = all
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

        SpringPage {
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

    /// VO简化列表（id + wardName）
    /// Java 端使用 String.valueOf(object[1]) 将 null 转为字符串 "null"，此处对齐
    pub async fn effectiveness_ward_vo(department_id: &str) -> Vec<WardBriefVo> {
        let rb = &CONTEXT.rbatis;
        let mut vos = WardEntity::effectiveness_ward_vo(rb, department_id)
            .await
            .unwrap_or_default();
        // Java's String.valueOf(null) returns the literal string "null"
        for vo in &mut vos {
            if vo.ward_name.is_none() {
                vo.ward_name = Some("null".to_string());
            }
        }
        vos
    }

    /// 条件搜索病房
    pub async fn find_all_by_search(
        department_id: &str,
        ward_name: &str,
        ward_type: &str,
    ) -> Vec<WardEntity> {
        let rb = &CONTEXT.rbatis;
        WardEntity::effectiveness_for_search(rb, department_id, ward_name, ward_type)
            .await
            .unwrap_or_default()
    }
}
