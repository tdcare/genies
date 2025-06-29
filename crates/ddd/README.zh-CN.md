# DDD 模块

`ddd` 模块基于领域驱动设计（DDD）原则，提供了聚合、领域事件、消息等核心功能。该模块帮助开发者在应用程序中实现领域驱动设计，确保业务逻辑的清晰和可维护性。

## 功能概述
- **聚合**: 管理领域对象的聚合，确保业务逻辑的一致性。
- **领域事件**: 处理领域事件，确保事件在不同领域对象之间的传递和处理。
- **消息**: 提供消息传递功能，支持领域对象之间的通信。

## 使用说明

### 聚合
`aggregate.rs` 中定义了聚合结构体 `Aggregate`，用于管理领域对象的聚合。

```rust
use ddd::aggregate::Aggregate;

let mut aggregate = Aggregate::new();
aggregate.apply_event("event").unwrap();
println!("Aggregate state: {:?}", aggregate.state);
```

### 领域事件
`event.rs` 中定义了领域事件结构体 `DomainEvent`，用于处理领域事件。

```rust
use ddd::event::DomainEvent;

let event = DomainEvent::new("event_type", "event_data");
println!("Event: {:?}", event);
```

### 消息
`message.rs` 中提供了消息传递功能，支持领域对象之间的通信。

```rust
use ddd::message::Message;

let message = Message::new("message_type", "message_data");
println!("Message: {:?}", message);
```

## 详细说明

### `aggregate.rs`
- **Aggregate**: 聚合结构体，包含以下功能：
  - `new`: 创建一个新的聚合实例。
  - `apply_event`: 应用领域事件并更新聚合状态。

### `event.rs`
- **DomainEvent**: 领域事件结构体，包含以下功能：
  - `new`: 创建一个新的领域事件实例。
  - `event_type`: 获取事件的类型。
  - `event_data`: 获取事件的数据。

### `message.rs`
- **Message**: 消息结构体，包含以下功能：
  - `new`: 创建一个新的消息实例。
  - `message_type`: 获取消息的类型。
  - `message_data`: 获取消息的数据。

## 贡献
欢迎提交 Pull Request 或 Issue 来改进本项目。

## 许可证
本项目采用 MIT 许可证，详情请参阅 `LICENSE` 文件。 