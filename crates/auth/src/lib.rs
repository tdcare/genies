//! Auth 模块 - Casbin 权限管理系统
//!
//! 提供基于 Casbin 的完整权限管理方案，包括：
//! - API 接口级访问控制
//! - 字段级权限过滤
//! - 动态策略管理
//! - OpenApi Schema 自动同步
//!
//! # 核心组件
//! - [`EnforcerManager`] - Casbin Enforcer 管理器，支持热更新
//! - [`casbin_auth`] - API 权限中间件
//! - [`auth_admin_router`] - Admin API 路由
//! - [`extract_and_sync_schemas`] - Schema 同步函数
//!
//! # 快速开始
//! ```ignore
//! use genies_auth::{EnforcerManager, casbin_auth, auth_admin_router};
//!
//! // 1. 初始化 Enforcer
//! let mgr = Arc::new(EnforcerManager::new().await?);
//!
//! // 2. 配置路由
//! let router = Router::new()
//!     .hoop(affix_state::inject(mgr.clone()))
//!     .hoop(casbin_auth)
//!     .push(auth_admin_router());
//! ```

// ============================================================================
// 模块声明
// ============================================================================

/// RBatis Casbin Adapter - 数据库策略存储
pub mod adapter;

/// API Schema 提取与同步
pub mod schema_extractor;

/// 版本同步层（多实例 Enforcer 同步）
pub mod version_sync;

/// Casbin Enforcer 管理器
pub mod enforcer_manager;

/// API 权限中间件
pub mod middleware;

/// Admin API 端点
pub mod admin_api;

/// Admin UI 静态资源服务
pub mod admin_ui;

/// 数据库迁移模块
pub mod models;

// ============================================================================
// 公开 API Re-exports
// ============================================================================

pub use enforcer_manager::EnforcerManager;
pub use middleware::casbin_auth;
pub use middleware::casbin_filter_object;
pub use admin_api::auth_admin_router;
pub use admin_api::auth_public_router;
pub use admin_ui::auth_admin_ui_router;
pub use schema_extractor::extract_and_sync_schemas;
