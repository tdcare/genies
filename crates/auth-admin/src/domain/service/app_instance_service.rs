//! 应用实例域服务
//!
//! 封装应用实例聚合根的持久化操作，
//! 所有方法内部自行管理事务边界。

use crate::domain::entity::app_instance_entity::AppInstanceEntity;
use crate::domain::entity::application_entity::ApplicationEntity;
use genies::context::CONTEXT;
use rbatis::rbdc::db::ExecResult;

pub struct AppInstanceDomainService;

impl AppInstanceDomainService {
    /// 注册或更新实例（按 instance_id upsert）
    ///
    /// 存在则更新 base_url/version/status/last_heartbeat_at，不存在则插入。
    /// 同时检查 app_name 是否在 auth_applications 表中存在，不存在则自动创建。
    pub async fn register_or_update(entity: &AppInstanceEntity) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        let instance_id = entity.instance_id.ok_or("instance_id is required")?;
        let app_name = entity.app_name.as_deref().unwrap_or_default();

        // 检查 app_name 是否在 auth_applications 表中存在，不存在则自动创建
        if !app_name.is_empty() {
            let existing_app = ApplicationEntity::find_by_app_name(&tx, app_name)
                .await
                .map_err(|e| e.to_string())?;
            if existing_app.is_none() {
                let app_base_url = entity.base_url.as_deref().unwrap_or_default();
                if app_base_url.is_empty() {
                    tx.rollback().await.map_err(|e| e.to_string())?;
                    return Err("无法自动创建应用：base_url 不能为空".to_string());
                }
                ApplicationEntity::insert_app(&tx, app_name, app_name, "", app_base_url, &1)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }

        // 查询是否已存在该 instance_id
        let existing = AppInstanceEntity::select_by_instance_id(&tx, &instance_id)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(mut old) = existing.into_iter().next() {
            // 存在则更新
            old.base_url = entity.base_url.clone();
            old.version = entity.version.clone();
            old.status = Some(1);
            old.last_heartbeat_at = Some(rbdc::DateTime::now());
            old.app_name = entity.app_name.clone();
            old.metadata = entity.metadata.clone();
            AppInstanceEntity::update_by_map(&tx, &old, "id".into())
                .await
                .map_err(|e| e.to_string())?;
        } else {
            // 不存在则插入
            let mut new_entity = entity.clone();
            new_entity.status = Some(1);
            new_entity.registered_at = Some(rbdc::DateTime::now());
            new_entity.last_heartbeat_at = Some(rbdc::DateTime::now());
            AppInstanceEntity::insert(&tx, &new_entity)
                .await
                .map_err(|e| e.to_string())?;
        }

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 心跳更新
    pub async fn heartbeat(instance_id: i64) -> Result<ExecResult, String> {
        let rb = &CONTEXT.rbatis;
        AppInstanceEntity::update_heartbeat(rb, &instance_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// 注销实例（设置 status=0）
    pub async fn deregister(instance_id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        let existing = AppInstanceEntity::select_by_instance_id(&tx, &instance_id)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(mut entity) = existing.into_iter().next() {
            entity.status = Some(0);
            AppInstanceEntity::update_by_map(&tx, &entity, "id".into())
                .await
                .map_err(|e| e.to_string())?;
        }

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 清理过期实例（将超过阈值未心跳的实例标记为离线）
    pub async fn cleanup_stale(threshold_seconds: i64) -> Result<ExecResult, String> {
        let rb = &CONTEXT.rbatis;
        AppInstanceEntity::mark_stale_offline(rb, &threshold_seconds)
            .await
            .map_err(|e| e.to_string())
    }

    /// 删除离线超过指定时间的实例
    pub async fn delete_stale_instances(threshold_seconds: i64) -> Result<ExecResult, String> {
        let rb = &CONTEXT.rbatis;
        AppInstanceEntity::delete_offline_older_than(rb, &threshold_seconds)
            .await
            .map_err(|e| e.to_string())
    }
}
