//! OAuth 授权码实体

use rbatis::crud;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OAuthAuthorizationCodeEntity {
    pub id: Option<i64>,
    pub code: Option<String>,
    pub client_id: Option<i64>,
    pub user_id: Option<i64>,
    pub redirect_uri: Option<String>,
    pub scopes: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub expires_at: Option<rbdc::DateTime>,
    pub used: Option<i8>,
    pub created_at: Option<rbdc::DateTime>,
}

crud!(OAuthAuthorizationCodeEntity {}, "auth_oauth_authorization_codes");
