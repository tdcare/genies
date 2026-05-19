//! 认证 Handler — 登录、登出、Token 刷新、个人信息、验证码

use std::sync::Arc;

use salvo::prelude::*;
use salvo::oapi::extract::*;
use salvo::http::StatusCode;

use genies::core::RespVO;
use genies_auth::{LocalAuthConfig, verify_token};

use crate::application::dto::{LoginRequest, LoginResponse, ChangePasswordRequest, TwoFactorVerifyRequest};
use crate::application::service::AuthService;
use crate::application::settings_service::SettingsAppService;
use crate::domain::service::CaptchaService;

/// POST /login — 用户名密码登录，返回 JWT
#[endpoint(tags("auth"), summary = "用户名密码登录")]
pub async fn auth_login(
    body: JsonBody<LoginRequest>,
    depot: &mut Depot,
    res: &mut Response,
) -> Json<RespVO<LoginResponse>> {
    let login_req = body.into_inner();

    // 1. 验证码校验（如启用）
    if let Ok(captcha_cfg) = SettingsAppService::get_captcha_settings().await {
        if captcha_cfg.enabled {
            let captcha_id = login_req.captcha_id.as_deref().unwrap_or("");
            let captcha_text = login_req.captcha_text.as_deref().unwrap_or("");
            if let Err(msg) = CaptchaService::verify(captcha_id, captcha_text).await {
                res.status_code(StatusCode::BAD_REQUEST);
                return Json(RespVO::from_error_info("-1", msg));
            }
        }
    }

    // 2. 获取 JWT 配置
    let config = match depot.obtain::<Arc<LocalAuthConfig>>() {
        Ok(c) => c,
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return Json(RespVO::from_error_info("-1", "认证配置错误"));
        }
    };

    match AuthService::login(
        &login_req.username,
        &login_req.password,
        &config.secret,
        config.expires_in_secs,
    ).await {
        Ok(resp) => Json(RespVO::from(&resp)),
        Err(msg) => {
            if msg.contains("用户名或密码错误") {
                res.status_code(StatusCode::UNAUTHORIZED);
            } else if msg.contains("用户已被禁用") {
                res.status_code(StatusCode::FORBIDDEN);
            } else {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Json(RespVO::from_error_info("-1", &msg))
        }
    }
}

/// POST /logout — 登出（客户端丢弃 Token 即可）
#[endpoint(tags("auth"), summary = "用户登出")]
pub async fn auth_logout() -> Json<RespVO<()>> {
    Json(RespVO::from_error_info("0", "ok"))
}

/// GET /me — 获取当前登录用户信息
#[endpoint(tags("auth"), summary = "获取当前登录用户信息")]
pub async fn get_me(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Json<RespVO<serde_json::Value>> {
    let config = match depot.obtain::<Arc<LocalAuthConfig>>() {
        Ok(c) => c,
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return Json(RespVO::from_error_info("-1", "认证配置错误"));
        }
    };

    let token = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let claims = match verify_token(token, &config.secret) {
        Ok(c) => c,
        Err(e) => {
            res.status_code(StatusCode::UNAUTHORIZED);
            return Json(RespVO::from_error_info("-1", &format!("令牌无效: {}", e)));
        }
    };

    match claims.uid {
        Some(uid) => {
            match AuthService::get_current_user(uid).await {
                Ok(data) => Json(RespVO::from(&data)),
                Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
            }
        }
        None => Json(RespVO::from_error_info("-1", "令牌中无用户ID")),
    }
}

