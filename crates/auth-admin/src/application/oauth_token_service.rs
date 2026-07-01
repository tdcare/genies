//! OAuth 2.0 Token 服务 — 应用服务层
//!
//! 实现 OAuth 2.0 RFC 6749 核心协议逻辑：
//! - 授权码流程（authorization_code + PKCE）
//! - 客户端凭证（client_credentials）
//! - 密码模式（password）
//! - 刷新令牌（refresh_token）
//! - Token 内省（RFC 7662）
//! - Token 撤销（RFC 7009）

use crate::application::oauth_dto::*;
use crate::domain::entity::oauth_client_entity::OAuthClientEntity;
use crate::domain::entity::oauth_access_token_entity::OAuthAccessTokenEntity;
use crate::domain::entity::oauth_refresh_token_entity::OAuthRefreshTokenEntity;
use crate::domain::entity::user_entity::AdminUser;
use crate::domain::service::OAuthDomainService;
use genies::context::CONTEXT;
use base64::Engine;

/// OAuth2 JWT Claims（用于签发 JWT 格式的 Access Token）
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct OAuthJwtClaims {
    sub: String,
    uid: Option<i64>,
    name: Option<String>,
    client_id: String,
    scope: String,
    jti: String,
    iat: usize,
    exp: usize,
}

pub struct OAuthTokenService;

impl OAuthTokenService {
    // ========================================================================
    // 授权端点
    // ========================================================================

    /// 处理授权请求，返回 redirect URL
    pub async fn authorize(req: &OAuthAuthorizeRequest, user_id: i64) -> Result<String, String> {
        if req.response_type != "code" {
            return Err("不支持的 response_type，仅支持 code".into());
        }

        let client = Self::validate_client(&req.client_id).await?;

        // 验证 redirect_uri
        let redirect_uris: Vec<String> = client.redirect_uris
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        if !redirect_uris.contains(&req.redirect_uri) {
            return Err("redirect_uri 未注册".into());
        }

        // 验证 grant_types 包含 authorization_code
        let grant_types: Vec<String> = client.grant_types
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        if !grant_types.iter().any(|g| g == "authorization_code") {
            return Err("客户端未授权 authorization_code 模式".into());
        }

        // PKCE 验证
        let require_pkce = client.require_pkce.unwrap_or(0) == 1;
        if require_pkce && req.code_challenge.is_none() {
            // 重定向错误
            let mut err_url = req.redirect_uri.clone();
            err_url.push_str("?error=invalid_request&error_description=PKCE+required");
            if let Some(ref state) = req.state {
                err_url.push_str(&format!("&state={}", state));
            }
            return Ok(err_url);
        }

        // 生成授权码
        let code = format!(
            "{}{}",
            uuid::Uuid::new_v4().to_string().replace("-", ""),
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );

        OAuthDomainService::store_authorization_code(
            &code,
            client.id.unwrap_or(0),
            Some(user_id),
            &req.redirect_uri,
            req.scope.as_deref(),
            req.code_challenge.as_deref(),
            req.code_challenge_method.as_deref(),
        ).await?;

        // 构建重定向 URL
        let mut redirect_url = req.redirect_uri.clone();
        if redirect_url.contains('?') {
            redirect_url.push_str(&format!("&code={}", code));
        } else {
            redirect_url.push_str(&format!("?code={}", code));
        }
        if let Some(ref state) = req.state {
            redirect_url.push_str(&format!("&state={}", state));
        }

        Ok(redirect_url)
    }

    // ========================================================================
    // Token 端点
    // ========================================================================

    /// 分发 token 请求
    pub async fn token(req: &OAuthTokenRequest) -> Result<OAuthTokenResponse, String> {
        match req.grant_type.as_str() {
            "authorization_code" => Self::handle_authorization_code(req).await,
            "client_credentials" => Self::handle_client_credentials(req).await,
            "password" => Self::handle_password(req).await,
            "refresh_token" => Self::handle_refresh_token(req).await,
            _ => Err(format!("不支持的 grant_type: {}", req.grant_type)),
        }
    }

