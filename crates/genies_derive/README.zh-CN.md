# Genies Derive 模块

`genies_derive` 模块提供自定义派生宏，用于简化代码生成。该模块帮助开发者在应用程序中快速生成常用代码结构，如配置类型、聚合类型、事件类型、主题类型和包装类型等。

## 功能概述
- **配置类型**: 自动生成配置类型的派生宏，简化配置管理。
- **聚合类型**: 自动生成聚合类型的派生宏，简化领域驱动设计中的聚合实现。
- **事件类型**: 自动生成事件类型的派生宏，简化领域事件的处理。
- **主题类型**: 自动生成主题类型的派生宏，用于从消息队列中取回消息，并使用 Redis 实现接口的幂等操作。
- **包装类型**: 自动生成包装类型的派生宏，用于对 `feighthttp` 请求进行包装，解决 HTTP 访问凭证的问题。

## 使用说明

### 配置类型
`config_type.rs` 中定义了配置类型派生宏 `Config`，用于自动生成配置类型的代码。

```rust
use genies_derive::Config;

#[derive(Config)]
struct MyConfig {
    field: String,
}
```

### 聚合类型
`aggregate_type.rs` 中定义了聚合类型派生宏 `AggregateType`，用于自动生成聚合类型的代码。

```rust
use genies_derive::AggregateType;

#[derive(AggregateType)]
struct MyAggregate {
    field: String,
}
```

### 事件类型
`event_type.rs` 中定义了事件类型派生宏 `EventType`，用于自动生成事件类型的代码。

```rust
use genies_derive::EventType;

#[derive(EventType)]
struct MyEvent {
    field: String,
}
```

### 主题类型
`topic.rs` 中定义了主题类型派生宏 `Topic`，用于从消息队列中取回消息，并使用 Redis 实现接口的幂等操作。

```rust
use genies_derive::topic;
use crate::domain::event::SignsMonitorSyncResultEvent::SignsMonitorSyncResultEvent;
use crate::domain::service::VitalSignSycDocService;

/// 文书回写结果
#[topic(name = "signsMonitorSyncNursingDoc", pubsub = "messagebus")]
pub async fn signsMonitorSyncNursingDoc(
    tx: &mut dyn Executor,
    event: SignsMonitorSyncResultEvent,
) -> anyhow::Result<u64> {
    return VitalSignSycDocService::onExcutedEvent(tx, event).await;
}
```

### 包装类型
`wrapper.rs` 中定义了包装类型派生宏 `Wrapper`，用于对 `feighthttp` 请求进行包装，解决 HTTP 访问凭证的问题。

```rust
use genies_derive::Wrapper;

#[derive(Wrapper)]
struct MyWrapper {
    field: String,
}
```

## 详细说明

### `config_type.rs`
- **ConfigCore**: 配置类型派生宏，包含以下功能：
  - 自动生成配置类型的 `new` 方法。
  - 自动生成配置类型的 `get` 和 `set` 方法。

### `aggregate_type.rs`
- **AggregateType**: 聚合类型派生宏，包含以下功能：
  - 自动生成聚合类型的 `new` 方法。
  - 自动生成聚合类型的 `apply_event` 方法。

### `event_type.rs`
- **EventType**: 事件类型派生宏，包含以下功能：
  - 自动生成事件类型的 `new` 方法。
  - 自动生成事件类型的 `event_type` 和 `event_data` 方法。

### `topic.rs`
- **Topic**: 主题类型派生宏，包含以下功能：
  - 自动生成主题类型的 `new` 方法。
  - 自动生成主题类型的 `topic_name` 方法。
  - 从消息队列中取回消息，并使用 Redis 实现接口的幂等操作。

### `wrapper.rs`
- **Wrapper**: 包装类型派生宏，包含以下功能：
  - 自动生成包装类型的 `new` 方法，用于初始化包装器。
  - 自动生成包装类型的 `wrap_request` 方法，用于包装 `feighthttp` 请求，解决 HTTP 访问凭证的问题。
  - 自动生成包装类型的 `unwrap_response` 方法，用于解包 `feighthttp` 响应。

## 贡献
欢迎提交 Pull Request 或 Issue 来改进本项目。

## 许可证
本项目采用 MIT 许可证，详情请参阅 `LICENSE` 文件。
