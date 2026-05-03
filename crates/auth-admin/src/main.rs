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
    casbin_auth, auth_admin_router, extract_and_sync_schemas,
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
    let auth_config = Arc::new(LocalAuthConfig::new(CONTEXT.config.jwt_secret.clone()));
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
        .push(auth_admin_router());

    let router = Router::new()
        .push(public_router)
        .push(internal_router)
        .push(protected_router);

    // 6. 生成 OpenAPI 文档并同步 Schema 到权限系统
    let doc = OpenApi::new("auth-admin", "1.0.0").merge_router(&router);
    extract_and_sync_schemas(&doc).await.ok();

    // 7. 启动 HTTP 服务
    let acceptor = TcpListener::new(&CONTEXT.config.server_url).bind().await;
    Server::new(acceptor).serve(router).await;
}
