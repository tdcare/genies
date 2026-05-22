//! auth-admin — 统一认证管理服务
//!
//! 提供完整的用户、角色、权限、组织架构管理功能。
//! 所有 CRUD 操作完成后通过 Dapr pub/sub 发布 CloudEvent，
//! 下游 genies_auth 接收事件后同步更新 casbin_rules。

use std::sync::Arc;

use salvo::prelude::*;
use salvo::affix_state;
use salvo::oapi::OpenApi;

use genies::context::CONTEXT;
use genies::k8s::k8s_health_check;
use genies_auth::{
    LocalAuthConfig, EnforcerManager,
    casbin_auth, auth_router, extract_and_sync_schemas,
};

#[tokio::main]
async fn main() {
    // 1. 初始化日志
    genies::config::log_config::init_log();

    log::info!(
        "[auth-admin] 服务启动: http://{}",
        CONTEXT.config.server_url.replace("0.0.0.0", "127.0.0.1")
    );

    // 2. 初始化数据库
    CONTEXT.init_database().await;

    // 3. 执行数据库迁移（先运行 auth 的迁移创建 casbin 表，再运行 auth-admin 自身迁移）
    genies_auth::models::run_migrations().await;
    genies_auth_admin::infrastructure::migration::run_migrations().await;

    // 4. 初始化认证配置和 Enforcer
    let auth_config = Arc::new(LocalAuthConfig::with_expiry(
        CONTEXT.config.jwt_secret.clone(),
        CONTEXT.config.jwt_expires_in_secs as usize,
    ));
    let mgr = Arc::new(
        match EnforcerManager::new().await {
            Ok(m) => m,
            Err(e) => {
                log::warn!("[auth-admin] EnforcerManager 初始化失败: {}, 使用空策略降级", e);
                EnforcerManager::empty().await
            }
        }
    );

    // 5. 构建路由 — 区分公开、内部和受保护路由
    // 公开路由：k8s 健康检查、Dapr 事件、登录/登出/刷新 Token、Admin UI
    let public_router = Router::new()
        .push(k8s_health_check())
        .push(genies::dapr_event_router())
        .hoop(affix_state::inject(auth_config.clone()))
        .push(genies_auth_admin::interfaces::router::public_routes());

    // 内部路由：仅需 JWT 签名验证，用于服务间调用
    let internal_router = Router::new()
        .push(genies_auth_admin::interfaces::router::internal_routes());

    // 受保护路由：需要 JWT 认证 + Casbin 权限检查
    let protected_router = Router::new()
        .hoop(affix_state::inject(auth_config))
        .hoop(genies_auth::local_auth)
        .hoop(affix_state::inject(mgr.clone()))
        .hoop(casbin_auth)
        .push(genies_auth_admin::interfaces::router::protected_routes())
        .push(auth_router());

    let router = Router::new()
        .push(public_router)
        .push(internal_router)
        .push(protected_router);

    // 6. 生成 OpenAPI 文档并同步 Schema 到权限系统
    let doc = OpenApi::new("auth-admin", "1.0.0").merge_router(&router);
    extract_and_sync_schemas(&doc).await.ok();

    // 7. 自注册 auth-admin 实例
    let instance_id: i64 = genies::core::id_gen::next_id().parse().expect("snowflake id should be valid i64");
    let now = rbdc::DateTime::now();
    let instance = genies_auth_admin::domain::entity::app_instance_entity::AppInstanceEntity {
        id: None,
        app_name: Some(CONTEXT.config.server_name.clone()),
        instance_id: Some(instance_id),
        base_url: Some(format!("http://{}", CONTEXT.config.server_url)),
        version: Some("1.0.0".to_string()),
        status: Some(1),
        last_heartbeat_at: Some(now.clone()),
        registered_at: Some(now),
        metadata: None,
    };
    if let Err(e) = genies_auth_admin::domain::service::AppInstanceDomainService::register_or_update(&instance).await {
        log::warn!("[auth-admin] 自注册失败: {}, 不影响启动", e);
    } else {
        log::info!("[auth-admin] 自注册成功, instance_id={}", instance_id);
    }

    // 8. 启动后台心跳循环（Redis 心跳，每 heartbeat_interval 秒一次）
    let heartbeat_interval = CONTEXT.config.heartbeat_interval;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(heartbeat_interval));
        loop {
            interval.tick().await;
            if let Err(e) = genies_auth_admin::domain::service::AppInstanceDomainService::heartbeat(instance_id).await {
                log::warn!("[auth-admin] self heartbeat failed: {}", e);
            }
        }
    });

    // 9. 启动后台实例清理任务（Redis 心跳检测 + DB 清理）
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            // 对比 DB 在线实例与 Redis 心跳 key，标记离线
            if let Err(e) = genies_auth_admin::domain::service::AppInstanceDomainService::cleanup_stale().await {
                log::warn!("[auth-admin] Failed to cleanup stale instances: {}", e);
            }
            // 删除离线超过1小时的 DB 记录
            if let Err(e) = genies_auth_admin::domain::service::AppInstanceDomainService::delete_stale_instances().await {
                log::warn!("[auth-admin] Failed to delete stale instances: {}", e);
            }
        }
    });

    // 10. 启动 HTTP 服务
    let acceptor = TcpListener::new(&CONTEXT.config.server_url).bind().await;
    Server::new(acceptor).serve(router).await;
}
