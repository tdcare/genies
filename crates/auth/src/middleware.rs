//! Casbin API 接口访问控制中间件
//!
//! 提供基于 Casbin 的 API 权限检查，支持黑名单模式（默认允许，仅 deny 规则生效）。
//!
//! # 功能特性
//! - 从 JWT Token 提取用户身份（subject）
//! - 使用 Casbin Enforcer 进行权限检查
//! - 将 enforcer 和 subject 注入 Depot，供后续 Writer 字段过滤使用
//!
//! # 使用示例
//! ```ignore
//! use salvo::prelude::*;
//! use genies_auth::middleware::casbin_auth;
//!
//! // 在路由中使用中间件
//! Router::new()
//!     .hoop(casbin_auth)
//!     .push(Router::with_path("api/<**rest>").get(handler));
//! ```

use std::sync::Arc;

use casbin::{CoreApi, Enforcer};
use salvo::http::StatusCode;
use salvo::prelude::*;

use crate::enforcer_manager::EnforcerManager;

/// API 接口权限中间件 - 检查用户是否有权访问该 API
///
/// 该中间件执行以下操作：
/// 1. 从 Depot 获取 JWT Token，提取用户身份（subject）
/// 2. 从 Depot 获取 EnforcerManager，获取 Enforcer 实例
/// 3. 使用 Casbin 检查用户是否有权访问当前 API（路径 + HTTP 方法）
/// 4. 将 enforcer 和 subject 注入 Depot，供后续 Writer 字段过滤使用
///
/// # 权限模式
/// 采用黑名单模式：默认允许访问，仅当存在明确的 deny 规则时才拒绝。
/// 如果 Casbin enforce 返回 `true` 表示允许，`false` 表示拒绝。
///
/// # 依赖
/// - `salvo_auth` 中间件必须在此中间件之前执行，以将 JWTToken 存入 Depot
/// - `EnforcerManager` 必须通过 `affix_state::inject` 注入到 Depot
#[handler]
pub async fn casbin_auth(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    // 1. 获取 subject（从 JWT token 的 preferred_username）
    //    salvo_auth 中间件已将 JWTToken 存入 depot.insert("jwtToken", data)
    //    JWTToken 类型来自 genies::core::jwt::JWTToken
    let subject = match depot.get::<genies::core::jwt::JWTToken>("jwtToken") {
        Ok(token) => token.preferred_username.clone().unwrap_or_else(|| "guest".into()),
        Err(_) => "guest".into(),
    };

    // 2. 获取 Enforcer（从 Depot 中的 EnforcerManager）
    //    EnforcerManager 通过 affix_state::inject 注入
    let mgr = match depot.obtain::<Arc<EnforcerManager>>() {
        Ok(m) => m,
        Err(_) => {
            log::error!("EnforcerManager 未注入到 Depot");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "code": "-1",
                "msg": "权限配置错误：EnforcerManager 未注入"
            })));
            ctrl.skip_rest();
            return;
        }
    };
    let enforcer = mgr.get_enforcer().await;

    // 3. 路径 + HTTP 方法检查
    let path = req.uri().path().to_string();
    let method = req.method().as_str().to_lowercase();

    // enforce(sub, obj, act) — 黑名单模式：默认允许，仅 deny 规则生效
    match enforcer.enforce((&subject, &path, &method)) {
        Ok(true) => {
            // 通过，继续执行
            log::debug!(
                "Casbin API 权限检查通过: subject={}, path={}, method={}",
                subject,
                path,
                method
            );
        }
        Ok(false) => {
            // 被策略拒绝
            log::warn!(
                "Casbin API 权限拒绝: subject={}, path={}, method={}",
                subject,
                path,
                method
            );
            res.status_code(StatusCode::FORBIDDEN);
            res.render(Json(serde_json::json!({
                "code": "-1",
                "msg": format!("Access denied: {} {} for user {}", method, path, subject)
            })));
            ctrl.skip_rest();
            return;
        }
        Err(e) => {
            // 策略引擎错误，拒绝访问并记录日志
            log::error!(
                "Casbin enforce error: {} (subject={}, path={}, method={})",
                e,
                subject,
                path,
                method
            );
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "code": "-1",
                "msg": "权限引擎错误，请稍后重试"
            })));
            ctrl.skip_rest();
            return;
        }
    }

    // 4. 注入 enforcer + subject 到 Depot（供 Writer 使用）
    depot.insert("casbin_enforcer", enforcer);
    depot.insert("casbin_subject", subject);

    ctrl.call_next(req, depot, res).await;
}

/// 按类型名过滤 JSON 对象的字段（Casbin 权限检查）
///
/// 遍历 JSON 对象的所有字段，对每个字段构造 `{type_name}.{field_name}` 格式的资源路径，
/// 通过 Casbin enforcer 检查 read 权限。如果权限被拒绝（enforce 返回 false），则从对象中移除该字段。
///
/// # 参数
/// - `value`: 待过滤的 JSON Value（应为 Object 类型，非 Object 则不处理）
/// - `type_name`: 类型名称，用于构造权限检查的资源路径（如 "Address"、"BankAccount"）
/// - `enforcer`: Casbin Enforcer 实例
/// - `subject`: 权限检查的主体（如用户名、角色）
///
/// # 示例
/// ```ignore
/// use genies_auth::casbin_filter_object;
///
/// let mut value = serde_json::json!({"city": "Beijing", "street": "Main St"});
/// casbin_filter_object(&mut value, "Address", &enforcer, "user1");
/// // 如果 user1 没有 Address.street 的 read 权限，street 字段将被移除
/// ```
pub fn casbin_filter_object(
    value: &mut serde_json::Value,
    type_name: &str,
    enforcer: &Enforcer,
    subject: &str,
) {
    if let serde_json::Value::Object(map) = value {
        let keys: Vec<String> = map.keys().cloned().collect();
        for key in keys {
            let resource = format!("{}.{}", type_name, key);
            match enforcer.enforce((subject, &resource, "read")) {
                Ok(false) => {
                    map.remove(&key);
                }
                _ => {} // Ok(true) 允许，Err 也保留（黑名单模式，默认允许）
            }
        }
    }
}
