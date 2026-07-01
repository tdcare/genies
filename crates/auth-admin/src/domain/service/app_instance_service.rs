//! 应用实例域服务
//!
//! 封装应用实例聚合根的持久化操作。
//!
//! 心跳机制：
//! - 自注册信息写入 DB（持久化记录）。
//! - 心跳通过 Redis `SET key EX TTL` 实现，避免 DB 锁争用。
//! - 清理任务对比 DB 在线实例与 Redis 心跳 key 判断存活状态。

use std::time::Duration;

use crate::domain::entity::app_instance_entity::AppInstanceEntity;
use crate::domain::entity::application_entity::ApplicationEntity;
use genies::context::CONTEXT;
use rbatis::rbdc::db::ExecResult;

/// Redis 心跳 key 前缀
const REDIS_HEARTBEAT_PREFIX: &str = "auth:instance:heartbeat:";
/// 心跳 TTL（秒），留 3 倍心跳间隔的余量
const HEARTBEAT_TTL_SECS: u64 = 90;
/// 删除离线超过此时间（秒）的 DB 记录
const DELETE_OFFLINE_THRESHOLD_SECS: i64 = 3600;

pub struct AppInstanceDomainService;

impl AppInstanceDomainService {
    /// 注册或更新实例（按 instance_id upsert）
    ///
    /// 存在则更新 base_url/version/status/last_heartbeat_at，不存在则插入。
    /// 同时检查 app_name 是否在 auth_applications 表中存在，不存在则自动创建。
    /// 注册成功后立即写入 Redis 心跳 key。
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

        // 注册成功后立即写入 Redis 心跳
        Self::set_redis_heartbeat(instance_id).await.ok();

        Ok(())
    }

    /// 心跳更新（Redis）
    ///
    /// 在 Redis 中写入 `auth:instance:heartbeat:{instance_id}` key，
    /// TTL = 90 秒（3 倍心跳间隔）。key 过期视为实例离线。
    pub async fn heartbeat(instance_id: i64) -> Result<(), String> {
        Self::set_redis_heartbeat(instance_id).await
    }

    /// 写入 Redis 心跳 key
    async fn set_redis_heartbeat(instance_id: i64) -> Result<(), String> {
        let cache = &CONTEXT.cache_service;
        let key = format!("{}{}", REDIS_HEARTBEAT_PREFIX, instance_id);
        cache
            .set_string_ex(&key, "1", Some(Duration::from_secs(HEARTBEAT_TTL_SECS)))
            .await
            .map(|_| ())
            .map_err(|e| format!("Redis heartbeat failed: {}", e))
    }

    /// 注销实例（设置 status=0 + 清除 Redis 心跳）
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

        // 清除 Redis 心跳 key
        let cache = &CONTEXT.cache_service;
        let key = format!("{}{}", REDIS_HEARTBEAT_PREFIX, instance_id);
        cache.del_string(&key).await.ok();

        Ok(())
    }

    /// 清理过期实例（基于 Redis 心跳检测）
    ///
    /// 从 DB 查询所有在线实例，逐一检查 Redis 心跳 key：
    /// - key 存在 → 实例存活
    /// - key 不存在（已过期）→ 标记 DB 记录为离线
    pub async fn cleanup_stale() -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let cache = &CONTEXT.cache_service;

        let online_instances = AppInstanceEntity::select_online(rb)
            .await
            .map_err(|e| format!("Failed to query online instances: {}", e))?;

        let mut offline_count = 0u32;
        for instance in &online_instances {
            if let Some(instance_id) = &instance.instance_id {
                let key = format!("{}{}", REDIS_HEARTBEAT_PREFIX, instance_id);
                let result = cache.get_string(&key).await.unwrap_or_default();
                if result.is_empty() {
                    // 心跳 key 已过期 → 标记离线
                    match AppInstanceEntity::mark_offline_by_id(rb, instance_id).await {
                        Ok(_) => offline_count += 1,
                        Err(e) => log::warn!(
                            "[auth-admin] Failed to mark instance {} offline: {}",
                            instance_id, e
                        ),
                    }
                }
            }
        }

        if offline_count > 0 {
            log::info!("[auth-admin] Marked {} stale instances offline", offline_count);
        }
        Ok(())
    }

    /// 删除离线超过指定时间的实例
    pub async fn delete_stale_instances() -> Result<ExecResult, String> {
        let rb = &CONTEXT.rbatis;
        AppInstanceEntity::delete_offline_older_than(rb, &DELETE_OFFLINE_THRESHOLD_SECS)
            .await
            .map_err(|e| e.to_string())
    }
}
