---
name: context-usage
description: Guide for using genies_context application context management. Use when initializing global context, managing database connections, configuring cache services, handling cross-service authentication tokens, or integrating K8s health status in Genies projects.
---

# Context Module (genies_context)

## Overview

genies_context 是 Genies 框架的应用上下文管理库，提供全局上下文单例、数据库连接池、缓存服务和 JWT 认证中间件。使用 `lazy_static` 模式实现全局访问。

**核心特性：**
- 全局上下文单例（CONTEXT）
- 多数据库支持（MySQL、PostgreSQL、SQLite、MSSQL、Oracle、TDengine）
- 数据库连接池（RBatis，自动选择驱动）
- Redis 缓存服务
- Keycloak JWT 验证
- 跨服务 Token 管理（REMOTE_TOKEN）
- K8s 健康探针状态（SERVICE_STATUS）
- Salvo 认证中间件（salvo_auth）

## Architecture

```
应用启动 → CONTEXT (lazy_static) → init_database() → 就绪
                │
                ├── ApplicationConfig (./application.yml)
                ├── Keycloak Keys (异步获取)
                ├── CacheService (Redis)
                └── RBatis (根据 URL scheme 自动选择驱动)
```

核心组件：
- `ApplicationContext` - 主上下文结构（config, rbatis, cache_service, keycloak_keys）
- `CONTEXT` - 全局单例
- `REMOTE_TOKEN` - 跨服务 Token（Mutex<RemoteToken>）
- `SERVICE_STATUS` - K8s 探针状态（Mutex<HashMap>）
- `init_database()` - 数据库初始化（自动选择驱动，Once 保证幂等）
- `init_mysql()` - 已废弃，`init_database` 的别名
- `salvo_auth` - JWT 认证中间件
- `checked_token` / `is_white_list_api` - 认证辅助函数

## Quick Start

### 1. Dependencies

```toml
[dependencies]
genies_context = { workspace = true }
genies_config = { workspace = true }
genies_cache = { workspace = true }
genies_core = { workspace = true }
rbatis = "4.x"
```

### 2. Initialize Database

```rust
use genies::context::CONTEXT;

#[tokio::main]
async fn main() {
    // 初始化数据库连接池（线程安全，幂等）
    // 根据 database_url 的 scheme 自动选择驱动
    CONTEXT.init_database().await;
    
    println!("数据库已连接: {:?}", CONTEXT.rbatis.get_pool().unwrap().state().await);
}
```

### 3. Use Database

```rust
use genies::context::CONTEXT;
use rbatis::executor::Executor;

// 直接查询
pub async fn query_users() -> Vec<User> {
    User::select_all(&CONTEXT.rbatis).await.unwrap()
}

// 使用事务
pub async fn create_user(user: &User) {
    let mut tx = CONTEXT.rbatis.acquire_begin().await.unwrap();
    User::insert(&mut tx, user).await.unwrap();
    tx.commit().await.unwrap();
}
```

### 4. Use Cache

```rust
use genies::context::CONTEXT;

// 标准缓存
CONTEXT.cache_service.set_string("key", "value").await?;
let value = CONTEXT.cache_service.get_string("key").await?;

// 持久化缓存
CONTEXT.redis_save_service.set_string("key", "data").await?;
```

### 5. Configure Auth Middleware

```rust
use genies::context::auth::salvo_auth;
use salvo::prelude::*;

let router = Router::new()
    .hoop(salvo_auth)  // JWT 认证
    .push(Router::with_path("/api/users").get(get_users));
```

## API Reference

### ApplicationContext

```rust
pub struct ApplicationContext {
    pub config: ApplicationConfig,      // 应用配置
    pub rbatis: RBatis,                 // 数据库连接池
    pub cache_service: CacheService,    // 标准缓存
    pub redis_save_service: CacheService, // 持久化缓存
    pub keycloak_keys: Keys,            // JWT 验证密钥
}

impl ApplicationContext {
    pub async fn init_database(&self);  // 初始化数据库（幂等，自动选择驱动）
    #[deprecated]
    pub async fn init_mysql(&self);     // 已废弃，使用 init_database
    pub fn new() -> Self;               // 创建上下文
}
```

### Global Singletons

```rust
lazy_static! {
    pub static ref CONTEXT: ApplicationContext = ApplicationContext::default();
    pub static ref REMOTE_TOKEN: Mutex<RemoteToken> = Mutex::new(RemoteToken::new());
    pub static ref SERVICE_STATUS: Mutex<HashMap<String, bool>> = Mutex::new({
        let mut map = HashMap::new();
        map.insert("readinessProbe".to_string(), true);
        map.insert("livenessProbe".to_string(), true);
        map
    });
}
```

