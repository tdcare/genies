
use genies_core::jwt::*;
use genies_core::error::*;
use genies_core::RespVO;
use salvo::prelude::*;
use salvo::http::StatusCode;
use crate::app_context::ApplicationContext;
use crate::CONTEXT;
use crate::request_token::REQUEST_TOKEN;

///是否处在白名单接口中
pub fn is_white_list_api(
    context: &ApplicationContext,
    path: &str) -> bool {
    if path.eq("/") {
        return true;
    }
    for x in &context.config.white_list_api {
        if x.contains(path) {
            return true;
        }
        if x.ends_with("*") {
            let white_apis: Vec<&str> = x.split("*").collect();
            let starts_with = white_apis.get(0).unwrap();
            if path.contains(starts_with) {
                return true;
            }
        }
    }
    return false;
}

///校验token是否有效，未过期
pub async fn checked_token(
    context: &ApplicationContext,
    token: &str,
    _path: &str,
) -> Result<JWTToken, Error> {
    //check token alive
    let token_value = token
        .strip_prefix("Bearer ")
        .unwrap_or("");

    match context.config.auth_mode.as_str() {
        "local" => {
            // 使用本地 HMAC-SHA256 验证（auth-admin 签发的 token）
            JWTToken::verify_local(&context.config.jwt_secret, token_value)
        }
        _ => {
            // 默认 Keycloak RSA 验证
            let keycloak = &context.keycloak_keys;
            JWTToken::verify_with_keycloak(keycloak, token_value)
        }
    }
}

///权限校验
pub async fn check_auth(
    _context: &ApplicationContext,
    _token: &JWTToken, _path: &str) -> Result<(), Error> {
    // let sys_res = CONTEXT.sys_res_service.finds_all().await?;
    //权限校验
    // for token_permission in &token.permissions {
    //     for x in &sys_res {
    //         match &x.permission {
    //             Some(permission) => match &x.path {
    //                 None => {}
    //                 Some(x_path) => {
    //                     if permission.eq(token_permission) && path.contains(x_path) {
    //                         return Ok(());
    //                     }
    //                 }
    //             },
    //             _ => {}
    //         }
    //     }
    // }
    // return Err(crate::error::Error::from("无权限访问!"));

    return Ok(());
}
/// salvo jwt check
#[handler]
pub async fn salvo_auth(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    let token = req.headers().get("Authorization").map(|v|v.to_str().unwrap_or_default().to_string()).unwrap_or_default();
    let path =req.uri().path().to_string();

    if !is_white_list_api(&CONTEXT,&path) {
        //非白名单检查token是否有效
        match checked_token(&CONTEXT, &token, &path).await {
            Ok(data) => {
                match check_auth(&CONTEXT, &data, &path).await {
                    Ok(_) => {
                        depot.insert("jwtToken",data.clone());
                        depot.insert("token",token.clone());
                        // 注入 subject 供 Casbin 使用
                        if let Some(ref username) = data.preferred_username {
                            depot.insert("subject", username.clone());
                        }
                        // 将下游调用链包裹在 REQUEST_TOKEN scope 中，
                        // 使得 #[remote] 宏和手写远程调用能自动获取当前请求的用户 token
                        REQUEST_TOKEN.scope(token, ctrl.call_next(req, depot, res)).await;
                        return;
                    }
                    Err(e) => {
                        //仅提示拦截
                        let resp: RespVO<String> = RespVO {
                            code: Some("-1".to_string()),
                            msg: Some(format!("无权限访问:{}", e.to_string())),
                            data: None,
                        };
                        // return Err(ErrorUnauthorized(serde_json::json!(&resp).to_string()));
                        res.status_code(StatusCode::FORBIDDEN);
                        res.render(Json(resp));

                    }
                }
            }
            Err(e) => {
                //401 http状态码会强制前端退出当前登陆状态
                let resp: RespVO<String> = RespVO {
                    code: Some("-1".to_string()),
                    msg: Some(format!("Unauthorized for:{}", e.to_string())),
                    data: None,
                };
                // return Err(ErrorUnauthorized(serde_json::json!(&resp).to_string()));
                res.status_code(StatusCode::UNAUTHORIZED);
                res.render(Json(resp));
            }
        }
    } else {
        // 白名单接口也尝试传递 token（如果有的话），方便下游远程调用
        if !token.is_empty() {
            REQUEST_TOKEN.scope(token, ctrl.call_next(req, depot, res)).await;
            return;
        }
        ctrl.call_next(req, depot, res).await;
    }
}