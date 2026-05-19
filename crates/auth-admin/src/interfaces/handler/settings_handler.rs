//! 设置 Handler — 系统设置的查询与修改

use salvo::prelude::*;
use salvo::oapi::extract::*;

use genies::core::RespVO;

use crate::application::settings_service::{
    SettingsAppService, TwoFactorSettings, CaptchaSettings, PasswordPolicySettings,
};

/// GET /settings — 获取所有设置
#[endpoint(tags("settings"), summary = "获取所有系统设置")]
pub async fn get_settings() -> Json<RespVO<serde_json::Value>> {
    match SettingsAppService::get_all().await {
        Ok(data) => Json(RespVO::from(&data)),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// PUT /settings/auth/password — 更新密码策略
#[endpoint(tags("settings"), summary = "更新密码策略设置")]
pub async fn update_password_policy(
    body: JsonBody<PasswordPolicySettings>,
) -> Json<RespVO<()>> {
    let settings = body.into_inner();
    match SettingsAppService::update_password_policy(&settings).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// PUT /settings/auth/captcha — 更新验证码设置
#[endpoint(tags("settings"), summary = "更新验证码设置")]
pub async fn update_captcha_settings(
    body: JsonBody<CaptchaSettings>,
) -> Json<RespVO<()>> {
    let settings = body.into_inner();
    match SettingsAppService::update_captcha(&settings).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// PUT /settings/auth/2fa — 更新双因素认证设置
#[endpoint(tags("settings"), summary = "更新双因素认证设置")]
pub async fn update_2fa_settings(
    body: JsonBody<TwoFactorSettings>,
) -> Json<RespVO<()>> {
    let settings = body.into_inner();
    match SettingsAppService::update_2fa(&settings).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// 构建设置路由（受保护路由组内使用）
pub fn routes() -> Router {
    Router::new()
        .push(Router::with_path("/settings").get(get_settings))
        .push(Router::with_path("/settings/auth/password").put(update_password_policy))
        .push(Router::with_path("/settings/auth/captcha").put(update_captcha_settings))
        .push(Router::with_path("/settings/auth/2fa").put(update_2fa_settings))
}
