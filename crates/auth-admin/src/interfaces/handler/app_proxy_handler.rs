//! 应用权限代理 Handler
//!
//! 根据 app_id 查出目标微服务 base_url，
//! 使用 reqwest::Client 将请求转发到目标服务的 /auth/* 端点。
//! 代理直接透传目标微服务的原始 HTTP 响应（状态码 + body），不做 RespVO 包装。

use std::sync::OnceLock;

use salvo::prelude::*;
use salvo::oapi::extract::*;
use salvo::writing::Text;

use crate::application::app_service::ApplicationAppService;
use crate::application::service::SyncAppService;

/// 共享的 reqwest::Client，避免每次代理请求都创建新的 HttpClient
fn shared_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(20)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to build shared reqwest client")
    })
}

/// 返回代理子路由列表，需由调用方挂载到 `{id}` 节点下
pub fn proxy_sub_routes() -> Vec<Router> {
    vec![
        Router::with_path("schemas").get(proxy_list_schemas),
        Router::with_path("policies")
            .get(proxy_list_policies)
            .post(proxy_add_policy),
        Router::with_path("policies/{policy_id}").delete(proxy_remove_policy),
        Router::with_path("roles")
            .get(proxy_list_roles)
            .post(proxy_add_role),
        Router::with_path("roles/{role_id}").delete(proxy_remove_role),
        Router::with_path("groups")
            .get(proxy_list_groups)
            .post(proxy_add_group),
        Router::with_path("groups/{group_id}").delete(proxy_remove_group),
        Router::with_path("reload").post(proxy_reload),
        Router::with_path("sync-user-roles").post(proxy_sync_user_roles),
    ]
}

/// 内部辅助：根据 app_id 获取 base_url
async fn resolve_base_url(app_id: i64) -> Result<String, String> {
    let app = ApplicationAppService::get_app(app_id).await?;
    let status = app.status.unwrap_or(0);
    if status != 1 {
        return Err("应用已禁用".to_string());
    }
    app.base_url.ok_or_else(|| "应用未配置访问地址".to_string())
}

/// 内部辅助：从请求中提取 Authorization 头
fn extract_auth_header(req: &Request) -> Option<String> {
    req.headers().get("Authorization").and_then(|v| v.to_str().ok()).map(|s| s.to_string())
}

