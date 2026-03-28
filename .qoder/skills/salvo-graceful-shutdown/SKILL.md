---
name: salvo-graceful-shutdown
description: Implement graceful server shutdown to handle in-flight requests before stopping. Use for zero-downtime deployments and proper resource cleanup.
version: 0.89.3
tags: [operations, shutdown, deployment]
---

# Salvo Graceful Shutdown

This skill helps implement graceful server shutdown in Salvo applications.

## Setup

```toml
[dependencies]
salvo = "0.89.3"
tokio = { version = "1", features = ["full", "signal"] }
```

## Basic Graceful Shutdown

```rust
use salvo::prelude::*;
use salvo::server::ServerHandle;
use tokio::signal;

#[handler]
async fn hello() -> &'static str {
    "Hello, World!"
}

#[tokio::main]
async fn main() {
    let router = Router::new().get(hello);
    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;

    let server = Server::new(acceptor);
    let handle = server.handle();

    tokio::spawn(listen_shutdown_signal(handle));

    server.serve(router).await;
}

async fn listen_shutdown_signal(handle: ServerHandle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(windows)]
    let terminate = async {
        signal::windows::ctrl_c()
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => println!("Received Ctrl+C, shutting down..."),
        _ = terminate => println!("Received terminate signal, shutting down..."),
    };

    handle.stop_graceful(None);
}
```

## Shutdown with Timeout

```rust
use std::time::Duration;
use salvo::server::ServerHandle;

async fn listen_shutdown_signal(handle: ServerHandle) {
    tokio::signal::ctrl_c().await.unwrap();

    println!("Shutting down, waiting up to 30 seconds for in-flight requests...");

    handle.stop_graceful(Some(Duration::from_secs(30)));
}
```

## Health Check During Shutdown

```rust
use salvo::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

#[handler]
async fn health_check(res: &mut Response) {
    if SHUTTING_DOWN.load(Ordering::Relaxed) {
        res.status_code(StatusCode::SERVICE_UNAVAILABLE);
        res.render(Json(serde_json::json!({
            "status": "shutting_down",
            "message": "Server is shutting down"
        })));
    } else {
        res.render(Json(serde_json::json!({
            "status": "healthy"
        })));
    }
}

async fn listen_shutdown_signal(handle: salvo::server::ServerHandle) {
    tokio::signal::ctrl_c().await.unwrap();

    SHUTTING_DOWN.store(true, Ordering::Relaxed);
    println!("Marked as shutting down, waiting for traffic to drain...");

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    println!("Stopping server...");
    handle.stop_graceful(Some(std::time::Duration::from_secs(30)));
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(Router::with_path("health").get(health_check))
        .push(Router::with_path("api/{**rest}").get(api_handler));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    let server = Server::new(acceptor);
    let handle = server.handle();

    tokio::spawn(listen_shutdown_signal(handle));

    server.serve(router).await;
}
```

## Kubernetes/Docker Shutdown

```rust
use salvo::prelude::*;
use salvo::server::ServerHandle;
use std::time::Duration;

async fn kubernetes_shutdown(handle: ServerHandle) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler");

        sigterm.recv().await;
        println!("SIGTERM received from Kubernetes");
    }

    #[cfg(windows)]
    {
        tokio::signal::ctrl_c().await.unwrap();
        println!("Shutdown signal received");
    }

    handle.stop_graceful(Some(Duration::from_secs(25)));
}

#[tokio::main]
async fn main() {
    let router = Router::new().get(handler);

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    let server = Server::new(acceptor);
    let handle = server.handle();

    tokio::spawn(kubernetes_shutdown(handle));

    server.serve(router).await;
}
```

## Best Practices

1. Wait for in-flight requests to complete
2. Set appropriate timeout
3. Mark as shutting down for health checks
4. Clean up resources before shutdown
5. Handle multiple signal types
6. Log shutdown events
7. For Kubernetes, use shorter timeout than grace period

## Related Skills

- **salvo-logging**: Log shutdown events
- **salvo-tls-acme**: Graceful shutdown for HTTPS servers
