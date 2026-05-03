//! 内部接口认证中间件
//!
//! 仅验证 JWT 签名有效性，不做 Casbin 权限检查。
//! 用于服务间内部调用的接口保护。

use genies::context::CONTEXT;
use genies_core::jwt::JWTToken;
use genies_core::RespVO;
use salvo::http::StatusCode;
use salvo::prelude::*;

/// 内部接口认证中间件：验证 Bearer Token 的 JWT 签名
#[handler]
pub async fn internal_auth_handler(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    let token_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let token_value = token_header.strip_prefix("Bearer ").unwrap_or("");

    if token_value.is_empty() {
        let resp: RespVO<String> = RespVO {
            code: Some("-1".to_string()),
            msg: Some("Unauthorized: missing Bearer token".to_string()),
            data: None,
        };
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render(Json(resp));
        ctrl.skip_rest();
        return;
    }

    match JWTToken::verify_local(&CONTEXT.config.jwt_secret, token_value) {
        Ok(_) => {
            // 签名验证通过，放行
            ctrl.call_next(req, depot, res).await;
        }
        Err(e) => {
            log::warn!("[internal_auth] JWT 验证失败: {}", e);
            let resp: RespVO<String> = RespVO {
                code: Some("-1".to_string()),
                msg: Some(format!("Unauthorized: {}", e)),
                data: None,
            };
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(resp));
            ctrl.skip_rest();
        }
    }
}
