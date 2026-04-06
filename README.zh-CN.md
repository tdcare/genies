[English](README.md) | 中文

# Genies (神灯)

<p align="center">
  <strong>一个基于 Rust 的 DDD + Dapr 微服务开发框架</strong>
</p>

<p align="center">
  <a href="https://github.com/tdcare/genies"><img src="https://img.shields.io/badge/version-1.6.0-blue.svg" alt="version"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-edition%202021-orange.svg" alt="rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="license"></a>
</p>

---

## 目录

- [项目简介](#项目简介)
- [特性亮点](#特性亮点)
- [快速开始](#快速开始)
- [项目架构](#项目架构)
- [核心功能详解](#核心功能详解)
- [配置参考](#配置参考)
- [权限模型详解](#权限模型详解)
- [API 参考](#api-参考)
- [示例项目](#示例项目)
- [许可证](#许可证)

---

## 项目简介

**Genies (神灯)** 是一个专为 Rust 生态设计的微服务开发框架（v1.6.0），它将 **DDD（领域驱动设计）** 理念与 **Dapr 微服务运行时** 深度整合，同时保持与基于 **Eventuate** 的 Java 项目的兼容性。

框架通过 **宏驱动架构** 提供声明式的聚合根、领域事件、权限控制和配置管理能力，让开发者能够以最小的样板代码构建企业级微服务应用。

### 技术栈

| 组件 | 版本 | 用途 |
|------|------|------|
| **Rust** | Edition 2021 | 编程语言 |
| **Salvo** | 0.89 | Web 框架 |
| **RBatis** | 4.8 | ORM 框架 |
| **Tokio** | 1.22 | 异步运行时 |
| **Casbin** | 2.10 | 权限引擎 |
| **Redis** | - | 缓存服务 |
| **jsonwebtoken** | - | JWT 认证 |
| **Dapr** | - | 微服务运行时 |

### 设计理念

1. **DDD 优先** - 以聚合根和领域事件为核心构建业务逻辑
2. **宏驱动开发** - 通过过程宏减少样板代码，提升开发效率
3. **云原生适配** - 原生支持 Dapr、Kubernetes 健康检查
4. **Java 兼容** - 与 Eventuate 框架的消息格式完全兼容

---

## 特性亮点

- **声明式聚合根** - 使用 `#[derive(Aggregate)]` 快速定义 DDD 聚合根
- **领域事件驱动** - 使用 `#[derive(DomainEvent)]` 标记领域事件，自动实现事件类型识别
- **Dapr 发布/订阅** - 通过 `#[topic]` 宏实现事件消费，自动处理幂等性和重试逻辑
- **字段级权限控制** - 基于 Casbin 的 `#[casbin]` 宏实现细粒度的字段访问控制
- **灵活配置管理** - 使用 `#[derive(Config)]` 支持 YAML 配置文件和环境变量覆盖
- **双后端缓存** - 支持 Redis 和内存两种缓存后端，可通过配置切换
- **JWT 认证中间件** - 内置 Keycloak 集成的 JWT 验证
- **K8s 健康检查** - 开箱即用的存活/就绪探针
- **HTTP 包装器** - `#[remote]` 宏自动处理跨微服务调用的 Token 刷新

---

## 快速开始

### 环境要求

- **Rust** >= 1.70.0 (Edition 2021)
- **MySQL** >= 5.7 (用于事件存储)
- **Redis** >= 6.0 (可选，用于缓存)
- **Dapr** >= 1.10 (可选，用于发布/订阅)

### 安装依赖

使用 `cargo add` 命令添加依赖（将自动获取最新版本）：

```sh
# 主框架（包含所有子模块的重导出）
cargo add genies

# 过程宏库（如需单独使用宏）
cargo add genies_derive

# 必需的依赖
cargo add rbatis --features debug_mode
cargo add tokio --features full
cargo add salvo --features rustls,oapi,affix-state
cargo add serde --features derive
cargo add serde_json
```

您也可以手动在 `Cargo.toml` 中添加依赖，请前往 [crates.io](https://crates.io) 查看最新版本。

### 最小化示例

```rust
use genies::context::CONTEXT;
use genies::k8s::k8s_health_check;
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    // 初始化日志
    genies::config::log_config::init_log();
    
    // 初始化数据库连接
    CONTEXT.init_mysql().await;
    
    log::info!("Server starting at: http://{}", 
        CONTEXT.config.server_url.replace("0.0.0.0", "127.0.0.1"));
    
    // 构建路由
    let router = Router::new()
        .push(k8s_health_check())  // K8s 健康检查
        .push(Router::with_path("/api")
            .hoop(genies::context::auth::salvo_auth)  // JWT 认证中间件
            .get(hello));
    
    // 启动服务
    let acceptor = TcpListener::new(&CONTEXT.config.server_url).bind().await;
    Server::new(acceptor).serve(router).await;
}

#[handler]
async fn hello() -> &'static str {
    "Hello, Genies!"
}
```

### 配置文件

在项目根目录创建 `application.yml`：

```yaml
debug: true
server_name: "my-service"
servlet_path: "/api"
server_url: "0.0.0.0:8080"
database_url: "mysql://user:password@localhost:3306/mydb"
redis_url: "redis://localhost:6379"
cache_type: "redis"
log_level: "debug"
white_list_api:
  - "/actuator/*"
  - "/health/*"
```

---

## 项目架构

### 目录结构

```
genies/
├── Cargo.toml              # Workspace 配置
├── model.conf              # Casbin RBAC 模型配置
├── policy.csv              # Casbin 策略文件
├── crates/
│   ├── genies/             # 主框架聚合入口
│   ├── core/               # genies_core - 核心基础
│   ├── genies_derive/      # 过程宏库
│   ├── config/             # genies_config - 配置管理
│   ├── context/            # genies_context - 应用上下文
│   ├── cache/              # genies_cache - 缓存服务
│   ├── dapr/               # genies_dapr - Dapr 集成
│   ├── ddd/                # genies_ddd - DDD 核心
│   ├── k8s/                # genies_k8s - K8s 健康检查
│   └── auth/               # genies_auth - 权限示例
└── examples/
    └── topic/              # 事件订阅示例
```

### Crate 依赖关系

```
                    ┌─────────────────┐
                    │     genies      │  (主入口，重导出所有子 crate)
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│ genies_config │   │genies_context │   │  genies_ddd   │
│   配置管理    │   │  应用上下文   │   │   DDD 核心    │
└───────┬───────┘   └───────┬───────┘   └───────┬───────┘
        │                   │                   │
        │           ┌───────┴───────┐           │
        │           ▼               ▼           │
        │   ┌───────────────┐ ┌───────────────┐ │
        │   │ genies_cache  │ │  genies_dapr  │◄┘
        │   │   缓存服务    │ │  Dapr 集成    │
        │   └───────────────┘ └───────────────┘
        │
        ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│  genies_core  │   │genies_derive  │   │  genies_k8s   │
│   核心基础    │   │   过程宏库    │   │  K8s 探针     │
└───────────────┘   └───────────────┘   └───────────────┘
```

### 各 Crate 职责

| Crate | 职责 |
|-------|------|
| **genies** | 主框架聚合入口，重导出所有子 crate，提供便捷宏 `pool!`、`tx_defer!`、`copy!` |
| **genies_core** | 核心基础设施：错误处理、JWT 验证、HTTP 响应模型（`RespVO`、`ResultDTO`） |
| **genies_derive** | 过程宏库：`DomainEvent`、`Aggregate`、`Config`、`topic`、`remote`、`casbin` |
| **genies_config** | 配置管理：`ApplicationConfig`、日志配置，支持 YAML + 环境变量 |
| **genies_context** | 全局上下文（`CONTEXT`）、JWT 认证中间件、服务状态管理 |
| **genies_cache** | 缓存抽象层：`CacheService` 支持 Redis 和内存双后端 |
| **genies_dapr** | Dapr 集成：CloudEvent、发布/订阅、Topic 注册 |
| **genies_ddd** | DDD 核心：聚合根 Trait、领域事件 Trait、消息发布器 |
| **genies_k8s** | Kubernetes 探针：`/actuator/health/liveness` 和 `/actuator/health/readiness` |

---

## 核心功能详解

### 1. 聚合根定义 (`#[derive(Aggregate)]`)

聚合根是 DDD 中的核心概念，Genies (神灯) 通过 `Aggregate` 派生宏简化定义：

```rust
use genies_derive::Aggregate;
use serde::{Deserialize, Serialize};

#[derive(Aggregate, Debug, Serialize, Deserialize, Clone)]
#[aggregate_type("me.tdcarefor.order.domain.Order")]  // 可选：指定聚合类型名称
#[id_field(id)]                                       // 必需：指定 ID 字段
#[initialize_with_defaults]                           // 可选：启用默认值初始化
pub struct Order {
    pub id: String,
    pub customer_id: String,
    pub total_amount: f64,
    pub status: String,
}
```

**属性说明：**

| 属性 | 必需 | 说明 |
|------|------|------|
| `#[aggregate_type("...")]` | 否 | 自定义聚合类型名称，默认为结构体名 |
| `#[id_field(field_name)]` | 是 | 指定作为聚合 ID 的字段 |
| `#[initialize_with_defaults]` | 否 | 自动实现 `InitializeAggregate` trait |

**生成的 Trait 实现：**

```rust
impl AggregateType for Order {
    fn aggregate_type(&self) -> String { ... }
    fn atype() -> String { ... }
}

impl WithAggregateId for Order {
    type Id = String;
    fn aggregate_id(&self) -> &Self::Id { &self.id }
}
```

---

### 2. 领域事件 (`#[derive(DomainEvent)]`)

领域事件用于记录聚合根状态变化，支持 struct 和 enum 两种形式：

```rust
use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

// Struct 形式
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.order.domain.Order")]
#[event_type("me.tdcarefor.order.event.OrderCreated")]
pub struct OrderCreatedEvent {
    pub order_id: String,
    pub customer_id: String,
    pub total_amount: f64,
}

// Enum 形式（多事件类型）
#[derive(DomainEvent, Debug, Serialize, Deserialize, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.order.domain.Order")]
pub enum OrderEvent {
    #[event_type("OrderCreated")]
    Created { order_id: String, customer_id: String },
    
    #[event_type("OrderShipped")]
    Shipped { order_id: String, tracking_number: String },
    
    #[event_type("OrderCancelled")]
    Cancelled { order_id: String, reason: String },
}
```

**属性说明：**

| 属性 | 说明 |
|------|------|
| `#[event_type("...")]` | 事件类型标识，用于反序列化路由 |
| `#[event_type_version("...")]` | 事件版本，默认 "V0" |
| `#[event_source("...")]` | 事件来源（通常是聚合根全限定名） |

**生成的 Trait 实现：**

```rust
impl DomainEvent for OrderCreatedEvent {
    fn event_type_version(&self) -> String { "V1".to_string() }
    fn event_type(&self) -> String { "me.tdcarefor.order.event.OrderCreated".to_string() }
    fn event_source(&self) -> String { "me.tdcarefor.order.domain.Order".to_string() }
    fn json(&self) -> String { serde_json::to_string(self).unwrap() }
}
```

---

### 3. 事件发布与消费

#### 发布事件

使用 `DomainEventPublisher` 将事件保存到数据库的 `message` 表：

```rust
use genies::ddd::DomainEventPublisher::{publish, publishGenericDomainEvent};

// 发布聚合根关联的事件
async fn create_order(tx: &mut dyn Executor, order: &Order) -> Result<()> {
    let event = OrderCreatedEvent {
        order_id: order.id.clone(),
        customer_id: order.customer_id.clone(),
        total_amount: order.total_amount,
    };
    
    // 事件将关联到 Order 聚合根
    publish(tx, order, Box::new(event)).await;
    Ok(())
}

// 发布通用领域事件（无聚合根关联）
async fn send_notification(tx: &mut dyn Executor) -> Result<()> {
    let event = NotificationEvent { message: "Hello".to_string() };
    publishGenericDomainEvent(tx, Box::new(event)).await;
    Ok(())
}
```

**消息表结构：**

```sql
CREATE TABLE message (
    id VARCHAR(36) PRIMARY KEY,
    destination VARCHAR(255),
    headers TEXT,
    payload TEXT,
    published INT DEFAULT 0,
    creation_time BIGINT
);
```

#### 消费事件 (`#[topic]` 宏)

使用 `#[topic]` 宏订阅 Dapr 发布的事件：

```rust
use genies_derive::topic;
use rbatis::executor::Executor;

#[topic(
    name = "me.tdcarefor.order.domain.Order",  // 订阅的 topic 名称
    pubsub = "messagebus"                       // Dapr pubsub 组件名
)]
pub async fn on_order_created(
    tx: &mut dyn Executor,
    event: OrderCreatedEvent
) -> anyhow::Result<u64> {
    log::info!("收到订单创建事件: {:?}", event);
    
    // 处理事件...
    // 事务会自动管理：成功则 commit，失败则 rollback 并触发重试
    
    Ok(0)
}
```

**`#[topic]` 宏参数：**

| 参数 | 必需 | 说明 |
|------|------|------|
| `name` | 否 | Topic 名称，默认使用聚合类型名 |
| `pubsub` | 否 | Dapr PubSub 组件名，默认 "messagebus" |
| `metadata` | 否 | 额外元数据，格式 "key1=value1,key2=value2" |

**生成的代码：**

`#[topic]` 宏会自动生成：
1. Salvo Handler（`{fn_name}_hoop`）用于接收 Dapr 消息
2. Dapr 订阅配置函数（`{fn_name}_dapr`）
3. 路由注册函数（`{fn_name}_hoop_router`）
4. 自动幂等性检查（基于 Redis）
5. 自动事务管理和重试逻辑

#### 注册事件消费者

```rust
use genies::dapr::dapr_sub::dapr_sub;
use crate::listeners::{on_order_created_hoop, on_order_shipped_hoop};

pub fn event_consumer_router() -> Router {
    Router::new().push(
        Router::with_path("/daprsub/consumers")
            .hoop(on_order_created_hoop)    // 事件处理器中间件
            .hoop(on_order_shipped_hoop)
            .post(dapr_sub)                  // Dapr 响应处理
    )
}
```

---

### 4. 配置管理 (`#[derive(Config)]`)

使用 `Config` 宏定义支持 YAML 和环境变量的配置结构：

```rust
use genies_derive::Config;
use serde::Deserialize;

#[derive(Config, Debug, Deserialize)]
pub struct MyAppConfig {
    #[config(default = "my-service")]
    pub server_name: String,
    
    #[config(default = "8080")]
    pub port: u32,
    
    #[config(default = "")]
    pub api_key: Option<String>,
    
    #[config(default = "")]
    pub allowed_origins: Vec<String>,
}

// 使用配置
fn main() {
    let config = MyAppConfig::from_sources("./application.yml").unwrap();
    println!("Server: {}:{}", config.server_name, config.port);
}
```

**配置加载优先级：**

1. 默认值（`#[config(default = "...")]`）
2. YAML 配置文件
3. 环境变量（支持 `field_name` 和 `FIELD_NAME` 两种格式）

**生成的方法：**

```rust
impl MyAppConfig {
    pub fn from_file(path: &str) -> Result<Self, ConfigError>;
    pub fn from_sources(file_path: &str) -> Result<Self, ConfigError>;
    pub fn validate(&self) -> Result<(), ConfigError>;
    pub fn merge(&mut self, other: Self);
    pub fn load_env(&mut self) -> Result<(), ConfigError>;
}
```

---

### 5. 字段级权限控制 (`#[casbin]` 宏)

基于 Casbin 实现动态字段级别的访问控制：

```rust
use genies_derive::casbin;
use serde::Deserialize;
use salvo::oapi::ToSchema;

#[casbin]                           // 必须放在最前面
#[derive(Deserialize, ToSchema)]
pub struct UserProfile {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub credit_card: String,
}

// 在 Handler 中使用
#[endpoint]
async fn get_user_profile(
    req: &mut Request,
    depot: &mut Depot
) -> Json<UserProfile> {
    let enforcer = depot.obtain::<Arc<Enforcer>>().unwrap();
    let current_user = req.query::<String>("user").unwrap_or("guest".into());
    
    let profile = UserProfile {
        id: 1,
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
        phone: "13800138000".to_string(),
        credit_card: "1234-5678-9012-3456".to_string(),
        enforcer: None,   // 宏自动添加的字段
        subject: None,    // 宏自动添加的字段
    };
    
    // 应用权限策略
    Json(profile.with_policy(Arc::clone(&enforcer), current_user))
}
```

**`#[casbin]` 宏自动生成：**

1. `enforcer` 和 `subject` 字段（标记为 `#[serde(skip)]`）
2. `with_policy(enforcer, subject)` 方法
3. `check_permission(field)` 方法
4. 自定义 `Serialize` 实现（根据权限过滤字段）

---

### 6. 缓存服务

Genies (神灯) 提供统一的缓存抽象，支持 Redis 和内存两种后端：

```rust
use genies::context::CONTEXT;
use std::time::Duration;

async fn cache_example() -> Result<()> {
    let cache = &CONTEXT.cache_service;
    
    // 字符串操作
    cache.set_string("key1", "value1").await?;
    let value = cache.get_string("key1").await?;
    cache.del_string("key1").await?;
    
    // 带过期时间
    cache.set_string_ex("session", "token123", Some(Duration::from_secs(3600))).await?;
    
    // JSON 操作
    let user = User { id: 1, name: "test".to_string() };
    cache.set_json("user:1", &user).await?;
    let user: User = cache.get_json("user:1").await?;
    
    // 获取 TTL
    let ttl = cache.ttl("session").await?;
    
    Ok(())
}
```

**缓存接口 (`ICacheService`)：**

```rust
#[async_trait]
pub trait ICacheService: Sync + Send {
    async fn set_string(&self, k: &str, v: &str) -> Result<String>;
    async fn get_string(&self, k: &str) -> Result<String>;
    async fn del_string(&self, k: &str) -> Result<String>;
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String>;
    async fn ttl(&self, k: &str) -> Result<i64>;
}
```

**切换缓存后端：**

在 `application.yml` 中配置：

```yaml
cache_type: "redis"  # 或 "mem" 使用内存缓存
redis_url: "redis://:password@localhost:6379"
```

---

### 7. HTTP 包装器 (`#[remote]` 宏)

用于包装跨微服务 HTTP 调用，自动处理 Token 刷新：

```rust
use genies_derive::remote;
use feignhttp::get;

#[remote]
#[get("${gateway}/user-service/api/users/{id}")]
pub async fn get_user_by_id(#[path] id: i64) -> feignhttp::Result<User> {}

// 使用时无需手动传递 Authorization header
async fn example() {
    let user = get_user_by_id(123).await.unwrap();
}
```

**`#[remote]` 宏功能：**

1. 自动从 `REMOTE_TOKEN` 获取访问令牌
2. 当遇到 401 错误时，自动刷新令牌并重试
3. 与 feignhttp 宏无缝配合

---

### 8. Kubernetes 健康检查

Genies (神灯) 内置 K8s 就绪和存活探针：

```rust
use genies::k8s::k8s_health_check;
use salvo::Router;

let router = Router::new()
    .push(k8s_health_check());  // 自动添加健康检查路由

// 提供的端点：
// GET /actuator/health/liveness  - 存活探针
// GET /actuator/health/readiness - 就绪探针
```

**修改服务状态：**

```rust
use genies::context::SERVICE_STATUS;
use std::ops::DerefMut;

fn set_not_ready() {
    let mut status = SERVICE_STATUS.lock().unwrap();
    let map = status.deref_mut();
    map.insert("readinessProbe".to_string(), false);
}
```

---

## 配置参考

### ApplicationConfig 完整配置项

| 配置项 | 类型 | 说明 | 示例值 |
|--------|------|------|--------|
| `debug` | bool | 调试模式 | `true` |
| `server_name` | String | 微服务名称 | `"my-service"` |
| `servlet_path` | String | 服务路由前缀 | `"/api"` |
| `server_url` | String | 服务监听地址 | `"0.0.0.0:8080"` |
| `gateway` | Option<String> | 网关地址（HTTP 协议）或 Dapr 模式 | `"http://gateway:8080"` |
| `redis_url` | String | Redis 缓存地址 | `"redis://:pwd@localhost:6379"` |
| `redis_save_url` | String | 持久化 Redis 地址 | `"redis://:pwd@localhost:6380"` |
| `database_url` | String | 数据库连接串 | `"mysql://user:pwd@localhost/db"` |
| `max_connections` | u32 | 最大连接数 | `20` |
| `min_connections` | u32 | 最小连接数 | `0` |
| `wait_timeout` | u64 | 连接等待超时(秒) | `60` |
| `create_timeout` | u64 | 创建连接超时(秒) | `120` |
| `max_lifetime` | u64 | 连接最大生命周期(秒) | `1800` |
| `log_level` | String | 日志级别 | `"debug,sqlx=warn"` |
| `white_list_api` | Vec<String> | 免认证白名单 | `["/health/*"]` |
| `cache_type` | String | 缓存类型 | `"redis"` 或 `"mem"` |
| `keycloak_auth_server_url` | String | Keycloak 服务地址 | `"http://keycloak/auth/"` |
| `keycloak_realm` | String | Keycloak Realm | `"myrealm"` |
| `keycloak_resource` | String | Keycloak Client ID | `"myclient"` |
| `keycloak_credentials_secret` | String | Keycloak Client Secret | `"xxx-xxx-xxx"` |
| `dapr_pubsub_name` | String | Dapr PubSub 组件名 | `"messagebus"` |
| `dapr_pub_message_limit` | i64 | 每次发布消息数量限制 | `50` |
| `dapr_cdc_message_period` | i64 | CDC 消息轮询周期(毫秒) | `5000` |
| `processing_expire_seconds` | i64 | 消息处理超时(秒) | `60` |
| `record_reserve_minutes` | i64 | 消息记录保留时间(分钟) | `10080` |

### application.yml 完整示例

```yaml
# 基础配置
debug: true
server_name: "order-service"
servlet_path: "/order"
server_url: "0.0.0.0:8080"

# 网关配置（HTTP 协议使用网关，否则使用 Dapr）
gateway: "http://api-gateway:8080"

# 缓存配置
cache_type: "redis"
redis_url: "redis://:password@redis:6379"
redis_save_url: "redis://:password@redis-persistent:6379"

# 数据库配置
database_url: "mysql://root:password@mysql:3306/order_db?serverTimezone=Asia/Shanghai"
max_connections: 20
min_connections: 2
wait_timeout: 60
create_timeout: 120
max_lifetime: 1800

# 日志配置
log_level: "debug,sqlx=warn,hyper=info"

# Keycloak 认证配置
keycloak_auth_server_url: "http://keycloak:8080/auth/"
keycloak_realm: "myrealm"
keycloak_resource: "order-service"
keycloak_credentials_secret: "your-client-secret"

# Dapr 配置
dapr_pubsub_name: "messagebus"
dapr_pub_message_limit: 50
dapr_cdc_message_period: 5000
processing_expire_seconds: 60
record_reserve_minutes: 10080

# 白名单接口（免登录）
white_list_api:
  - "/"
  - "/actuator/*"
  - "/dapr/*"
  - "/swagger-ui/*"
  - "/api-doc/*"
```

### 环境变量覆盖

支持两种格式覆盖配置：

```bash
# 原字段名格式
export database_url="mysql://prod:password@prod-db:3306/db"

# 大写下划线格式
export DATABASE_URL="mysql://prod:password@prod-db:3306/db"
export REDIS_URL="redis://:pwd@prod-redis:6379"
export LOG_LEVEL="info"
```

---

## 权限模型详解

### Casbin RBAC 模型

Genies (神灯) 使用 Casbin 实现字段级权限控制，配置文件 `model.conf`：

```ini
[request_definition]
r = sub, obj, act

[policy_definition]
p = sub, obj, act, eft

[role_definition]
g = _, _      # 用户-角色映射
g2 = _, _     # 资源-资源组映射

[policy_effect]
e = !some(where (p.eft == deny))

[matchers]
m = g(r.sub, p.sub) && g2(r.obj, p.obj) && r.act == p.act
```

**模型解读：**

| 组件 | 说明 |
|------|------|
| `r = sub, obj, act` | 请求定义：主体、对象、动作 |
| `p = sub, obj, act, eft` | 策略定义：包含效果（allow/deny） |
| `g = _, _` | 用户继承角色 |
| `g2 = _, _` | 资源继承资源组 |
| `e = !some(where (p.eft == deny))` | 默认允许，有 deny 则拒绝 |

### 策略文件格式

`policy.csv` 示例：

```csv
# 直接授权：用户 alice 不能读取 UserProfile.email
p, alice, genies_auth.vo.UserProfile.email, read, deny

# 直接授权：用户 bob 可以读取 User.email
p, bob, genies_auth.vo.User.email, read, allow

# 用户 bob 不能读取 UserProfile.email
p, bob, genies_auth.vo.UserProfile.email, read, deny

# 角色定义：data_group_admin 角色不能读取 data_group 组
p, data_group_admin, data_group, read, deny

# 用户-角色映射：alice 属于 data_group_admin 角色
g, alice, data_group_admin

# 资源-资源组映射：这些字段属于 data_group 组
g2, genies_auth.vo.UserProfile.credit_card, data_group
g2, genies_auth.vo.UserProfile.name, data_group
g2, genies_auth.vo.User.phone, data_group
```

### 字段级权限工作原理

1. `#[casbin]` 宏修改结构体，添加 `enforcer` 和 `subject` 字段
2. 自定义 `Serialize` 实现在序列化每个字段前调用 `check_permission`
3. `check_permission` 构造请求 `(subject, "StructName.field_name", "read")`
4. Casbin Enforcer 根据策略决定是否序列化该字段
5. 被拒绝的字段不会出现在 JSON 输出中

---

## API 参考

### 核心 Traits

#### `DomainEvent` (genies::ddd::event)

```rust
pub trait DomainEvent: Send {
    fn event_type_version(&self) -> String;  // 事件版本
    fn event_type(&self) -> String;          // 事件类型标识
    fn event_source(&self) -> String;        // 事件来源
    fn json(&self) -> String;                // 序列化为 JSON
}
```

#### `AggregateType` (genies::ddd::aggregate)

```rust
pub trait AggregateType {
    fn aggregate_type(&self) -> String;  // 获取聚合类型名
    fn atype() -> String;                // 静态方法获取类型名
}
```

#### `WithAggregateId` (genies::ddd::aggregate)

```rust
pub trait WithAggregateId {
    type Id: Debug + Clone + PartialEq + Serialize + DeserializeOwned;
    fn aggregate_id(&self) -> &Self::Id;  // 获取聚合 ID
}
```

#### `ICacheService` (genies::cache::cache_service)

```rust
#[async_trait]
pub trait ICacheService: Sync + Send {
    async fn set_string(&self, k: &str, v: &str) -> Result<String>;
    async fn get_string(&self, k: &str) -> Result<String>;
    async fn del_string(&self, k: &str) -> Result<String>;
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String>;
    async fn ttl(&self, k: &str) -> Result<i64>;
}
```

### 核心结构体

#### `RespVO<T>` (genies::core)

标准 HTTP 响应模型：

```rust
pub struct RespVO<T> {
    pub code: Option<String>,   // "SUCCESS" 或 "FAIL"
    pub msg: Option<String>,    // 错误消息
    pub data: Option<T>,        // 响应数据
}

impl<T> RespVO<T> {
    pub fn from_result(arg: &Result<T>) -> Self;
    pub fn from(arg: &T) -> Self;
    pub fn from_error(code: &str, arg: &Error) -> Self;
    pub fn from_error_info(code: &str, info: &str) -> Self;
}
```

#### `ResultDTO<T>` (genies::core)

兼容 Java 的响应模型：

```rust
pub struct ResultDTO<T> {
    pub status: Option<i32>,    // 1=成功, 0=失败
    pub message: Option<String>,
    pub data: Option<T>,
}

impl<T> ResultDTO<T> {
    pub fn success(message: &str, data: T) -> Self;
    pub fn error(message: &str) -> Self;
    pub fn success_empty(message: &str) -> ResultDTO<()>;
}
```

### 便捷宏

#### `pool!()` (genies)

获取数据库连接池：

```rust
let rb = pool!();
User::select_by_id(rb, 1).await?;
```

#### `tx_defer!()` (genies)

获取带自动回滚的事务：

```rust
let mut tx = tx_defer!();
User::insert(&mut tx, &user).await?;
tx.commit().await;  // 不 commit 会自动 rollback
```

#### `copy!(src, DestType)` (genies)

字段拷贝转换：

```rust
let user_dto = copy!(&user_entity, UserDTO);
```

---

## 示例项目

项目包含完整的示例代码，位于 `examples/` 目录：

### topic 示例

演示事件发布与订阅功能：

```
examples/topic/
├── Cargo.toml
├── application.yml        # 配置文件
└── src/
    ├── main.rs            # 服务入口
    ├── lib.rs             # 事件消费者路由配置
    ├── DeviceUseEvent.rs  # 领域事件定义
    └── UseDeviceListeners.rs  # 事件处理器
```

**运行示例：**

```bash
cd examples/topic
cargo run
```

**示例代码片段：**

```rust
// DeviceUseEvent.rs - 定义领域事件
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V2")]
#[event_source("me.tdcarefor.tdnis.device.domain.DeptDeviceEntity")]
#[event_type("me.tdcarefor.tdnis.device.event.DeviceUseEvent")]
pub struct DeviceUseEvent {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub deviceNo: Option<String>,
}

// UseDeviceListeners.rs - 事件消费者
#[topic(
    name = "me.tdcarefor.tdnis.device.domain.DeptDeviceEntity",
    pubsub = "messagebus"
)]
pub async fn onDeviceUseEvent(
    tx: &mut dyn Executor,
    event: DeviceUseEvent
) -> anyhow::Result<u64> {
    log::info!("处理设备使用事件: {:?}", event);
    Ok(0)
}
```

---

## 许可证

本项目基于 **MIT License** 开源。

```
MIT License

Copyright (c) tdcare

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

<p align="center">
  <sub>Built with ❤️ by <a href="https://github.com/tdcare">tdcare</a></sub>
</p>
