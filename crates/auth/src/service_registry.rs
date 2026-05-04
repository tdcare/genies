//! 微服务实例注册 / 心跳 / 注销客户端
//!
//! 启动时向 auth-admin 注册自身实例，后台定时发送心跳，
//! 进程退出时 best-effort 注销。
//!
//! # 使用方式
//! ```ignore
//! use genies_auth::try_register_and_heartbeat;
//!
//! let _guard = try_register_and_heartbeat(&config).await;
//! // guard 存活期间持续心跳，drop 时自动注销并停止心跳
//! ```

use genies_config::app_config::ApplicationConfig;
use jsonwebtoken::{encode, Header, EncodingKey};
use serde::{Deserialize, Serialize};

// ============================================================================
// JWT Claims（复用 startup_sync 的服务间通信模式）
// ============================================================================

/// 服务间调用的 JWT Claims
#[derive(Debug, Serialize, Deserialize)]
struct ServiceClaims {
    sub: String,
    name: Option<String>,
    iat: usize,
    exp: usize,
}

/// 生成短期服务 JWT（60 秒有效期）
fn get_local_service_token(jwt_secret: &str) -> anyhow::Result<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = ServiceClaims {
        sub: "auth-service".to_string(),
        name: Some("service-registry".to_string()),
        iat: now,
        exp: now + 60,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )?;

    Ok(token)
}

// ============================================================================
// 请求/响应结构
// ============================================================================

#[derive(Debug, Serialize)]
struct RegisterRequest {
    app_name: String,
    instance_id: i64,
    base_url: String,
    display_name: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct HeartbeatRequest {
    instance_id: i64,
}

#[derive(Debug, Serialize)]
struct DeregisterRequest {
    instance_id: i64,
}

// ============================================================================
// 实例 ID 生成
// ============================================================================

/// 生成实例 ID：使用雪花 ID
pub fn generate_instance_id() -> i64 {
    let id_str = genies_core::id_gen::next_id();
    id_str.parse::<i64>().unwrap_or_else(|_| {
        // fallback: 用时间戳
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    })
}

// ============================================================================
// 响应解析
// ============================================================================

/// 简单响应结构，用于解析 auth-admin 返回的 RespVO（避免泛型复杂度）
#[derive(Deserialize)]
struct SimpleResp {
    code: Option<String>,
    msg: Option<String>,
}

// ============================================================================
// 核心 HTTP 操作
// ============================================================================

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build reqwest client")
}

/// 启动时注册实例到 auth-admin
///
/// POST `{auth_admin_url}/auth-admin/internal/instances/register`
pub async fn register_instance(
    auth_admin_url: &str,
    jwt_secret: &str,
    instance_id: i64,
    config: &ApplicationConfig,
) -> anyhow::Result<()> {
    let url = format!(
        "{}/auth-admin/internal/instances/register",
        auth_admin_url.trim_end_matches('/')
    );

    let token = get_local_service_token(jwt_secret)?;

    let body = RegisterRequest {
        app_name: config.server_name.clone(),
        instance_id,
        base_url: format!("http://{}", config.server_url),
        display_name: config.server_name.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let resp = build_client()
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("register_instance 失败: status={}, body={}", status, text);
    }

    let resp_body: SimpleResp = resp.json().await?;
    let code = resp_body.code.as_deref().unwrap_or("");
    if code != "SUCCESS" {
        anyhow::bail!(
            "register_instance 业务失败: code={}, msg={}",
            code,
            resp_body.msg.as_deref().unwrap_or("unknown")
        );
    }

    Ok(())
}

/// 发送单次心跳
///
/// POST `{auth_admin_url}/auth-admin/internal/instances/heartbeat`
pub async fn send_heartbeat(
    auth_admin_url: &str,
    jwt_secret: &str,
    instance_id: i64,
) -> anyhow::Result<()> {
    let url = format!(
        "{}/auth-admin/internal/instances/heartbeat",
        auth_admin_url.trim_end_matches('/')
    );

    let token = get_local_service_token(jwt_secret)?;

    let body = HeartbeatRequest { instance_id };

    let resp = build_client()
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("send_heartbeat 失败: status={}, body={}", status, text);
    }

    let resp_body: SimpleResp = resp.json().await?;
    let code = resp_body.code.as_deref().unwrap_or("");
    if code != "SUCCESS" {
        anyhow::bail!(
            "send_heartbeat 业务失败: code={}, msg={}",
            code,
            resp_body.msg.as_deref().unwrap_or("unknown")
        );
    }

    Ok(())
}

