//! 路由配置
//!
//! 汇总所有 HTTP 路由，区分公开路由（不需要认证）和受保护路由。
//! 各模块具体路由定义在对应的 handler 文件中，此处仅负责组装。

use salvo::prelude::*;

use super::handler::{
    auth_handler, user_handler, role_handler, permission_handler, department_handler,
    application_handler, instance_handler, sync_handler,
};
use super::admin_ui;
use super::internal_auth::internal_auth_handler;

/// 构建不需要认证的公开路由（登录、登出、刷新 Token）
pub fn public_routes() -> Router {
    Router::new()
        .push(auth_handler::public_routes())
        .push(admin_ui::auth_admin_ui_router())
}

/// 构建仅需 JWT 签名验证的内部接口路由（服务间调用）
pub fn internal_routes() -> Router {
    Router::new()
        .hoop(internal_auth_handler)
        .push(sync_handler::internal_routes())
        .push(instance_handler::internal_instance_routes())
}

/// 构建需要认证的受保护路由
pub fn protected_routes() -> Router {
    Router::new()
        .push(auth_handler::protected_routes())
        .push(user_handler::routes())
        .push(role_handler::routes())
        .push(permission_handler::routes())
        .push(department_handler::routes())
        .push(application_handler::routes())
        .push(instance_handler::protected_instance_routes())
}
