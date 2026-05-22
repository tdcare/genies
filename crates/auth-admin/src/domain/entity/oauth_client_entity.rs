//! OAuth 客户端实体

use rbatis::crud;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OAuthClientEntity {
    pub id: Option<i64>,
    pub client_id: Option<String>,
    pub client_secret_hash: Option<String>,
    pub application_id: Option<i64>,
    pub client_name: Option<String>,
    pub redirect_uris: Option<String>,
    pub grant_types: Option<String>,
    pub scopes: Option<String>,
    pub token_format: Option<String>,
    pub access_token_ttl: Option<i32>,
    pub refresh_token_ttl: Option<i32>,
    pub require_pkce: Option<i8>,
    pub status: Option<i8>,
    pub created_at: Option<rbdc::DateTime>,
    pub updated_at: Option<rbdc::DateTime>,
}

crud!(OAuthClientEntity {}, "auth_oauth_clients");
