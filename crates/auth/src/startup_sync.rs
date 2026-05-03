//! 启动时用户-角色同步模块
//!
//! 在 local 认证模式下，微服务启动时从 auth-admin 拉取用户-角色映射，
//! 全量替换本地 casbin_rules 表中的 g 规则。
//!
//! # 使用方式
//! ```ignore
//! use genies_auth::startup_sync::try_sync_on_startup;
//!
//! // 在数据库迁移后、Enforcer 初始化前调用
//! try_sync_on_startup(&config).await;
//! ```

use genies::context::CONTEXT;
use genies_config::app_config::ApplicationConfig;
use genies_core::{RespVO, CODE_SUCCESS};
use jsonwebtoken::{encode, Header, EncodingKey};
use serde::{Deserialize, Serialize};

// ============================================================================
// 响应数据结构
// ============================================================================

/// auth-admin 同步接口返回的单条 g 规则
#[derive(Debug, Deserialize)]
struct UserRoleRule {
    #[allow(dead_code)]
    ptype: String,
    v0: String,
    v1: String,
}

/// 服务间调用的 JWT Claims
#[derive(Debug, Serialize, Deserialize)]
struct ServiceClaims {
    sub: String,
    name: Option<String>,
    iat: usize,
    exp: usize,
}

// ============================================================================
// 核心同步逻辑
// ============================================================================

/// 全量替换 casbin_rules 中的 g 规则（事务内执行）
///
/// 1. 删除所有 ptype='g' 的规则
/// 2. 批量插入新规则
///
/// # 参数
/// * `rules` - 用户-角色映射列表，每个元素为 (用户标识, 角色标识)
///
/// # Returns
/// * `Ok(count)` - 成功替换的规则数量
pub async fn replace_g_rules(rules: &[(String, String)]) -> anyhow::Result<usize> {
    let count = rules.len();
    let rb = &CONTEXT.rbatis;
    let tx = rb.acquire_begin().await?;

    // 清除旧的用户-角色映射 g 规则
    tx.exec("DELETE FROM casbin_rules WHERE ptype = 'g'", vec![]).await?;

    // 批量插入新规则
    for (v0, v1) in rules {
        tx.exec(
            "INSERT INTO casbin_rules (ptype, v0, v1) VALUES ('g', ?, ?)",
            vec![rbs::value!(v0), rbs::value!(v1)],
        ).await?;
    }

    tx.commit().await?;
    Ok(count)
}

/// 从 auth-admin 拉取用户-角色映射并全量替换本地 casbin_rules 中的 g 规则
///
/// # 逻辑
/// 1. HTTP GET `{auth_admin_url}/auth-admin/sync/user-roles`
/// 2. 解析 JSON 响应中的 g 规则列表
/// 3. 事务内：先删除所有旧 g 规则，再批量插入新规则
///
/// # Returns
/// * `Ok(count)` - 成功同步的规则条数
/// * `Err(_)` - HTTP 请求或数据库操作失败
pub async fn sync_user_roles_from_admin(auth_admin_url: &str, jwt_secret: &str) -> anyhow::Result<usize> {
    let url = format!("{}/auth-admin/sync/user-roles", auth_admin_url.trim_end_matches('/'));

    // 生成短期服务 JWT（60 秒有效期）
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = ServiceClaims {
        sub: "auth-service".to_string(),
        name: Some("auth-sync".to_string()),
        iat: now,
        exp: now + 60,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp: RespVO<Vec<UserRoleRule>> = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?
        .json()
        .await?;

    // 校验业务响应码
    let code = resp.code.as_deref().unwrap_or("");
    if code != CODE_SUCCESS {
        anyhow::bail!(
            "auth-admin 同步接口返回业务错误: code={}, msg={}",
            code,
            resp.msg.as_deref().unwrap_or("unknown")
        );
    }

    let data = resp.data.unwrap_or_default();
    let rules: Vec<(String, String)> = data.iter()
        .filter(|r| r.ptype == "g")
        .map(|r| (r.v0.clone(), r.v1.clone()))
        .collect();

    replace_g_rules(&rules).await
}

/// 启动时尝试同步用户-角色映射
///
/// 仅在 `auth_mode == "local"` 且 `auth_admin_url` 非空时执行同步。
/// 同步失败不会中断启动流程，仅打印 warn 日志。
pub async fn try_sync_on_startup(config: &ApplicationConfig) {
    if config.auth_mode == "local" && !config.auth_admin_url.is_empty() {
        match sync_user_roles_from_admin(&config.auth_admin_url, &config.jwt_secret).await {
            Ok(count) => log::info!("从 auth-admin 同步了 {} 条用户-角色规则", count),
            Err(e) => log::warn!("从 auth-admin 同步用户-角色失败（服务将使用本地已有规则）: {}", e),
        }
    }
}
