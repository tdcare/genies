# genies_dapr

A Dapr integration library for the Genies (神灯) framework, providing pub/sub messaging, CloudEvent handling, and automatic topic subscription management.

## Overview

genies_dapr provides seamless integration with Dapr's pub/sub building block. It enables automatic topic subscription discovery, CloudEvent processing, and idempotent message consumption. The library works with the `#[topic]` macro from genies_derive to minimize boilerplate code.

## Features

- **Automatic Topic Discovery**: `collect_topic_subscriptions()` auto-collects all `#[topic]` handlers
- **Router Auto-Collection**: `collect_topic_routers()` builds unified handler routing
- **One-Line Setup**: `dapr_event_router()` configures complete Dapr subscription routing
- **CloudEvent Support**: Parse and process CloudEvent format messages
- **Idempotent Consumption**: Built-in support for idempotent message processing pattern
- **Dapr Sidecar Integration**: Compatible with Dapr subscription discovery protocol

## Architecture

### Core Components

| Component | File | Description |
|-----------|------|-------------|
| `Topicpoint` | topicpoint.rs | Topic handler registration struct (inventory pattern) |
| `collect_topic_routers` | topicpoint.rs | Auto-collect topic handlers and build unified router |
| `collect_topic_subscriptions` | topicpoint.rs | Auto-collect Dapr subscription configurations |
| `dapr_subscribe_handler` | topicpoint.rs | GET /dapr/subscribe endpoint handler |
| `dapr_event_router` | topicpoint.rs | One-line complete Dapr routing setup |
| `dapr_sub` | dapr_sub.rs | Event consumption endpoint handler (SUCCESS/RETRY) |
| `CloudEvent` | cloud_event.rs | CloudEvent data structure |
| `DaprTopicSubscription` | pubsub.rs | Dapr subscription configuration struct |
| `DaprClient` | client.rs | Dapr client trait |

### Subscription Discovery Flow

```
Dapr Sidecar ─GET /dapr/subscribe─> dapr_subscribe_handler ─> JSON subscription list
                                                                      │
CloudEvent ───POST /daprsub/consumers──> topic handlers ─> dapr_sub ─> SUCCESS/RETRY
```

### Handler Execution Flow

```
CloudEvent → Parse → Match event_type → Handler(tx, event) → OK: SUCCESS / Err: RETRY
```

## Quick Start

### 1. Add Dependency

Use `cargo add` to add dependencies (automatically fetches the latest version):

```sh
cargo add genies_dapr genies_derive genies_ddd salvo
```

You can also manually add dependencies in `Cargo.toml`. Visit [crates.io](https://crates.io) for the latest versions.

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

/// Handle device use event
#[topic(
    name = "com.example.device.domain.Device",
    pubsub = "messagebus"
)]
pub async fn on_device_use(tx: &mut dyn Executor, event: DeviceUseEvent) -> anyhow::Result<u64> {
    // Process event
    println!("Received event: {:?}", event);
    Ok(0)
}
```

The `#[topic]` macro generates:
- `{fn_name}_hoop` - Salvo middleware for event routing
- `{fn_name}_dapr` - Function returning `DaprTopicSubscription`
- Auto-registration with `inventory` for collection

### 4. Configure Routes

**Option A: Full Auto (Recommended)**

```rust
use genies::dapr::dapr_event_router;

fn main_router() -> Router {
    Router::new()
        .push(dapr_event_router())  // One line does everything
}
```

**Option B: Semi-Auto**

```rust
use genies::dapr::{collect_topic_routers, dapr_subscribe_handler, dapr_sub::dapr_sub};

fn main_router() -> Router {
    Router::new()
        .push(Router::with_path("/dapr/subscribe").get(dapr_subscribe_handler))
        .push(collect_topic_routers().post(dapr_sub))
}
```

**Option C: Manual**

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

## API Reference

### Topicpoint Struct

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

### Collection Functions

```rust
/// Auto-collect all topic handlers, returns Router with hoops mounted
/// Path: /daprsub/consumers
pub fn collect_topic_routers() -> Router;

/// Auto-collect all Dapr subscription configurations
pub fn collect_topic_subscriptions() -> Vec<DaprTopicSubscription>;

/// GET /dapr/subscribe handler - returns subscription JSON
#[handler]
pub async fn dapr_subscribe_handler(res: &mut Response);

/// Complete Dapr event router setup
/// - GET /dapr/subscribe
/// - POST /daprsub/consumers (with all topic handlers)
pub fn dapr_event_router() -> Router;
```

### CloudEvent Structure

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
    pub data: Value,  // Contains MessageImpl
}
```

### DaprTopicSubscription Structure

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

## Idempotent Consumption Pattern

The library supports idempotent message consumption using cache-based locking:

```rust
// Key format: {server_name}-{handler_name}-{event_type}-{msg_id}
// States: CONSUMING (60s TTL) → CONSUMED (7 days TTL)

// 1. NX lock attempt
let acquired = cache.set_string_ex_nx(key, "CONSUMING", Some(60s)).await?;

if !acquired {
    let status = cache.get_string(key).await?;
    if status == "CONSUMED" {
        return Ok(());  // Skip - already processed
    } else {
        return Err("RETRY");  // Another instance processing
    }
}

// 2. Process event
handle_event(event).await?;

// 3. Mark consumed
cache.set_string_ex(key, "CONSUMED", Some(7 * 24 * 60 * 60s)).await?;
```

## Dapr Configuration

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

### Topic Macro Attributes

| Attribute | Description | Example |
|-----------|-------------|---------|
| `name` | Topic/aggregate type name | `"com.example.Device"` |
| `pubsub` | Dapr pubsub component name | `"messagebus"` |

## Response Status

| Status | Meaning | Action |
|--------|---------|--------|
| `SUCCESS` | All handlers succeeded | Dapr ACKs message |
| `RETRY` | At least one handler failed | Dapr retries delivery |

## Dependencies

- **salvo** - Web framework
- **inventory** - Compile-time plugin registration
- **serde** / **serde_json** - Serialization
- **log** - Logging

## Integration with Other Crates

- **genies_ddd**: Domain events published via `Message` table are consumed by this module
- **genies_derive**: Provides `#[topic]` macro for handler registration
- **genies_cache**: Provides cache service for idempotent consumption
- **genies_context**: Provides `CONTEXT.rbatis` for database transactions

## Testing

```bash
cargo test -p topic -- --nocapture
```

Tests cover: subscription collection, router structure, idempotent consumption, concurrent processing, TTL expiration, etc.

## License

See the project root for license information.
