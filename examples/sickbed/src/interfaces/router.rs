use salvo::prelude::*;
use genies::context::auth::salvo_auth;
use genies_auth::casbin_auth;

use crate::interfaces::handler::sickbed_controller;
use crate::interfaces::handler::ward_controller;

/// 组装所有路由
pub fn sickbed_router() -> Router {
    Router::new()
        .hoop(salvo_auth)
        .hoop(casbin_auth)
        .push(sickbed_controller::sickbed_router())
        .push(ward_controller::ward_router())
}