    /// authorization_code grant
    async fn handle_authorization_code(req: &OAuthTokenRequest) -> Result<OAuthTokenResponse, String> {
        let code = req.code.as_deref().ok_or("缺少 code 参数")?;
        let client_id_str = req.client_id.as_deref().ok_or("缺少 client_id")?;
        let client = Self::validate_client(client_id_str).await?;

        // 消费授权码
        let code_entity = OAuthDomainService::consume_authorization_code(code)
            .await?
            .ok_or("授权码无效或已使用")?;

        // 验证未过期
        let now = rbdc::DateTime::now();
        if code_entity.expires_at
            .as_ref()
            .map(|e| e < &now)
            .unwrap_or(true)
        {
            return Err("授权码已过期".into());
        }

        // 验证 client
        let code_client_id = code_entity.client_id.unwrap_or(0);
        if code_client_id != client.id.unwrap_or(0) {
            return Err("client_id 与授权码不匹配".into());
        }

        // PKCE 验证（空字符串视为未设置 PKCE）
        if let Some(ref challenge) = code_entity.code_challenge {
            if !challenge.is_empty() {
                let verifier = req.code_verifier.as_deref().ok_or("缺少 code_verifier (PKCE)")?;
                Self::verify_pkce(challenge, verifier)?;
            }
        }

        // 验证 redirect_uri（如果提供了）
        if let Some(ref req_redirect) = req.redirect_uri {
            if let Some(ref code_redirect) = code_entity.redirect_uri {
                if req_redirect != code_redirect {
                    return Err("redirect_uri 不匹配".into());
                }
            }
        }

        let user_id = code_entity.user_id.ok_or("授权码缺少用户信息")?;
        let scopes = code_entity.scopes.as_deref();

        Self::issue_tokens(&client, Some(user_id), scopes).await
    }

    /// client_credentials grant
    async fn handle_client_credentials(req: &OAuthTokenRequest) -> Result<OAuthTokenResponse, String> {
        let client_id_str = req.client_id.as_deref().ok_or("缺少 client_id")?;
        let client_secret = req.client_secret.as_deref().ok_or("缺少 client_secret")?;
        let client = Self::authenticate_client(client_id_str, client_secret).await?;

        // 验证 grant_type
        let grant_types: Vec<String> = client.grant_types
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        if !grant_types.iter().any(|g| g == "client_credentials") {
            return Err("客户端未授权 client_credentials 模式".into());
        }

        Self::issue_tokens(&client, None, req.scope.as_deref()).await
    }

    /// password grant
    async fn handle_password(req: &OAuthTokenRequest) -> Result<OAuthTokenResponse, String> {
        let client_id_str = req.client_id.as_deref().ok_or("缺少 client_id")?;
        let client_secret = req.client_secret.as_deref().ok_or("缺少 client_secret")?;
        let username = req.username.as_deref().ok_or("缺少 username")?;
        let password = req.password.as_deref().ok_or("缺少 password")?;

        let client = Self::authenticate_client(client_id_str, client_secret).await?;

        // 验证 grant_type
        let grant_types: Vec<String> = client.grant_types
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        if !grant_types.iter().any(|g| g == "password") {
            return Err("客户端未授权 password 模式".into());
        }

        // 验证用户
        let rb = &CONTEXT.rbatis;
        let user = AdminUser::find_by_username(rb, username)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("用户名或密码错误")?;

        // 验证密码
        let valid = bcrypt::verify(password, &user.password_hash)
            .unwrap_or(false);
        if !valid {
            return Err("用户名或密码错误".into());
        }

        // 检查 2FA — password grant 不支持交互式 2FA
        let user_id = user.id.ok_or("用户数据异常")?;

        Self::issue_tokens(&client, Some(user_id), req.scope.as_deref()).await
    }

    /// refresh_token grant
    async fn handle_refresh_token(req: &OAuthTokenRequest) -> Result<OAuthTokenResponse, String> {
        let client_id_str = req.client_id.as_deref().ok_or("缺少 client_id")?;
        let client_secret = req.client_secret.as_deref().ok_or("缺少 client_secret")?;
        let refresh_token = req.refresh_token.as_deref().ok_or("缺少 refresh_token")?;

        let client = Self::authenticate_client(client_id_str, client_secret).await?;

        // 验证 grant_type
        let grant_types: Vec<String> = client.grant_types
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        if !grant_types.iter().any(|g| g == "refresh_token") {
            return Err("客户端未授权 refresh_token 模式".into());
        }

        // 查找 refresh token
        let token_hash = Self::hash_token(refresh_token);
        let rb = &CONTEXT.rbatis;
        let rt_entity = OAuthRefreshTokenEntity::find_by_token_hash(rb, &token_hash)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("refresh_token 无效")?;

        // 验证未过期、未撤销
        let now = rbdc::DateTime::now();
        if rt_entity.revoked == Some(1) {
            return Err("refresh_token 已撤销".into());
        }
        if rt_entity.expires_at.as_ref().map(|e| e < &now).unwrap_or(true) {
            return Err("refresh_token 已过期".into());
        }
        if rt_entity.client_id != client.id {
            return Err("refresh_token 与 client 不匹配".into());
        }

        let user_id = rt_entity.user_id.unwrap_or(0);
        let scopes = rt_entity.scopes.as_deref();

        // 轮转：撤销旧 refresh token
        OAuthDomainService::revoke_refresh_token_by_hash(&token_hash).await?;

        // 如果是 JWT 模式，access_token_id 为 None
        // 如果是 opaque 模式，撤销关联的 access token
        if let Some(at_id) = rt_entity.access_token_id {
            OAuthAccessTokenEntity::revoke_by_id(rb, &at_id)
                .await
                .map_err(|e| e.to_string())?;
        }

        Self::issue_tokens(&client, Some(user_id), scopes).await
    }

