---
name: ddd-usage
description: Guide for using genies_ddd Domain-Driven Design primitives. Use when defining aggregates, domain events, implementing event sourcing patterns, or building DDD-based microservices with the Genies framework.
---

# DDD Module (genies_ddd)

## Overview

genies_ddd 是 Genies 框架的领域驱动设计基础库，提供聚合根模式、领域事件和事件发布能力。采用 Outbox 模式将领域事件持久化到数据库，由 Dapr CDC 异步投递。

**核心特性：**
- 聚合根标识（AggregateType + WithAggregateId）
- 领域事件接口（DomainEvent trait）
- 事件发布器（publish / publishGenericDomainEvent）
- CloudEvent 兼容消息格式
- 与 genies_derive 宏无缝配合

## Architecture

```
聚合根 → 领域事件 → publish() → Message 表 → CDC/Outbox → Dapr PubSub → 订阅者
```

核心组件：
- `AggregateType` - 聚合类型标识 trait
- `WithAggregateId` - 聚合 ID 访问 trait
- `AggregateIdOf<A>` - 聚合 ID 类型别名
- `DomainEvent` - 领域事件接口（event_type, event_type_version, event_source, json）
- `Message` - 数据库持久化消息
- `Headers` - CloudEvent 兼容消息头
- `publish` / `publishGenericDomainEvent` - 事件发布函数

## Quick Start

### 1. Dependencies

```toml
[dependencies]
genies_ddd = { workspace = true }
genies_derive = { workspace = true }  # for #[derive(Aggregate)], #[derive(DomainEvent)]
serde = { version = "1.0", features = ["derive"] }
```

### 2. Define Aggregate Root

```rust
use genies_derive::Aggregate;
use serde::{Deserialize, Serialize};

#[derive(Aggregate, Debug, Clone, Serialize, Deserialize)]
#[aggregate_type("Device")]
pub struct Device {
    pub id: String,        // 第一个字段作为 aggregate_id
    pub name: String,
    pub status: String,
}
```

`#[derive(Aggregate)]` 宏自动生成：
- `AggregateType` trait：`aggregate_type()` 返回 "Device"
- `WithAggregateId` trait：`aggregate_id()` 返回 `&self.id`

### 3. Define Domain Event

```rust
use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("com.example.device.domain.Device")]
#[event_type("com.example.device.event.DeviceCreated")]
pub struct DeviceCreatedEvent {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}
```

`#[derive(DomainEvent)]` 宏自动生成：
- `event_type()` → "com.example.device.event.DeviceCreated"
- `event_type_version()` → "V1"
- `event_source()` → "com.example.device.domain.Device"
- `json()` → serde_json::to_string(self)

### 4. Publish Domain Events

```rust
use genies_ddd::DomainEventPublisher::{publish, publishGenericDomainEvent};
use rbatis::executor::Executor;

// 从聚合发布事件（包含聚合上下文）
pub async fn create_device(tx: &mut dyn Executor, device: &Device) {
    let event = DeviceCreatedEvent {
        id: device.id.clone(),
        name: device.name.clone(),
        created_at: chrono::Utc::now().timestamp_millis(),
    };
    
    // 带聚合上下文发布 — headers 自动填充 aggregate_type/aggregate_id
    publish(tx, device, Box::new(event)).await;
}

// 发布通用事件（不带聚合上下文）
pub async fn send_notification(tx: &mut dyn Executor) {
    let event = NotificationEvent { /* ... */ };
    // destination 固定为 "GenericDomainEvent"
    publishGenericDomainEvent(tx, Box::new(event)).await;
}
```

## API Reference

### AggregateType Trait

```rust
pub trait AggregateType {
    fn aggregate_type(&self) -> String;  // 实例方法
    fn atype() -> String;                // 静态方法
}
```

### WithAggregateId Trait

```rust
pub trait WithAggregateId {
    type Id: Debug + Clone + PartialEq + Serialize + DeserializeOwned;
    fn aggregate_id(&self) -> &Self::Id;
}

pub type AggregateIdOf<A> = <A as WithAggregateId>::Id;
```

### DomainEvent Trait

```rust
pub trait DomainEvent: Send {
    fn event_type_version(&self) -> String;  // e.g., "V1"
    fn event_type(&self) -> String;          // fully qualified name
    fn event_source(&self) -> String;        // aggregate type
    fn json(&self) -> String;                // JSON payload
}
```

### Message Structure

```rust
pub struct Message {
    pub id: Option<String>,
    pub destination: Option<String>,    // aggregate_type 或 "GenericDomainEvent"
    pub headers: Option<String>,        // JSON serialized Headers
    pub payload: String,                // event JSON
    pub published: Option<u32>,         // 0 = 未发布
    pub creation_time: Option<i64>,     // 毫秒时间戳
}
```

### Headers Structure

```rust
pub struct Headers {
    pub ID: Option<String>,
    pub PARTITION_ID: Option<String>,
    pub DESTINATION: Option<String>,
    pub DATE: Option<String>,
    #[serde(rename = "event-aggregate-type")]
    pub event_aggregate_type: Option<String>,
    #[serde(rename = "event-aggregate-id")]
    pub event_aggregate_id: Option<String>,
    #[serde(rename = "event-type")]
    pub event_type: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
```

## Database Table

```sql
CREATE TABLE message (
    id VARCHAR(36) PRIMARY KEY,
    destination VARCHAR(255),
    headers TEXT,
    payload TEXT NOT NULL,
    published INT DEFAULT 0,
    creation_time BIGINT
);
```

## Derive Macro Attributes

### #[derive(Aggregate)]

- `#[aggregate_type("TypeName")]` — 指定聚合类型名称（必选）

### #[derive(DomainEvent)]

- `#[event_type("fully.qualified.EventType")]` — 事件类型（必选）
- `#[event_type_version("V1")]` — 事件版本（必选）
- `#[event_source("fully.qualified.AggregateType")]` — 事件来源（必选）

## Integration

- **genies_dapr**: Message 表由 Dapr CDC/Outbox 消费，通过 PubSub 投递给 `#[topic]` 标记的订阅者
- **genies_derive**: 提供 `#[derive(Aggregate)]` 和 `#[derive(DomainEvent)]` 宏
- **genies_context**: 提供 `CONTEXT.rbatis` 数据库连接

## Key Files

- [crates/ddd/src/lib.rs](file:///d:/tdcare/genies/crates/ddd/src/lib.rs) - 模块入口
- [crates/ddd/src/aggregate.rs](file:///d:/tdcare/genies/crates/ddd/src/aggregate.rs) - 聚合 traits
- [crates/ddd/src/event.rs](file:///d:/tdcare/genies/crates/ddd/src/event.rs) - DomainEvent trait
- [crates/ddd/src/message.rs](file:///d:/tdcare/genies/crates/ddd/src/message.rs) - Message/Headers 结构
- [crates/ddd/src/DomainEventPublisher.rs](file:///d:/tdcare/genies/crates/ddd/src/DomainEventPublisher.rs) - 事件发布函数
- [crates/genies_derive/src/aggregate_type.rs](file:///d:/tdcare/genies/crates/genies_derive/src/aggregate_type.rs) - Aggregate 宏实现
- [crates/genies_derive/src/event_type.rs](file:///d:/tdcare/genies/crates/genies_derive/src/event_type.rs) - DomainEvent 宏实现
