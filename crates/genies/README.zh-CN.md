# genies

Genies 框架的主入口 crate，提供所有子 crate 的统一 re-export 和便捷宏，用于 Rust 中的 DDD + Dapr 开发。

## 概述

genies 是整合所有 Genies 框架组件的中心枢纽 crate，开发者只需添加一个依赖即可访问整个框架。它还提供了数据库事务、对象复制、Dapr 网关配置等常用操作的实用宏。

## 核心特性

- **统一 Re-export**：通过单一导入访问所有子 crate（core、config、context、cache、dapr、ddd、k8s）
- **数据库宏**：RBatis 连接池和事务管理的便捷宏
- **对象复制**：基于 JSON 的不同结构体类型间字段复制
- **Dapr 集成**：服务间通信的网关 URL 配置
- **Topic 收集**：重新导出的 Dapr topic 订阅路由函数

## 架构设计

### 依赖关系图

```
genies (主入口)
  ├── genies_core      # 核心工具（JWT、错误处理、条件构建）
  ├── genies_config    # 配置管理
  ├── genies_context   # 应用上下文和认证
  ├── genies_cache     # Redis/内存缓存
  ├── genies_dapr      # Dapr 集成（PubSub、CloudEvent）
  ├── genies_ddd       # DDD 原语（聚合根、领域事件）
  └── genies_k8s       # Kubernetes 工具
```

### Re-export 模块

| 模块 | 别名 | 功能 |
|------|------|------|
| `genies_core` | `core` | JWT 处理、错误类型、条件辅助函数 |
| `genies_config` | `config` | YAML/环境变量配置加载 |
| `genies_context` | `context` | `CONTEXT` 全局变量、认证中间件 |
| `genies_cache` | `cache` | Redis 和内存缓存服务 |
| `genies_dapr` | `dapr` | Dapr PubSub、CloudEvent、topic 路由 |
| `genies_ddd` | `ddd` | Aggregate、DomainEvent、Message traits |
| `genies_k8s` | `k8s` | Kubernetes 集成工具 |

## 快速开始

### 1. 添加依赖

```toml
[dependencies]
genies = { path = "../path/to/genies" }
genies_derive = { path = "../path/to/genies_derive" }
```

### 2. 访问子模块

```rust
use genies::core;       // JWT、错误处理
use genies::config;     // 配置管理
use genies::context::CONTEXT;  // 全局应用上下文
use genies::cache;      // 缓存服务
use genies::dapr;       // Dapr 集成
use genies::ddd;        // DDD 原语
```

## 宏参考

### `pool!()` - 获取数据库连接

从全局上下文返回克隆的 RBatis 连接。

```rust
use genies::pool;

async fn query_users() -> Result<Vec<User>, Error> {
    let users = User::select_all(pool!()).await?;
    Ok(users)
}
```

### `tx_defer!()` - 带自动回滚守卫的事务

创建一个事务，如果未显式提交则自动回滚。

```rust
use genies::tx_defer;

async fn transfer_funds(from: u64, to: u64, amount: f64) -> Result<(), Error> {
    let mut tx = tx_defer!();
    
    // 从源账户扣款
    Account::deduct(&mut tx, from, amount).await?;
    
    // 向目标账户入账
    Account::credit(&mut tx, to, amount).await?;
    
    // 提交事务（未调用则自动回滚）
    tx.commit().await?;
    Ok(())
}
```

**使用自定义 RBatis 实例：**

```rust
let custom_rb = get_custom_rbatis();
let mut tx = tx_defer!(custom_rb);
// ... 操作 ...
tx.commit().await?;
```

### `copy!()` - 对象字段复制

通过 JSON 序列化将字段从一个结构体复制到另一个。适用于 DTO 转换。

```rust
use genies::copy;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct UserEntity {
    id: u64,
    name: String,
    email: String,
    password_hash: String,
}

#[derive(Serialize, Deserialize)]
struct UserDto {
    id: u64,
    name: String,
    email: String,
}

let entity = UserEntity { /* ... */ };
let dto: UserDto = copy!(&entity, UserDto);
```

### `config_gateway!()` - Dapr/Gateway URL 配置

配置服务调用的网关 URL，支持 Dapr sidecar 和直接 HTTP 网关两种模式。

```rust
use genies::config_gateway;

// 定义服务网关
static PATIENT_SERVICE: Lazy<String> = config_gateway!("/patient");

// 在 HTTP 客户端中使用
let url = format!("{}/api/patients/{}", *PATIENT_SERVICE, patient_id);
```

**行为说明：**
- 如果 `gateway` 配置包含 `http://` 或 `https://`，使用直接网关 URL
- 否则，使用 Dapr 服务调用 URL：`http://localhost:3500/v1.0/invoke{service}/method`

## Dapr Topic 函数

从 `genies_dapr` 重新导出，方便使用：

```rust
use genies::{
    collect_topic_routers,      // 收集所有 topic 处理路由
    collect_topic_subscriptions, // 收集所有 topic 订阅
    dapr_subscribe_handler,      // Dapr 订阅端点处理器
    dapr_event_router,           // 事件路由处理器
};

// 在 main.rs 中
let topic_routers = collect_topic_routers();
let subscriptions = collect_topic_subscriptions();
```

## 集成示例

```rust
use genies::context::CONTEXT;
use genies::{pool, tx_defer, copy};
use genies_derive::{Aggregate, DomainEvent};

#[derive(Aggregate)]
#[aggregate_type("Order")]
#[id_field(id)]
struct Order {
    id: String,
    status: String,
    items: Vec<OrderItem>,
}

async fn create_order(cmd: CreateOrderCommand) -> Result<Order, Error> {
    // 初始化上下文
    CONTEXT.init_mysql().await;
    
    // 使用带自动回滚的事务
    let mut tx = tx_defer!();
    
    let order = Order {
        id: generate_id(),
        status: "CREATED".to_string(),
        items: cmd.items,
    };
    
    Order::insert(&mut tx, &order).await?;
    tx.commit().await?;
    
    Ok(order)
}
```

## 依赖项

- **rbatis** - 数据库操作 ORM 框架
- **serde** / **serde_json** - `copy!()` 宏的序列化支持
- **once_cell** - `config_gateway!()` 的延迟静态初始化
- **log** - 事务守卫中的日志记录

## 相关 Crate

- [genies_derive](../genies_derive) - 过程宏（`#[derive(Aggregate)]`、`#[topic]` 等）
- [genies_auth](../auth) - 基于 Casbin 的权限管理
- [genies_context](../context) - 应用上下文和全局状态

## 许可证

MIT/Apache-2.0