    // ========================================================================
    // Token 发行
    // ========================================================================

    /// 根据 client 配置发行 JWT 或 opaque token
    async fn issue_tokens(
        client: &OAuthClientEntity,
        user_id: Option<i64>,
        scopes: Option<&str>,
    ) -> Result<OAuthTokenResponse, String> {
        let token_format = client.token_format.as_deref().unwrap_or("jwt");
        let access_ttl = client.access_token_ttl.unwrap_or(3600) as i64;
        let refresh_ttl = client.refresh_token_ttl.unwrap_or(2592000) as i64;

        // 查找用户信息
        let (sub, uid, name) = if let Some(uid_val) = user_id {
            let rb = &CONTEXT.rbatis;
            match AdminUser::find_by_id(rb, &uid_val)
                .await
                .map_err(|e| e.to_string())?
            {
                Some(u) => (
                    u.username.clone(),
                    Some(uid_val),
                    Some(u.display_name.clone()),
                ),
                None => ("unknown".into(), Some(uid_val), None),
            }
        } else {
            (client.client_id.as_deref().unwrap_or("unknown").into(), None, None)
        };

        let scope_str = scopes.unwrap_or("").to_string();

        let (access_token, access_token_id) = match token_format {
            "opaque" => {
                let token = generate_opaque_token();
                let token_hash = Self::hash_token(&token);
                let at_id = OAuthDomainService::store_access_token(
                    &token_hash,
                    client.id.unwrap_or(0),
                    user_id,
                    scopes,
                    access_ttl,
                ).await?;
                (token, Some(at_id))
            }
            _ => {
                // JWT
                let jti = uuid::Uuid::new_v4().to_string();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as usize;

                let claims = OAuthJwtClaims {
                    sub,
                    uid,
                    name,
                    client_id: client.client_id.as_deref().unwrap_or("").into(),
                    scope: scope_str.clone(),
                    jti,
                    iat: now,
                    exp: now + access_ttl as usize,
                };

                let token = jsonwebtoken::encode(
                    &jsonwebtoken::Header::default(),
                    &claims,
                    &jsonwebtoken::EncodingKey::from_secret(CONTEXT.config.jwt_secret.as_bytes()),
                )
                .map_err(|e| format!("JWT 签发失败: {}", e))?;

                (token, None)
            }
        };

        // 发行 refresh token
        let refresh_token_str = generate_opaque_token();
        let refresh_hash = Self::hash_token(&refresh_token_str);

        OAuthDomainService::store_refresh_token(
            &refresh_hash,
            client.id.unwrap_or(0),
            uid.unwrap_or(0),
            scopes,
            access_token_id,
            refresh_ttl,
        ).await?;

        Ok(OAuthTokenResponse {
            access_token,
            token_type: "Bearer".into(),
            expires_in: access_ttl,
            refresh_token: Some(refresh_token_str),
            scope: if scope_str.is_empty() { None } else { Some(scope_str) },
        })
    }

    // ========================================================================
    // Token 内省 (RFC 7662)
    // ========================================================================

