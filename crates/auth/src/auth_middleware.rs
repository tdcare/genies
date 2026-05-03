//! Auth 认证中间件
//!
//! 提供 JWT 验证 + Casbin 授权功能：
//! - `local_auth` — JWT 验证中间件
//! - `combined_auth` — JWT 认证 + Casbin API 授权组合中间件
//! - `verify_token` — JWT 验证函数
//!
//! 登录功能由 auth-admin 服务统一提供，本模块仅负责 Token 验证。

use std::sync::Arc;

use jsonwebtoken::{decode, DecodingKey, Validation};
use salvo::http::StatusCode;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use casbin::CoreApi;

// ============================================================================
// JWT Claims
// ============================================================================

/// 本地 JWT 声明结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalClaims {
    /// 用户名
    pub sub: String,
    /// 用户 ID
    pub uid: Option<i64>,
    /// 显示名称
    pub name: Option<String>,
    /// 签发时间 (UTC 秒)
    pub iat: usize,
    /// 过期时间 (UTC 秒)
    pub exp: usize,
}

// ============================================================================
// 本地认证状态（JWT Secret）
// ============================================================================

/// 本地认证配置（通过 affix_state 注入）
#[derive(Clone)]
pub struct LocalAuthConfig {
    /// JWT 签名密钥
    pub secret: String,
    /// Token 有效期（秒），默认 7200 (2小时)
    pub expires_in_secs: usize,
}

impl Default for LocalAuthConfig {
    fn default() -> Self {
        Self {
            secret: "genies_auth_local_jwt_secret_change_me".to_string(),
            expires_in_secs: 7200,
        }
    }
}

impl LocalAuthConfig {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            expires_in_secs: 7200,
        }
    }

    pub fn with_expiry(secret: impl Into<String>, expires_in_secs: usize) -> Self {
        Self {
            secret: secret.into(),
            expires_in_secs,
        }
    }
}

// ============================================================================
// JWT 验证函数
// ============================================================================

/// 验证本地 JWT Token，返回 Claims
pub fn verify_token(token: &str, secret: &str) -> Result<LocalClaims, String> {
    let token = token.strip_prefix("Bearer ").unwrap_or(token);
    decode::<LocalClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| format!("JWT 验证失败: {}", e))
}

// ============================================================================
// Middleware: local_auth — JWT 认证中间件
// ============================================================================

/// JWT 认证中间件
///
/// 解析 Authorization 头中的 Bearer Token，验证后注入 Depot。
///
/// # 依赖
/// - `LocalAuthConfig` 必须通过 `affix_state::inject` 注入到 Depot
///
/// # 注入到 Depot 的数据
/// - `"jwtToken"` → `genies::core::jwt::JWTToken`（兼容 casbin_auth）
/// - `"local_user"` → `LocalClaims`
/// - `"subject"` → 用户名（供 Casbin 中间件使用）
#[handler]
pub async fn local_auth(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    let config = match depot.obtain::<Arc<LocalAuthConfig>>() {
        Ok(c) => c,
        Err(_) => {
            log::error!("LocalAuthConfig 未注入到 Depot");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "code": "-1",
                "msg": "认证配置错误"
            })));
            ctrl.skip_rest();
            return;
        }
    };

    let token = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if token.is_empty() {
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render(Json(serde_json::json!({
            "code": "-1",
            "msg": "未提供认证令牌"
        })));
        ctrl.skip_rest();
        return;
    }

    let claims = match verify_token(token, &config.secret) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("JWT 验证失败: {}", e);
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({
                "code": "-1",
                "msg": format!("令牌验证失败: {}", e)
            })));
            ctrl.skip_rest();
            return;
        }
    };

    let jwt_token = genies::core::jwt::JWTToken {
        preferred_username: Some(claims.sub.clone()),
        user_id: claims.uid.map(|id| id.to_string()),
        name: claims.name.clone(),
        id: None,
        exp: None,
        iat: None,
        jti: None,
        iss: None,
        sub: None,
        typ: None,
        azp: None,
        session_state: None,
        acr: None,
        realm_access: None,
        resource_access: None,
        scope: None,
        department_name: None,
        department_code: None,
        department_id: None,
        roles: None,
        groups: None,
        dept: None,
        given_name: None,
        department_abstract: None,
    };

    depot.insert("jwtToken", jwt_token);
    depot.insert("local_user", claims.clone());
    depot.insert("subject", claims.sub);

    ctrl.call_next(req, depot, res).await;
}

