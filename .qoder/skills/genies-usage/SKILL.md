---
name: genies-usage
description: Guide for using the Genies Rust microservice framework with DDD and Dapr. Use when developing with Genies, creating aggregates, domain events, Dapr subscriptions, Casbin field-level permissions, configuration management, or when the user asks about Genies framework usage patterns.
---

# Genies 框架使用指南

## 1. 框架简介

**Genies** 是一个基于 Rust 的 DDD + Dapr 微服务开发框架（v1.4.x），通过宏驱动架构提供声明式的聚合根、领域事件、权限控制和配置管理能力。

## 2. 快速引用

```toml
[dependencies]
genies = "1.4"
genies_derive = "1.4"
rbatis = { version = "4.5", features = ["debug_mode"] }
tokio = { version = "1.22", features = ["full"] }
salvo = { version = "0.79", features = ["rustls", "oapi", "affix-state"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## 3. 核心宏速查表

| 宏 | 用途 | 示例 |
|---|---|---|
| `#[derive(Aggregate)]` | 定义 DDD 聚合根 | `#[derive(Aggregate)] struct Order {...}` |
| `#[derive(DomainEvent)]` | 定义领域事件 | `#[derive(DomainEvent)] struct OrderCreated {...}` |
| `#[topic(...)]` | 订阅 Dapr 事件 | `#[topic(name="...", pubsub="messagebus")]` |
| `#[derive(Config)]` | 定义配置结构 | `#[derive(Config)] struct MyConfig {...}` |
| `#[derive(ConfigCore)]` | 框架内部配置（避免循环依赖） | 同 Config |
| `#[casbin]` | 字段级权限控制 | `#[casbin] struct UserProfile {...}` |
| `#[wrapper]` | HTTP 调用包装（自动刷新 Token） | `#[wrapper] #[get("...")] async fn ...` |

## 4. 聚合根定义

使用 `#[derive(Aggregate)]` 定义 DDD 聚合根：

```rust
use genies_derive::Aggregate;
use serde::{Deserialize, Serialize};

#[derive(Aggregate, Debug, Serialize, Deserialize, Clone)]
#[aggregate_type("me.tdcarefor.order.domain.Order")]  // 可选：自定义类型名
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
- `#[aggregate_type("...")]` - 自定义聚合类型名称，默认为结构体名
- `#[id_field(field_name)]` - 必需，指定聚合 ID 字段
- `#[initialize_with_defaults]` - 自动实现 `InitializeAggregate` trait

**生成的 Traits：**
- `AggregateType` - 提供 `aggregate_type()` 和 `atype()` 方法
- `WithAggregateId` - 提供 `aggregate_id()` 方法

## 5. 领域事件

使用 `#[derive(DomainEvent)]` 标记领域事件：

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
    Created { order_id: String },
    #[event_type("OrderShipped")]
    Shipped { tracking_number: String },
}
```

**属性说明：**
- `#[event_type("...")]` - 事件类型标识
- `#[event_type_version("...")]` - 事件版本，默认 "V0"
- `#[event_source("...")]` - 事件来源（通常是聚合根全限定名）

## 6. 事件发布

使用 `DomainEventPublisher` 发布领域事件：

```rust
use genies::ddd::DomainEventPublisher::{publish, publishGenericDomainEvent};
use rbatis::executor::Executor;

// 发布聚合根关联的事件
async fn create_order(tx: &mut dyn Executor, order: &Order) {
    let event = OrderCreatedEvent {
        order_id: order.id.clone(),
        customer_id: order.customer_id.clone(),
        total_amount: order.total_amount,
    };
    publish(tx, order, Box::new(event)).await;
}

// 发布通用领域事件（无聚合根关联）
async fn send_notification(tx: &mut dyn Executor) {
    let event = NotificationEvent { message: "Hello".to_string() };
    publishGenericDomainEvent(tx, Box::new(event)).await;
}
```

**消息表结构（message）：**
- `id` - VARCHAR(36) 主键
- `destination` - VARCHAR(255) 目标
- `headers` - TEXT 消息头
- `payload` - TEXT 消息体
- `published` - INT 发布状态（0=未发布）
- `creation_time` - BIGINT 创建时间

## 7. 事件消费

使用 `#[topic]` 宏订阅 Dapr 事件：

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
    // 处理事件逻辑...
    Ok(0)
}
```

**`#[topic]` 参数：**
- `name` - Topic 名称，默认使用聚合类型名
- `pubsub` - PubSub 组件名，默认 "messagebus"
- `metadata` - 额外元数据，格式 "key1=value1,key2=value2"

**宏自动生成：**
- `{fn_name}_hoop` - Salvo Handler
- `{fn_name}_dapr` - Dapr 订阅配置
- `{fn_name}_hoop_router` - 路由注册
- 自动幂等性检查（基于 Redis）
- 自动事务管理和重试逻辑

**注册消费者路由：**
```rust
use genies::dapr::dapr_sub::dapr_sub;

pub fn event_router() -> Router {
    Router::new().push(
        Router::with_path("/daprsub/consumers")
            .hoop(on_order_created_hoop)
            .post(dapr_sub)
    )
}
```

## 8. 配置管理

使用 `#[derive(Config)]` 定义配置结构：

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

// 使用
let config = MyAppConfig::from_sources("./application.yml").unwrap();
```

**配置加载优先级：** 默认值 → YAML 文件 → 环境变量

**环境变量格式：** 支持 `field_name` 和 `FIELD_NAME` 两种格式

**生成的方法：**
- `from_file(path)` - 从文件加载
- `from_sources(path)` - 从多源加载（推荐）
- `validate()` - 验证配置
- `merge(other)` - 合并配置
- `load_env()` - 加载环境变量

## 9. 字段级权限

使用 `#[casbin]` 宏实现 Casbin 字段级访问控制：

