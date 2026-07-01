//! OAuth 客户端管理 — 应用服务层

use crate::application::oauth_dto::*;
use crate::domain::entity::oauth_client_entity::OAuthClientEntity;
use crate::domain::service::OAuthDomainService;
use genies::context::CONTEXT;

pub struct OAuthClientAppService;

impl OAuthClientAppService {
    /// 分页列表
    pub async fn list_clients(
        page: u64,
        size: u64,
        keyword: &str,
    ) -> Result<(Vec<OAuthClientVO>, u64), String> {
        let rb = &CONTEXT.rbatis;
        let total = OAuthClientEntity::count(rb, keyword)
            .await
            .map_err(|e| e.to_string())?;

        let all = OAuthClientEntity::list(rb, keyword)
            .await
            .map_err(|e| e.to_string())?;

        let offset = ((page.max(1) - 1) * size) as usize;
        let list: Vec<OAuthClientVO> = all.into_iter()
            .skip(offset)
            .take(size as usize)
            .map(|e| entity_to_vo(e))
            .collect();

        Ok((list, total))
    }

    /// 详情
    pub async fn get_client(id: i64) -> Result<OAuthClientVO, String> {
        let rb = &CONTEXT.rbatis;
        let entity = OAuthClientEntity::find_by_id(rb, &id)
            .await
            .map_err(|e| e.to_string())?
            .into_iter().next()
            .ok_or_else(|| "客户端不存在".to_string())?;

        Ok(entity_to_vo(entity))
    }

    /// 创建客户端（返回一次性的 secret）
    pub async fn create_client(req: &CreateOAuthClientRequest) -> Result<OAuthClientCreateResponse, String> {
        let rb = &CONTEXT.rbatis;

        if req.client_name.is_empty() {
            return Err("客户端名称不能为空".into());
        }

        // 生成 client_id 和 client_secret
        let client_id = format!("client_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..16].to_string());
        let client_secret = generate_secret();
        let secret_hash = bcrypt::hash(&client_secret, bcrypt::DEFAULT_COST)
            .map_err(|e| format!("密钥加密失败: {}", e))?;

        // 检查 client_id 唯一性
        if !OAuthClientEntity::find_by_client_id(rb, &client_id)
            .await
            .map_err(|e| e.to_string())?
            .is_empty()
        {
            return Err("客户端标识冲突，请重试".into());
        }

        let redirect_uris_json = serde_json::to_string(&req.redirect_uris)
            .map_err(|e| e.to_string())?;
        let grant_types_json = serde_json::to_string(&req.grant_types)
            .map_err(|e| e.to_string())?;
        let scopes_json = serde_json::to_string(&req.scopes)
            .map_err(|e| e.to_string())?;

        let id = OAuthDomainService::create_client(
            &client_id,
            &secret_hash,
            req.application_id,
            &req.client_name,
            &redirect_uris_json,
            &grant_types_json,
            &scopes_json,
            &req.token_format,
            req.access_token_ttl,
            req.refresh_token_ttl,
            req.require_pkce,
            1,
        ).await?;

        Ok(OAuthClientCreateResponse {
            id,
            client_id,
            client_secret,
            client_name: req.client_name.clone(),
            application_id: req.application_id,
            redirect_uris: req.redirect_uris.clone(),
            grant_types: req.grant_types.clone(),
            scopes: req.scopes.clone(),
            token_format: req.token_format.clone(),
            access_token_ttl: req.access_token_ttl,
            refresh_token_ttl: req.refresh_token_ttl,
            require_pkce: req.require_pkce,
            status: 1,
        })
    }

    /// 更新客户端
    pub async fn update_client(id: i64, req: &UpdateOAuthClientRequest) -> Result<(), String> {
        let redirect_uris_json = serde_json::to_string(&req.redirect_uris)
            .map_err(|e| e.to_string())?;
        let grant_types_json = serde_json::to_string(&req.grant_types)
            .map_err(|e| e.to_string())?;
        let scopes_json = serde_json::to_string(&req.scopes)
            .map_err(|e| e.to_string())?;

        OAuthDomainService::update_client(
            id,
            &req.client_name,
            &redirect_uris_json,
            &grant_types_json,
            &scopes_json,
            &req.token_format,
            req.access_token_ttl,
            req.refresh_token_ttl,
            req.require_pkce,
            req.status,
        ).await
    }

    /// 删除客户端
    pub async fn delete_client(id: i64) -> Result<(), String> {
        OAuthDomainService::delete_client(id).await
    }

    /// 重新生成密钥
    pub async fn regenerate_secret(id: i64) -> Result<String, String> {
        let rb = &CONTEXT.rbatis;

        // 验证客户端存在
        let _entity = OAuthClientEntity::find_by_id(rb, &id)
            .await
            .map_err(|e| e.to_string())?
            .into_iter().next()
            .ok_or_else(|| "客户端不存在".to_string())?;

        let new_secret = generate_secret();
        let secret_hash = bcrypt::hash(&new_secret, bcrypt::DEFAULT_COST)
            .map_err(|e| format!("密钥加密失败: {}", e))?;

        OAuthDomainService::update_client_secret(id, &secret_hash).await?;

        Ok(new_secret)
    }

    /// 通过 client_id 查找客户端
    pub async fn find_by_client_id(client_id: &str) -> Result<OAuthClientEntity, String> {
        let rb = &CONTEXT.rbatis;
        OAuthClientEntity::find_by_client_id(rb, client_id)
            .await
            .map_err(|e| e.to_string())?
            .into_iter().next()
            .ok_or_else(|| "客户端不存在".to_string())
    }
}

fn generate_secret() -> String {
    use rand::Rng;
    let bytes: Vec<u8> = (0..48).map(|_| rand::thread_rng().gen::<u8>()).collect();
    hex::encode(bytes)
}

fn entity_to_vo(e: OAuthClientEntity) -> OAuthClientVO {
    let redirect_uris: Vec<String> = e.redirect_uris
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let grant_types: Vec<String> = e.grant_types
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let scopes: Vec<String> = e.scopes
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    OAuthClientVO {
        id: e.id.unwrap_or(0),
        client_id: e.client_id.unwrap_or_default(),
        application_id: e.application_id.unwrap_or(0),
        client_name: e.client_name.unwrap_or_default(),
        redirect_uris,
        grant_types,
        scopes,
        token_format: e.token_format.unwrap_or_else(|| "jwt".into()),
        access_token_ttl: e.access_token_ttl.unwrap_or(3600),
        refresh_token_ttl: e.refresh_token_ttl.unwrap_or(2592000),
        require_pkce: e.require_pkce.unwrap_or(0),
        status: e.status.unwrap_or(1),
        created_at: e.created_at.map(|d| d.to_string()),
        updated_at: e.updated_at.map(|d| d.to_string()),
    }
}