/// 内部辅助：将目标服务的原始响应直接透传到 Salvo Response
async fn forward_response(resp: reqwest::Response, res: &mut Response) {
    let status = resp.status();
    let salvo_status = StatusCode::from_u16(status.as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    res.status_code(salvo_status);
    match resp.text().await {
        Ok(body) => {
            res.render(Text::Json(body));
        }
        Err(e) => {
            res.status_code(StatusCode::BAD_GATEWAY);
            let err = serde_json::json!({"code": "-1", "msg": format!("读取目标响应失败: {}", e)});
            res.render(Text::Json(err.to_string()));
        }
    }
}

/// 内部辅助：向 Salvo Response 写入 JSON 错误
fn write_error(res: &mut Response, msg: &str) {
    res.status_code(StatusCode::BAD_GATEWAY);
    let err = serde_json::json!({"code": "-1", "msg": msg});
    res.render(Text::Json(err.to_string()));
}

/// GET /apps/{id}/schemas — 代理查询 API Schema
#[endpoint(tags("app-proxy"), summary = "代理查询应用 Schema")]
pub async fn proxy_list_schemas(
    id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    let query_string = req.uri().query().unwrap_or("");
    let url = if query_string.is_empty() {
        format!("{}/auth/schemas", base_url.trim_end_matches('/'))
    } else {
        format!("{}/auth/schemas?{}", base_url.trim_end_matches('/'), query_string)
    };

    let mut builder = shared_client().get(&url);
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}

/// GET /apps/{id}/policies — 代理查询策略
#[endpoint(tags("app-proxy"), summary = "代理查询应用策略")]
pub async fn proxy_list_policies(
    id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    let query_string = req.uri().query().unwrap_or("");
    let url = if query_string.is_empty() {
        format!("{}/auth/policies", base_url.trim_end_matches('/'))
    } else {
        format!("{}/auth/policies?{}", base_url.trim_end_matches('/'), query_string)
    };

    let mut builder = shared_client().get(&url);
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}

/// POST /apps/{id}/policies — 代理添加策略
#[endpoint(tags("app-proxy"), summary = "代理添加应用策略")]
pub async fn proxy_add_policy(
    id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
    body: JsonBody<serde_json::Value>,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    let url = format!("{}/auth/policies", base_url.trim_end_matches('/'));

    let mut builder = shared_client().post(&url).json(&body.into_inner());
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}

/// DELETE /apps/{id}/policies/{policy_id} — 代理删除策略
#[endpoint(tags("app-proxy"), summary = "代理删除应用策略")]
pub async fn proxy_remove_policy(
    id: PathParam<i64>,
    policy_id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    let url = format!("{}/auth/policies/{}", base_url.trim_end_matches('/'), policy_id.into_inner());

    let mut builder = shared_client().delete(&url);
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}

/// GET /apps/{id}/roles — 代理查询角色
#[endpoint(tags("app-proxy"), summary = "代理查询应用角色分配")]
pub async fn proxy_list_roles(
    id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    let query_string = req.uri().query().unwrap_or("");
    let url = if query_string.is_empty() {
        format!("{}/auth/roles", base_url.trim_end_matches('/'))
    } else {
        format!("{}/auth/roles?{}", base_url.trim_end_matches('/'), query_string)
    };

    let mut builder = shared_client().get(&url);
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}

/// POST /apps/{id}/roles — 代理添加角色
#[endpoint(tags("app-proxy"), summary = "代理添加应用角色分配")]
pub async fn proxy_add_role(
    id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
    body: JsonBody<serde_json::Value>,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    let url = format!("{}/auth/roles", base_url.trim_end_matches('/'));

    let mut builder = shared_client().post(&url).json(&body.into_inner());
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}

/// GET /apps/{id}/groups — 代理查询分组
#[endpoint(tags("app-proxy"), summary = "代理查询应用对象分组")]
pub async fn proxy_list_groups(
    id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    let query_string = req.uri().query().unwrap_or("");
    let url = if query_string.is_empty() {
        format!("{}/auth/groups", base_url.trim_end_matches('/'))
    } else {
        format!("{}/auth/groups?{}", base_url.trim_end_matches('/'), query_string)
    };

    let mut builder = shared_client().get(&url);
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}

/// POST /apps/{id}/groups — 代理添加分组
#[endpoint(tags("app-proxy"), summary = "代理添加应用对象分组")]
pub async fn proxy_add_group(
    id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
    body: JsonBody<serde_json::Value>,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    let url = format!("{}/auth/groups", base_url.trim_end_matches('/'));

    let mut builder = shared_client().post(&url).json(&body.into_inner());
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}

/// DELETE /apps/{id}/roles/{role_id} — 代理删除角色
#[endpoint(tags("app-proxy"), summary = "代理删除应用角色分配")]
pub async fn proxy_remove_role(
    id: PathParam<i64>,
    role_id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    let url = format!("{}/auth/roles/{}", base_url.trim_end_matches('/'), role_id.into_inner());

    let mut builder = shared_client().delete(&url);
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}

/// DELETE /apps/{id}/groups/{group_id} — 代理删除分组
#[endpoint(tags("app-proxy"), summary = "代理删除应用对象分组")]
pub async fn proxy_remove_group(
    id: PathParam<i64>,
    group_id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    let url = format!("{}/auth/groups/{}", base_url.trim_end_matches('/'), group_id.into_inner());

    let mut builder = shared_client().delete(&url);
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}

/// POST /apps/{id}/sync-user-roles — 将用户-角色映射推送到目标微服务
#[endpoint(tags("app-proxy"), summary = "推送用户-角色映射到目标应用")]
pub async fn proxy_sync_user_roles(
    id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    // 查询所有活跃的用户-角色映射
    let mappings = match SyncAppService::list_active_user_roles().await {
        Ok(data) => data,
        Err(msg) => return write_error(res, &format!("查询用户-角色映射失败: {}", msg)),
    };

    let url = format!("{}/auth/sync/receive-user-roles", base_url.trim_end_matches('/'));

    let mut builder = shared_client().post(&url).json(&mappings);
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}

/// POST /apps/{id}/reload — 代理重载 Enforcer
#[endpoint(tags("app-proxy"), summary = "代理重载应用 Enforcer")]
pub async fn proxy_reload(
    id: PathParam<i64>,
    req: &mut Request,
    res: &mut Response,
) {
    let auth_header = extract_auth_header(req);
    let base_url = match resolve_base_url(id.into_inner()).await {
        Ok(u) => u,
        Err(msg) => return write_error(res, &msg),
    };

    let url = format!("{}/auth/reload", base_url.trim_end_matches('/'));

    let mut builder = shared_client().post(&url);
    if let Some(auth) = &auth_header {
        builder = builder.header("Authorization", auth.as_str());
    }
    match builder.send().await {
        Ok(resp) => forward_response(resp, res).await,
        Err(e) => write_error(res, &format!("目标服务不可达: {}", e)),
    }
}
