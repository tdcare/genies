#![allow(non_snake_case)]

use std::sync::Arc;
use std::thread;
use salvo::prelude::*;
use salvo::affix_state;
use genies::context::CONTEXT;
use genies::k8s::k8s_health_check;
use genies_auth::{
    extract_and_sync_schemas,
    auth_router,
    auth_public_router,
    auth_full_router,
    LocalAuthConfig,
    EnforcerManager,
};

#[tokio::main]
async fn main() {
    genies::config::log_config::init_log();

    log::info!(
        "Sickbed Service: http://{}",
        CONTEXT.config.server_url.replace("0.0.0.0", "127.0.0.1")
    );

    // 初始化数据库并运行迁移
    CONTEXT.init_database().await;
    sickbed::infrastructure::migration::run_migrations().await;
    genies_auth::models::run_migrations().await;

    // 注册实例并启动心跳
    let _registry_guard = genies_auth::try_register_and_heartbeat(&CONTEXT.config).await;

    // 构建业务路由（用于 OpenAPI 文档生成）
    let business_router = sickbed::interfaces::router::sickbed_router();

    // OpenApi Schema 同步：合并业务路由和 auth admin 路由
    let auth_admin = auth_router();
    let doc = OpenApi::new("sickbed", "0.0.1")
        .merge_router(&business_router)
        .merge_router(&auth_admin);

    // 将 API 端点和 Schema 对象同步到权限管理数据库
    if let Err(e) = extract_and_sync_schemas(&doc).await {
        log::warn!("Schema 同步失败: {}", e);
    }

    // 初始化 Casbin EnforcerManager
    let mgr = Arc::new(EnforcerManager::new().await.unwrap());

    let _server = thread::spawn(|| async move {
        // 开发模式下加 servlet_path 前缀（模拟 nginx gateway 的一级目录路由）
        // 生产模式下无前缀（由 nginx gateway 处理）
        let prefix = if CONTEXT.config.debug {
            CONTEXT.config.servlet_path.clone()
        } else {
            String::new()
        };

        let mut app_router = Router::new()
            .push(k8s_health_check())
            .push(genies::dapr_event_router());

        if CONTEXT.config.debug {
            // 开发模式：所有服务路由挂载在 servlet_path 前缀下
            app_router = app_router.push(
                Router::with_path(&CONTEXT.config.servlet_path)
                    .hoop(affix_state::inject(mgr.clone()))
                    .push(business_router)
                    .push(auth_full_router())
                    .push(auth_public_router()),
            );
        } else {
            // 生产模式：无前缀，nginx gateway 负责一级目录路由
            app_router = app_router
                .push(auth_full_router().hoop(affix_state::inject(mgr.clone())))
                .push(auth_public_router())
                .push(
                    Router::with_path("/")
                        .hoop(affix_state::inject(mgr.clone()))
                        .push(business_router),
                );
        }

        let app_router = app_router
            .unshift(doc.into_router(prefix.clone() + "/api-doc/openapi.json"))
            .unshift(
                SwaggerUi::new(prefix.clone() + "/api-doc/openapi.json")
                    .into_router(prefix.clone() + "/swagger-ui"),
            );

        let acceptor = TcpListener::new(&CONTEXT.config.server_url).bind().await;
        Server::new(acceptor).serve(app_router).await;
    });
    _server.join().unwrap().await
}
