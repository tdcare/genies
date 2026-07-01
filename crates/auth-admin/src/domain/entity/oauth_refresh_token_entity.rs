//! OAuth Refresh Token 实体

use rbatis::crud;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OAuthRefreshTokenEntity {
    pub id: Option<i64>,
    pub token_hash: Option<String>,
    pub client_id: Option<i64>,
    pub user_id: Option<i64>,
    pub scopes: Option<String>,
    pub access_token_id: Option<i64>,
    pub expires_at: Option<rbdc::DateTime>,
    pub revoked: Option<i8>,
    pub created_at: Option<rbdc::DateTime>,
}

crud!(OAuthRefreshTokenEntity {}, "auth_oauth_refresh_tokens");
