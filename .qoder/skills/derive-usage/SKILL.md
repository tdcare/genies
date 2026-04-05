---
name: derive-usage
description: Guide for using genies_derive procedural macros. Use when implementing DDD aggregates with #[derive(Aggregate)], domain events with #[derive(DomainEvent)], configuration loading with #[derive(Config)], Dapr topic consumption with #[topic], HTTP request wrapping with #[remote], or field-level permission control with #[casbin].
---

# Procedural Macros (genies_derive)

## Overview

genies_derive 提供 7 个核心过程宏，用于简化 DDD + Dapr + Casbin 应用开发。纯过程宏库，无运行时依赖。

**核心宏:**
- `#[derive(Aggregate)]` - 聚合根派生，实现 AggregateType/WithAggregateId/InitializeAggregate
- `#[derive(DomainEvent)]` - 领域事件派生，实现 DomainEvent trait
- `#[derive(Config)]` - 配置派生，支持 YAML + ENV 加载
- `#[derive(ConfigCore)]` - 内部配置派生（避免循环依赖）
- `#[topic(...)]` - Dapr topic 消费，Redis 幂等
- `#[remote(...)]` - HTTP 请求包装，JWT 自动刷新
- `#[casbin]` - 字段级权限控制，自动嵌套检测

## Dependencies

```toml
[dependencies]
genies_derive = { workspace = true }
genies = { workspace = true }  # for runtime support
```

## Macro 1: Aggregate

聚合根派生，实现 DDD 聚合类型标识和 ID 访问。

### Attributes

| Attribute | Required | Description |
|-----------|----------|-------------|
| `#[aggregate_type("Name")]` | No | 覆盖聚合类型名（默认：struct 名） |
| `#[id_field(field)]` | No | 指定 ID 字段，生成 WithAggregateId |
| `#[initialize_with_defaults]` | No | 生成 InitializeAggregate（需要 id_field） |

### Example

```rust
use genies_derive::Aggregate;
use serde::{Deserialize, Serialize};

#[derive(Aggregate, Serialize, Deserialize, Default)]
#[aggregate_type("Order")]
#[id_field(id)]
#[initialize_with_defaults]
pub struct Order {
    pub id: String,
    pub status: String,
    pub total: f64,
}

// Generated:
// impl AggregateType for Order {
//     fn aggregate_type(&self) -> String { "Order".to_string() }
//     fn atype() -> String { "Order".to_string() }
// }
// impl WithAggregateId for Order {
//     type Id = String;
//     fn aggregate_id(&self) -> &Self::Id { &self.id }
// }
// impl InitializeAggregate for Order {
//     fn initialize(id: String) -> Self {
//         Self { id, status: Default::default(), total: Default::default() }
//     }
// }
```

## Macro 2: DomainEvent

领域事件派生，支持 struct 和 enum。

### Attributes

| Attribute | Required | Description |
|-----------|----------|-------------|
| `#[event_type("Name")]` | No | 事件类型名（默认：struct/variant 名） |
| `#[event_type_version("V1")]` | No | 版本（默认："V0"） |
| `#[event_source("service")]` | No | 来源标识（默认：""） |

### Struct Example

```rust
use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

#[derive(DomainEvent, Serialize, Deserialize, Default)]
#[event_type("OrderCreated")]
#[event_type_version("V1")]
#[event_source("order-service")]
pub struct OrderCreatedEvent {
    pub order_id: String,
    pub customer_id: String,
}

// Generated: DomainEvent trait
// fn event_type(&self) -> String { "OrderCreated".to_string() }
// fn event_type_version(&self) -> String { "V1".to_string() }
// fn event_source(&self) -> String { "order-service".to_string() }
// fn json(&self) -> String { serde_json::to_string(self).unwrap() }
```

### Enum Example

```rust
#[derive(DomainEvent, Serialize, Deserialize)]
#[event_type_version("V1")]
pub enum OrderEvent {
    #[event_type("OrderCreated")]
    Created { order_id: String },
    
    #[event_type("OrderShipped")]
    Shipped { tracking: String },
}
```

## Macro 3: Config

配置派生，从 YAML + 环境变量加载。

### Field Attribute

| Attribute | Description |
|-----------|-------------|
| `#[config(default = "value")]` | 默认值 |

### Example

```rust
use genies_derive::Config;
use serde::{Deserialize, Serialize};

#[derive(Config, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    #[config(default = "localhost")]
    pub host: String,
    
    #[config(default = "8080")]
    pub port: u16,
    
    #[config(default = "t1,t2")]
    pub topics: Vec<String>,
    
    pub db_url: Option<String>,
}

// Usage:
let config = AppConfig::from_sources("application.yml")?;
```

### Generated Methods

- `default()` - 使用默认值创建
- `from_file(path)` - 从 YAML 加载
- `from_sources(path)` - YAML + ENV 综合加载（推荐）
- `load_env(&mut self)` - 从环境变量覆盖
- `merge(&mut self, other)` - 合并配置
- `validate(&self)` - 验证配置

### ENV Support

