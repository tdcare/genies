//! 聚合根模块

pub mod department;
pub mod permission;
pub mod role;
pub mod user;

pub use role::RoleAggregate;
pub use user::UserAggregate;
