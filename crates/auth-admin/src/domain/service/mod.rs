//! 领域服务模块
//!
//! 封装跨实体的业务逻辑，将"持久化 + 事件发布"统一在同一事务中执行。

pub mod user_service;
pub mod role_service;
pub mod application_service;
pub mod app_instance_service;
pub mod settings_service;
pub mod password_policy_service;
pub mod captcha_service;
pub mod totp_service;
pub mod sms_service;
pub mod second_password_service;
pub mod oauth_domain_service;

pub use user_service::UserDomainService;
pub use role_service::RoleDomainService;
pub use application_service::ApplicationDomainService;
pub use app_instance_service::AppInstanceDomainService;
pub use settings_service::SettingsDomainService;
pub use password_policy_service::PasswordPolicyService;
pub use captcha_service::CaptchaService;
pub use totp_service::TotpService;
pub use sms_service::SmsService;
pub use second_password_service::SecondPasswordService;
pub use oauth_domain_service::OAuthDomainService;
