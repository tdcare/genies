---
name: k8s-usage
description: Guide for using genies_k8s Kubernetes health probes. Use when configuring liveness and readiness probes, integrating health check endpoints into Salvo routers, or deploying Genies services to Kubernetes.
---

# K8s Module (genies_k8s)

## Overview

genies_k8s 是 Genies 框架的 Kubernetes 健康探针库，提供存活（liveness）和就绪（readiness）检查端点。纯库 crate，无 binary。

**核心特性：**
- 存活探针：`/actuator/health/liveness`
- 就绪探针：`/actuator/health/readiness`
- 全局状态：`SERVICE_STATUS` 控制健康状态
- Spring Boot 兼容路径
- 200 OK / 503 Service Unavailable 响应

## Quick Start

### 添加健康端点

```rust
use salvo::prelude::*;
use genies_k8s::k8s_health_check;

let router = Router::new()
    .push(k8s_health_check())  // 添加 /actuator/health/*
    .push(Router::with_path("/api").get(handler));
```

### 控制健康状态

```rust
use genies_context::SERVICE_STATUS;
use std::ops::DerefMut;

// 设置就绪
fn set_ready() {
    let mut status = SERVICE_STATUS.lock().unwrap();
    let map = status.deref_mut();
    map.insert("readinessProbe".to_string(), true);
    map.insert("livenessProbe".to_string(), true);
}

// 设置不就绪（用于优雅关闭）
fn set_not_ready() {
    let mut status = SERVICE_STATUS.lock().unwrap();
    let map = status.deref_mut();
    map.insert("readinessProbe".to_string(), false);
}
```

## 端点说明

| 端点 | 用途 | 健康响应 | 不健康响应 |
|------|------|----------|------------|
| `/actuator/health/liveness` | 进程存活检查 | 200 "Ok" | 503 |
| `/actuator/health/readiness` | 流量接收能力 | 200 "Ok" | 503 |

## SERVICE_STATUS

```rust
// 全局状态（来自 genies_context）
pub static SERVICE_STATUS: Lazy<Mutex<HashMap<String, bool>>>

// 键
"livenessProbe"   // 存活探针状态
"readinessProbe"  // 就绪探针状态
```

## 使用模式

### 完整启动流程

```rust
use genies_k8s::k8s_health_check;
use genies_context::SERVICE_STATUS;
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    // 1. 服务启动时默认状态为 true
    
    // 2. 初始化组件
    init_database().await;
    init_cache().await;
    
    // 3. 显式标记就绪
    {
        let mut status = SERVICE_STATUS.lock().unwrap();
        status.insert("readinessProbe".to_string(), true);
        status.insert("livenessProbe".to_string(), true);
    }
    
    // 4. 构建路由
    let router = Router::new()
        .push(k8s_health_check())
        .push(api_routes());
    
    // 5. 启动服务
    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

### 优雅关闭

```rust
use tokio::signal;
use std::time::Duration;

async fn graceful_shutdown() {
    // Step 1: 标记不就绪，停止接收新流量
    {
        let mut status = SERVICE_STATUS.lock().unwrap();
        status.insert("readinessProbe".to_string(), false);
    }
    
    // Step 2: 等待 Kubernetes 更新 Service endpoints
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Step 3: 等待处理中的请求完成
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    // Step 4: 清理资源
    cleanup_resources().await;
}

#[tokio::main]
async fn main() {
    let server = create_server();
    
    tokio::select! {
        _ = server.serve(router) => {},
        _ = signal::ctrl_c() => {
            graceful_shutdown().await;
        }
    }
}
```

### 依赖健康监控

```rust
async fn health_monitor_task() {
    loop {
        // 检查依赖
        let db_ok = check_database().await.is_ok();
        let cache_ok = check_cache().await.is_ok();
        let healthy = db_ok && cache_ok;
        
        // 更新就绪状态
        {
            let mut status = SERVICE_STATUS.lock().unwrap();
            status.insert("readinessProbe".to_string(), healthy);
        }
        
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

#[tokio::main]
async fn main() {
    // 启动健康监控
    tokio::spawn(health_monitor_task());
    
    // ... 启动服务器
}
```

## Kubernetes 配置

### Deployment YAML

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-service
spec:
  replicas: 3
  template:
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

### 探针参数建议

| 探针 | initialDelaySeconds | periodSeconds | timeoutSeconds | failureThreshold |
|------|---------------------|---------------|----------------|------------------|
| liveness | 10-30 | 10 | 5 | 3 |
| readiness | 5-10 | 5 | 3 | 3 |

**说明：**
- `initialDelaySeconds`: 容器启动后等待时间
- `periodSeconds`: 探测间隔
- `timeoutSeconds`: 探测超时
- `failureThreshold`: 连续失败次数判定为不健康

## 与其他 Crate 集成

### 添加到白名单

```yaml
# application.yml
white_list_api:
  - "/actuator/*"
```

### 路由顺序

```rust
let router = Router::new()
    // 健康端点放在最前面，无需认证
    .push(k8s_health_check())
    
    // 受保护的 API
    .push(
        Router::with_path("/api")
            .hoop(salvo_auth)
            .hoop(casbin_auth)
            .push(api_routes())
    );
```

## 存活 vs 就绪探针

| 探针 | 失败后果 | 使用场景 |
|------|----------|----------|
| Liveness | 容器被杀死重启 | 死锁、进程卡死 |
| Readiness | 停止接收流量 | 启动中、依赖不可用、优雅关闭 |

**最佳实践：**
- Liveness 应该只检查进程是否存活
- Readiness 检查是否能处理请求（依赖可用）
- 优雅关闭时只设置 readiness=false，保持 liveness=true

## 测试

```rust
#[cfg(test)]
mod tests {
    use genies_k8s::k8s_health_check;
    use genies_context::SERVICE_STATUS;
    use salvo::test::TestClient;
    
    #[tokio::test]
    async fn test_liveness_returns_ok() {
        {
            let mut status = SERVICE_STATUS.lock().unwrap();
            status.insert("livenessProbe".to_string(), true);
        }
        
        let router = k8s_health_check();
        let resp = TestClient::get("/actuator/health/liveness")
            .send(&router)
            .await;
        
        assert_eq!(resp.status_code(), 200);
    }
    
    #[tokio::test]
    async fn test_readiness_returns_503_when_not_ready() {
        {
            let mut status = SERVICE_STATUS.lock().unwrap();
            status.insert("readinessProbe".to_string(), false);
        }
        
        let router = k8s_health_check();
        let resp = TestClient::get("/actuator/health/readiness")
            .send(&router)
            .await;
        
        assert_eq!(resp.status_code(), 503);
    }
}
```

## Key Files

- [crates/k8s/src/lib.rs](file:///d:/tdcare/genies/crates/k8s/src/lib.rs) - k8s_health_check 和处理器
- [crates/context/src/lib.rs](file:///d:/tdcare/genies/crates/context/src/lib.rs) - SERVICE_STATUS 定义
