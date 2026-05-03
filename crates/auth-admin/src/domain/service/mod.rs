//! 领域服务模块
//!
//! 封装跨实体的业务逻辑，将"持久化 + 事件发布"统一在同一事务中执行。

pub mod user_service;
pub mod role_service;
pub mod application_service;

pub use user_service::UserDomainService;
pub use role_service::RoleDomainService;
pub use application_service::ApplicationDomainService;
