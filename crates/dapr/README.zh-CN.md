# genies_dapr

Genies (神灯) 框架的 Dapr 集成库，提供 pub/sub 消息处理、CloudEvent 解析和自动 topic 订阅管理。

## 概述

genies_dapr 提供与 Dapr pub/sub 构建块的无缝集成。它支持自动 topic 订阅发现、CloudEvent 处理和幂等消息消费。该库与 genies_derive 的 `#[topic]` 宏配合使用，最大程度减少样板代码。

## 核心特性

- **自动 Topic 发现**：`collect_topic_subscriptions()` 自动收集所有 `#[topic]` handler
- **路由自动收集**：`collect_topic_routers()` 构建统一的 handler 路由
- **一行配置**：`dapr_event_router()` 配置完整的 Dapr 订阅路由
- **CloudEvent 支持**：解析和处理 CloudEvent 格式消息
- **幂等消费**：内置幂等消息处理模式支持
- **Dapr Sidecar 集成**：兼容 Dapr 订阅发现协议

## 架构设计

### 核心组件

| 组件 | 文件 | 功能 |
|------|------|------|
| `Topicpoint` | topicpoint.rs | Topic handler 注册结构（inventory 模式） |
| `collect_topic_routers` | topicpoint.rs | 自动收集 topic handler 并构建统一路由 |
| `collect_topic_subscriptions` | topicpoint.rs | 自动收集 Dapr 订阅配置 |
| `dapr_subscribe_handler` | topicpoint.rs | GET /dapr/subscribe 端点 handler |
| `dapr_event_router` | topicpoint.rs | 一行代码完成完整 Dapr 路由配置 |
| `dapr_sub` | dapr_sub.rs | 事件消费端点 handler（SUCCESS/RETRY） |
| `CloudEvent` | cloud_event.rs | CloudEvent 数据结构 |
| `DaprTopicSubscription` | pubsub.rs | Dapr 订阅配置结构 |
| `DaprClient` | client.rs | Dapr 客户端 trait |

### 订阅发现流程

```
Dapr Sidecar ─GET /dapr/subscribe─> dapr_subscribe_handler ─> JSON 订阅列表
                                                                      │
CloudEvent ───POST /daprsub/consumers──> topic handlers ─> dapr_sub ─> SUCCESS/RETRY
```

### Handler 执行流程

```
CloudEvent → 解析 → 匹配 event_type → Handler(tx, event) → OK: SUCCESS / Err: RETRY
```

## 快速开始

### 1. 添加依赖

```sh
cargo add genies_dapr genies_derive genies_ddd salvo
```

