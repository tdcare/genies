# genies_derive

Genies (神灯) 框架的过程宏库，为 DDD 聚合根、领域事件、配置管理、Dapr 事件消费、HTTP 请求包装和字段级权限控制提供派生宏和属性宏。

## 概述

genies_derive 提供 7 个核心宏，简化 DDD + Dapr + Casbin 应用中的常见模式：

| 宏 | 类型 | 用途 |
|---|------|------|
| `#[derive(Aggregate)]` | 派生宏 | DDD 聚合根实现 |
| `#[derive(DomainEvent)]` | 派生宏 | 领域事件类型实现 |
| `#[derive(Config)]` | 派生宏 | 从 YAML/环境变量加载配置 |
| `#[derive(ConfigCore)]` | 派生宏 | 内部配置（避免循环依赖） |
| `#[topic(...)]` | 属性宏 | Dapr topic 消费，支持 Redis 幂等 |
| `#[remote(...)]` | 属性宏 | HTTP 请求包装，支持 JWT 自动刷新 |
| `#[casbin]` | 属性宏 | 字段级权限控制 |

## 快速开始

```sh
cargo add genies_derive genies
```

> 也可以手动在 `Cargo.toml` 中添加依赖，请前往 [crates.io](https://crates.io) 查看最新版本。

## 宏参考

### 1. `#[derive(Aggregate)]` - 聚合根

实现 `AggregateType`、`WithAggregateId` 以及可选的 `InitializeAggregate` trait。

**属性说明：**

| 属性 | 必需 | 说明 |
|------|------|------|
| `#[aggregate_type("Name")]` | 否 | 覆盖聚合类型名（默认：结构体名） |
| `#[id_field(field_name)]` | 否 | 指定 ID 字段 |
| `#[initialize_with_defaults]` | 否 | 生成 `InitializeAggregate` 实现（需配合 `id_field`） |

**示例：**

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
    pub total_amount: f64,
    pub items: Vec<OrderItem>,
}

// 生成的 trait:
// - AggregateType::aggregate_type(&self) -> String  // 返回 "Order"
// - AggregateType::atype() -> String                // 静态版本
// - WithAggregateId::aggregate_id(&self) -> &String
// - InitializeAggregate::initialize(id: String) -> Order
```

### 2. `#[derive(DomainEvent)]` - 领域事件

为事件溯源模式实现 `DomainEvent` trait。

**属性说明：**

| 属性 | 必需 | 说明 |
|------|------|------|
| `#[event_type("Name")]` | 否 | 覆盖事件类型（默认：结构体/枚举变体名） |
| `#[event_type_version("V1")]` | 否 | 事件版本（默认："V0"） |
| `#[event_source("service")]` | 否 | 事件来源标识（默认：""） |

**结构体示例：**

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
    pub total: f64,
}

// 生成: DomainEvent trait，包含 event_type()、event_type_version()、event_source()、json()
```

**枚举示例：**

```rust
#[derive(DomainEvent, Serialize, Deserialize)]
#[event_type_version("V1")]
pub enum OrderEvent {
    #[event_type("OrderCreated")]
    Created { order_id: String },
    
    #[event_type("OrderShipped")]
    Shipped { tracking_number: String },
}
```

### 3. `#[derive(Config)]` - 配置

生成从 YAML 文件和环境变量加载配置的方法。

**字段属性：**

| 属性 | 说明 |
|------|------|
| `#[config(default = "value")]` | 字段默认值 |

**示例：**

```rust
use genies_derive::Config;
use serde::{Deserialize, Serialize};

#[derive(Config, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    #[config(default = "localhost")]
    pub host: String,
    
    #[config(default = "8080")]
    pub port: u16,
    
    #[config(default = "topic1,topic2")]
    pub topics: Vec<String>,
    
    pub database_url: Option<String>,
}

// 生成的方法:
// - AppConfig::default() -> Self
// - AppConfig::from_file(path: &str) -> Result<Self, ConfigError>
// - AppConfig::from_sources(path: &str) -> Result<Self, ConfigError>
// - AppConfig::load_env(&mut self) -> Result<(), ConfigError>
// - AppConfig::merge(&mut self, other: Self)
// - AppConfig::validate(&self) -> Result<(), ConfigError>
```

**环境变量支持：**

```bash
# 支持两种格式:
export host="production.example.com"     # 小写
export HOST="production.example.com"      # SCREAMING_SNAKE_CASE
```

### 4. `#[derive(ConfigCore)]` - 内部配置

与 `Config` 相同，但使用 `genies_core::error::ConfigError` 而非 `genies::core::error::ConfigError`。用于内部以避免循环依赖。

### 5. `#[topic(...)]` - Dapr Topic 消费

将异步函数转换为 Dapr topic 消费者，支持基于 Redis 的幂等性。

**属性说明：**

| 属性 | 必需 | 说明 |
|------|------|------|
| `name = "topic_name"` | 否 | Topic 名称（默认：聚合类型） |
| `pubsub = "pubsub_name"` | 否 | PubSub 组件（默认："messagebus"） |
| `metadata = "k1=v1,k2=v2"` | 否 | Topic 元数据 |

**示例：**

