# genies_ddd

A Domain-Driven Design (DDD) primitives library for the Genies (神灯) framework, providing aggregate root patterns, domain events, and event publishing capabilities.

## Overview

genies_ddd provides the foundational building blocks for implementing Domain-Driven Design in Rust microservices. It offers trait definitions for aggregates and domain events, along with a robust event publishing mechanism that persists events to a database for reliable delivery.

## Features

- **Aggregate Root Pattern**: `AggregateType` and `WithAggregateId` traits for aggregate identification
- **Domain Events**: `DomainEvent` trait with versioning, source tracking, and JSON serialization
- **Event Publishing**: Persist domain events to database for reliable asynchronous delivery
- **Message Structure**: CloudEvent-compatible message format with headers and payload
- **Derive Macro Support**: Works with `#[derive(Aggregate)]` and `#[derive(DomainEvent)]` from genies_derive

## Architecture

### Core Components

| Component | File | Description |
|-----------|------|-------------|
| `AggregateType` | aggregate.rs | Trait for aggregate type identification |
| `WithAggregateId` | aggregate.rs | Trait for aggregate ID access |
| `AggregateIdOf<A>` | aggregate.rs | Type alias for aggregate ID extraction |
| `InitializeAggregate` | aggregate.rs | Trait for aggregate initialization |
| `DomainEvent` | event.rs | Trait for domain event interface |
| `Message` | message.rs | Database-persistent message structure |
| `MessageImpl` | message.rs | In-memory message with headers |
| `Headers` | message.rs | CloudEvent-compatible message headers |
| `publish` | DomainEventPublisher.rs | Publish aggregate domain events |
| `publishGenericDomainEvent` | DomainEventPublisher.rs | Publish generic domain events |

### Event Flow

```
Aggregate → DomainEvent → publish() → Message (DB) → CDC/Outbox → Dapr PubSub
```

## Quick Start

### 1. Add Dependency

Use `cargo add` to add dependencies (automatically fetches the latest version):

```sh
cargo add genies_ddd genies_derive
cargo add serde --features derive
```

You can also manually add dependencies in `Cargo.toml`. Visit [crates.io](https://crates.io) for the latest versions.

### 2. Define an Aggregate Root

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

The `#[derive(Aggregate)]` macro generates:
- `AggregateType` trait implementation with `aggregate_type()` and `atype()` methods
- `WithAggregateId` trait implementation using the first field as aggregate ID

### 3. Define a Domain Event

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

The `#[derive(DomainEvent)]` macro generates:
- `DomainEvent` trait implementation with `event_type()`, `event_type_version()`, `event_source()`, and `json()` methods
- Automatic JSON serialization via serde

### 4. Publish Domain Events

```rust
use genies_ddd::DomainEventPublisher::{publish, publishGenericDomainEvent};
use rbatis::executor::Executor;

// Publish event from an aggregate (includes aggregate context)
pub async fn create_device(tx: &mut dyn Executor, device: &Device) {
    let event = DeviceCreatedEvent {
        id: device.id.clone(),
        name: device.name.clone(),
        created_at: chrono::Utc::now().timestamp_millis(),
    };
    
    // Publish with aggregate context
    publish(tx, device, Box::new(event)).await;
}

// Publish generic event (without aggregate context)
pub async fn send_notification(tx: &mut dyn Executor) {
    let event = NotificationEvent { /* ... */ };
    publishGenericDomainEvent(tx, Box::new(event)).await;
}
```

## API Reference

### AggregateType Trait

```rust
pub trait AggregateType {
    /// Returns the aggregate type name for this instance
    fn aggregate_type(&self) -> String;
    
    /// Returns the aggregate type name (static method)
    fn atype() -> String;
}
```

### WithAggregateId Trait

```rust
pub trait WithAggregateId {
    type Id: Debug + Clone + PartialEq + Serialize + DeserializeOwned;

    /// Returns a reference to the aggregate ID
    fn aggregate_id(&self) -> &Self::Id;
}

/// Type alias for extracting aggregate ID type
pub type AggregateIdOf<A> = <A as WithAggregateId>::Id;
```

### DomainEvent Trait

```rust
pub trait DomainEvent: Send {
    /// Domain event version (e.g., "V1", "V2")
    fn event_type_version(&self) -> String;
    
    /// Fully qualified event type name
    fn event_type(&self) -> String;
    
    /// Event source (typically the aggregate type)
    fn event_source(&self) -> String;
    
    /// JSON serialized representation
    fn json(&self) -> String;
}
```

### Message Structure

```rust
/// Database-persistent message for outbox pattern
pub struct Message {
    pub id: Option<String>,
    pub destination: Option<String>,
    pub headers: Option<String>,
    pub payload: String,
    pub published: Option<u32>,      // 0 = not published, 1 = published
    pub creation_time: Option<i64>,
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

The `Message` struct requires a `message` table:

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

## Integration with genies_derive

The DDD traits are designed to work seamlessly with genies_derive macros:

| Macro | Generated Traits |
|-------|-----------------|
| `#[derive(Aggregate)]` | `AggregateType`, `WithAggregateId` |
| `#[derive(DomainEvent)]` | `DomainEvent` |

### Aggregate Macro Attributes

- `#[aggregate_type("TypeName")]` - Specify the aggregate type name

### DomainEvent Macro Attributes

- `#[event_type("fully.qualified.EventType")]` - Fully qualified event type
- `#[event_type_version("V1")]` - Event version
- `#[event_source("fully.qualified.AggregateType")]` - Event source

## Dependencies

- **rbatis** - ORM framework for message persistence
- **serde** / **serde_json** - Serialization
- **fastdate** - Timestamp generation
- **uuid** - Message ID generation

## Integration with Other Crates

- **genies_dapr**: Domain events published via `Message` table are consumed by Dapr CDC/Outbox and delivered to subscribers
- **genies_derive**: Provides `#[derive(Aggregate)]` and `#[derive(DomainEvent)]` macros
- **genies_context**: Provides database connection (`CONTEXT.rbatis`) for event publishing

## License

See the project root for license information.