/// 发送注销请求
///
/// POST `{auth_admin_url}/auth-admin/internal/instances/deregister`
pub async fn deregister_instance(
    auth_admin_url: &str,
    jwt_secret: &str,
    instance_id: i64,
) -> anyhow::Result<()> {
    let url = format!(
        "{}/auth-admin/internal/instances/deregister",
        auth_admin_url.trim_end_matches('/')
    );

    let token = get_local_service_token(jwt_secret)?;

    let body = DeregisterRequest { instance_id };

    let resp = build_client()
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("deregister_instance 失败: status={}, body={}", status, text);
    }

    let resp_body: SimpleResp = resp.json().await?;
    let code = resp_body.code.as_deref().unwrap_or("");
    if code != "SUCCESS" {
        anyhow::bail!(
            "deregister_instance 业务失败: code={}, msg={}",
            code,
            resp_body.msg.as_deref().unwrap_or("unknown")
        );
    }

    Ok(())
}

// ============================================================================
// 心跳循环
// ============================================================================

/// 启动后台心跳循环
pub fn start_heartbeat_loop(
    auth_admin_url: String,
    jwt_secret: String,
    instance_id: i64,
    interval_secs: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        // 跳过第一次立即触发
        interval.tick().await;

        loop {
            interval.tick().await;
            match send_heartbeat(&auth_admin_url, &jwt_secret, instance_id).await {
                Ok(()) => log::debug!("心跳发送成功 (instance_id={})", instance_id),
                Err(e) => log::warn!("心跳发送失败 (instance_id={}): {}", instance_id, e),
            }
        }
    })
}

// ============================================================================
// ServiceRegistryGuard
// ============================================================================

/// 服务注册守卫
///
/// 持有心跳任务句柄和注销所需信息。
/// Drop 时 abort 心跳任务，并 best-effort 发送注销请求。
pub struct ServiceRegistryGuard {
    heartbeat_handle: tokio::task::JoinHandle<()>,
    auth_admin_url: String,
    jwt_secret: String,
    instance_id: i64,
}

impl Drop for ServiceRegistryGuard {
    fn drop(&mut self) {
        // 停止心跳
        self.heartbeat_handle.abort();

        // best-effort 注销：在独立线程+临时 runtime 中执行
        let url = self.auth_admin_url.clone();
        let secret = self.jwt_secret.clone();
        let id = self.instance_id;

        std::thread::spawn(move || {
            if let Ok(rt) = tokio::runtime::Runtime::new() {
                rt.block_on(async {
                    if let Err(e) = deregister_instance(&url, &secret, id).await {
                        log::warn!("实例注销失败 (instance_id={}): {}", id, e);
                    } else {
                        log::info!("实例已注销 (instance_id={})", id);
                    }
                });
            }
        });
    }
}

// ============================================================================
// 一站式入口
// ============================================================================

/// 一站式入口：注册 + 启动心跳 + 返回 guard
///
/// 条件：`config.auth_admin_url` 非空时才执行。
/// 注册失败不会中断主流程，仅打印 warn 日志并返回 `None`。
pub async fn try_register_and_heartbeat(config: &ApplicationConfig) -> Option<ServiceRegistryGuard> {
    if config.auth_admin_url.is_empty() {
        log::debug!("auth_admin_url 为空，跳过服务注册");
        return None;
    }

    let instance_id = generate_instance_id();
    let auth_admin_url = config.auth_admin_url.clone();
    let jwt_secret = config.jwt_secret.clone();

    // 注册
    match register_instance(&auth_admin_url, &jwt_secret, instance_id, config).await {
        Ok(()) => {
            log::info!(
                "服务实例注册成功: app_name={}, instance_id={}, base_url=http://{}",
                config.server_name,
                instance_id,
                config.server_url,
            );
        }
        Err(e) => {
            log::warn!(
                "服务实例注册失败（将继续启动但不发送心跳）: {}",
                e
            );
            return None;
        }
    }

    // 启动心跳
    let heartbeat_handle = start_heartbeat_loop(
        auth_admin_url.clone(),
        jwt_secret.clone(),
        instance_id,
        config.heartbeat_interval,
    );

    log::info!(
        "心跳循环已启动: interval={}s, instance_id={}",
        config.heartbeat_interval,
        instance_id,
    );

    Some(ServiceRegistryGuard {
        heartbeat_handle,
        auth_admin_url,
        jwt_secret,
        instance_id,
    })
}