> 也可以手动在 `Cargo.toml` 中添加依赖，请前往 [crates.io](https://crates.io) 查看最新版本。

### 2. 定义领域事件

```rust
use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("com.example.device.domain.Device")]
#[event_type("com.example.device.event.DeviceUseEvent")]
pub struct DeviceUseEvent {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub device_no: Option<String>,
}
```

### 3. 定义 Topic Handler

```rust
use genies_derive::topic;
use rbatis::executor::Executor;

/// 处理设备使用事件
#[topic(
    name = "com.example.device.domain.Device",
    pubsub = "messagebus"
)]
pub async fn on_device_use(tx: &mut dyn Executor, event: DeviceUseEvent) -> anyhow::Result<u64> {
    // 处理事件
    println!("收到事件: {:?}", event);
    Ok(0)
}
```

`#[topic]` 宏自动生成：
- `{fn_name}_hoop` - Salvo 中间件用于事件路由
- `{fn_name}_dapr` - 返回 `DaprTopicSubscription` 的函数
- 通过 `inventory` 自动注册供收集

### 4. 配置路由

**方式一：完全自动化（推荐）**

```rust
use genies::dapr::dapr_event_router;

fn main_router() -> Router {
    Router::new()
        .push(dapr_event_router())  // 一行代码搞定一切
}
```

**方式二：半自动**

```rust
use genies::dapr::{collect_topic_routers, dapr_subscribe_handler, dapr_sub::dapr_sub};

fn main_router() -> Router {
    Router::new()
        .push(Router::with_path("/dapr/subscribe").get(dapr_subscribe_handler))
        .push(collect_topic_routers().post(dapr_sub))
}
```

**方式三：手动注册**

```rust
use crate::handlers::{on_device_use_hoop, on_device_use_dapr};
use genies::dapr::dapr_sub::dapr_sub;

fn main_router() -> Router {
    Router::new()
        .push(Router::with_path("/dapr/subscribe").get(manual_subscribe_handler))
        .push(
            Router::with_path("/daprsub/consumers")
                .hoop(on_device_use_hoop)
                .post(dapr_sub)
        )
}

#[handler]
async fn manual_subscribe_handler(res: &mut Response) {
    let subscriptions = vec![on_device_use_dapr()];
    res.render(Json(&subscriptions));
}
```

## API 参考

### Topicpoint 结构体

```rust
pub struct Topicpoint {
    pub subscribe: fn() -> DaprTopicSubscription,
    pub hoop: fn() -> Router,
}

impl Topicpoint {
    pub const fn new(subscribe: fn() -> DaprTopicSubscription, hoop: fn() -> Router) -> Self;
}

inventory::collect!(Topicpoint);
```

### 收集函数

```rust
/// 自动收集所有 topic handler，返回挂载了 hoops 的 Router
/// 路径: /daprsub/consumers
pub fn collect_topic_routers() -> Router;

/// 自动收集所有 Dapr 订阅配置
pub fn collect_topic_subscriptions() -> Vec<DaprTopicSubscription>;

/// GET /dapr/subscribe handler - 返回订阅 JSON
#[handler]
pub async fn dapr_subscribe_handler(res: &mut Response);

/// 完整的 Dapr 事件路由配置
/// - GET /dapr/subscribe
/// - POST /daprsub/consumers（包含所有 topic handlers）
pub fn dapr_event_router() -> Router;
```

### CloudEvent 结构体

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CloudEvent {
    pub id: Option<String>,
    pub traceid: Option<String>,
    pub topic: Option<String>,
    #[serde(rename = "pubsubname")]
    pub pubsub_name: Option<String>,
    pub source: Option<String>,
    pub r#type: Option<String>,
    pub specversion: Option<String>,
    pub datacontenttype: Option<String>,
    pub data: Value,  // 包含 MessageImpl
}
```

### DaprTopicSubscription 结构体

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaprTopicSubscription {
    #[serde(rename = "pubsubName")]
    pub pubsub_name: Option<String>,
    pub topic: Option<String>,
    pub route: Option<String>,
    pub routes: Option<DaprRoute>,
    pub metadata: Option<HashMap<String, String>>,
}
```

## 幂等消费模式

该库支持基于缓存锁的幂等消息消费：

```rust
// Key 格式: {server_name}-{handler_name}-{event_type}-{msg_id}
// 状态: CONSUMING (60s TTL) → CONSUMED (7天 TTL)

// 1. NX 抢锁
let acquired = cache.set_string_ex_nx(key, "CONSUMING", Some(60s)).await?;

if !acquired {
    let status = cache.get_string(key).await?;
    if status == "CONSUMED" {
        return Ok(());  // 跳过 - 已处理
    } else {
        return Err("RETRY");  // 其他实例正在处理
    }
}

// 2. 处理事件
handle_event(event).await?;

// 3. 标记已消费
cache.set_string_ex(key, "CONSUMED", Some(7 * 24 * 60 * 60s)).await?;
```

## Dapr 配置

### application.yaml (Dapr Sidecar)

```yaml
apiVersion: dapr.io/v1alpha1
kind: Component
metadata:
  name: messagebus
spec:
  type: pubsub.redis
  version: v1
  metadata:
    - name: redisHost
      value: "localhost:6379"
```

### Topic 宏属性

| 属性 | 说明 | 示例 |
|------|------|------|
| `name` | Topic/聚合类型名称 | `"com.example.Device"` |
| `pubsub` | Dapr pubsub 组件名 | `"messagebus"` |

## 响应状态

| 状态 | 含义 | 动作 |
|------|------|------|
| `SUCCESS` | 所有 handler 成功 | Dapr 确认消息 |
| `RETRY` | 至少一个 handler 失败 | Dapr 重新投递 |

## 依赖项

- **salvo** - Web 框架
- **inventory** - 编译期插件注册
- **serde** / **serde_json** - 序列化
- **log** - 日志

## 与其他 Crate 集成

- **genies_ddd**：通过 `Message` 表发布的领域事件由本模块消费
- **genies_derive**：提供 `#[topic]` 宏用于 handler 注册
- **genies_cache**：提供缓存服务用于幂等消费
- **genies_context**：提供 `CONTEXT.rbatis` 用于数据库事务

## 测试

```bash
cargo test -p topic -- --nocapture
```

测试覆盖：订阅收集、路由结构、幂等消费、并发处理、TTL 过期等。

## 许可证

请参阅项目根目录的许可证信息。
