---
name: salvo-cors
description: Configure Cross-Origin Resource Sharing (CORS) and security headers. Use for APIs accessed from browsers on different domains.
version: 0.89.3
tags: [security, cors, cross-origin, headers]
---

# Salvo CORS Configuration

This skill helps configure CORS and security headers in Salvo applications.

## Setup

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["cors"] }
```

## Basic CORS Configuration

```rust
use salvo::cors::Cors;
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    let cors = Cors::new()
        .allow_origin("https://example.com")
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allow_headers(vec!["Content-Type", "Authorization"])
        .into_handler();

    let router = Router::new()
        .hoop(cors)
        .push(Router::with_path("api").get(api_handler));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Allow All Origins (Development)

```rust
use salvo::cors::Cors;

// WARNING: Only use in development
let cors = Cors::new()
    .allow_origin("*")
    .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
    .allow_headers(vec!["*"])
    .into_handler();
```

## Production CORS Configuration

```rust
use salvo::cors::Cors;
use salvo::http::Method;

let cors = Cors::new()
    .allow_origin(["https://app.example.com", "https://admin.example.com"])
    .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers(vec!["Authorization", "Content-Type", "X-Requested-With"])
    .allow_credentials(true)
    .max_age(3600)
    .into_handler();
```

## Permissive CORS

```rust
use salvo::cors::Cors;

let cors = Cors::permissive();

let router = Router::new()
    .hoop(cors)
    .get(handler);
```

## CORS with Specific Routes

```rust
let cors = Cors::new()
    .allow_origin("https://app.example.com")
    .allow_methods(vec!["GET", "POST"])
    .into_handler();

let router = Router::new()
    .push(
        Router::with_path("api")
            .hoop(cors)
            .push(Router::with_path("users").get(list_users))
    )
    .push(
        Router::with_path("health")
            .get(health_check)
    );
```

## Security Headers Middleware

```rust
use salvo::prelude::*;

#[handler]
async fn security_headers(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    res.headers_mut().insert(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'".parse().unwrap()
    );

    res.headers_mut().insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains; preload".parse().unwrap()
    );

    res.headers_mut().insert(
        "X-Frame-Options",
        "DENY".parse().unwrap()
    );

    res.headers_mut().insert(
        "X-Content-Type-Options",
        "nosniff".parse().unwrap()
    );

    res.headers_mut().insert(
        "X-XSS-Protection",
        "1; mode=block".parse().unwrap()
    );

    res.headers_mut().insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin".parse().unwrap()
    );

    ctrl.call_next(req, depot, res).await;
}
```

## Complete Example with CORS and Security Headers

```rust
use salvo::cors::Cors;
use salvo::http::Method;
use salvo::prelude::*;

#[handler]
async fn security_headers(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    res.headers_mut().insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    res.headers_mut().insert("X-Frame-Options", "DENY".parse().unwrap());
    res.headers_mut().insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    ctrl.call_next(req, depot, res).await;
}

#[handler]
async fn api_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

#[tokio::main]
async fn main() {
    let cors = Cors::new()
        .allow_origin(["https://app.example.com"])
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(vec!["Authorization", "Content-Type"])
        .allow_credentials(true)
        .max_age(86400)
        .into_handler();

    let router = Router::new()
        .hoop(security_headers)
        .hoop(cors)
        .push(Router::with_path("api").get(api_handler));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Dynamic CORS Origins

```rust
use salvo::cors::Cors;
use salvo::prelude::*;

fn create_cors() -> Cors {
    Cors::new()
        .allow_origin(|origin: &str, _req: &Request| {
            origin.ends_with(".example.com") || origin == "https://example.com"
        })
        .allow_methods(vec!["GET", "POST"])
        .allow_credentials(true)
}
```

## Best Practices

1. Specify exact origins in production
2. Limit allowed methods
3. Validate headers
4. Use HTTPS
5. Set appropriate max_age
6. Apply security headers
7. Test preflight requests

## Related Skills

- **salvo-csrf**: CSRF protection
- **salvo-auth**: CORS for authenticated APIs
