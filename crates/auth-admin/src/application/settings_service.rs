//! 设置应用服务

use serde::{Deserialize, Serialize};
use serde_json::json;
use salvo::oapi::ToSchema;

use crate::domain::service::settings_service::SettingsDomainService;

// ============================================================================
// 类型化设置值 DTO
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoFactorSettings {
    pub enabled: bool,
    #[serde(default)]
    pub methods: Vec<String>,
    /// 宽限期天数，0 表示立即强制
    #[serde(default)]
    pub grace_days: u32,
    /// 2FA 首次启用时间（ISO8601），用于计算截止时间
    #[serde(default)]
    pub enabled_at: Option<String>,
}

impl Default for TwoFactorSettings {
    fn default() -> Self {
        Self { enabled: false, methods: vec![], grace_days: 0, enabled_at: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CaptchaSettings {
    pub enabled: bool,
}

impl Default for CaptchaSettings {
    fn default() -> Self {
        Self { enabled: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PasswordPolicySettings {
    #[serde(default = "default_min_length")]
    pub min_length: u32,
    #[serde(default)]
    pub require_uppercase: bool,
    #[serde(default)]
    pub require_lowercase: bool,
    #[serde(default)]
    pub require_digit: bool,
    #[serde(default)]
    pub require_special: bool,
}

fn default_min_length() -> u32 { 6 }

impl Default for PasswordPolicySettings {
    fn default() -> Self {
        Self {
            min_length: default_min_length(),
            require_uppercase: false,
            require_lowercase: false,
            require_digit: false,
            require_special: false,
        }
    }
}

// ============================================================================
// SettingsAppService
// ============================================================================

pub struct SettingsAppService;

impl SettingsAppService {
    const KEY_2FA: &'static str = "auth.2fa";
    const KEY_CAPTCHA: &'static str = "auth.captcha";
    const KEY_PASSWORD: &'static str = "auth.password";

    /// 获取双因素认证设置
    pub async fn get_2fa_settings() -> Result<TwoFactorSettings, String> {
        SettingsDomainService::get_typed::<TwoFactorSettings>(Self::KEY_2FA)
            .await
            .or_else(|_| Ok(TwoFactorSettings::default()))
    }

    /// 获取验证码设置
    pub async fn get_captcha_settings() -> Result<CaptchaSettings, String> {
        SettingsDomainService::get_typed::<CaptchaSettings>(Self::KEY_CAPTCHA)
            .await
            .or_else(|_| Ok(CaptchaSettings::default()))
    }

    /// 获取密码策略设置
    pub async fn get_password_policy() -> Result<PasswordPolicySettings, String> {
        SettingsDomainService::get_typed::<PasswordPolicySettings>(Self::KEY_PASSWORD)
            .await
            .or_else(|_| Ok(PasswordPolicySettings::default()))
    }

    /// 更新双因素认证设置
    pub async fn update_2fa(settings: &TwoFactorSettings) -> Result<(), String> {
        let mut final_settings = settings.clone();

        // 读取旧设置，自动管理 enabled_at
        let old = Self::get_2fa_settings().await.unwrap_or_default();
        if settings.enabled && !old.enabled {
            // 首次启用：记录启用时间
            final_settings.enabled_at = Some(chrono::Utc::now().to_rfc3339());
        } else if settings.enabled && old.enabled {
            // 保持启用：保留原有启用时间
            final_settings.enabled_at = old.enabled_at;
        } else {
            // 禁用：清除启用时间
            final_settings.enabled_at = None;
        }

        let value = serde_json::to_value(&final_settings).map_err(|e| e.to_string())?;
        SettingsDomainService::set(Self::KEY_2FA, &value, "双因素认证配置").await
    }

    /// 更新验证码设置
    pub async fn update_captcha(settings: &CaptchaSettings) -> Result<(), String> {
        let value = serde_json::to_value(settings).map_err(|e| e.to_string())?;
        SettingsDomainService::set(Self::KEY_CAPTCHA, &value, "登录验证码配置").await
    }

    /// 更新密码策略设置
    pub async fn update_password_policy(settings: &PasswordPolicySettings) -> Result<(), String> {
        let value = serde_json::to_value(settings).map_err(|e| e.to_string())?;
        SettingsDomainService::set(Self::KEY_PASSWORD, &value, "密码强度策略").await
    }

    /// 获取所有设置（聚合返回）
    pub async fn get_all() -> Result<serde_json::Value, String> {
        let (two_fa, captcha, password) = tokio::join!(
            Self::get_2fa_settings(),
            Self::get_captcha_settings(),
            Self::get_password_policy(),
        );

        Ok(json!({
            "two_fa": two_fa.unwrap_or_default(),
            "captcha": captcha.unwrap_or_default(),
            "password": password.unwrap_or_default(),
        }))
    }
}
