# genies_context

Genies (神灯) 框架的应用上下文管理库，提供全局上下文、数据库连接、缓存服务和跨服务认证。

## 概述

genies_context 提供应用运行时上下文的集中管理，包括数据库连接池、缓存服务、Keycloak 认证密钥和跨服务 Token 管理。使用 `lazy_static` 模式在整个应用中提供全局单例访问。

## 核心特性

- **全局上下文单例**：`CONTEXT` 提供对配置、数据库和缓存的访问
- **多数据库支持**：通过 feature flags 支持 MySQL、PostgreSQL、SQLite、MSSQL、Oracle、TDengine
- **数据库连接池**：通过 RBatis 实现异步连接池，自动选择驱动
- **缓存服务**：Redis 支持的缓存，分离数据缓存和持久化缓存通道
- **Keycloak 集成**：JWT 密钥获取和 Token 验证
- **跨服务 Token**：`REMOTE_TOKEN` 用于服务间认证
- **K8s 健康状态**：`SERVICE_STATUS` 用于就绪/存活探针
- **Salvo 认证中间件**：`salvo_auth` 用于 JWT 认证

## 架构设计

### 核心组件

| 组件 | 文件 | 功能 |
|------|------|------|
| `ApplicationContext` | app_context.rs | 主上下文结构，包含配置、rbatis、缓存服务 |
| `CONTEXT` | lib.rs | 通过 `lazy_static` 实现的全局单例 |
| `REMOTE_TOKEN` | lib.rs | 跨服务 Token 存储（`Mutex<RemoteToken>`） |
| `SERVICE_STATUS` | lib.rs | K8s 探针状态（`Mutex<HashMap>`） |
| `init_database` | app_context.rs | 异步数据库连接池初始化（自动选择驱动） |
| `init_mysql` | app_context.rs | `init_database` 的已废弃别名 |
| `RemoteToken` | app_context.rs | 服务间认证 Token |
| `salvo_auth` | auth.rs | Salvo JWT 认证中间件 |
| `checked_token` | auth.rs | Token 验证函数 |
| `is_white_list_api` | auth.rs | API 白名单检查 |

### 初始化流程

```
应用启动 → CONTEXT (lazy_static) → init_database() → 就绪
                │
                ├── ApplicationContext::new()
                │       ├── ApplicationConfig (./application.yml)
                │       ├── Keycloak Keys (异步获取)
                │       ├── CacheService (Redis)
                │       └── Snowflake ID 生成器 (worker_id 解析)
                │
                └── RBatis (根据 URL scheme 自动选择驱动)
```

## 快速开始

### 1. 添加依赖

```sh
cargo add genies_context genies_config genies_cache genies_core rbatis
```

