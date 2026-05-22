//! OAuth 2.0 协议 Handler
//!
//! 标准 OAuth 2.0 端点：授权、令牌、内省、撤销

use salvo::prelude::*;
use salvo::http::StatusCode;

use crate::application::oauth_dto::*;
use crate::application::oauth_token_service::OAuthTokenService;

/// 公开 OAuth 路由
pub fn oauth_routes() -> Router {
    Router::new()
        .push(Router::with_path("/oauth/authorize").get(oauth_authorize))
        .push(Router::with_path("/oauth/token").post(oauth_token))
        .push(Router::with_path("/oauth/introspect").post(oauth_introspect))
        .push(Router::with_path("/oauth/revoke").post(oauth_revoke))
}

/// GET /oauth/authorize — 授权端点
///
/// 需要用户已登录（通过 Bearer Token 或 session）。
/// 验证 client_id、redirect_uri、scope 后生成授权码并重定向。
#[endpoint(tags("oauth"), summary = "OAuth 2.0 授权端点")]
pub async fn oauth_authorize(
    req: &mut Request,
    res: &mut Response,
) {
    // 从 query 参数解析
    let response_type = req.query::<String>("response_type").unwrap_or_default();
    let client_id = req.query::<String>("client_id").unwrap_or_default();
    let redirect_uri = req.query::<String>("redirect_uri").unwrap_or_default();
    let scope = req.query::<String>("scope");
    let state = req.query::<String>("state");
    let code_challenge = req.query::<String>("code_challenge");
    let code_challenge_method = req.query::<String>("code_challenge_method");

    // 验证用户已登录（从 Authorization header 提取）
    let token = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let user_id = match extract_user_id_from_token(token) {
        Ok(uid) => uid,
        Err(_) => {
            // 未登录：重定向到登录页面，保留原始 OAuth 参数
            let login_path = format!(
                "/ui/#/login?redirect=/oauth/authorize%3Fresponse_type%3D{}%26client_id%3D{}%26redirect_uri%3D{}%26scope%3D{}%26state%3D{}",
                response_type,
                urlencoding(&client_id),
                urlencoding(&redirect_uri),
                urlencoding(&scope.unwrap_or_default()),
                urlencoding(&state.unwrap_or_default()),
            );
            res.render(Redirect::found(&login_path));
            return;
        }
    };

    let auth_req = OAuthAuthorizeRequest {
        response_type,
        client_id,
        redirect_uri,
        scope,
        state,
        code_challenge,
        code_challenge_method,
    };

    match OAuthTokenService::authorize(&auth_req, user_id).await {
        Ok(redirect_url) => {
            res.render(Redirect::found(&redirect_url));
        }
        Err(msg) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(serde_json::json!({
                "error": "invalid_request",
                "error_description": msg
            })));
        }
    }
}

/// POST /oauth/token — 令牌端点
///
/// 支持 4 种 grant_type：
/// - authorization_code
/// - client_credentials
/// - password
/// - refresh_token
#[endpoint(tags("oauth"), summary = "OAuth 2.0 令牌端点")]
pub async fn oauth_token(
    req: &mut Request,
    res: &mut Response,
) {
    // 从请求体中提取 form-urlencoded 数据
    let body = req.payload().await.cloned().unwrap_or_default();
    let body_str = String::from_utf8_lossy(&body).to_string();

    let grant_type = extract_form_field(&body_str, "grant_type").unwrap_or_default();
    let code = extract_form_field(&body_str, "code");
    let redirect_uri = extract_form_field(&body_str, "redirect_uri");
    let code_verifier = extract_form_field(&body_str, "code_verifier");
    let username = extract_form_field(&body_str, "username");
    let password = extract_form_field(&body_str, "password");
    let refresh_token = extract_form_field(&body_str, "refresh_token");
    let client_id = extract_form_field(&body_str, "client_id");
    let client_secret = extract_form_field(&body_str, "client_secret");
    let scope = extract_form_field(&body_str, "scope");

    let token_req = OAuthTokenRequest {
        grant_type,
        code,
        redirect_uri,
        code_verifier,
        username,
        password,
        refresh_token,
        client_id,
        client_secret,
        scope,
    };

    match OAuthTokenService::token(&token_req).await {
        Ok(token_resp) => {
            res.render(Json(token_resp));
        }
        Err(msg) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(serde_json::json!({
                "error": "invalid_grant",
                "error_description": msg
            })));
        }
    }
}

/// POST /oauth/introspect — 令牌内省 (RFC 7662)
#[endpoint(tags("oauth"), summary = "OAuth 2.0 令牌内省")]
pub async fn oauth_introspect(
    req: &mut Request,
    res: &mut Response,
) {
    let body = req.payload().await.cloned().unwrap_or_default();
    let body_str = String::from_utf8_lossy(&body).to_string();

    let token = extract_form_field(&body_str, "token").unwrap_or_default();
    let token_type_hint = extract_form_field(&body_str, "token_type_hint");

    let intro_req = OAuthIntrospectRequest {
        token,
        token_type_hint,
    };

    let result = OAuthTokenService::introspect(&intro_req).await;
    res.render(Json(result));
}

/// POST /oauth/revoke — 令牌撤销 (RFC 7009)
#[endpoint(tags("oauth"), summary = "OAuth 2.0 令牌撤销")]
pub async fn oauth_revoke(
    req: &mut Request,
    res: &mut Response,
) {
    let body = req.payload().await.cloned().unwrap_or_default();
    let body_str = String::from_utf8_lossy(&body).to_string();

    let token = extract_form_field(&body_str, "token").unwrap_or_default();
    let token_type_hint = extract_form_field(&body_str, "token_type_hint");

    let revoke_req = OAuthRevokeRequest {
        token,
        token_type_hint,
    };

    let _ = OAuthTokenService::revoke(&revoke_req).await;
    // RFC 7009: 总是返回 200 OK
    res.status_code(StatusCode::OK);
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 从 form-urlencoded 字符串中提取字段值
fn extract_form_field(body: &str, field: &str) -> Option<String> {
    let prefix = format!("{}=", field);
    for part in body.split('&') {
        if let Some(stripped) = part.strip_prefix(&prefix) {
            return Some(url_decode(stripped));
        }
    }
    // 也尝试匹配以 & 开头的
    let prefix2 = format!("&{}=", field);
    if let Some(pos) = body.find(&prefix2) {
        let start = pos + prefix2.len();
        let end = body[start..].find('&').map(|p| start + p).unwrap_or(body.len());
        return Some(url_decode(&body[start..end]));
    }
    None
}

/// URL 解码
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(hex) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    result.push(hex as char);
                    i += 3;
                    continue;
                }
            }
            b'+' => {
                result.push(' ');
                i += 1;
                continue;
            }
            _ => {}
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

/// URL 编码（简单实现）
fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

/// 从 Authorization header 提取用户 ID（解析 JWT）
fn extract_user_id_from_token(token: &str) -> Result<i64, String> {
    let token = token.strip_prefix("Bearer ").unwrap_or(token);
    let jwt_secret = &genies::context::CONTEXT.config.jwt_secret;

    let token_data = jsonwebtoken::decode::<serde_json::Value>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(jwt_secret.as_bytes()),
        &jsonwebtoken::Validation::default(),
    )
    .map_err(|e| format!("JWT 验证失败: {}", e))?;

    token_data.claims["uid"]
        .as_i64()
        .ok_or_else(|| "缺少 uid".to_string())
}
