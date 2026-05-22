//! 实体模块

pub mod department_entity;
pub mod permission_entity;
pub mod role_entity;
pub mod user_entity;
pub mod user_department_entity;
pub mod application_entity;
pub mod app_instance_entity;
pub mod settings_entity;
pub mod user_2fa_entity;
pub mod oauth_client_entity;
pub mod oauth_authorization_code_entity;
pub mod oauth_access_token_entity;
pub mod oauth_refresh_token_entity;

pub use department_entity::AdminDepartment;
pub use permission_entity::AdminPermission;
pub use role_entity::{AdminRole, RolePermission};
pub use user_entity::{AdminUser, UserRole, UserRoleMapping};
pub use user_department_entity::UserDepartment;
pub use application_entity::ApplicationEntity;
pub use app_instance_entity::AppInstanceEntity;
pub use settings_entity::AdminSetting;
pub use user_2fa_entity::UserTwoFactor;
pub use oauth_client_entity::OAuthClientEntity;
pub use oauth_authorization_code_entity::OAuthAuthorizationCodeEntity;
pub use oauth_access_token_entity::OAuthAccessTokenEntity;
pub use oauth_refresh_token_entity::OAuthRefreshTokenEntity;
