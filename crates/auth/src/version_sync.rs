//! Auth 模块的版本同步层
//!
//! 提供 Enforcer 多实例同步的版本控制机制。
//!
//! # 功能
//! - 版本号管理：当策略发生变更时更新版本号，通知其他实例重载

use genies::context::CONTEXT;

// ============================================================================
// Redis Key 常量定义
// ============================================================================

/// Enforcer 版本号 Key（用于多实例缓存失效同步）
const KEY_ENFORCER_VERSION: &str = "auth:enforcer_ver";

// ============================================================================
// 缓存失效与版本控制
// ============================================================================

/// 更新版本号并触发多实例同步
///
/// 当策略发生变更时调用此函数，更新版本号通知其他实例需要重载。
///
/// 其他实例可以通过轮询版本号来检测缓存失效，并重新加载数据。
///
/// # Returns
/// * `Ok(())` - 版本号更新成功
/// * `Err(_)` - Redis 写入失败
pub async fn invalidate_and_reload() -> anyhow::Result<()> {
    // 递增版本号用于多实例同步
    // 由于 ICacheService 没有 INCR 操作，使用时间戳作为版本号
    let ver = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_string();
    CONTEXT
        .redis_save_service
        .inner
        .set_string(KEY_ENFORCER_VERSION, &ver)
        .await?;

    Ok(())
}

/// 获取当前 Enforcer 版本号
///
/// 用于多实例场景下检测缓存是否需要刷新。
/// 各实例可以定期对比本地版本号与 Redis 中的版本号，
/// 若不一致则触发本地缓存刷新。
///
/// # Returns
/// * `Ok(Some(version))` - 成功获取版本号
/// * `Ok(None)` - 版本号不存在或 Redis 不可用
pub async fn get_enforcer_version() -> anyhow::Result<Option<String>> {
    match CONTEXT
        .redis_save_service
        .inner
        .get_string(KEY_ENFORCER_VERSION)
        .await
    {
        Ok(ver) => {
            if ver.is_empty() {
                Ok(None)
            } else {
                Ok(Some(ver))
            }
        }
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_constants() {
        assert_eq!(KEY_ENFORCER_VERSION, "auth:enforcer_ver");
    }
}
