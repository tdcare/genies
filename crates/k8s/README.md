# genies_k8s

Kubernetes health probe endpoints for the Genies (神灯) framework, providing liveness and readiness checks for Salvo-based microservices.

## Overview

genies_k8s provides Kubernetes-compatible health check endpoints that integrate with Salvo web framework:

- **Liveness Probe**: `/actuator/health/liveness` - indicates if the service is alive
- **Readiness Probe**: `/actuator/health/readiness` - indicates if the service is ready to receive traffic
- **Global Status Control**: `SERVICE_STATUS` for programmatic health state management
- **Spring-Compatible Paths**: Uses Spring Boot Actuator-style paths for consistency

## Features

- **Zero Configuration**: Single function call to add health endpoints
- **Standard HTTP Codes**: 200 OK for healthy, 503 Service Unavailable for unhealthy
- **Graceful Degradation**: Control readiness during startup/shutdown
- **Kubernetes Native**: Works out-of-the-box with Kubernetes probe configurations

## Core Components

| Component | Description |
|-----------|-------------|
| `k8s_health_check()` | Returns a Salvo Router with health endpoints |
| `SERVICE_STATUS` | Global `HashMap<String, bool>` for status control |
| `/actuator/health/liveness` | Liveness endpoint handler |
| `/actuator/health/readiness` | Readiness endpoint handler |

## API Reference

### k8s_health_check()

```rust
pub fn k8s_health_check() -> Router
```

Returns a Salvo Router with two endpoints:
- `GET /actuator/health/liveness`
- `GET /actuator/health/readiness`

### SERVICE_STATUS

```rust
// From genies_context
pub static SERVICE_STATUS: Lazy<Mutex<HashMap<String, bool>>> = ...;
```

Keys:
- `"livenessProbe"` - Controls liveness endpoint response
- `"readinessProbe"` - Controls readiness endpoint response

## Quick Start

### 1. Add Dependency

Use `cargo add` to add dependencies (automatically fetches the latest version):

```sh
cargo add genies_k8s genies_context
```