// ============================================================================
// Middleware: combined_auth — 组合认证+授权中间件
// ============================================================================

/// 组合认证授权中间件
///
/// 先执行 JWT 认证，再执行 Casbin API 权限检查。
/// 使用时需要在 Depot 中注入 `LocalAuthConfig` 和 `EnforcerManager`。
///
/// # 使用示例
/// ```ignore
/// let auth_config = Arc::new(LocalAuthConfig::new("my-secret"));
/// let mgr = Arc::new(EnforcerManager::new().await?);
///
/// Router::new()
///     .hoop(affix_state::inject(auth_config))
///     .hoop(affix_state::inject(mgr))
///     .hoop(combined_auth)
///     .push(api_routes());
/// ```
#[handler]
pub async fn combined_auth(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    // Step 1: JWT 认证
    let config = match depot.obtain::<Arc<LocalAuthConfig>>() {
        Ok(c) => c,
        Err(_) => {
            log::error!("LocalAuthConfig 未注入到 Depot");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "code": "-1",
                "msg": "认证配置错误"
            })));
            ctrl.skip_rest();
            return;
        }
    };

    let token = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if token.is_empty() {
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render(Json(serde_json::json!({
            "code": "-1",
            "msg": "未提供认证令牌"
        })));
        ctrl.skip_rest();
        return;
    }

    let claims = match verify_token(token, &config.secret) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("JWT 验证失败: {}", e);
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({
                "code": "-1",
                "msg": format!("令牌验证失败: {}", e)
            })));
            ctrl.skip_rest();
            return;
        }
    };

    let jwt_token = genies::core::jwt::JWTToken {
        preferred_username: Some(claims.sub.clone()),
        user_id: claims.uid.map(|id| id.to_string()),
        name: claims.name.clone(),
        id: None, exp: None, iat: None, jti: None, iss: None, sub: None,
        typ: None, azp: None, session_state: None, acr: None,
        realm_access: None, resource_access: None, scope: None,
        department_name: None, department_code: None, department_id: None,
        roles: None, groups: None, dept: None, given_name: None, department_abstract: None,
    };

    depot.insert("jwtToken", jwt_token);
    depot.insert("local_user", claims.clone());
    depot.insert("subject", claims.sub);

    // Step 2: Casbin API 权限检查
    let subject = match depot.get::<genies::core::jwt::JWTToken>("jwtToken") {
        Ok(token) => token.preferred_username.clone().unwrap_or_else(|| "guest".into()),
        Err(_) => "guest".into(),
    };

    let enforcer = match depot.obtain::<Arc<crate::enforcer_manager::EnforcerManager>>() {
        Ok(mgr) => mgr.get_enforcer().await,
        Err(_) => {
            log::error!("EnforcerManager 未注入到 Depot");
            ctrl.call_next(req, depot, res).await;
            return;
        }
    };

    let method = req.method().to_string();
    let uri = req.uri().to_string();
    let path = uri.split('?').next().unwrap_or(&uri).to_string();

    if let Err(e) = enforcer.enforce((&subject, &path, &method)) {
        log::warn!("Casbin 权限拒绝: sub={}, path={}, method={}, err={}", subject, path, method, e);
        res.status_code(StatusCode::FORBIDDEN);
        res.render(Json(serde_json::json!({
            "code": "-1",
            "msg": format!("无权限访问: {}", path)
        })));
        ctrl.skip_rest();
        return;
    }

    ctrl.call_next(req, depot, res).await;
}
