# genies_ddd

Genies (神灯) 框架的领域驱动设计（DDD）基础库，提供聚合根模式、领域事件和事件发布能力。

## 概述

genies_ddd 提供在 Rust 微服务中实现领域驱动设计的基础构建块。它提供聚合和领域事件的 trait 定义，以及将事件持久化到数据库以实现可靠投递的事件发布机制。

## 核心特性

- **聚合根模式**：`AggregateType` 和 `WithAggregateId` trait 用于聚合标识
- **领域事件**：`DomainEvent` trait 支持版本控制、来源追踪和 JSON 序列化
- **事件发布**：将领域事件持久化到数据库，实现可靠的异步投递
- **消息结构**：CloudEvent 兼容的消息格式，包含 headers 和 payload
- **派生宏支持**：与 genies_derive 的 `#[derive(Aggregate)]` 和 `#[derive(DomainEvent)]` 配合使用

## 架构设计

### 核心组件

| 组件 | 文件 | 功能 |
|------|------|------|
| `AggregateType` | aggregate.rs | 聚合类型标识 trait |
| `WithAggregateId` | aggregate.rs | 聚合 ID 访问 trait |
| `AggregateIdOf<A>` | aggregate.rs | 聚合 ID 类型别名 |
| `InitializeAggregate` | aggregate.rs | 聚合初始化 trait |
| `DomainEvent` | event.rs | 领域事件接口 trait |
| `Message` | message.rs | 数据库持久化消息结构 |
| `MessageImpl` | message.rs | 内存消息（含 headers） |
| `Headers` | message.rs | CloudEvent 兼容消息头 |
| `publish` | DomainEventPublisher.rs | 发布聚合领域事件 |
| `publishGenericDomainEvent` | DomainEventPublisher.rs | 发布通用领域事件 |

### 事件流程

```
聚合根 → 领域事件 → publish() → Message (数据库) → CDC/Outbox → Dapr PubSub
```

## 快速开始

### 1. 添加依赖

```sh
cargo add genies_ddd genies_derive serde --features serde/derive
```

> 也可以手动在 `Cargo.toml` 中添加依赖，请前往 [crates.io](https://crates.io) 查看最新版本。

### 2. 定义聚合根

```rust
use genies_derive::Aggregate;
use serde::{Deserialize, Serialize};

#[derive(Aggregate, Debug, Clone, Serialize, Deserialize)]
#[aggregate_type("Device")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub status: String,
}
```

`#[derive(Aggregate)]` 宏自动生成：
- `AggregateType` trait 实现，提供 `aggregate_type()` 和 `atype()` 方法
- `WithAggregateId` trait 实现，使用第一个字段作为聚合 ID

### 3. 定义领域事件

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
- `DomainEvent` trait 实现，提供 `event_type()`、`event_type_version()`、`event_source()` 和 `json()` 方法
- 通过 serde 自动 JSON 序列化

### 4. 发布领域事件

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
    
    // 带聚合上下文发布
    publish(tx, device, Box::new(event)).await;
}

// 发布通用事件（不带聚合上下文）
pub async fn send_notification(tx: &mut dyn Executor) {
    let event = NotificationEvent { /* ... */ };
    publishGenericDomainEvent(tx, Box::new(event)).await;
}
```

## API 参考

### AggregateType Trait

```rust
pub trait AggregateType {
    /// 返回此实例的聚合类型名称
    fn aggregate_type(&self) -> String;
    
    /// 返回聚合类型名称（静态方法）
    fn atype() -> String;
}
```

### WithAggregateId Trait

```rust
pub trait WithAggregateId {
    type Id: Debug + Clone + PartialEq + Serialize + DeserializeOwned;

    /// 返回聚合 ID 的引用
    fn aggregate_id(&self) -> &Self::Id;
}

/// 提取聚合 ID 类型的类型别名
pub type AggregateIdOf<A> = <A as WithAggregateId>::Id;
```

### DomainEvent Trait

```rust
pub trait DomainEvent: Send {
    /// 领域事件版本（如 "V1"、"V2"）
    fn event_type_version(&self) -> String;
    
    /// 完整限定的事件类型名称
    fn event_type(&self) -> String;
    
    /// 事件来源（通常是聚合类型）
    fn event_source(&self) -> String;
    
    /// JSON 序列化表示
    fn json(&self) -> String;
}
```

### Message 结构体

```rust
/// 用于 outbox 模式的数据库持久化消息
pub struct Message {
    pub id: Option<String>,
    pub destination: Option<String>,
    pub headers: Option<String>,
    pub payload: String,
    pub published: Option<u32>,      // 0 = 未发布, 1 = 已发布
    pub creation_time: Option<i64>,
}
```

### Headers 结构体

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

## 数据库表

`Message` 结构需要 `message` 表：

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

## 与 genies_derive 集成

DDD traits 设计为与 genies_derive 宏无缝配合：

| 宏 | 生成的 Traits |
|----|--------------|
| `#[derive(Aggregate)]` | `AggregateType`, `WithAggregateId` |
| `#[derive(DomainEvent)]` | `DomainEvent` |

### Aggregate 宏属性

- `#[aggregate_type("TypeName")]` - 指定聚合类型名称

### DomainEvent 宏属性

- `#[event_type("fully.qualified.EventType")]` - 完整限定的事件类型
- `#[event_type_version("V1")]` - 事件版本
- `#[event_source("fully.qualified.AggregateType")]` - 事件来源

## 依赖项

- **rbatis** - 消息持久化 ORM 框架
- **serde** / **serde_json** - 序列化
- **fastdate** - 时间戳生成
- **uuid** - 消息 ID 生成

## 与其他 Crate 集成

- **genies_dapr**：通过 `Message` 表发布的领域事件由 Dapr CDC/Outbox 消费并投递给订阅者
- **genies_derive**：提供 `#[derive(Aggregate)]` 和 `#[derive(DomainEvent)]` 宏
- **genies_context**：提供事件发布所需的数据库连接（`CONTEXT.rbatis`）

## 许可证

请参阅项目根目录的许可证信息。