/// PUT /me/password — 修改当前用户密码
#[endpoint(tags("auth"), summary = "修改当前用户密码")]
pub async fn change_password(
    body: JsonBody<ChangePasswordRequest>,
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Json<RespVO<()>> {
    let config = match depot.obtain::<Arc<LocalAuthConfig>>() {
        Ok(c) => c,
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return Json(RespVO::from_error_info("-1", "认证配置错误"));
        }
    };

    let token = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let claims = match verify_token(token, &config.secret) {
        Ok(c) => c,
        Err(e) => {
            res.status_code(StatusCode::UNAUTHORIZED);
            return Json(RespVO::from_error_info("-1", &format!("令牌无效: {}", e)));
        }
    };

    let uid = match claims.uid {
        Some(uid) => uid,
        None => {
            return Json(RespVO::from_error_info("-1", "令牌中无用户ID"));
        }
    };

    let pwd_req = body.into_inner();
    match AuthService::change_password(uid, &pwd_req).await {
        Ok(()) => Json(RespVO::from_error_info("0", "ok")),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /captcha — 获取登录验证码图片
#[endpoint(tags("auth"), summary = "获取登录验证码")]
pub async fn get_captcha() -> Json<RespVO<serde_json::Value>> {
    // 检查是否启用验证码
    if let Ok(captcha_cfg) = SettingsAppService::get_captcha_settings().await {
        if !captcha_cfg.enabled {
            return Json(RespVO::from_error_info("-1", "验证码功能未启用"));
        }
    }

    let (captcha_id, image_base64) = CaptchaService::generate_cached().await;
    Json(RespVO::from(&serde_json::json!({
        "captcha_id": captcha_id,
        "image_base64": image_base64,
    })))
}

/// POST /2fa/verify — 验证双因素认证
#[endpoint(tags("auth"), summary = "双因素认证验证")]
pub async fn auth_verify_2fa(
    body: JsonBody<TwoFactorVerifyRequest>,
    depot: &mut Depot,
    res: &mut Response,
) -> Json<RespVO<LoginResponse>> {
    let verify_req = body.into_inner();

    let config = match depot.obtain::<Arc<LocalAuthConfig>>() {
        Ok(c) => c,
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return Json(RespVO::from_error_info("-1", "认证配置错误"));
        }
    };

    match AuthService::verify_2fa(
        &verify_req.preauth_token,
        &verify_req.code,
        &verify_req.method,
        &config.secret,
        config.expires_in_secs,
    ).await {
        Ok(resp) => Json(RespVO::from(&resp)),
        Err(msg) => {
            if msg.contains("无效") || msg.contains("错误") {
                res.status_code(StatusCode::UNAUTHORIZED);
            } else {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Json(RespVO::from_error_info("-1", &msg))
        }
    }
}

/// 公开路由（登录、登出、刷新 Token、验证码、2FA 验证）
pub fn public_routes() -> Router {
    Router::new()
        .push(Router::with_path("/captcha").get(get_captcha))
        .push(Router::with_path("/login").post(auth_login))
        .push(Router::with_path("/logout").post(auth_logout))
        .push(Router::with_path("/refresh").post(refresh_token))
        .push(Router::with_path("/2fa/verify").post(auth_verify_2fa))
}

/// 受保护路由（需要认证）
pub fn protected_routes() -> Router {
    Router::new()
        .push(Router::with_path("/me").get(get_me))
        .push(Router::with_path("/me/password").put(change_password))
}

/// POST /refresh — 刷新 Token
#[endpoint(tags("auth"), summary = "刷新 Token")]
pub async fn refresh_token(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Json<RespVO<serde_json::Value>> {
    let config = match depot.obtain::<Arc<LocalAuthConfig>>() {
        Ok(c) => c,
        Err(_) => {
            return Json(RespVO::from_error_info("-1", "认证配置错误"));
        }
    };

    let token = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let claims = match verify_token(token, &config.secret) {
        Ok(c) => c,
        Err(e) => {
            res.status_code(StatusCode::UNAUTHORIZED);
            return Json(RespVO::from_error_info("-1", &format!("令牌无效: {}", e)));
        }
    };

    match AuthService::refresh_token(&claims, &config.secret, config.expires_in_secs).await {
        Ok(data) => Json(RespVO::from(&data)),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}