```bash
# 支持两种格式
export host="prod.example.com"   # 原名
export HOST="prod.example.com"    # SCREAMING_SNAKE
```

## Macro 4: ConfigCore

与 Config 相同，但使用 `genies_core::error::ConfigError`。用于框架内部避免循环依赖。

## Macro 5: #[topic]

Dapr topic 消费宏，自动生成 Salvo handler + Redis 幂等。

### Attributes

| Attribute | Required | Default | Description |
|-----------|----------|---------|-------------|
| `name = "..."` | No | aggregate type | Topic 名称 |
| `pubsub = "..."` | No | "messagebus" | PubSub 组件名 |
| `metadata = "k=v,..."` | No | - | Topic 元数据 |

### Example

```rust
use genies_derive::topic;
use rbatis::executor::Executor;

#[topic(name = "order-events", pubsub = "messagebus")]
pub async fn handle_order_created(
    tx: &mut dyn Executor,
    event: OrderCreatedEvent,
) -> anyhow::Result<u64> {
    Order::insert(tx, &order).await?;
    Ok(1)
}
```

### Generated Functions

1. `handle_order_created(tx, event)` - 业务逻辑
2. `handle_order_created_hoop` - Salvo handler（`#[handler]`）
3. `handle_order_created_dapr()` - 返回 DaprTopicSubscription
4. `handle_order_created_hoop_router()` - Salvo Router

### Idempotency Flow

```
Message → CloudEvent 解析 → event_type 匹配
    ↓
Redis key: {server}-{handler}-{event_type}-{id}
    ↓
SET NX (原子) → 处理 → SET CONSUMED
    ↓
失败自动 rollback，删除 key，Dapr 重发
```

## Macro 6: #[remote]

feignhttp 请求包装，401 时自动刷新 JWT。

### Example

```rust
use genies_derive::remote;
use feignhttp::get;

#[remote]
#[get("${GATEWAY}/api/patients/{id}")]
pub async fn get_patient(#[path] id: String) -> Result<Patient, Error> {
    // feignhttp body
}
```

### Generated

```rust
// 原函数 + Authorization header
pub async fn get_patient_feignhttp(
    #[header] Authorization: &str,
    #[path] id: String
) -> Result<Patient, Error>

// 包装函数（自动 token 管理）
pub async fn get_patient(id: String) -> Result<Patient, Error> {
    // 1. 使用 REMOTE_TOKEN 发起请求
    // 2. 如果 401，从 Keycloak 刷新 token
    // 3. 重试请求
}
```

## Macro 7: #[casbin]

字段级权限控制，自动生成 Serialize + Writer。

### Usage

```rust
use genies_derive::casbin;
use serde::Deserialize;
use salvo::oapi::ToSchema;

#[casbin]
#[derive(Deserialize, ToSchema)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,       // 可被 deny
    pub address: Address,    // 自动嵌套检测
    pub accounts: Vec<BankAccount>,  // Vec 自动检测
}
```

### Auto Nested Detection

宏自动识别非原始类型：
- `Address` → 递归过滤
- `Option<Address>` → 非 null 时递归
- `Vec<BankAccount>` → 遍历每项递归

### Generated

```rust
impl User {
    pub fn casbin_filter(
        value: &mut serde_json::Value,
        enforcer: &casbin::Enforcer,
        subject: &str,
    ) { ... }
}

impl salvo::writing::Writer for User {
    // 从 Depot 提取 enforcer/subject，过滤后渲染
}
```

### Policy Examples

```sql
-- 禁止 bob 看 email
INSERT INTO casbin_rules (ptype,v0,v1,v2,v3)
VALUES ('p','bob','User.email','read','deny');

-- 禁止 guest 看 phone
INSERT INTO casbin_rules (ptype,v0,v1,v2,v3)
VALUES ('p','guest','User.phone','read','deny');
```

## Debug Mode

启用 debug_mode 打印生成代码：

```toml
[dependencies]
genies_derive = { path = "...", features = ["debug_mode"] }
```

## Key Files

- [crates/genies_derive/src/lib.rs](file:///d:/tdcare/genies/crates/genies_derive/src/lib.rs) - 宏入口
- [crates/genies_derive/src/aggregate_type.rs](file:///d:/tdcare/genies/crates/genies_derive/src/aggregate_type.rs) - Aggregate 实现
- [crates/genies_derive/src/event_type.rs](file:///d:/tdcare/genies/crates/genies_derive/src/event_type.rs) - DomainEvent 实现
- [crates/genies_derive/src/config_type.rs](file:///d:/tdcare/genies/crates/genies_derive/src/config_type.rs) - Config 实现
- [crates/genies_derive/src/topic.rs](file:///d:/tdcare/genies/crates/genies_derive/src/topic.rs) - topic 实现
- [crates/genies_derive/src/remote.rs](file:///d:/tdcare/genies/crates/genies_derive/src/remote.rs) - remote 实现
- [crates/genies_derive/src/casbin.rs](file:///d:/tdcare/genies/crates/genies_derive/src/casbin.rs) - casbin 实现