### RemoteToken

```rust
pub struct RemoteToken {
    pub access_token: String,
}

impl RemoteToken {
    pub fn new() -> Self;  // 从 Keycloak 获取 token
}
```

### Auth Functions

```rust
// 检查 API 白名单
pub fn is_white_list_api(context: &ApplicationContext, path: &str) -> bool;

// 验证 Token
pub async fn checked_token(
    context: &ApplicationContext,
    token: &str,
    path: &str,
) -> Result<JWTToken, Error>;

// Salvo JWT 中间件
#[handler]
pub async fn salvo_auth(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl);
```

## Configuration

### application.yml

```yaml
server_url: "0.0.0.0:5800"

# 数据库 URL（根据 scheme 自动选择驱动）
# MySQL:      mysql://user:pass@localhost:3306/db
# PostgreSQL: postgres://user:pass@localhost:5432/db
# SQLite:     sqlite://./data.db
# MSSQL:      mssql://user:pass@localhost:1433/db
# Oracle:     oracle://user:pass@localhost:1521/ORCL
# TDengine:   taos://user:pass@localhost:6030/db
database_url: "mysql://user:pass@localhost:3306/db"
max_connections: 10
wait_timeout: 30
max_lifetime: 3600

redis_host: "localhost"
redis_port: 6379

keycloak_auth_server_url: "http://localhost:8080"
keycloak_realm: "myrealm"
keycloak_resource: "myapp"
keycloak_credentials_secret: "secret"

white_list_api:
  - "/health"
  - "/dapr/*"
  - "/swagger-ui/*"
```

### Feature Flags

| Feature | 驱动 | URL Scheme |
|---------|------|------------|
| `mysql`（默认） | rbdc-mysql | `mysql://` |
| `postgres` | rbdc-pg | `postgres://`, `postgresql://` |
| `sqlite` | rbdc-sqlite | `sqlite://` |
| `mssql` | rbdc-mssql | `mssql://`, `sqlserver://` |
| `oracle` | rbdc-oracle | `oracle://` |
| `tdengine` | rbdc-tdengine | `taos://`, `taos+ws://` |
| `all-db` | 所有驱动 | 以上所有 |

**切换数据库：**

```toml
# 使用 PostgreSQL
[dependencies]
genies = { version = "1.5", default-features = false, features = ["postgres"] }

# 或直接使用 genies_context
genies_context = { version = "1.5", default-features = false, features = ["postgres"] }
```

## Auth Middleware Flow

```
请求 → salvo_auth
    │
    ├── 白名单? → 跳过认证
    │
    └── 否 → checked_token()
              │
              ├── 有效 → depot.insert("jwtToken") → 继续
              └── 无效 → 401 Unauthorized
```

## K8s Health Status

```rust
use genies::context::SERVICE_STATUS;

// 更新就绪状态
{
    let mut status = SERVICE_STATUS.lock().unwrap();
    status.insert("readinessProbe".to_string(), false);
}

// 检查存活状态
{
    let status = SERVICE_STATUS.lock().unwrap();
    let is_alive = *status.get("livenessProbe").unwrap_or(&false);
}
```

## Thread Safety

- `CONTEXT`: `lazy_static` 单次初始化，字段线程安全
- `init_database()`: `Once` 保证幂等
- `REMOTE_TOKEN`: `Mutex` 线程安全
- `SERVICE_STATUS`: `Mutex` 线程安全

## Integration

- **genies_auth**: 使用 `CONTEXT.rbatis` 存储策略，`salvo_auth` 进行 JWT 验证
- **genies_ddd**: 使用 `CONTEXT.rbatis` 发布事件
- **genies_dapr**: 使用 `CONTEXT.rbatis` 事务管理
- **genies_config**: 提供 `ApplicationConfig`

## Key Files

- [crates/context/src/lib.rs](file:///d:/tdcare/genies/crates/context/src/lib.rs) - 模块入口，全局单例定义
- [crates/context/src/app_context.rs](file:///d:/tdcare/genies/crates/context/src/app_context.rs) - ApplicationContext 结构
- [crates/context/src/auth.rs](file:///d:/tdcare/genies/crates/context/src/auth.rs) - 认证中间件和辅助函数
- [crates/config/src/app_config.rs](file:///d:/tdcare/genies/crates/config/src/app_config.rs) - ApplicationConfig 定义
- [crates/cache/src/cache_service.rs](file:///d:/tdcare/genies/crates/cache/src/cache_service.rs) - CacheService 实现
