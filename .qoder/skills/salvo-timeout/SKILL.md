---
name: salvo-timeout
description: Configure request timeouts to prevent slow requests from blocking resources. Use for protecting APIs from long-running operations.
version: 0.89.3
tags: [performance, timeout, request-timeout]
---

# Salvo Request Timeout

This skill helps configure request timeouts in Salvo applications.

## Setup

Timeout is built into Salvo core:

```toml
[dependencies]
salvo = "0.89.3"
```

## Basic Timeout

```rust
use std::time::Duration;
use salvo::prelude::*;

#[handler]
async fn fast_handler() -> &'static str {
    "Hello, World!"
}

#[handler]
async fn slow_handler() -> &'static str {
    tokio::time::sleep(Duration::from_secs(10)).await;
    "This takes a while..."
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .hoop(Timeout::new(Duration::from_secs(5)))
        .push(Router::with_path("fast").get(fast_handler))
        .push(Router::with_path("slow").get(slow_handler));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Route-Specific Timeouts

```rust
use std::time::Duration;
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(
            Router::with_path("quick")
                .hoop(Timeout::new(Duration::from_secs(2)))
                .get(quick_handler)
        )
        .push(
            Router::with_path("standard")
                .hoop(Timeout::new(Duration::from_secs(30)))
                .get(standard_handler)
        )
        .push(
            Router::with_path("long-running")
                .hoop(Timeout::new(Duration::from_secs(300)))
                .get(long_running_handler)
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Timeout with API Routes

```rust
use std::time::Duration;
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    let api_timeout = Timeout::new(Duration::from_secs(10));
    let upload_timeout = Timeout::new(Duration::from_secs(120));

    let router = Router::new()
        .push(
            Router::with_path("api")
                .hoop(api_timeout)
                .push(Router::with_path("users").get(list_users))
                .push(Router::with_path("products").get(list_products))
        )
        .push(
            Router::with_path("upload")
                .hoop(upload_timeout)
                .post(handle_upload)
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Custom Error Response

```rust
use std::time::Duration;
use salvo::prelude::*;
use salvo::catcher::Catcher;

#[handler]
async fn handle_timeout(res: &mut Response, ctrl: &mut FlowCtrl) {
    if res.status_code() == Some(StatusCode::REQUEST_TIMEOUT) {
        res.render(Json(serde_json::json!({
            "error": "Request Timeout",
            "message": "The request took too long to process",
            "code": 408
        })));
        ctrl.skip_rest();
    }
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .hoop(Timeout::new(Duration::from_secs(5)))
        .get(slow_handler);

    let service = Service::new(router).catcher(
        Catcher::default().hoop(handle_timeout)
    );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(service).await;
}
```

## Practical Timeout Values

| Endpoint Type | Recommended Timeout |
|---------------|---------------------|
| Health checks | 1-2 seconds |
| Simple API calls | 5-10 seconds |
| Database queries | 10-30 seconds |
| File uploads | 60-300 seconds |
| Report generation | 120-600 seconds |
| Real-time endpoints | Consider no timeout |

## Best Practices

1. Set appropriate timeouts based on expected duration
2. Use shorter timeouts for public APIs
3. Longer timeouts for internal services
4. Log timeouts for debugging
5. Consider client expectations
6. Combine with rate limiting
7. Graceful degradation on timeout
8. No timeout for WebSocket connections

## Related Skills

- **salvo-concurrency-limiter**: Limit concurrent requests
- **salvo-rate-limiter**: Rate limit alongside timeouts
- **salvo-graceful-shutdown**: Handle timeouts during shutdown
