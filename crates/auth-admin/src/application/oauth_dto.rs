//! OAuth 2.0 请求/响应 DTO

use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

// ============================================================================
// 授权端点
// ============================================================================

/// GET /oauth/authorize 请求参数
#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthAuthorizeRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub code_challenge: Option<String>,
    #[serde(default)]
    pub code_challenge_method: Option<String>,
}

// ============================================================================
// Token 端点
// ============================================================================

/// POST /oauth/token 请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthTokenRequest {
    pub grant_type: String,
    // authorization_code
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub code_verifier: Option<String>,
    // password
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    // refresh_token
    #[serde(default)]
    pub refresh_token: Option<String>,
    // client_credentials & all
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

/// POST /oauth/token 响应
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// OAuth 错误响应 (RFC 6749)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OAuthErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
}

// ============================================================================
// Introspect 端点 (RFC 7662)
// ============================================================================

/// POST /oauth/introspect 请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthIntrospectRequest {
    pub token: String,
    #[serde(default)]
    pub token_type_hint: Option<String>,
}

/// POST /oauth/introspect 响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OAuthIntrospectResponse {
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
}

// ============================================================================
// Revoke 端点 (RFC 7009)
// ============================================================================

/// POST /oauth/revoke 请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthRevokeRequest {
    pub token: String,
    #[serde(default)]
    pub token_type_hint: Option<String>,
}

// ============================================================================
// Client 管理 DTO
// ============================================================================

/// 创建 OAuth 客户端请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOAuthClientRequest {
    pub client_name: String,
    pub application_id: i64,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    #[serde(default = "default_grant_types")]
    pub grant_types: Vec<String>,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    #[serde(default = "default_token_format")]
    pub token_format: String,
    #[serde(default = "default_access_token_ttl")]
    pub access_token_ttl: i32,
    #[serde(default = "default_refresh_token_ttl")]
    pub refresh_token_ttl: i32,
    #[serde(default)]
    pub require_pkce: i8,
}

fn default_grant_types() -> Vec<String> {
    vec!["authorization_code".into(), "refresh_token".into()]
}
fn default_scopes() -> Vec<String> {
    vec!["openid".into(), "profile".into()]
}
fn default_token_format() -> String { "jwt".into() }
fn default_access_token_ttl() -> i32 { 3600 }
fn default_refresh_token_ttl() -> i32 { 2592000 }

/// 更新 OAuth 客户端请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOAuthClientRequest {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub scopes: Vec<String>,
    pub token_format: String,
    pub access_token_ttl: i32,
    pub refresh_token_ttl: i32,
    pub require_pkce: i8,
    pub status: i8,
}

/// OAuth 客户端 VO（列表/详情用，不含 secret）
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct OAuthClientVO {
    pub id: i64,
    pub client_id: String,
    pub application_id: i64,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub scopes: Vec<String>,
    pub token_format: String,
    pub access_token_ttl: i32,
    pub refresh_token_ttl: i32,
    pub require_pkce: i8,
    pub status: i8,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// OAuth 客户端创建响应（含一次性显示的 secret）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OAuthClientCreateResponse {
    pub id: i64,
    pub client_id: String,
    pub client_secret: String,
    pub client_name: String,
    pub application_id: i64,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub scopes: Vec<String>,
    pub token_format: String,
    pub access_token_ttl: i32,
    pub refresh_token_ttl: i32,
    pub require_pkce: i8,
    pub status: i8,
}
