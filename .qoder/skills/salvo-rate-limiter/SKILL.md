---
name: salvo-rate-limiter
description: Implement rate limiting to protect APIs from abuse. Use for preventing DDoS attacks and ensuring fair resource usage.
version: 0.89.3
tags: [security, rate-limiting, throttling]
---

# Salvo Rate Limiting

This skill helps implement rate limiting in Salvo applications.

## Setup

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["rate-limiter"] }
```

## Basic Rate Limiting

```rust
use salvo::prelude::*;
use salvo::rate_limiter::{BasicQuota, FixedGuard, MokaStore, RateLimiter, RemoteIpIssuer};

#[handler]
async fn api_handler() -> &'static str {
    "API response"
}

#[tokio::main]
async fn main() {
    let limiter = RateLimiter::new(
        FixedGuard::new(),
        MokaStore::new(),
        RemoteIpIssuer,
        BasicQuota::per_second(10),
    );

    let router = Router::new()
        .hoop(limiter)
        .push(Router::with_path("api").get(api_handler));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Quota Types

```rust
use salvo::rate_limiter::BasicQuota;
use std::time::Duration;

BasicQuota::per_second(10)
BasicQuota::per_minute(100)
BasicQuota::per_hour(1000)
BasicQuota::new(50, Duration::from_secs(30))
```

## Rate Limit by IP Address

```rust
use salvo::rate_limiter::{BasicQuota, FixedGuard, MokaStore, RateLimiter, RemoteIpIssuer};

let limiter = RateLimiter::new(
    FixedGuard::new(),
    MokaStore::new(),
    RemoteIpIssuer,
    BasicQuota::per_minute(60),
);
```

## Rate Limit by User ID

```rust
use salvo::prelude::*;
use salvo::rate_limiter::{BasicQuota, FixedGuard, MokaStore, RateLimiter, RateIssuer};

struct UserIdIssuer;

impl RateIssuer for UserIdIssuer {
    type Key = String;

    async fn issue(&self, req: &mut Request, depot: &Depot) -> Option<Self::Key> {
        depot.get::<String>("user_id").cloned()
    }
}

let limiter = RateLimiter::new(
    FixedGuard::new(),
    MokaStore::new(),
    UserIdIssuer,
    BasicQuota::per_minute(100),
);
```

## Rate Limit by API Key

```rust
struct ApiKeyIssuer;

impl RateIssuer for ApiKeyIssuer {
    type Key = String;

    async fn issue(&self, req: &mut Request, _depot: &Depot) -> Option<Self::Key> {
        req.header::<String>("X-API-Key")
    }
}

let limiter = RateLimiter::new(
    FixedGuard::new(),
    MokaStore::new(),
    ApiKeyIssuer,
    BasicQuota::per_minute(1000),
);
```

## Sliding Window Rate Limiting

```rust
use salvo::rate_limiter::{BasicQuota, SlidingGuard, MokaStore, RateLimiter, RemoteIpIssuer};

let limiter = RateLimiter::new(
    SlidingGuard::new(),
    MokaStore::new(),
    RemoteIpIssuer,
    BasicQuota::per_minute(60),
);
```

## Rate Limiting Specific Routes

```rust
#[tokio::main]
async fn main() {
    let login_limiter = RateLimiter::new(
        FixedGuard::new(),
        MokaStore::new(),
        RemoteIpIssuer,
        BasicQuota::per_minute(5),
    );

    let api_limiter = RateLimiter::new(
        FixedGuard::new(),
        MokaStore::new(),
        RemoteIpIssuer,
        BasicQuota::per_minute(100),
    );

    let router = Router::new()
        .push(
            Router::with_path("login")
                .hoop(login_limiter)
                .post(login_handler)
        )
        .push(
            Router::with_path("api")
                .hoop(api_limiter)
                .get(api_handler)
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Complete Example

```rust
use salvo::prelude::*;
use salvo::rate_limiter::{BasicQuota, FixedGuard, MokaStore, RateLimiter, RemoteIpIssuer};

#[handler]
async fn public_api() -> &'static str {
    "Public API response"
}

#[handler]
async fn login() -> &'static str {
    "Login successful"
}

#[tokio::main]
async fn main() {
    let api_limiter = RateLimiter::new(
        FixedGuard::new(),
        MokaStore::new(),
        RemoteIpIssuer,
        BasicQuota::per_minute(100),
    );

    let login_limiter = RateLimiter::new(
        FixedGuard::new(),
        MokaStore::new(),
        RemoteIpIssuer,
        BasicQuota::per_minute(5),
    );

    let router = Router::new()
        .push(
            Router::with_path("api")
                .hoop(api_limiter)
                .get(public_api)
        )
        .push(
            Router::with_path("login")
                .hoop(login_limiter)
                .post(login)
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Best Practices

1. Choose appropriate limits based on usage patterns
2. Use different limits for different endpoints
3. Identify users correctly (IP for anonymous, user ID for authenticated)
4. Return rate limit headers
5. Log rate limit hits
6. Consider sliding window for smoother limiting
7. Handle gracefully with helpful error messages

## Related Skills

- **salvo-concurrency-limiter**: Limit concurrent requests
- **salvo-auth**: Combine rate limiting with authentication
- **salvo-timeout**: Set timeouts alongside rate limits
