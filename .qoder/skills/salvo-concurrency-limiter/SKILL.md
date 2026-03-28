---
name: salvo-concurrency-limiter
description: Limit concurrent requests to protect resources. Use for file uploads, expensive operations, and preventing resource exhaustion.
version: 0.89.3
tags: [performance, concurrency, limiter]
---

# Salvo Concurrency Limiter

This skill helps limit concurrent requests in Salvo applications.

## Setup

Concurrency limiter is built into Salvo core:

```toml
[dependencies]
salvo = "0.89.3"
```

## Basic Concurrency Limit

```rust
use salvo::prelude::*;

#[handler]
async fn upload(req: &mut Request, res: &mut Response) {
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    res.render("Upload complete");
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(
            Router::with_path("upload")
                .hoop(max_concurrency(1))
                .post(upload)
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Different Limits for Different Routes

```rust
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(
            Router::with_path("upload")
                .hoop(max_concurrency(2))
                .post(upload_handler)
        )
        .push(
            Router::with_path("reports/generate")
                .hoop(max_concurrency(1))
                .post(generate_report)
        )
        .push(
            Router::with_path("api/{**rest}")
                .hoop(max_concurrency(100))
                .goal(api_handler)
        )
        .push(Router::with_path("health").get(health_check));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Combining with Rate Limiting

```rust
use salvo::prelude::*;
use salvo::rate_limiter::{BasicQuota, FixedGuard, MokaStore, RateLimiter, RemoteIpIssuer};

#[tokio::main]
async fn main() {
    let rate_limiter = RateLimiter::new(
        FixedGuard::new(),
        MokaStore::new(),
        RemoteIpIssuer,
        BasicQuota::per_second(10),
    );

    let router = Router::new()
        .push(
            Router::with_path("api/{**rest}")
                .hoop(rate_limiter)
                .hoop(max_concurrency(50))
                .goal(api_handler)
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Combining with Timeout

```rust
use std::time::Duration;
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(
            Router::with_path("process")
                .hoop(Timeout::new(Duration::from_secs(30)))
                .hoop(max_concurrency(5))
                .post(process_handler)
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Use Cases

### CPU-Intensive Operations

```rust
let router = Router::new()
    .push(
        Router::with_path("resize")
            .hoop(max_concurrency(num_cpus::get()))
            .post(resize_image)
    );
```

### Database Connection Protection

```rust
let db_pool_size = 10;

let router = Router::new()
    .push(
        Router::with_path("heavy-query")
            .hoop(max_concurrency(db_pool_size))
            .get(heavy_query_handler)
    );
```

### External API Limits

```rust
let router = Router::new()
    .push(
        Router::with_path("external")
            .hoop(max_concurrency(5))
            .get(call_external_api)
    );
```

## Recommended Concurrency Limits

| Operation Type | Recommended Limit |
|----------------|-------------------|
| File uploads | 1-5 |
| Image processing | CPU cores |
| Report generation | 1-2 |
| Database heavy queries | DB pool size |
| External API calls | API limit |
| General API endpoints | 50-200 |

## Best Practices

1. Set based on resource constraints
2. Consider downstream limits
3. Combine with timeout
4. Monitor active requests
5. Return meaningful errors on limit reached
6. Use different limits for different endpoints

## Related Skills

- **salvo-rate-limiter**: Rate limit requests
- **salvo-timeout**: Set timeouts for requests