```rust
use genies_derive::topic;
use rbatis::executor::Executor;

#[topic(name = "order-events", pubsub = "messagebus")]
pub async fn handle_order_created(
    tx: &mut dyn Executor,
    event: OrderCreatedEvent,
) -> anyhow::Result<u64> {
    // 处理事件
    Order::insert(tx, &order).await?;
    Ok(1)
}
```

**生成的代码：**

1. **处理函数**: `handle_order_created(tx, event)` - 你的业务逻辑
2. **Salvo 处理器**: `handle_order_created_hoop` - Dapr 的 HTTP 处理器
3. **Dapr 配置**: `handle_order_created_dapr()` - 返回 `DaprTopicSubscription`
4. **路由**: `handle_order_created_hoop_router()` - Salvo 路由
5. **自动注册**: 通过 `inventory::submit!`

**幂等性流程：**

```
Dapr 消息 → 解析 CloudEvent → 提取 event_type
     ↓
检查 Redis key: {server}-{handler}-{event_type}-{message_id}
     ↓
如果 CONSUMING: 稍后重试
如果 CONSUMED: 跳过
如果不存在: SET NX（原子操作）→ 处理 → SET CONSUMED
```

### 6. `#[remote(...)]` - HTTP 请求包装

包装 feignhttp 请求，在 401 错误时自动刷新 JWT token。

**示例：**

```rust
use genies_derive::remote;
use feignhttp::get;

#[remote]
#[get("${GATEWAY}/api/patients/{id}")]
pub async fn get_patient(#[path] id: String) -> Result<Patient, Error> {
    // feignhttp 实现
}
```

**生成的代码：**

```rust
// 带 Authorization header 的原函数
pub async fn get_patient_feignhttp(
    #[header] Authorization: &str,
    #[path] id: String
) -> Result<Patient, Error> { ... }

// 带自动 token 刷新的包装函数
pub async fn get_patient(id: String) -> Result<Patient, Error> {
    let bearer = format!("Bearer {}", REMOTE_TOKEN.lock().unwrap().access_token);
    let result = get_patient_feignhttp(&bearer, id).await;
    
    if result.is_err() && error.contains("401 Unauthorized") {
        // 从 Keycloak 刷新 token
        let new_token = get_temp_access_token(...).await?;
        REMOTE_TOKEN.lock().unwrap().access_token = new_token;
        return get_patient_feignhttp(&new_bearer, id).await;
    }
    result
}
```

### 7. `#[casbin]` - 字段级权限控制

生成自定义 `Serialize` 和 Salvo `Writer` 实现，用于字段级权限过滤。

**示例：**

```rust
use genies_derive::casbin;
use serde::Deserialize;
use salvo::oapi::ToSchema;

#[casbin]
#[derive(Deserialize, ToSchema)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,      // 可被策略过滤
    pub phone: String,      // 可被策略过滤
    pub address: Address,   // 嵌套类型 - 自动检测
    pub accounts: Vec<BankAccount>,  // Vec 类型 - 自动检测
}
```

**自动嵌套检测：**

宏自动检测非原始类型并递归过滤：
- 普通结构体字段：`Address`
- Option 包装：`Option<Address>`
- Vec 包装：`Vec<BankAccount>`

**生成的代码：**

```rust
impl User {
    pub fn casbin_filter(
        value: &mut serde_json::Value,
        enforcer: &casbin::Enforcer,
        subject: &str,
    ) {
        // 1. 过滤自身字段
        // 2. 递归过滤嵌套字段
    }
}

#[async_trait]
impl salvo::writing::Writer for User {
    async fn write(self, req, depot, res) {
        let enforcer = depot.get::<Arc<Enforcer>>("casbin_enforcer");
        let subject = depot.get::<String>("casbin_subject");
        
        let mut value = serde_json::to_value(&self)?;
        Self::casbin_filter(&mut value, enforcer, subject);
        res.render(Json(value));
    }
}
```

**策略示例：**

```sql
-- 禁止 bob 读取 User.email 字段
INSERT INTO casbin_rules (ptype, v0, v1, v2, v3) 
VALUES ('p', 'bob', 'User.email', 'read', 'deny');

-- 禁止 guest 角色读取 phone 字段
INSERT INTO casbin_rules (ptype, v0, v1, v2, v3) 
VALUES ('p', 'guest', 'User.phone', 'read', 'deny');
```

## 依赖项

- **proc-macro2** / **quote** / **syn** - 过程宏基础设施
- **genies_core** - ConfigCore 的错误类型
- **serde** / **serde_yaml** - 配置解析
- **convert_case** - 环境变量名转换
- **async-trait** - 异步 trait 支持

## 与其他 Crate 的集成

| 宏 | 集成对象 |
|---|---------|
| `Aggregate` | `genies_ddd::aggregate` traits |
| `DomainEvent` | `genies_ddd::event` traits |
| `Config` / `ConfigCore` | `genies_core::error::ConfigError` |
| `#[topic]` | `genies_dapr`、`genies_context::CONTEXT`、Redis |
| `#[remote]` | `genies_core::jwt`、`genies_context::REMOTE_TOKEN` |
| `#[casbin]` | `genies_auth`、`casbin::Enforcer`、Salvo |

## 调试模式

启用 `debug_mode` 特性可在编译时打印生成的代码：

```toml
[dependencies]
genies_derive = { path = "...", features = ["debug_mode"] }
```

## 许可证

MIT
