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
    // 1. 从 URL 路径提取文件路径
    // Salvo 0.79+ 使用 {**path} 语法，参数名称为 "path"
    let path = req.param::<String>("path").unwrap_or_default();
    
    // 处理路径：去除前导斜杠，空路径或 "/" 默认为 index.html
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    // 2. 尝试获取静态资源
    match AdminUiAssets::get(path) {
        Some(content) => {
            serve_embedded_file(res, path, &content.data);
        }
        None => {
            // 3. SPA fallback：尝试返回 index.html
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
    // 推断 MIME 类型
    let mime_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    // 设置 Content-Type
    res.headers_mut()
        .insert(CONTENT_TYPE, mime_type.parse().unwrap());

    // 设置 Cache-Control
    // - 对 assets/ 下的文件（通常带 hash）设置长期缓存
    // - 对 index.html 不缓存，确保每次获取最新版本
    let cache_control = if path == "index.html" {
        "no-cache, no-store, must-revalidate"
    } else if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=3600"
    };
    res.headers_mut()
        .insert(CACHE_CONTROL, cache_control.parse().unwrap());

    // 写入响应体
    res.body(ResBody::Once(data.to_vec().into()));
}

/// 创建 Admin UI 路由
///
/// # 用法
/// ```ignore
/// let router = Router::new()
///     .push(auth_admin_ui_router());
/// ```
///
/// # 访问路径
/// - `/auth/ui/` - 主页面
/// - `/auth/ui/assets/*` - 静态资源
/// - `/auth/ui/*` - SPA 子路由（fallback 到 index.html）
pub fn auth_admin_ui_router() -> Router {
    Router::with_path("auth/ui")
        .get(serve_admin_ui_entry)
        .push(Router::with_path("{**path}").get(serve_admin_ui))
}

/// 处理 /auth/ui 和 /auth/ui/ 的入口
/// - 无尾斜杠时重定向到带尾斜杠的路径（确保相对路径资源正确解析）
/// - 有尾斜杠时直接返回 index.html
#[handler]
async fn serve_admin_ui_entry(req: &mut Request, res: &mut Response) {
    let path = req.uri().path().to_string();
    if !path.ends_with('/') {
        res.render(Redirect::other(format!("{}/", path)));
    } else {
        // 直接返回 index.html
        if let Some(index) = AdminUiAssets::get("index.html") {
            serve_embedded_file(res, "index.html", &index.data);
        } else {
            res.status_code(StatusCode::NOT_FOUND);
            res.render(Text::Plain("404 Not Found"));
        }
    }
}
