//! OAuth 2.0 资源服务器中间件
//!
//! 提供 OAuth 2.0 Access Token 验证中间件：
//! - `oauth2_auth` — OAuth Token 认证中间件（JWT 本地验证 + Opaque 远程内省）
//! - `combined_oauth2_auth` — OAuth 认证 + Casbin 授权组合中间件
//!
//! JWT Token：本地验证（使用共享 jwt_secret），无需网络调用。
//! Opaque Token：调用 auth-admin 的 /oauth/introspect 端点进行内省。

use std::sync::Arc;

use salvo::http::StatusCode;
use salvo::prelude::*;
use casbin::CoreApi;

use crate::auth_middleware::{verify_oauth_token, OAuthClaims};

// ============================================================================
// OAuth2 认证配置
// ============================================================================

/// OAuth 2.0 资源服务器配置
#[derive(Clone)]
pub struct OAuth2AuthConfig {
    /// JWT 共享密钥（用于本地验证 JWT Token）
    pub jwt_secret: String,
    /// auth-admin Token 内省 URL
    pub introspect_url: String,
    /// 调用内省端点时使用的服务 Token（可选）
    pub service_token: Option<String>,
}

impl OAuth2AuthConfig {
    pub fn new(jwt_secret: impl Into<String>, introspect_url: impl Into<String>) -> Self {
        Self {
            jwt_secret: jwt_secret.into(),
            introspect_url: introspect_url.into(),
            service_token: None,
        }
    }

    pub fn with_service_token(mut self, token: impl Into<String>) -> Self {
        self.service_token = Some(token.into());
        self
    }
}

// ============================================================================
// OAuth2 Auth Middleware
// ============================================================================

/// OAuth 2.0 认证中间件
///
/// 先尝试本地 JWT 验证，失败后调用 introspect 端点验证 opaque token。
///
/// # 依赖
/// - `OAuth2AuthConfig` 必须通过 `affix_state::inject` 注入到 Depot
///
/// # 注入到 Depot 的数据
/// - `"jwtToken"` → `genies::core::jwt::JWTToken`（兼容 casbin_auth）
/// - `"oauth_claims"` → `OAuthClaims`
/// - `"subject"` → 用户名（供 Casbin 使用）
#[handler]
pub async fn oauth2_auth(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    let config = match depot.obtain::<Arc<OAuth2AuthConfig>>() {
        Ok(c) => c,
        Err(_) => {
            log::error!("OAuth2AuthConfig 未注入到 Depot");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "code": "-1",
                "msg": "OAuth2 认证配置错误"
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

    let token_value = token.strip_prefix("Bearer ").unwrap_or(token);

    // Step 1: 尝试本地 JWT 验证
    if let Ok(claims) = verify_oauth_token(token_value, &config.jwt_secret) {
        inject_user(depot, claims);
        ctrl.call_next(req, depot, res).await;
        return;
    }

    // Step 2: JWT 验证失败，尝试 introspect
    match introspect_token(token_value, &config).await {
        Ok(Some(claims)) => {
            inject_user(depot, claims);
            ctrl.call_next(req, depot, res).await;
        }
        Ok(None) => {
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({
                "code": "-1",
                "msg": "令牌无效或已过期"
            })));
            ctrl.skip_rest();
        }
        Err(e) => {
            log::warn!("OAuth2 introspect 失败: {}", e);
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({
                "code": "-1",
                "msg": "令牌验证失败"
            })));
            ctrl.skip_rest();
        }
    }
}

// ============================================================================
// Combined OAuth2 Auth + Casbin Middleware
// ============================================================================

/// 组合 OAuth2 认证 + Casbin 授权中间件
///
/// 先执行 OAuth2 认证，再执行 Casbin API 权限检查。
/// 使用时需要在 Depot 中注入 `OAuth2AuthConfig` 和 `EnforcerManager`。
#[handler]
pub async fn combined_oauth2_auth(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    // Step 1: OAuth2 认证
    let config = match depot.obtain::<Arc<OAuth2AuthConfig>>() {
        Ok(c) => c,
        Err(_) => {
            log::error!("OAuth2AuthConfig 未注入到 Depot");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "code": "-1",
                "msg": "OAuth2 认证配置错误"
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

    let token_value = token.strip_prefix("Bearer ").unwrap_or(token);

    // 认证
    let claims = if let Ok(c) = verify_oauth_token(token_value, &config.jwt_secret) {
        c
    } else {
        match introspect_token(token_value, &config).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                res.status_code(StatusCode::UNAUTHORIZED);
                res.render(Json(serde_json::json!({
                    "code": "-1",
                    "msg": "令牌无效或已过期"
                })));
                ctrl.skip_rest();
                return;
            }
            Err(e) => {
                log::warn!("OAuth2 introspect 失败: {}", e);
                res.status_code(StatusCode::UNAUTHORIZED);
                res.render(Json(serde_json::json!({
                    "code": "-1",
                    "msg": "令牌验证失败"
                })));
                ctrl.skip_rest();
                return;
            }
        }
    };

    inject_user(depot, claims);

    // Step 2: Casbin 权限检查
    let subject = depot
        .get::<genies::core::jwt::JWTToken>("jwtToken")
        .map(|t| t.preferred_username.clone().unwrap_or_else(|| "guest".into()))
        .unwrap_or_else(|_| "guest".into());

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

// ============================================================================
// 内部辅助
// ============================================================================

/// 将 OAuthClaims 注入到 Depot
fn inject_user(depot: &mut Depot, claims: OAuthClaims) {
    let jwt_token = genies::core::jwt::JWTToken {
        preferred_username: Some(claims.sub.clone()),
        user_id: claims.uid.map(|id| id.to_string()),
        name: claims.name.clone(),
        scope: Some(claims.scope.clone()),
        id: Some(claims.jti.clone()),
        exp: Some(claims.exp),
        iat: Some(claims.iat),
        ..Default::default()
    };

    depot.insert("jwtToken", jwt_token);
    depot.insert("subject", claims.sub.clone());
    depot.insert("oauth_claims", claims);
}

/// 调用 auth-admin 的 /oauth/introspect 端点
async fn introspect_token(token: &str, config: &OAuth2AuthConfig) -> Result<Option<OAuthClaims>, String> {
    let client = reqwest::Client::new();
    let mut request = client
        .post(&config.introspect_url)
        .header("Content-Type", "application/x-www-form-urlencoded");

    if let Some(ref svc_token) = config.service_token {
        request = request.header("Authorization", format!("Bearer {}", svc_token));
    }

    let body = format!("token={}&token_type_hint=access_token", token);
    let resp = request
        .body(body)
        .send()
        .await
        .map_err(|e| format!("introspect 请求失败: {}", e))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("introspect 响应解析失败: {}", e))?;

    if json.get("active").and_then(|v| v.as_bool()).unwrap_or(false) {
        Ok(Some(OAuthClaims {
            sub: json["sub"].as_str().unwrap_or("unknown").to_string(),
            uid: json["uid"].as_i64(),
            name: json["username"].as_str().map(|s| s.to_string()),
            client_id: json["client_id"].as_str().unwrap_or("").to_string(),
            scope: json["scope"].as_str().unwrap_or("").to_string(),
            jti: String::new(),
            iat: json["iat"].as_u64().unwrap_or(0) as usize,
            exp: json["exp"].as_u64().unwrap_or(0) as usize,
        }))
    } else {
        Ok(None)
    }
}