You can also manually add dependencies in `Cargo.toml`. Visit [crates.io](https://crates.io) for the latest versions.

### 2. Add Health Endpoints to Router

```rust
use salvo::prelude::*;
use genies_k8s::k8s_health_check;

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(k8s_health_check())  // Add health endpoints
        .push(Router::with_path("/api").get(my_handler));
    
    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

### 3. Control Health Status

```rust
use genies_context::SERVICE_STATUS;
use std::ops::DerefMut;

// Set service as unhealthy during shutdown
fn shutdown() {
    let mut status = SERVICE_STATUS.lock().unwrap();
    let map = status.deref_mut();
    map.insert("readinessProbe".to_string(), false);
}

// Set service as ready after initialization
fn on_ready() {
    let mut status = SERVICE_STATUS.lock().unwrap();
    let map = status.deref_mut();
    map.insert("readinessProbe".to_string(), true);
    map.insert("livenessProbe".to_string(), true);
}
```

## HTTP Response Behavior

| Endpoint | Status Value | HTTP Response |
|----------|-------------|---------------|
| `/actuator/health/liveness` | `true` | 200 OK, body: "Ok" |
| `/actuator/health/liveness` | `false` | 503 Service Unavailable |
| `/actuator/health/readiness` | `true` | 200 OK, body: "Ok" |
| `/actuator/health/readiness` | `false` | 503 Service Unavailable |

## Kubernetes Deployment Configuration

### Deployment YAML Example

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-service
spec:
  replicas: 3
  selector:
    matchLabels:
      app: my-service
  template:
    metadata:
      labels:
        app: my-service
    spec:
      containers:
      - name: my-service
        image: my-service:latest
        ports:
        - containerPort: 5800
        livenessProbe:
          httpGet:
            path: /actuator/health/liveness
            port: 5800
          initialDelaySeconds: 10
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /actuator/health/readiness
            port: 5800
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
```

### Probe Configuration Recommendations

| Probe | Initial Delay | Period | Timeout | Failure Threshold |
|-------|---------------|--------|---------|-------------------|
| Liveness | 10-30s | 10s | 5s | 3 |
| Readiness | 5-10s | 5s | 3s | 3 |

## Usage Patterns

### Startup Sequence

```rust
use genies_context::SERVICE_STATUS;

#[tokio::main]
async fn main() {
    // Service starts with default status (typically true)
    
    // Initialize components
    init_database().await;
    init_cache().await;
    
    // Mark as ready
    {
        let mut status = SERVICE_STATUS.lock().unwrap();
        status.insert("readinessProbe".to_string(), true);
        status.insert("livenessProbe".to_string(), true);
    }
    
    // Start server
    let router = Router::new()
        .push(k8s_health_check())
        .push(app_routes());
    
    Server::new(acceptor).serve(router).await;
}
```

### Graceful Shutdown

```rust
use tokio::signal;

async fn graceful_shutdown() {
    // Mark as not ready to stop receiving traffic
    {
        let mut status = SERVICE_STATUS.lock().unwrap();
        status.insert("readinessProbe".to_string(), false);
    }
    
    // Wait for in-flight requests to complete
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Cleanup resources
    cleanup().await;
}

#[tokio::main]
async fn main() {
    // ... setup ...
    
    tokio::select! {
        _ = server.serve(router) => {},
        _ = signal::ctrl_c() => {
            graceful_shutdown().await;
        }
    }
}
```

### Health Check with Dependencies

```rust
async fn check_dependencies() -> bool {
    let db_ok = check_database().await.is_ok();
    let cache_ok = check_cache().await.is_ok();
    
    db_ok && cache_ok
}

async fn health_monitor() {
    loop {
        let healthy = check_dependencies().await;
        
        {
            let mut status = SERVICE_STATUS.lock().unwrap();
            status.insert("readinessProbe".to_string(), healthy);
        }
        
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
```

## Integration with Salvo Router

```rust
use salvo::prelude::*;
use genies_k8s::k8s_health_check;

let router = Router::new()
    // Health endpoints (no auth required)
    .push(k8s_health_check())
    
    // Protected routes
    .push(
        Router::with_path("/api")
            .hoop(auth_middleware)
            .push(api_routes())
    );
```

## White List Configuration

Add health endpoints to auth whitelist in `application.yml`:

```yaml
white_list_api:
  - "/actuator/*"
  - "/actuator/health/liveness"
  - "/actuator/health/readiness"
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use salvo::test::TestClient;
    
    #[tokio::test]
    async fn test_liveness_healthy() {
        // Set healthy status
        {
            let mut status = SERVICE_STATUS.lock().unwrap();
            status.insert("livenessProbe".to_string(), true);
        }
        
        let router = k8s_health_check();
        let response = TestClient::get("/actuator/health/liveness")
            .send(&router)
            .await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_readiness_unhealthy() {
        // Set unhealthy status
        {
            let mut status = SERVICE_STATUS.lock().unwrap();
            status.insert("readinessProbe".to_string(), false);
        }
        
        let router = k8s_health_check();
        let response = TestClient::get("/actuator/health/readiness")
            .send(&router)
            .await;
        
        assert_eq!(response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
```

## Dependencies

- **salvo** - Web framework
- **genies_context** - Provides `SERVICE_STATUS` global

## Integration with Other Crates

- **genies_context**: Provides `SERVICE_STATUS` global variable
- **genies_config**: `white_list_api` should include `/actuator/*`
- **genies_auth**: Health endpoints should bypass authentication

## Endpoint Comparison with Spring Boot

| Genies | Spring Boot | Description |
|--------|-------------|-------------|
| `/actuator/health/liveness` | `/actuator/health/liveness` | Liveness probe |
| `/actuator/health/readiness` | `/actuator/health/readiness` | Readiness probe |

## License

See the project root for license information.
