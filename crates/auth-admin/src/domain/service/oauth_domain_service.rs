//! OAuth 领域服务
//!
//! 封装 OAuth 客户端、授权码、令牌的持久化操作，
//! 所有方法内部自行管理事务边界。

use std::str::FromStr;

use crate::domain::entity::oauth_client_entity::OAuthClientEntity;
use crate::domain::entity::oauth_authorization_code_entity::OAuthAuthorizationCodeEntity;
use crate::domain::entity::oauth_access_token_entity::OAuthAccessTokenEntity;
use crate::domain::entity::oauth_refresh_token_entity::OAuthRefreshTokenEntity;
use genies::context::CONTEXT;

pub struct OAuthDomainService;

/// 计算本地时间的过期时间（与 rbdc::DateTime::now() 保持一致时区）
fn expire_at(ttl_secs: i64) -> rbdc::DateTime {
    let t = chrono::Local::now() + chrono::Duration::seconds(ttl_secs);
    let s = t.format("%Y-%m-%d %H:%M:%S").to_string();
    rbdc::DateTime::from_str(&s).unwrap_or(rbdc::DateTime::now())
}

impl OAuthDomainService {
    // ========================================================================
    // OAuth 客户端 CRUD
    // ========================================================================

    /// 创建 OAuth 客户端
    pub async fn create_client(
        client_id: &str,
        client_secret_hash: &str,
        application_id: i64,
        client_name: &str,
        redirect_uris: &str,
        grant_types: &str,
        scopes: &str,
        token_format: &str,
        access_token_ttl: i32,
        refresh_token_ttl: i32,
        require_pkce: i8,
        status: i8,
    ) -> Result<i64, String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        let entity = OAuthClientEntity {
            id: None,
            client_id: Some(client_id.to_string()),
            client_secret_hash: Some(client_secret_hash.to_string()),
            application_id: Some(application_id),
            client_name: Some(client_name.to_string()),
            redirect_uris: Some(redirect_uris.to_string()),
            grant_types: Some(grant_types.to_string()),
            scopes: Some(scopes.to_string()),
            token_format: Some(token_format.to_string()),
            access_token_ttl: Some(access_token_ttl),
            refresh_token_ttl: Some(refresh_token_ttl),
            require_pkce: Some(require_pkce),
            status: Some(status),
            created_at: None,
            updated_at: None,
        };