    pub async fn introspect(req: &OAuthIntrospectRequest) -> OAuthIntrospectResponse {
        // 先尝试 JWT 验证
        let jwt_secret = &CONTEXT.config.jwt_secret;
        if let Ok(token_data) = jsonwebtoken::decode::<OAuthJwtClaims>(
            &req.token,
            &jsonwebtoken::DecodingKey::from_secret(jwt_secret.as_bytes()),
            &jsonwebtoken::Validation::default(),
        ) {
            let claims = token_data.claims;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as usize;
            if claims.exp > now {
                return OAuthIntrospectResponse {
                    active: true,
                    scope: Some(claims.scope),
                    client_id: Some(claims.client_id),
                    username: Some(claims.sub.clone()),
                    sub: Some(claims.sub),
                    exp: Some(claims.exp as i64),
                    iat: Some(claims.iat as i64),
                    token_type: Some("Bearer".into()),
                };
            }
        }

        // 尝试 opaque token
        let token_hash = Self::hash_token(&req.token);
        let rb = &CONTEXT.rbatis;
        if let Ok(Some(entity)) = OAuthAccessTokenEntity::find_by_token_hash(rb, &token_hash).await {
            let now = rbdc::DateTime::now();
            let active = entity.revoked != Some(1)
                && entity.expires_at.as_ref().map(|e| e > &now).unwrap_or(false);

            if active {
                let username = if let Some(uid) = entity.user_id {
                    AdminUser::find_by_id(rb, &uid).await.ok().flatten().map(|u| u.username)
                } else {
                    None
                };

                let client_id_str = if let Some(cid) = entity.client_id {
                    OAuthClientEntity::find_by_id(rb, &cid).await.ok()
                        .and_then(|v| v.into_iter().next())
                        .and_then(|c| c.client_id)
                } else {
                    None
                };

                return OAuthIntrospectResponse {
                    active: true,
                    scope: entity.scopes.clone(),
                    client_id: client_id_str,
                    username: username.clone(),
                    sub: username,
                    exp: entity.expires_at.map(|e| e.0.unix_timestamp()),
                    iat: None,
                    token_type: Some("Bearer".into()),
                };
            }
        }

        OAuthIntrospectResponse { active: false, scope: None, client_id: None, username: None, sub: None, exp: None, iat: None, token_type: None }
    }

    // ========================================================================
    // Token 撤销 (RFC 7009)
    // ========================================================================

    pub async fn revoke(req: &OAuthRevokeRequest) -> Result<(), String> {
        let token_hash = Self::hash_token(&req.token);
        let hint = req.token_type_hint.as_deref().unwrap_or("access_token");

        if hint == "refresh_token" {
            let _ = OAuthDomainService::revoke_refresh_token_by_hash(&token_hash).await;
        } else {
            let _ = OAuthDomainService::revoke_access_token_by_hash(&token_hash).await;
            // 级联撤销关联的 refresh token
            let rb = &CONTEXT.rbatis;
            if let Ok(Some(entity)) = OAuthAccessTokenEntity::find_by_token_hash(rb, &token_hash).await {
                if let Some(ref id) = entity.id {
                    let _ = OAuthRefreshTokenEntity::revoke_chain(rb, id).await;
                }
            }
        }

        Ok(()) // RFC 7009: 无论 token 是否存在都返回 200
    }

    // ========================================================================
    // 内部辅助方法
    // ========================================================================

    /// 验证客户端存在且未禁用
    async fn validate_client(client_id: &str) -> Result<OAuthClientEntity, String> {
        let rb = &CONTEXT.rbatis;
        let client = OAuthClientEntity::find_by_client_id(rb, client_id)
            .await
            .map_err(|e| e.to_string())?
            .into_iter().next()
            .ok_or("客户端不存在")?;

        if client.status == Some(0) {
            return Err("客户端已禁用".into());
        }

        Ok(client)
    }

    /// 验证客户端并认证 client_secret
    async fn authenticate_client(client_id: &str, client_secret: &str) -> Result<OAuthClientEntity, String> {
        let client = Self::validate_client(client_id).await?;
        let hash = client.client_secret_hash.as_deref().ok_or("客户端密钥未设置")?;
        let valid = bcrypt::verify(client_secret, hash).unwrap_or(false);
        if !valid {
            return Err("client_secret 不正确".into());
        }
        Ok(client)
    }

    /// PKCE S256 验证
    fn verify_pkce(challenge: &str, verifier: &str) -> Result<(), String> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let computed = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&hash);
        if computed != challenge {
            return Err("PKCE code_verifier 不匹配".into());
        }
        Ok(())
    }

    /// SHA-256 哈希（用于 opaque token 存储）
    fn hash_token(token: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// 生成 opaque token（64 字符十六进制随机串）
fn generate_opaque_token() -> String {
    use rand::Rng;
    let bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen::<u8>()).collect();
    hex::encode(bytes)
}
