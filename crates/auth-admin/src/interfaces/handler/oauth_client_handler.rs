//! OAuth 客户端管理 Handler — CRUD

use salvo::prelude::*;
use salvo::oapi::extract::*;

use genies::core::RespVO;

use crate::application::oauth_dto::*;
use crate::application::oauth_client_service::OAuthClientAppService;

/// 客户端管理路由（受保护，需 admin 登录）
pub fn routes() -> Router {
    let id_router = Router::with_path("{id}")
        .get(get_client)
        .put(update_client)
        .delete(delete_client);

    Router::with_path("/oauth/clients")
        .get(list_clients)
        .post(create_client)
        .push(id_router)
        .push(
            Router::with_path("{id}/regenerate-secret")
                .post(regenerate_secret)
        )
}

/// GET /oauth/clients — 分页列表
#[endpoint(tags("oauth-clients"), summary = "OAuth 客户端列表")]
pub async fn list_clients(
    page: QueryParam<u64, false>,
    size: QueryParam<u64, false>,
    keyword: QueryParam<String, false>,
) -> Json<RespVO<serde_json::Value>> {
    let page = page.into_inner().unwrap_or(1);
    let size = size.into_inner().unwrap_or(10);
    let keyword = keyword.into_inner().unwrap_or_default();

    match OAuthClientAppService::list_clients(page, size, &keyword).await {
        Ok((list, total)) => Json(RespVO::from(&serde_json::json!({
            "total": total,
            "page": page,
            "size": size,
            "list": list,
        }))),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// GET /oauth/clients/{id} — 详情
#[endpoint(tags("oauth-clients"), summary = "OAuth 客户端详情")]
pub async fn get_client(
    id: PathParam<i64>,
) -> Json<RespVO<OAuthClientVO>> {
    let id = id.into_inner();
    match OAuthClientAppService::get_client(id).await {
        Ok(vo) => Json(RespVO::from(&vo)),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// POST /oauth/clients — 创建客户端
#[endpoint(tags("oauth-clients"), summary = "创建 OAuth 客户端")]
pub async fn create_client(
    body: JsonBody<CreateOAuthClientRequest>,
) -> Json<RespVO<OAuthClientCreateResponse>> {
    let req = body.into_inner();
    match OAuthClientAppService::create_client(&req).await {
        Ok(resp) => Json(RespVO::from(&resp)),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// PUT /oauth/clients/{id} — 更新客户端
#[endpoint(tags("oauth-clients"), summary = "更新 OAuth 客户端")]
pub async fn update_client(
    id: PathParam<i64>,
    body: JsonBody<UpdateOAuthClientRequest>,
) -> Json<RespVO<serde_json::Value>> {
    let id = id.into_inner();
    let req = body.into_inner();
    match OAuthClientAppService::update_client(id, &req).await {
        Ok(()) => Json(RespVO::from(&serde_json::json!({"msg": "更新成功"}))),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// DELETE /oauth/clients/{id} — 删除客户端
#[endpoint(tags("oauth-clients"), summary = "删除 OAuth 客户端")]
pub async fn delete_client(
    id: PathParam<i64>,
) -> Json<RespVO<serde_json::Value>> {
    let id = id.into_inner();
    match OAuthClientAppService::delete_client(id).await {
        Ok(()) => Json(RespVO::from(&serde_json::json!({"msg": "删除成功"}))),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}

/// POST /oauth/clients/{id}/regenerate-secret — 重新生成密钥
#[endpoint(tags("oauth-clients"), summary = "重新生成 OAuth 客户端密钥")]
pub async fn regenerate_secret(
    id: PathParam<i64>,
) -> Json<RespVO<serde_json::Value>> {
    let id = id.into_inner();
    match OAuthClientAppService::regenerate_secret(id).await {
        Ok(secret) => Json(RespVO::from(&serde_json::json!({
            "client_secret": secret,
            "msg": "密钥已重新生成，请立即保存"
        }))),
        Err(msg) => Json(RespVO::from_error_info("-1", &msg)),
    }
}
