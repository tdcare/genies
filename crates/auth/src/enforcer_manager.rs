//! Casbin Enforcer 管理器
//!
//! 提供支持热更新的 Enforcer 管理，从数据库加载 Casbin 模型和策略。
//! 
//! # 功能特性
//! - 从数据库加载 Casbin 模型定义（casbin_model 表）
//! - 使用 RBatisAdapter 从数据库加载策略规则（casbin_rules 表）
//! - 支持热更新：运行时重新加载模型和策略
//! - 读写锁保护，高并发安全
//!
//! # 使用示例
//! ```ignore
//! use genies_auth::enforcer_manager::EnforcerManager;
//!
//! // 初始化管理器
//! let manager = EnforcerManager::new().await?;
//!
//! // 获取 Enforcer 进行权限检查
//! let enforcer = manager.get_enforcer().await;
//! let allowed = enforcer.enforce(("alice", "data1", "read"))?;
//!
//! // 热更新（Admin API 触发）
//! manager.reload().await?;
//! ```

use std::sync::Arc;

use casbin::prelude::*;
use rbs::value;
use tokio::sync::RwLock;

use genies::context::CONTEXT;

use crate::adapter::RBatisAdapter;
use crate::version_sync;

/// Casbin Enforcer 管理器
///
/// 管理 Enforcer 实例的生命周期，支持热更新。
/// 使用读写锁保护，允许并发读取，串行写入。
pub struct EnforcerManager {
    /// Enforcer 实例，使用 Arc 支持并发读取快照
    enforcer: RwLock<Arc<Enforcer>>,
}

impl EnforcerManager {
    /// 初始化 Enforcer 管理器
    ///
    /// 从数据库加载 Casbin 模型和策略，创建 Enforcer 实例。
    ///
    /// # Returns
    /// * `Ok(Self)` - 初始化成功
    /// * `Err(_)` - 数据库查询失败或 Enforcer 创建失败
    ///
    /// # Errors
    /// - 数据库连接失败
    /// - casbin_model 表中找不到指定模型
    /// - 模型定义格式错误
    /// - casbin_rules 表加载失败
    pub async fn new() -> anyhow::Result<Self> {
        let enforcer = Self::build_enforcer().await?;
        Ok(Self {
            enforcer: RwLock::new(Arc::new(enforcer)),
        })
    }

    /// 从数据库构建 Enforcer
    ///
    /// 执行以下步骤：
    /// 1. 从 casbin_model 表读取模型定义文本
    /// 2. 解析模型定义创建 Model 实例
    /// 3. 使用 RBatisAdapter 从 casbin_rules 表加载策略
    /// 4. 创建并返回 Enforcer 实例
    async fn build_enforcer() -> anyhow::Result<Enforcer> {
        // 1. 从 casbin_model 表读取模型定义
        let model_text = load_model_from_db("default").await?;
        
        // 2. 从字符串解析创建 Model
        let model = DefaultModel::from_str(&model_text)
            .await
            .map_err(|e| anyhow::anyhow!("解析 Casbin 模型失败: {}", e))?;

        // 3. 使用 RBatisAdapter 加载策略
        let adapter = RBatisAdapter::new();
        
        // 4. 创建 Enforcer
        let enforcer = Enforcer::new(model, adapter)
            .await
            .map_err(|e| anyhow::anyhow!("创建 Casbin Enforcer 失败: {}", e))?;

        log::info!("Casbin Enforcer 构建成功，从数据库加载模型和策略");
        Ok(enforcer)
    }

    /// 获取 Enforcer 快照（高并发安全）
    ///
    /// 返回当前 Enforcer 的 Arc 引用，可以安全地在多个任务中并发使用。
    /// 即使在此期间发生 reload，持有的快照仍然有效。
    ///
    /// # Returns
    /// 当前 Enforcer 实例的 Arc 引用
    ///
    /// # 并发安全性
    /// - 多个读取操作可以并发执行
    /// - 读取操作不会阻塞其他读取操作
    /// - 只有在 reload 写入时会短暂阻塞读取
    pub async fn get_enforcer(&self) -> Arc<Enforcer> {
        self.enforcer.read().await.clone()
    }

    /// 热更新：从数据库重新加载模型和策略
    ///
    /// 当 Admin API 修改了权限配置后调用此方法，执行以下操作：
    /// 1. 从数据库重新构建 Enforcer
    /// 2. 原子替换当前 Enforcer 实例
    /// 3. 更新 Redis 版本号通知其他实例
    ///
    /// # Returns
    /// * `Ok(())` - 重载成功
    /// * `Err(_)` - 重载失败（当前 Enforcer 保持不变）
    ///
    /// # 并发安全性
    /// - reload 操作会获取写锁，期间阻塞所有读取和写入
    /// - 如果构建新 Enforcer 失败，原有 Enforcer 不受影响
    /// - Redis 版本号更新失败不影响 Enforcer 更新（仅记录警告）
    pub async fn reload(&self) -> anyhow::Result<()> {
        // 1. 先构建新的 Enforcer（不持有锁）
        let new_enforcer = Self::build_enforcer().await?;

        // 2. 获取写锁并原子替换
        let mut guard = self.enforcer.write().await;
        *guard = Arc::new(new_enforcer);

        // 3. 更新 Redis 版本号通知其他实例
        // 即使失败也不影响 Enforcer 更新，仅记录日志
        if let Err(e) = version_sync::invalidate_and_reload().await {
            log::warn!("更新 Redis 版本号失败: {}，其他实例可能需要手动刷新", e);
        }

        log::info!("Casbin Enforcer 已重载");
        Ok(())
    }
}

/// 从 casbin_model 表读取模型文本
///
/// # Arguments
/// * `name` - 模型名称，对应 casbin_model 表的 model_name 字段
///
/// # Returns
/// * `Ok(String)` - 模型定义文本
/// * `Err(_)` - 查询失败或模型不存在
async fn load_model_from_db(name: &str) -> anyhow::Result<String> {
    /// 用于反序列化的内部结构
    #[derive(serde::Deserialize)]
    struct ModelRow {
        model_text: Option<String>,
    }

    let sql = "SELECT model_text FROM casbin_model WHERE model_name = ?";
    let value: rbs::Value = CONTEXT
        .rbatis
        .query(sql, vec![value!(name)])
        .await
        .map_err(|e| anyhow::anyhow!("查询 casbin_model 失败: {}", e))?;

    let rows: Vec<ModelRow> = rbs::from_value(value)
        .map_err(|e| anyhow::anyhow!("反序列化 casbin_model 失败: {}", e))?;

    // 提取第一行的 model_text 字段
    if let Some(row) = rows.into_iter().next() {
        if let Some(text) = row.model_text {
            if !text.is_empty() {
                return Ok(text);
            }
        }
    }

    Err(anyhow::anyhow!(
        "未找到名为 '{}' 的 Casbin 模型，请确认 casbin_model 表中存在对应记录",
        name
    ))
}

#[cfg(test)]
mod tests {
    // 单元测试需要数据库连接，暂不实现
    // 可通过集成测试验证功能
}
