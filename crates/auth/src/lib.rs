//! Auth 模块 - 权限管理系统
//!
//! 提供基于 Casbin 的运行时权限控制方案，被各业务微服务集成：
//! - API 接口级访问控制 + 字段级权限过滤
//! - JWT 认证中间件（Token 由 auth-admin 统一签发）
//! - Casbin 策略 Admin API（权限管理界面由 auth-admin 独立服务提供）
//! - Dapr 事件驱动的 casbin_rules 同步
//! - OpenApi Schema 自动同步
//!
//! # 核心组件
//! - [`EnforcerManager`] - Casbin Enforcer 管理器，支持热更新
//! - [`casbin_auth`] - API Casbin 权限中间件
//! - [`local_auth`] - JWT 认证中间件
//! - [`combined_auth`] - 组合认证+授权中间件（JWT + Casbin）
//! - [`auth_router`] - Casbin 策略 Admin API 路由
//! - [`extract_and_sync_schemas`] - Schema 同步函数
//!
//! # 快速开始
//! ```ignore
//! use std::sync::Arc;
//! use genies_auth::{
//!     EnforcerManager, LocalAuthConfig, combined_auth
//! };
//!
//! // 1. 初始化认证配置和 Enforcer
//! let auth_config = Arc::new(LocalAuthConfig::new("my-jwt-secret"));
//! let mgr = Arc::new(EnforcerManager::new().await?);
//!
//! // 2. 配置路由
//! let router = Router::new()
//!     .hoop(affix_state::inject(auth_config.clone()))
//!     .hoop(affix_state::inject(mgr.clone()))
//!     .hoop(combined_auth)
//!     .push(business_routes());
//! ```
//!
//! # 登录说明
//! 登录由独立的 auth-admin 服务统一提供，业务微服务不需要内置登录端点。
//! 用户获得 Token 后，在本模块中通过 JWT 验证即可。

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

/// API Casbin 权限中间件
pub mod middleware;

/// Admin API 端点（Casbin 策略管理）
pub mod admin_api;

/// 数据库迁移模块
pub mod models;

/// 领域事件类型定义
pub mod event;

/// 认证中间件（JWT 验证 + 组合中间件）
pub mod auth_middleware;

/// OAuth 2.0 资源服务器认证中间件
pub mod oauth2_middleware;

/// Dapr 事件订阅处理（接收 auth-admin 服务的同步事件）
pub mod event_handler;

/// 启动时用户-角色同步（从 auth-admin 拉取 g 规则）
pub mod startup_sync;

/// 微服务实例注册 / 心跳 / 注销客户端
pub mod service_registry;

// ============================================================================
// 公开 API Re-exports
// ============================================================================

// Casbin 相关
pub use enforcer_manager::EnforcerManager;
pub use middleware::casbin_auth;
pub use middleware::casbin_filter_object;

// Admin API
pub use admin_api::auth_router;
pub use admin_api::auth_public_router;

// Schema 同步
pub use schema_extractor::extract_and_sync_schemas;

// 认证
pub use auth_middleware::{
    LocalAuthConfig, LocalClaims,
    local_auth, combined_auth,
    verify_token,
};

// OAuth 2.0 认证
pub use auth_middleware::{OAuthClaims, verify_oauth_token};
pub use oauth2_middleware::{
    OAuth2AuthConfig,
    oauth2_auth, combined_oauth2_auth,
};

// 事件类型（供 auth-admin 发布事件使用）
pub use event::{
    UserCreatedEvent, UserUpdatedEvent, UserDeletedEvent,
    RoleCreatedEvent, RoleUpdatedEvent, RoleDeletedEvent,
    PermissionCreatedEvent, PermissionUpdatedEvent, PermissionDeletedEvent,
    UserRoleAssignedEvent, UserRoleRevokedEvent,
    RolePermissionAssignedEvent, RolePermissionRevokedEvent,
    OAuthClientCreatedEvent, OAuthClientUpdatedEvent, OAuthClientDeletedEvent,
};

// 服务注册
pub use service_registry::try_register_and_heartbeat;
