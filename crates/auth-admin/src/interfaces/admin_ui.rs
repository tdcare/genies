//! Admin UI 静态资源服务模块
//!
//! 使用 rust-embed 将前端构建产物嵌入到二进制中，
//! 并通过 Salvo 路由提供静态资源服务。
//!
//! # 特性
//! - 自动 MIME 类型推断
//! - SPA fallback 支持（未匹配路径返回 index.html）
//! - 智能缓存控制（assets 长期缓存，index.html 不缓存）

use rust_embed::Embed;
use salvo::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use salvo::http::ResBody;
use salvo::prelude::*;

/// 嵌入 static/ 目录下的所有前端资源
#[derive(Embed)]
#[folder = "static/"]
struct AdminUiAssets;

/// 静态资源服务 Handler
#[handler]
async fn serve_admin_ui(req: &mut Request, res: &mut Response) {
    let path = req.param::<String>("path").unwrap_or_default();

    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match AdminUiAssets::get(path) {
        Some(content) => {
            serve_embedded_file(res, path, &content.data);
        }
        None => {
            // SPA fallback：尝试返回 index.html
            if let Some(index) = AdminUiAssets::get("index.html") {
                serve_embedded_file(res, "index.html", &index.data);
            } else {
                res.status_code(StatusCode::NOT_FOUND);
                res.render(Text::Plain("404 Not Found"));
            }
        }
    }
}

/// 将嵌入的文件内容写入响应
fn serve_embedded_file(res: &mut Response, path: &str, data: &[u8]) {
    let mime_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    res.headers_mut()
        .insert(CONTENT_TYPE, mime_type.parse().unwrap());

    let cache_control = if path == "index.html" {
        "no-cache, no-store, must-revalidate"
    } else if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=3600"
    };
    res.headers_mut()
        .insert(CACHE_CONTROL, cache_control.parse().unwrap());

    res.body(ResBody::Once(data.to_vec().into()));
}

/// 创建 Admin UI 路由
///
/// # 访问路径
/// - `/auth-admin/ui/` - 主页面
/// - `/auth-admin/ui/assets/*` - 静态资源
/// - `/auth-admin/ui/*` - SPA 子路由（fallback 到 index.html）
pub fn auth_admin_ui_router() -> Router {
    Router::with_path("auth-admin/ui")
        .get(serve_admin_ui_entry)
        .push(Router::with_path("{**path}").get(serve_admin_ui))
}

/// 处理 /auth-admin/ui 和 /auth-admin/ui/ 的入口
#[handler]
async fn serve_admin_ui_entry(req: &mut Request, res: &mut Response) {
    let path = req.uri().path().to_string();
    if !path.ends_with('/') {
        res.render(Redirect::other(format!("{}/", path)));
    } else {
        if let Some(index) = AdminUiAssets::get("index.html") {
            serve_embedded_file(res, "index.html", &index.data);
        } else {
            res.status_code(StatusCode::NOT_FOUND);
            res.render(Text::Plain("404 Not Found"));
        }
    }
}
