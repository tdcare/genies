//! 实体模块

pub mod department_entity;
pub mod permission_entity;
pub mod role_entity;
pub mod user_entity;
pub mod user_department_entity;
pub mod application_entity;

pub use department_entity::AdminDepartment;
pub use permission_entity::AdminPermission;
pub use role_entity::{AdminRole, RolePermission};
pub use user_entity::{AdminUser, UserRole, UserRoleMapping};
pub use user_department_entity::UserDepartment;
pub use application_entity::ApplicationEntity;
