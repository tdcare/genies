
use genies_core::jwt::*;
use genies_core::error::*;
use genies_core::RespVO;
use salvo::prelude::*;
use salvo::http::StatusCode;
use crate::app_context::ApplicationContext;
use crate::CONTEXT;

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

    match &context.config.keycloak_auth_server_url.is_empty() {
        false => {
            //   return  JWTToken::verify(&CONTEXT.config.jwt_secret, token_value);
            //  let n=&CONTEXT.keycloak_keys.keys[0].n.as_ref().clone().unwrap();
            //  let e=&CONTEXT.keycloak_keys.keys[0].e.as_ref().clone().unwrap();
            let keycloak = &context.keycloak_keys;
            return JWTToken::verify_with_keycloak(keycloak, token_value);
        }
        _ => {
            // return JWTToken::verify(&context.config.jwt_secret, token_value);
            return Err(genies_core::error::Error::from("jwt_key error".to_string()));
        }
    };

    // match jwt_token {
    //     Ok(token) => {
    //         return Ok(jwt_token);
    //     }
    //     Err(e) => {
    //         return Err(crate::error::Error::from(e.to_string()));
    //     }
    // }
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
pub async fn salvo_auth(req: &mut Request,_depot: &mut Depot,res: &mut Response, _ctrl: &mut FlowCtrl) {
    let token = req.headers().get("Authorization").map(|v|v.to_str().unwrap_or_default().to_string()).unwrap_or_default();
    let path =req.uri().path().to_string();

    if !is_white_list_api(&CONTEXT,&path) {
        //非白名单检查token是否有效
        match checked_token(&CONTEXT, &token, &path).await {
            Ok(data) => {
                match check_auth(&CONTEXT, &data, &path).await {
                    Ok(_) => {
                        _depot.insert("jwtToken",data.clone());
                        _depot.insert("token",token);
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
    }
}