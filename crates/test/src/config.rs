//! 配置函数 — HTTP Client 工厂。

use reqwest::Client;

/// 创建禁用代理的 HTTP client（支持通过 TOKEN 环境变量注入 Bearer 认证）
pub fn http_client() -> Client {
    let mut builder = Client::builder().no_proxy();
    if let Ok(token) = std::env::var("TOKEN") {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        );
        builder = builder.default_headers(headers);
    }
    builder.build().unwrap()
}