```rust
use genies_derive::casbin;
use serde::Deserialize;
use salvo::oapi::ToSchema;

#[casbin]  // 必须放在最前面
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
async fn get_profile(depot: &mut Depot) -> Json<UserProfile> {
    let enforcer = depot.obtain::<Arc<Enforcer>>().unwrap();
    let current_user = "alice".to_string();
    
    let profile = UserProfile {
        id: 1,
        name: "张三".to_string(),
        email: "test@example.com".to_string(),
        phone: "13800138000".to_string(),
        credit_card: "1234-5678-9012-3456".to_string(),
        enforcer: None,
        subject: None,
    };
    
    Json(profile.with_policy(Arc::clone(&enforcer), current_user))
}
```

**宏自动生成：**
- `enforcer` 和 `subject` 字段（`#[serde(skip)]`）
- `with_policy(enforcer, subject)` 方法
- `check_permission(field)` 方法
- 自定义 `Serialize` 实现

**model.conf 配置：**
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

**policy.csv 示例：**
```csv
p, alice, UserProfile.email, read, deny
p, data_group_admin, data_group, read, deny
g, alice, data_group_admin
g2, UserProfile.credit_card, data_group
```

## 10. 缓存服务

使用 `CacheService` 进行缓存操作：

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
    cache.set_string_ex("session", "token", Some(Duration::from_secs(3600))).await?;
    
    // JSON 操作
    cache.set_json("user:1", &user).await?;
    let user: User = cache.get_json("user:1").await?;
    
    // 获取 TTL
    let ttl = cache.ttl("session").await?;
    Ok(())
}
```

**ICacheService 接口：**
- `set_string(k, v)` / `get_string(k)` / `del_string(k)`
- `set_string_ex(k, v, ex)` - 带过期时间
- `ttl(k)` - 获取剩余生存时间

**切换后端（application.yml）：**
```yaml
cache_type: "redis"  # 或 "mem"
redis_url: "redis://:password@localhost:6379"
```

## 11. HTTP 包装器

使用 `#[wrapper]` 宏包装跨微服务 HTTP 调用：

```rust
use genies_derive::wrapper;
use feignhttp::get;

#[wrapper]
#[get("${gateway}/user-service/api/users/{id}")]
pub async fn get_user_by_id(#[path] id: i64) -> feignhttp::Result<User> {}

// 使用时无需手动传递 Authorization header
async fn example() {
    let user = get_user_by_id(123).await.unwrap();
}
```

**功能：**
- 自动从 `REMOTE_TOKEN` 获取访问令牌
- 遇到 401 时自动刷新令牌并重试

**配置 gateway（使用 `config_gateway!` 宏）：**
```rust
lazy_static! {
    pub static ref GATEWAY: String = config_gateway!("/user");
}
```

## 12. 便捷宏

```rust
// pool!() - 获取数据库连接池
let rb = pool!();
User::select_by_id(rb, 1).await?;

// tx_defer!() - 获取带自动回滚的事务
let mut tx = tx_defer!();
User::insert(&mut tx, &user).await?;
tx.commit().await;  // 不 commit 会自动 rollback

// copy!(src, DestType) - 字段拷贝转换
let user_dto = copy!(&user_entity, UserDTO);

// config_gateway!(servlet_path) - 配置 gateway URL
// 如果 gateway 是 http/https 协议则使用网关，否则使用 Dapr
```

## 13. K8s 健康检查

```rust
use genies::k8s::k8s_health_check;
use salvo::Router;

let router = Router::new()
    .push(k8s_health_check());  // 自动添加健康检查路由
```

**端点：**
- `GET /actuator/health/liveness` - 存活探针
- `GET /actuator/health/readiness` - 就绪探针

**修改服务状态：**
```rust
use genies::context::SERVICE_STATUS;
use std::ops::DerefMut;

let mut status = SERVICE_STATUS.lock().unwrap();
status.deref_mut().insert("readinessProbe".to_string(), false);
```

## 14. 典型开发流程

1. **项目初始化**
   - 创建 Cargo.toml 添加依赖
   - 创建 application.yml 配置文件
   - 创建 model.conf 和 policy.csv（如需权限控制）

2. **定义聚合根**
   - 使用 `#[derive(Aggregate)]` 定义领域模型
   - 指定 `#[id_field]` 标识字段

3. **定义领域事件**
   - 使用 `#[derive(DomainEvent)]` 定义事件
   - 设置 `event_type`、`event_source`、`event_type_version`

4. **实现业务逻辑**
   - 使用 `tx_defer!()` 管理事务
   - 使用 `DomainEventPublisher::publish()` 发布事件

5. **订阅事件**
   - 使用 `#[topic]` 宏定义事件处理器
   - 注册到路由中

6. **启动服务**
   ```rust
   #[tokio::main]
   async fn main() {
       genies::config::log_config::init_log();
       CONTEXT.init_mysql().await;
       
       let router = Router::new()
           .push(k8s_health_check())
           .push(Router::with_path("/api")
               .hoop(genies::context::auth::salvo_auth)
               .push(business_router()));
       
       let acceptor = TcpListener::new(&CONTEXT.config.server_url).bind().await;
       Server::new(acceptor).serve(router).await;
   }
   ```

7. **部署**
   - 配置 Dapr sidecar
   - 设置 K8s 健康检查探针
   - 通过环境变量覆盖配置
