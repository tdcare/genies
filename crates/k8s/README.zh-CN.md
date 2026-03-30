# genies_k8s

Genies (神灯) 框架的 Kubernetes 健康探针端点，为基于 Salvo 的微服务提供存活和就绪检查。

## 概述

genies_k8s 提供与 Kubernetes 兼容的健康检查端点，与 Salvo Web 框架集成：

- **存活探针**：`/actuator/health/liveness` - 指示服务是否存活
- **就绪探针**：`/actuator/health/readiness` - 指示服务是否准备好接收流量
- **全局状态控制**：`SERVICE_STATUS` 用于程序化健康状态管理
- **Spring 兼容路径**：使用 Spring Boot Actuator 风格的路径保持一致性

## 核心特性

- **零配置**：单次函数调用即可添加健康端点
- **标准 HTTP 状态码**：200 OK 表示健康，503 Service Unavailable 表示不健康
- **优雅降级**：在启动/关闭期间控制就绪状态
- **Kubernetes 原生**：开箱即用的 Kubernetes 探针配置

## 核心组件

| 组件 | 说明 |
|------|------|
| `k8s_health_check()` | 返回包含健康端点的 Salvo Router |
| `SERVICE_STATUS` | 全局 `HashMap<String, bool>` 用于状态控制 |
| `/actuator/health/liveness` | 存活端点处理器 |
| `/actuator/health/readiness` | 就绪端点处理器 |

## API 参考

### k8s_health_check()

```rust
pub fn k8s_health_check() -> Router
```

返回包含两个端点的 Salvo Router：
- `GET /actuator/health/liveness`
- `GET /actuator/health/readiness`

### SERVICE_STATUS

```rust
// 来自 genies_context
pub static SERVICE_STATUS: Lazy<Mutex<HashMap<String, bool>>> = ...;
```

键：
- `"livenessProbe"` - 控制存活端点响应
- `"readinessProbe"` - 控制就绪端点响应

## 快速开始

### 1. 添加依赖

```sh
cargo add genies_k8s genies_context
```

> 也可以手动在 `Cargo.toml` 中添加依赖，请前往 [crates.io](https://crates.io) 查看最新版本。

### 2. 添加健康端点到路由

```rust
use salvo::prelude::*;
use genies_k8s::k8s_health_check;

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(k8s_health_check())  // 添加健康端点
        .push(Router::with_path("/api").get(my_handler));
    
    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

### 3. 控制健康状态

```rust
use genies_context::SERVICE_STATUS;
use std::ops::DerefMut;

// 关闭期间设置为不健康
fn shutdown() {
    let mut status = SERVICE_STATUS.lock().unwrap();
    let map = status.deref_mut();
    map.insert("readinessProbe".to_string(), false);
}

// 初始化后设置为就绪
fn on_ready() {
    let mut status = SERVICE_STATUS.lock().unwrap();
    let map = status.deref_mut();
    map.insert("readinessProbe".to_string(), true);
    map.insert("livenessProbe".to_string(), true);
}
```

## HTTP 响应行为

| 端点 | 状态值 | HTTP 响应 |
|------|--------|-----------|
| `/actuator/health/liveness` | `true` | 200 OK，body: "Ok" |
| `/actuator/health/liveness` | `false` | 503 Service Unavailable |
| `/actuator/health/readiness` | `true` | 200 OK，body: "Ok" |
| `/actuator/health/readiness` | `false` | 503 Service Unavailable |

## Kubernetes 部署配置

### Deployment YAML 示例

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

### 探针配置建议

| 探针 | 初始延迟 | 周期 | 超时 | 失败阈值 |
|------|----------|------|------|----------|
| 存活 | 10-30s | 10s | 5s | 3 |
| 就绪 | 5-10s | 5s | 3s | 3 |

## 使用模式

### 启动顺序

```rust
use genies_context::SERVICE_STATUS;

#[tokio::main]
async fn main() {
    // 服务启动时使用默认状态（通常为 true）
    
    // 初始化组件
    init_database().await;
    init_cache().await;
    
    // 标记为就绪
    {
        let mut status = SERVICE_STATUS.lock().unwrap();
        status.insert("readinessProbe".to_string(), true);
        status.insert("livenessProbe".to_string(), true);
    }
    
    // 启动服务器
    let router = Router::new()
        .push(k8s_health_check())
        .push(app_routes());
    
    Server::new(acceptor).serve(router).await;
}
```

### 优雅关闭

```rust
use tokio::signal;

async fn graceful_shutdown() {
    // 标记为未就绪以停止接收流量
    {
        let mut status = SERVICE_STATUS.lock().unwrap();
        status.insert("readinessProbe".to_string(), false);
    }
    
    // 等待处理中的请求完成
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // 清理资源
    cleanup().await;
}

#[tokio::main]
async fn main() {
    // ... 设置 ...
    
    tokio::select! {
        _ = server.serve(router) => {},
        _ = signal::ctrl_c() => {
            graceful_shutdown().await;
        }
    }
}
```

### 依赖健康检查

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

## 与 Salvo Router 集成

```rust
use salvo::prelude::*;
use genies_k8s::k8s_health_check;

let router = Router::new()
    // 健康端点（无需认证）
    .push(k8s_health_check())
    
    // 受保护路由
    .push(
        Router::with_path("/api")
            .hoop(auth_middleware)
            .push(api_routes())
    );
```

## 白名单配置

在 `application.yml` 中将健康端点添加到认证白名单：

```yaml
white_list_api:
  - "/actuator/*"
  - "/actuator/health/liveness"
  - "/actuator/health/readiness"
```

## 测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use salvo::test::TestClient;
    
    #[tokio::test]
    async fn test_liveness_healthy() {
        // 设置健康状态
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
        // 设置不健康状态
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

## 依赖项

- **salvo** - Web 框架
- **genies_context** - 提供 `SERVICE_STATUS` 全局变量

## 与其他 Crate 集成

- **genies_context**：提供 `SERVICE_STATUS` 全局变量
- **genies_config**：`white_list_api` 应包含 `/actuator/*`
- **genies_auth**：健康端点应绕过认证

## 与 Spring Boot 端点对比

| Genies | Spring Boot | 说明 |
|--------|-------------|------|
| `/actuator/health/liveness` | `/actuator/health/liveness` | 存活探针 |
| `/actuator/health/readiness` | `/actuator/health/readiness` | 就绪探针 |

## 许可证

请参阅项目根目录的许可证信息。
