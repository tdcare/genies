//! 双因素认证 Handler — 2FA 绑定与管理

use salvo::prelude::*;
use salvo::oapi::extract::*;

use genies::core::RespVO;
use genies_auth::LocalClaims;

use crate::application::two_factor_service::TwoFactorAppService;
use crate::application::settings_service::SettingsAppService;

/// 从 Depot 中提取当前用户 ID
fn get_current_uid(depot: &Depot) -> Result<i64, Json<RespVO<serde_json::Value>>> {
    match depot.get::<LocalClaims>("local_user") {
        Ok(claims) => match claims.uid {
            Some(uid) => Ok(uid),
            None => Err(Json(RespVO::from_error_info("-1", "令牌中无用户ID"))),
        },
        Err(_) => Err(Json(RespVO::from_error_info("-1", "未获取到用户信息"))),
    }
}

/// GET /me/2fa — 获取当前用户的 2FA 状态
#[endpoint(tags("2fa"), summary = "获取当前用户 2FA 状态")]
pub async fn get_my_2fa(
    _req: &mut Request,
    depot: &mut Depot,
) -> Json<RespVO<serde_json::Value>> {
    let uid = match get_current_uid(depot) {
        Ok(uid) => uid,
        Err(e) => return e,
    };

    let (status, two_fa_settings) = tokio::join!(
        TwoFactorAppService::get_status(uid),
        SettingsAppService::get_2fa_settings(),
    );

    let allowed_methods = two_fa_settings
        .as_ref()
        .map(|s| if s.enabled { s.methods.clone() } else { vec![] })
        .unwrap_or_default();

    match status {
        Ok(Some(two_fa)) => Json(RespVO::from(&serde_json::json!({
            "enabled": two_fa.enabled == 1,
            "method": two_fa.method,
            "phone": two_fa.phone,
            "allowed_methods": allowed_methods,
        }))),
        _ => Json(RespVO::from(&serde_json::json!({
            "enabled": false,
            "method": "",
            "phone": "",
            "allowed_methods": allowed_methods,
        }))),
    }
}

