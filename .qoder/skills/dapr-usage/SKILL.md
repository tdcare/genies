---
name: dapr-usage
description: Guide for using genies_dapr Dapr integration. Use when implementing pub/sub messaging, handling CloudEvents, configuring topic subscriptions, or integrating Dapr sidecar with Genies microservices.
---

# Dapr Module (genies_dapr)

## Overview

genies_dapr 是 Genies 框架的 Dapr 集成库，提供 pub/sub 消息处理、CloudEvent 解析和自动 topic 订阅管理。通过 `#[topic]` 宏实现零样板代码的事件订阅。

**核心特性：**
- 自动 Topic 发现（collect_topic_subscriptions）
- 路由自动收集（collect_topic_routers）
- 一行配置完整 Dapr 路由（dapr_event_router）
- CloudEvent 格式支持
- 幂等消费模式
- Dapr Sidecar 协议兼容

## Architecture

```
Dapr Sidecar ─GET /dapr/subscribe─> dapr_subscribe_handler ─> JSON 订阅列表
CloudEvent ───POST /daprsub/consumers──> topic handlers ─> dapr_sub ─> SUCCESS/RETRY
```

核心组件：
- `Topicpoint` - Topic handler 注册结构（inventory 模式）
- `collect_topic_routers()` - 自动收集 handler 构建统一路由
- `collect_topic_subscriptions()` - 自动收集订阅配置
- `dapr_subscribe_handler` - GET /dapr/subscribe 端点
- `dapr_event_router()` - 一行代码完成完整路由配置
- `dapr_sub` - 事件消费端点（SUCCESS/RETRY）
- `CloudEvent` - CloudEvent 数据结构
- `DaprTopicSubscription` - Dapr 订阅配置

## Quick Start

### 1. Dependencies

```toml
[dependencies]
genies_dapr = { workspace = true }
genies_derive = { workspace = true }  # for #[topic] macro
genies_ddd = { workspace = true }     # for DomainEvent
salvo = { version = "0.x" }
```

### 2. Define Domain Event

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

### 3. Define Topic Handler

```rust
use genies_derive::topic;
use rbatis::executor::Executor;

#[topic(
    name = "com.example.device.domain.Device",
    pubsub = "messagebus"
)]
pub async fn on_device_use(tx: &mut dyn Executor, event: DeviceUseEvent) -> anyhow::Result<u64> {
    println!("收到事件: {:?}", event);
    Ok(0)
}
```

`#[topic]` 宏自动生成：
- `on_device_use_hoop` - Salvo 中间件
- `on_device_use_dapr` - 返回 DaprTopicSubscription

### 4. Configure Routes

**方式一：完全自动化（推荐）**

```rust
use genies::dapr::dapr_event_router;

let router = Router::new()
    .push(dapr_event_router());  // 一行代码搞定
```

**方式二：半自动**

```rust
use genies::dapr::{collect_topic_routers, dapr_subscribe_handler, dapr_sub::dapr_sub};

let router = Router::new()
    .push(Router::with_path("/dapr/subscribe").get(dapr_subscribe_handler))
    .push(collect_topic_routers().post(dapr_sub));
```

**方式三：手动注册**

```rust
use crate::handlers::{on_device_use_hoop, on_device_use_dapr};

let router = Router::new()
    .push(Router::with_path("/dapr/subscribe").get(manual_handler))
    .push(Router::with_path("/daprsub/consumers")
        .hoop(on_device_use_hoop)
        .post(dapr_sub));

#[handler]
async fn manual_handler(res: &mut Response) {
    res.render(Json(&vec![on_device_use_dapr()]));
}
```

## API Reference

### Collection Functions

```rust
// 自动收集所有 topic handler，返回 /daprsub/consumers 路由
pub fn collect_topic_routers() -> Router;

// 自动收集所有 Dapr 订阅配置
pub fn collect_topic_subscriptions() -> Vec<DaprTopicSubscription>;

// GET /dapr/subscribe handler
pub async fn dapr_subscribe_handler(res: &mut Response);

// 完整 Dapr 事件路由（GET /dapr/subscribe + POST /daprsub/consumers）
pub fn dapr_event_router() -> Router;
```

### Topicpoint Struct

```rust
pub struct Topicpoint {
    pub subscribe: fn() -> DaprTopicSubscription,
    pub hoop: fn() -> Router,
}

inventory::collect!(Topicpoint);
```

### CloudEvent Structure

```rust
pub struct CloudEvent {
    pub id: Option<String>,
    pub traceid: Option<String>,
    pub topic: Option<String>,
    pub pubsub_name: Option<String>,
    pub source: Option<String>,
    pub r#type: Option<String>,
    pub specversion: Option<String>,
    pub datacontenttype: Option<String>,
    pub data: Value,  // 包含 MessageImpl
}
```

### DaprTopicSubscription Structure

```rust
pub struct DaprTopicSubscription {
    pub pubsub_name: Option<String>,
    pub topic: Option<String>,
    pub route: Option<String>,
    pub routes: Option<DaprRoute>,
    pub metadata: Option<HashMap<String, String>>,
}
```

## Idempotent Consumption Pattern

幂等消费使用缓存锁实现，防止重复处理：

```rust
// Key: {server}-{handler}-{event_type}-{msg_id}
// 状态流转: CONSUMING (60s) → CONSUMED (7天)

// 1. NX 抢锁
let acquired = cache.set_string_ex_nx(key, "CONSUMING", 60s).await?;
if !acquired {
    let status = cache.get_string(key).await?;
    return if status == "CONSUMED" { Ok(()) } else { Err("RETRY") };
}

// 2. 处理事件
handle_event(event).await?;

// 3. 标记已消费
cache.set_string_ex(key, "CONSUMED", 7 * 24 * 60 * 60s).await?;
```

## Topic Macro Attributes

| 属性 | 说明 | 必选 |
|------|------|------|
| `name` | Topic/聚合类型名称 | 是 |
| `pubsub` | Dapr pubsub 组件名 | 是 |

## Response Status

| 状态 | 含义 | Dapr 动作 |
|------|------|----------|
| `SUCCESS` | 所有 handler 成功 | ACK 消息 |
| `RETRY` | 至少一个失败 | 重新投递 |

## Dapr Pubsub Component Example

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

## Integration

- **genies_ddd**: 领域事件通过 Message 表发布，由本模块消费
- **genies_derive**: 提供 `#[topic]` 宏
- **genies_cache**: 幂等消费缓存服务
- **genies_context**: 数据库事务支持

## Key Files

- [crates/dapr/src/lib.rs](file:///d:/tdcare/genies/crates/dapr/src/lib.rs) - 模块入口
- [crates/dapr/src/topicpoint.rs](file:///d:/tdcare/genies/crates/dapr/src/topicpoint.rs) - Topic 收集与路由
- [crates/dapr/src/cloud_event.rs](file:///d:/tdcare/genies/crates/dapr/src/cloud_event.rs) - CloudEvent 结构
- [crates/dapr/src/pubsub.rs](file:///d:/tdcare/genies/crates/dapr/src/pubsub.rs) - 订阅配置结构
- [crates/dapr/src/dapr_sub.rs](file:///d:/tdcare/genies/crates/dapr/src/dapr_sub.rs) - 事件消费 handler
- [crates/genies_derive/src/topic.rs](file:///d:/tdcare/genies/crates/genies_derive/src/topic.rs) - #[topic] 宏实现
- [examples/topic/src/UseDeviceListeners.rs](file:///d:/tdcare/genies/examples/topic/src/UseDeviceListeners.rs) - 使用示例
