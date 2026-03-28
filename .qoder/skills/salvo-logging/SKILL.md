---
name: salvo-logging
description: Implement request logging, tracing, and observability. Use for debugging, monitoring, and production observability.
version: 0.89.3
tags: [operations, logging, tracing, observability]
---

# Salvo Logging and Tracing

This skill helps implement logging and tracing in Salvo applications.

## Setup

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["logging"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## Basic Request Logging

```rust
use salvo::logging::Logger;
use salvo::prelude::*;

#[handler]
async fn hello() -> &'static str {
    "Hello, World!"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let router = Router::new()
        .get(hello)
        .push(Router::with_path("error").get(error));

    let service = Service::new(router).hoop(Logger::new());

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(service).await;
}
```

## Custom Log Format

```rust
use salvo::prelude::*;
use tracing::info;
use std::time::Instant;

#[handler]
async fn request_logger(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let remote_addr = req.remote_addr().map(|a| a.to_string());

    ctrl.call_next(req, depot, res).await;

    let duration = start.elapsed();
    let status = res.status_code().unwrap_or(StatusCode::OK);

    info!(
        method = %method,
        path = %path,
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        remote_addr = ?remote_addr,
        "Request completed"
    );
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let router = Router::new()
        .hoop(request_logger)
        .get(hello);

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Structured Logging with Tracing

```rust
use salvo::prelude::*;
use tracing::{info, warn, error, debug, instrument};

#[handler]
#[instrument(skip(req, res), fields(user_id))]
async fn get_user(req: &mut Request, res: &mut Response) {
    let user_id: u32 = req.param("id").unwrap_or(0);

    tracing::Span::current().record("user_id", user_id);

    debug!("Fetching user from database");

    match fetch_user(user_id).await {
        Ok(user) => {
            info!(user_id = %user_id, "User found");
            res.render(Json(user));
        }
        Err(e) => {
            warn!(user_id = %user_id, error = %e, "User not found");
            res.status_code(StatusCode::NOT_FOUND);
        }
    }
}
```

## Log Levels and Filtering

```rust
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new("info")
                .add_directive("salvo=debug".parse().unwrap())
                .add_directive("hyper=warn".parse().unwrap())
        });

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() {
    init_logging();
    // Application code...
}
```

## JSON Logging for Production

```rust
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

fn init_json_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true)
        )
        .init();
}
```

## Request ID Tracking

```rust
use salvo::prelude::*;
use uuid::Uuid;
use tracing::info;

#[handler]
async fn add_request_id(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    let request_id = req
        .header::<String>("X-Request-ID")
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    depot.insert("request_id", request_id.clone());

    res.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );

    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %req.method(),
        path = %req.uri().path()
    );

    let _enter = span.enter();
    ctrl.call_next(req, depot, res).await;
}
```

## Error Logging

```rust
use salvo::prelude::*;
use tracing::{error, warn};

#[handler]
async fn error_handler(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    ctrl.call_next(req, depot, res).await;

    if let Some(status) = res.status_code() {
        if status.is_server_error() {
            error!(
                status = %status.as_u16(),
                path = %req.uri().path(),
                "Server error occurred"
            );
        } else if status.is_client_error() {
            warn!(
                status = %status.as_u16(),
                path = %req.uri().path(),
                "Client error"
            );
        }
    }
}
```

## Best Practices

1. Use structured logging with key-value pairs
2. Include request ID for tracing
3. Log timing information
4. Use appropriate log levels
5. Filter verbose logs in production
6. Use JSON format for log aggregation
7. Don't log sensitive data

## Related Skills

- **salvo-error-handling**: Log and handle errors
- **salvo-graceful-shutdown**: Log shutdown events
