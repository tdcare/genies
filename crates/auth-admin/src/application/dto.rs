//! 请求/响应 DTO 定义

use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

/// 分页查询参数
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct PageQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub keyword: Option<String>,
}

/// 分页结果
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PageResult<T: Serialize> {
    pub total: u64,
    pub page: u64,
    pub size: u64,
    pub list: Vec<T>,
}

/// 登录请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    /// 验证码 ID（验证码功能启用时必填）
    #[serde(default)]
    pub captcha_id: Option<String>,
    /// 验证码文本
    #[serde(default)]
    pub captcha_text: Option<String>,
}

/// 登录响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    /// 完整 JWT（非 2FA 模式）或空字符串（2FA 模式需二次验证）
    pub access_token: String,
    pub token_type: String,
    pub expires_in: usize,
    pub username: String,
    pub display_name: String,
    /// 是否需要二次验证
    #[serde(default)]
    pub require_2fa: bool,
    /// 预授权 Token（2FA 模式下用于二次验证）
    #[serde(default)]
    pub preauth_token: Option<String>,
    /// 可用的 2FA 验证方式
    #[serde(default)]
    pub available_methods: Vec<String>,
    /// 是否需要强制设置 2FA（系统已启用 2FA 但用户未配置）
    #[serde(default)]
    pub require_2fa_setup: bool,
    /// 2FA 设置截止时间（UNIX 时间戳秒），超过后不可跳过
    #[serde(default)]
    pub two_fa_setup_deadline: Option<usize>,
}

/// 2FA 预授权 Token 声明
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreAuthClaims {
    /// 用户 ID
    pub uid: i64,
    /// 签发时间
    pub iat: usize,
    /// 过期时间（5 分钟）
    pub exp: usize,
    /// 用途标识
    pub purpose: String,
}

/// 2FA 验证请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct TwoFactorVerifyRequest {
    /// 预授权 Token
    pub preauth_token: String,
    /// 验证码
    pub code: String,
    /// 验证方式: "totp" | "sms" | "second_password"
    pub method: String,
}

/// 修改密码请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// 添加部门成员请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddMemberRequest {
    pub user_id: i64,
}