        let result = OAuthClientEntity::insert(&tx, &entity)
            .await
            .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(result.last_insert_id.as_i64().unwrap_or(0))
    }

    /// 更新 OAuth 客户端
    pub async fn update_client(
        id: i64,
        client_name: &str,
        redirect_uris: &str,
        grant_types: &str,
        scopes: &str,
        token_format: &str,
        access_token_ttl: i32,
        refresh_token_ttl: i32,
        require_pkce: i8,
        status: i8,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        let mut entity = OAuthClientEntity::find_by_id(&tx, &id)
            .await
            .map_err(|e| e.to_string())?
            .into_iter().next()
            .ok_or_else(|| "客户端不存在".to_string())?;

        entity.client_name = Some(client_name.to_string());
        entity.redirect_uris = Some(redirect_uris.to_string());
        entity.grant_types = Some(grant_types.to_string());
        entity.scopes = Some(scopes.to_string());
        entity.token_format = Some(token_format.to_string());
        entity.access_token_ttl = Some(access_token_ttl);
        entity.refresh_token_ttl = Some(refresh_token_ttl);
        entity.require_pkce = Some(require_pkce);
        entity.status = Some(status);

        OAuthClientEntity::update_by_map(&tx, &entity, "id".into())
            .await
            .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 删除 OAuth 客户端
    pub async fn delete_client(id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        OAuthClientEntity::delete_by_id(&tx, &id)
            .await
            .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 更新客户端密钥哈希
    pub async fn update_client_secret(id: i64, secret_hash: &str) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        OAuthClientEntity::update_secret_hash(rb, &id, secret_hash)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ========================================================================
    // 授权码操作
    // ========================================================================

    /// 存储授权码（10分钟过期）
    pub async fn store_authorization_code(
        code: &str,
        client_id: i64,
        user_id: Option<i64>,
        redirect_uri: &str,
        scopes: Option<&str>,
        code_challenge: Option<&str>,
        code_challenge_method: Option<&str>,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        let entity = OAuthAuthorizationCodeEntity {
            id: None,
            code: Some(code.to_string()),
            client_id: Some(client_id),
            user_id,
            redirect_uri: Some(redirect_uri.to_string()),
            scopes: scopes.map(|s| s.to_string()),
            code_challenge: code_challenge.map(|s| s.to_string()),
            code_challenge_method: code_challenge_method.map(|s| s.to_string()),
            expires_at: Some(expire_at(600)), // 10分钟
            used: Some(0),
            created_at: None,
        };

        OAuthAuthorizationCodeEntity::insert(&tx, &entity)
            .await
            .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 消费授权码（原子标记已使用）
    pub async fn consume_authorization_code(code: &str) -> Result<Option<OAuthAuthorizationCodeEntity>, String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        let entity = OAuthAuthorizationCodeEntity::find_by_code(&tx, code)
            .await
            .map_err(|e| e.to_string())?;

        match entity {
            Some(ref e) if e.used == Some(0) => {
                let id = e.id.unwrap_or(0);
                OAuthAuthorizationCodeEntity::mark_used(&tx, &id)
                    .await
                    .map_err(|e| e.to_string())?;
                tx.commit().await.map_err(|e| e.to_string())?;
                Ok(entity)
            }
            _ => {
                tx.commit().await.map_err(|e| e.to_string())?;
                Ok(None)
            }
        }
    }

    // ========================================================================
    // Access Token 操作（opaque 模式）
    // ========================================================================

    /// 存储 opaque Access Token
    pub async fn store_access_token(
        token_hash: &str,
        client_id: i64,
        user_id: Option<i64>,
        scopes: Option<&str>,
        ttl_secs: i64,
    ) -> Result<i64, String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        let entity = OAuthAccessTokenEntity {
            id: None,
            token_hash: Some(token_hash.to_string()),
            client_id: Some(client_id),
            user_id,
            scopes: scopes.map(|s| s.to_string()),
            expires_at: Some(expire_at(ttl_secs)),
            revoked: Some(0),
            created_at: None,
        };

        let result = OAuthAccessTokenEntity::insert(&tx, &entity)
            .await
            .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(result.last_insert_id.as_i64().unwrap_or(0))
    }

    /// 撤销 Access Token
    pub async fn revoke_access_token_by_hash(token_hash: &str) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        if let Some(entity) = OAuthAccessTokenEntity::find_by_token_hash(rb, token_hash)
            .await
            .map_err(|e| e.to_string())?
        {
            let id = entity.id.unwrap_or(0);
            OAuthAccessTokenEntity::revoke_by_id(rb, &id)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // ========================================================================
    // Refresh Token 操作
    // ========================================================================

    /// 存储 Refresh Token
    pub async fn store_refresh_token(
        token_hash: &str,
        client_id: i64,
        user_id: i64,
        scopes: Option<&str>,
        access_token_id: Option<i64>,
        ttl_secs: i64,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        let entity = OAuthRefreshTokenEntity {
            id: None,
            token_hash: Some(token_hash.to_string()),
            client_id: Some(client_id),
            user_id: Some(user_id),
            scopes: scopes.map(|s| s.to_string()),
            access_token_id,
            expires_at: Some(expire_at(ttl_secs)),
            revoked: Some(0),
            created_at: None,
        };

        OAuthRefreshTokenEntity::insert(&tx, &entity)
            .await
            .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 撤销 Refresh Token（轮转）
    pub async fn revoke_refresh_token_by_hash(token_hash: &str) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        if let Some(entity) = OAuthRefreshTokenEntity::find_by_token_hash(rb, token_hash)
            .await
            .map_err(|e| e.to_string())?
        {
            let id = entity.id.unwrap_or(0);
            OAuthRefreshTokenEntity::revoke_by_id(rb, &id)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // ========================================================================
    // 清理过期数据
    // ========================================================================

    /// 清理所有过期的 OAuth 数据
    pub async fn cleanup_expired() {
        let rb = &CONTEXT.rbatis;
        let _ = OAuthAuthorizationCodeEntity::delete_expired(rb).await;
        let _ = OAuthAccessTokenEntity::delete_expired(rb).await;
        let _ = OAuthRefreshTokenEntity::delete_expired(rb).await;
    }
}
