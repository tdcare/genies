//! 设置领域服务 — 缓存穿透读取 + 写入失效

use std::time::Duration;

use genies::context::CONTEXT;
use serde::de::DeserializeOwned;

use crate::domain::entity::settings_entity::AdminSetting;

pub struct SettingsDomainService;

impl SettingsDomainService {
    const CACHE_PREFIX: &'static str = "auth:settings:";
    const CACHE_TTL: Duration = Duration::from_secs(3600);

    /// 获取单个设置值（缓存 → DB → 缓存回填）
    pub async fn get(key: &str) -> Result<serde_json::Value, String> {
        let cache_key = format!("{}{}", Self::CACHE_PREFIX, key);

        // 1. 查缓存
        let cached = CONTEXT.cache_service.get_string(&cache_key).await.unwrap_or_default();
        if !cached.is_empty() {
            return serde_json::from_str(&cached).map_err(|e| e.to_string());
        }

        // 2. 查 DB
        let rb = &CONTEXT.rbatis;
        let setting = AdminSetting::find_by_key(rb, key)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("设置项不存在: {}", key))?;

        // 3. 回填缓存
        let _ = CONTEXT.cache_service
            .set_string_ex(&cache_key, &setting.setting_value.to_string(), Some(Self::CACHE_TTL))
            .await;

        Ok(setting.setting_value)
    }

    /// 泛型方式获取设置
    pub async fn get_typed<T: DeserializeOwned>(key: &str) -> Result<T, String> {
        let value = Self::get(key).await?;
        serde_json::from_value(value).map_err(|e| format!("设置 {} 反序列化失败: {}", key, e))
    }

    /// 写入设置（DB upsert + 失效对应缓存）
    pub async fn set(key: &str, value: &serde_json::Value, description: &str) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let value_str = value.to_string();

        AdminSetting::upsert(rb, key, &value_str, description)
            .await
            .map_err(|e| e.to_string())?;

        // 失效缓存
        let cache_key = format!("{}{}", Self::CACHE_PREFIX, key);
        let _ = CONTEXT.cache_service.del_string(&cache_key).await;

        Ok(())
    }

    /// 获取全部设置
    pub async fn list_all() -> Result<Vec<AdminSetting>, String> {
        let rb = &CONTEXT.rbatis;
        AdminSetting::list_all(rb).await.map_err(|e| e.to_string())
    }
}