> 也可以手动在 `Cargo.toml` 中添加依赖，请前往 [crates.io](https://crates.io) 查看最新版本。

### 2. 初始化数据库

```rust
use genies::context::CONTEXT;

#[tokio::main]
async fn main() {
    // 初始化数据库连接池（线程安全，只执行一次）
    // 根据 database_url 的 scheme 自动选择驱动
    CONTEXT.init_database().await;
    
    // 现在可以使用 CONTEXT.rbatis
    println!("数据库已连接: {:?}", CONTEXT.rbatis.get_pool().unwrap().state().await);
}
```

### 3. 使用数据库连接

```rust
use genies::context::CONTEXT;
use rbatis::executor::Executor;

pub async fn query_users() -> Vec<User> {
    let rb = &CONTEXT.rbatis;
    User::select_all(rb).await.unwrap()
}

// 使用事务
pub async fn create_user(user: &User) {
    let mut tx = CONTEXT.rbatis.acquire_begin().await.unwrap();
    User::insert(&mut tx, user).await.unwrap();
    tx.commit().await.unwrap();
}
```

### 4. 使用缓存服务

```rust
use genies::context::CONTEXT;

pub async fn cache_example() {
    // 使用 cache_service（标准缓存）
    CONTEXT.cache_service.set_string("key", "value").await.unwrap();
    let value = CONTEXT.cache_service.get_string("key").await.unwrap();
    
    // 使用 redis_save_service（持久化缓存）
    CONTEXT.redis_save_service.set_string("persistent_key", "data").await.unwrap();
}
```

### 5. 配置 Salvo 认证中间件

```rust
use genies::context::auth::salvo_auth;
use salvo::prelude::*;

let router = Router::new()
    .hoop(salvo_auth)  // JWT 认证中间件
    .push(Router::with_path("/api/users").get(get_users));
```

## API 参考

### ApplicationContext 结构体

```rust
pub struct ApplicationContext {
    /// 来自 ./application.yml 的应用配置
    pub config: ApplicationConfig,
    
    /// RBatis 数据库连接池
    pub rbatis: RBatis,
    
    /// 标准缓存服务（Redis）
    pub cache_service: CacheService,
    
    /// 持久化缓存服务（Redis）
    pub redis_save_service: CacheService,
    
    /// Keycloak JWT 验证密钥
    pub keycloak_keys: Keys,
}

impl ApplicationContext {
    /// 初始化数据库连接池（线程安全，幂等）
    /// 根据 database_url 的 scheme 自动选择驱动
    pub async fn init_database(&self);
    
    /// 初始化数据库（已废弃，请使用 init_database）
    #[deprecated(note = "请使用 init_database()")]
    pub async fn init_mysql(&self);
    
    /// 创建新的 ApplicationContext（读取 ./application.yml）
    pub fn new() -> Self;
}
```

### 雪花 ID 生成器初始化

在 `ApplicationContext::new()` 期间，框架会自动解析唯一的 `worker_id` 并初始化全局雪花 ID 生成器。

#### Worker ID 解析优先级

| 优先级 | 来源 | 条件 | 说明 |
|--------|------|------|------|
| 1 | Redis 槽位 | `cache_type = "redis"` | 通过 `SETNX snowflake:slot:{server_name}:{0..1023}` 注册槽位，TTL 为 1 小时。后台任务每 30 分钟续期。 |
| 2 | K8s HOSTNAME | `HOSTNAME` 环境变量以数字结尾 | 提取 Pod 序号（如 `sickbed-service-2` → 2），对 1024 取模 |
| 3 | 配置文件 | application.yml 中的 `machine_id` | 直接使用配置的值 |
| 4 | 兜底 | 始终 | 默认为 `1` |

### 全局单例

```rust
lazy_static! {
    /// 全局应用上下文（数据库、缓存、配置）
    pub static ref CONTEXT: ApplicationContext = ApplicationContext::default();
    
    /// 跨服务访问 Token 存储
    pub static ref REMOTE_TOKEN: Mutex<RemoteToken> = Mutex::new(RemoteToken::new());
    
    /// K8s 服务健康状态
    pub static ref SERVICE_STATUS: Mutex<HashMap<String, bool>> = Mutex::new({
        let mut map = HashMap::new();
        map.insert("readinessProbe".to_string(), true);
        map.insert("livenessProbe".to_string(), true);
        map
    });
}
```

### RemoteToken 结构体

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RemoteToken {
    pub access_token: String,
}

impl RemoteToken {
    /// 从 Keycloak 获取创建新的 RemoteToken
    pub fn new() -> Self;
}
```

### 认证函数

```rust
/// 检查路径是否在 API 白名单中
pub fn is_white_list_api(context: &ApplicationContext, path: &str) -> bool;

/// 验证 Token 并返回 JWTToken
pub async fn checked_token(
    context: &ApplicationContext,
    token: &str,
    path: &str,
) -> Result<JWTToken, Error>;

/// 检查授权（当前返回 Ok）
pub async fn check_auth(
    context: &ApplicationContext,
    token: &JWTToken,
    path: &str,
) -> Result<(), Error>;

/// Salvo JWT 认证中间件
#[handler]
pub async fn salvo_auth(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
);
```

## 配置说明

### application.yml 结构

```yaml
server_url: "0.0.0.0:5800"
database_url: "mysql://user:pass@localhost:3306/db"
max_connections: 10
wait_timeout: 30
max_lifetime: 3600

# Redis
redis_host: "localhost"
redis_port: 6379

# Keycloak
keycloak_auth_server_url: "http://localhost:8080"
keycloak_realm: "myrealm"
keycloak_resource: "myapp"
keycloak_credentials_secret: "secret"

# 白名单 API（跳过认证）
white_list_api:
  - "/health"
  - "/dapr/*"
  - "/swagger-ui/*"
```

#### 雪花 ID 配置

```yaml
# 可选：手动设置 machine_id（优先级 3，在 Redis 和 K8s 之后）
machine_id: 1
```

当 `cache_type` 为 `"redis"` 时，框架会自动注册 Redis 槽位，无需手动配置。

## 认证中间件流程

```
请求 → salvo_auth
    │
    ├── is_white_list_api? → 是 → 继续（跳过认证）
    │
    └── 否 → checked_token()
               │
               ├── 有效 → check_auth() → depot.insert("jwtToken", token)
               │                              → 继续
               │
               └── 无效 → 401 Unauthorized
```

## K8s 健康状态

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

## 多数据库支持

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

### 使用方法

**在 Cargo.toml 中切换数据库：**

```toml
# 使用 PostgreSQL 代替默认的 MySQL
[dependencies]
genies_context = { version = "1.5", default-features = false, features = ["postgres"] }

# 或通过 genies 门面 crate
genies = { version = "1.5", default-features = false, features = ["postgres"] }
```

**application.yml 中的数据库 URL 示例：**

```yaml
# MySQL
database_url: "mysql://user:password@localhost:3306/mydb"

# PostgreSQL
database_url: "postgres://user:password@localhost:5432/mydb"

# SQLite
database_url: "sqlite://./data.db"

# MSSQL
database_url: "mssql://user:password@localhost:1433/mydb"

# Oracle
database_url: "oracle://user:password@localhost:1521/ORCL"

# TDengine
database_url: "taos://user:password@localhost:6030/mydb"
```

### 向后兼容性

- 默认 feature 为 `mysql`，现有 MySQL 项目无需任何修改
- `init_mysql()` 仍可使用但已废弃，请使用 `init_database()` 替代
- 运行时根据 `database_url` 的 scheme 自动选择驱动

## 依赖项

- **genies_config** - 应用配置
- **genies_cache** - 缓存服务抽象
- **genies_core** - JWT 工具、错误类型
- **rbatis** - ORM 框架
- **rbdc-mysql** - MySQL 驱动（`mysql` feature）
- **rbdc-pg** - PostgreSQL 驱动（`postgres` feature）
- **rbdc-sqlite** - SQLite 驱动（`sqlite` feature）
- **rbdc-mssql** - MSSQL 驱动（`mssql` feature）
- **rbdc-oracle** - Oracle 驱动（`oracle` feature）
- **rbdc-tdengine** - TDengine 驱动（`tdengine` feature）
- **lazy_static** - 全局单例模式
- **tokio** - 异步运行时
- **salvo** - Web 框架（用于认证中间件）

## 与其他 Crate 集成

- **genies_auth**：使用 `CONTEXT.rbatis` 存储策略，使用 `salvo_auth` 进行 JWT 验证
- **genies_ddd**：使用 `CONTEXT.rbatis` 发布事件
- **genies_dapr**：使用 `CONTEXT.rbatis` 进行事务管理
- **genies_config**：为上下文初始化提供 `ApplicationConfig`

## 线程安全

- `CONTEXT`：`lazy_static` 确保单次初始化，字段是线程安全的
- `init_database()`：使用 `Once` 实现幂等初始化
- `REMOTE_TOKEN`：`Mutex<RemoteToken>` 实现线程安全访问
- `SERVICE_STATUS`：`Mutex<HashMap>` 实现线程安全状态更新

## 许可证

请参阅项目根目录的许可证信息。
