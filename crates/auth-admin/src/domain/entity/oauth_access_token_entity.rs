//! OAuth Access Token 实体 (opaque 模式)

use rbatis::crud;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OAuthAccessTokenEntity {
    pub id: Option<i64>,
    pub token_hash: Option<String>,
    pub client_id: Option<i64>,
    pub user_id: Option<i64>,
    pub scopes: Option<String>,
    pub expires_at: Option<rbdc::DateTime>,
    pub revoked: Option<i8>,
    pub created_at: Option<rbdc::DateTime>,
}

crud!(OAuthAccessTokenEntity {}, "auth_oauth_access_tokens");
