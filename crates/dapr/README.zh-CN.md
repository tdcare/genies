# Dapr 模块

`dapr` 模块负责与 Dapr 相关的功能，包括客户端、云事件、发布订阅等。该模块帮助开发者在应用程序中轻松集成 Dapr，实现微服务之间的通信和事件驱动架构。

## 功能概述
- **Dapr 客户端**: 提供与 Dapr 交互的客户端功能，包括调用服务、发布消息等。
- **云事件**: 处理云事件，确保事件在不同服务之间的传递和处理。
- **发布订阅**: 提供发布订阅功能，支持消息的发布和订阅。

## 使用说明

### Dapr 客户端
`client.rs` 中定义了 Dapr 客户端结构体 `DaprClient`，用于与 Dapr 交互。

```rust
use dapr::client::DaprClient;

let client = DaprClient::new("http://127.0.0.1:3500");
client.publish("pubsub", "topic", "message").unwrap();
```

### 云事件
`cloud_event.rs` 中定义了云事件结构体 `CloudEvent`，用于处理云事件。

```rust
use dapr::cloud_event::CloudEvent;

let event = CloudEvent::new("source", "type", "data");
println!("Event: {:?}", event);
```

### 发布订阅
`pubsub.rs` 中提供了发布订阅功能，支持消息的发布和订阅。

```rust
use dapr::pubsub::PubSub;

let pubsub = PubSub::new("pubsub");
pubsub.publish("topic", "message").unwrap();
```

## 详细说明

### `client.rs`
- **DaprClient**: Dapr 客户端结构体，包含以下功能：
  - `new`: 创建一个新的 Dapr 客户端实例。
  - `publish`: 发布消息到指定的主题。
  - `invoke_service`: 调用指定的服务。

### `cloud_event.rs`
- **CloudEvent**: 云事件结构体，包含以下功能：
  - `new`: 创建一个新的云事件实例。
  - `source`: 获取事件的来源。
  - `type`: 获取事件的类型。
  - `data`: 获取事件的数据。

### `pubsub.rs`
- **PubSub**: 发布订阅结构体，包含以下功能：
  - `new`: 创建一个新的发布订阅实例。
  - `publish`: 发布消息到指定的主题。
  - `subscribe`: 订阅指定的主题并处理消息。

## 贡献
欢迎提交 Pull Request 或 Issue 来改进本项目。

## 许可证
本项目采用 MIT 许可证，详情请参阅 `LICENSE` 文件。 