/// POST /me/2fa/totp/setup — 发起 TOTP 绑定
#[endpoint(tags("2fa"), summary = "发起 TOTP 绑定")]
pub async fn setup_totp(
    _req: &mut Request,
    depot: &mut Depot,
) -> Json<RespVO<serde_json::Value>> {
    let uid = match get_current_uid(depot) {
        Ok(uid) => uid,
        Err(e) => return e,
    };

    let username = depot
        .get::<LocalClaims>("local_user")
        .map(|c| c.sub.clone())
        .unwrap_or_default();

    match TwoFactorAppService::setup_totp(uid, &username).await {
        Ok(data) => Json(RespVO::from(&data)),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// POST /me/2fa/totp/confirm — 确认 TOTP 绑定
#[endpoint(tags("2fa"), summary = "确认 TOTP 绑定")]
pub async fn confirm_totp(
    body: JsonBody<serde_json::Value>,
    _req: &mut Request,
    depot: &mut Depot,
) -> Json<RespVO<serde_json::Value>> {
    let uid = match get_current_uid(depot) {
        Ok(uid) => uid,
        Err(e) => return e,
    };

    let code = body.0.get("code").and_then(|v| v.as_str()).unwrap_or("");

    match TwoFactorAppService::confirm_totp(uid, code).await {
        Ok(backup_codes) => Json(RespVO::from(&serde_json::json!({
            "backup_codes": backup_codes,
        }))),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// POST /me/2fa/second-password — 设置二次密码
#[endpoint(tags("2fa"), summary = "设置二次密码")]
pub async fn setup_second_password(
    body: JsonBody<serde_json::Value>,
    _req: &mut Request,
    depot: &mut Depot,
) -> Json<RespVO<()>> {
    let uid = match depot.get::<LocalClaims>("local_user") {
        Ok(claims) => match claims.uid {
            Some(uid) => uid,
            None => return Json(RespVO::from_error_info("-1", "令牌中无用户ID")),
        },
        Err(_) => return Json(RespVO::from_error_info("-1", "未获取到用户信息")),
    };

    let password = body.0.get("password").and_then(|v| v.as_str()).unwrap_or("");

    if password.len() < 4 {
        return Json(RespVO::from_error_info("-1", "二次密码至少需要4位"));
    }

    match TwoFactorAppService::setup_second_password(uid, password).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// DELETE /me/2fa — 关闭 2FA
#[endpoint(tags("2fa"), summary = "关闭 2FA")]
pub async fn disable_2fa(
    _req: &mut Request,
    depot: &mut Depot,
) -> Json<RespVO<()>> {
    let uid = match depot.get::<LocalClaims>("local_user") {
        Ok(claims) => match claims.uid {
            Some(uid) => uid,
            None => return Json(RespVO::from_error_info("-1", "令牌中无用户ID")),
        },
        Err(_) => return Json(RespVO::from_error_info("-1", "未获取到用户信息")),
    };

    match TwoFactorAppService::disable(uid).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// POST /me/2fa/sms/setup — 发起短信 2FA 绑定
#[endpoint(tags("2fa"), summary = "发起短信 2FA 绑定")]
pub async fn setup_sms(
    body: JsonBody<serde_json::Value>,
    _req: &mut Request,
    depot: &mut Depot,
) -> Json<RespVO<()>> {
    let uid = match depot.get::<LocalClaims>("local_user") {
        Ok(claims) => match claims.uid {
            Some(uid) => uid,
            None => return Json(RespVO::from_error_info("-1", "令牌中无用户ID")),
        },
        Err(_) => return Json(RespVO::from_error_info("-1", "未获取到用户信息")),
    };

    let phone = body.0.get("phone").and_then(|v| v.as_str()).unwrap_or("");

    if phone.is_empty() {
        return Json(RespVO::from_error_info("-1", "手机号码不能为空"));
    }

    match TwoFactorAppService::setup_sms(uid, phone).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// POST /me/2fa/sms/verify — 验证短信验证码并启用短信 2FA
#[endpoint(tags("2fa"), summary = "确认短信 2FA 绑定")]
pub async fn verify_sms(
    body: JsonBody<serde_json::Value>,
    _req: &mut Request,
    depot: &mut Depot,
) -> Json<RespVO<()>> {
    let uid = match depot.get::<LocalClaims>("local_user") {
        Ok(claims) => match claims.uid {
            Some(uid) => uid,
            None => return Json(RespVO::from_error_info("-1", "令牌中无用户ID")),
        },
        Err(_) => return Json(RespVO::from_error_info("-1", "未获取到用户信息")),
    };

    let code = body.0.get("code").and_then(|v| v.as_str()).unwrap_or("");

    if code.is_empty() {
        return Json(RespVO::from_error_info("-1", "验证码不能为空"));
    }

    match TwoFactorAppService::verify_sms_setup(uid, code).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// POST /me/2fa/sms/send — 发送短信验证码
#[endpoint(tags("2fa"), summary = "发送短信验证码")]
pub async fn send_sms_code(
    _req: &mut Request,
    depot: &mut Depot,
) -> Json<RespVO<()>> {
    let uid = match depot.get::<LocalClaims>("local_user") {
        Ok(claims) => match claims.uid {
            Some(uid) => uid,
            None => return Json(RespVO::from_error_info("-1", "令牌中无用户ID")),
        },
        Err(_) => return Json(RespVO::from_error_info("-1", "未获取到用户信息")),
    };

    match TwoFactorAppService::send_sms_code(uid).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// POST /admin/users/{id}/2fa/reset — 管理员强制重置用户 2FA
#[endpoint(tags("admin-2fa"), summary = "管理员重置用户 2FA")]
pub async fn admin_reset_2fa(
    id: PathParam<i64>,
    _req: &mut Request,
    _depot: &mut Depot,
) -> Json<RespVO<()>> {
    match TwoFactorAppService::admin_reset(id.into_inner()).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(e) => Json(RespVO::from_error_info("-1", &e)),
    }
}

/// 用户 2FA 自服务路由（Protected）
pub fn routes() -> Router {
    Router::new()
        .push(Router::with_path("/me/2fa").get(get_my_2fa))
        .push(Router::with_path("/me/2fa/totp/setup").post(setup_totp))
        .push(Router::with_path("/me/2fa/totp/confirm").post(confirm_totp))
        .push(Router::with_path("/me/2fa/second-password").post(setup_second_password))
        .push(Router::with_path("/me/2fa/sms/setup").post(setup_sms))
        .push(Router::with_path("/me/2fa/sms/verify").post(verify_sms))
        .push(Router::with_path("/me/2fa/sms/send").post(send_sms_code))
        .push(Router::with_path("/me/2fa").delete(disable_2fa))
}

/// 管理员 2FA 管理路由（Protected）
pub fn admin_routes() -> Router {
    Router::new()
        .push(Router::with_path("/admin/users/{id}/2fa/reset").post(admin_reset_2fa))
}
