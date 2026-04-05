# genies_derive

A powerful procedural macro library for the Genies (神灯) framework, providing derive macros and attribute macros for DDD aggregates, domain events, configuration, Dapr topic consumption, HTTP request wrapping, and field-level permission control.

## Overview

genies_derive provides 7 key macros that simplify common patterns in DDD + Dapr + Casbin applications:

| Macro | Type | Purpose |
|-------|------|---------|
| `#[derive(Aggregate)]` | Derive | DDD Aggregate root implementation |
| `#[derive(DomainEvent)]` | Derive | Domain event type implementation |
| `#[derive(Config)]` | Derive | Configuration loading from YAML/ENV |
| `#[derive(ConfigCore)]` | Derive | Internal config (avoids circular deps) |
| `#[topic(...)]` | Attribute | Dapr topic consumption with Redis idempotency |
| `#[remote(...)]` | Attribute | HTTP request wrapping with JWT auto-refresh |
| `#[casbin]` | Attribute | Field-level permission control |

## Quick Start

Use `cargo add` to add dependencies (automatically fetches the latest version):

```sh
cargo add genies_derive genies
```

You can also manually add dependencies in `Cargo.toml`. Visit [crates.io](https://crates.io) for the latest versions.

## Macro Reference

### 1. `#[derive(Aggregate)]` - Aggregate Root

Implements `AggregateType`, `WithAggregateId`, and optionally `InitializeAggregate` traits.

**Attributes:**

| Attribute | Required | Description |
|-----------|----------|-------------|
| `#[aggregate_type("Name")]` | No | Override aggregate type name (default: struct name) |
| `#[id_field(field_name)]` | No | Specify the ID field |
| `#[initialize_with_defaults]` | No | Generate `InitializeAggregate` impl (requires `id_field`) |

**Example:**

```rust
use genies_derive::Aggregate;
use serde::{Deserialize, Serialize};

#[derive(Aggregate, Serialize, Deserialize, Default)]
#[aggregate_type("Order")]
#[id_field(id)]
#[initialize_with_defaults]
pub struct Order {
    pub id: String,
    pub status: String,
    pub total_amount: f64,
    pub items: Vec<OrderItem>,
}

// Generated traits:
// - AggregateType::aggregate_type(&self) -> String  // Returns "Order"
// - AggregateType::atype() -> String                // Static version
// - WithAggregateId::aggregate_id(&self) -> &String
// - InitializeAggregate::initialize(id: String) -> Order
```

### 2. `#[derive(DomainEvent)]` - Domain Event

Implements `DomainEvent` trait for event sourcing patterns.

**Attributes:**

| Attribute | Required | Description |
|-----------|----------|-------------|
| `#[event_type("Name")]` | No | Override event type (default: struct/variant name) |
| `#[event_type_version("V1")]` | No | Event version (default: "V0") |
| `#[event_source("service")]` | No | Event source identifier (default: "") |

**Struct Example:**

```rust
use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

#[derive(DomainEvent, Serialize, Deserialize, Default)]
#[event_type("OrderCreated")]
#[event_type_version("V1")]
#[event_source("order-service")]
pub struct OrderCreatedEvent {
    pub order_id: String,
    pub customer_id: String,
    pub total: f64,
}

// Generated: DomainEvent trait with event_type(), event_type_version(), event_source(), json()
```

**Enum Example:**

```rust
#[derive(DomainEvent, Serialize, Deserialize)]
#[event_type_version("V1")]
pub enum OrderEvent {
    #[event_type("OrderCreated")]
    Created { order_id: String },
    
    #[event_type("OrderShipped")]
    Shipped { tracking_number: String },
}
```

### 3. `#[derive(Config)]` - Configuration

Generates configuration loading from YAML files and environment variables.

**Field Attribute:**

| Attribute | Description |
|-----------|-------------|
| `#[config(default = "value")]` | Default value for the field |

**Example:**

```rust
use genies_derive::Config;
use serde::{Deserialize, Serialize};

#[derive(Config, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    #[config(default = "localhost")]
    pub host: String,
    
    #[config(default = "8080")]
    pub port: u16,
    
    #[config(default = "topic1,topic2")]
    pub topics: Vec<String>,
    
    pub database_url: Option<String>,
}

// Generated methods:
// - AppConfig::default() -> Self
// - AppConfig::from_file(path: &str) -> Result<Self, ConfigError>
// - AppConfig::from_sources(path: &str) -> Result<Self, ConfigError>
// - AppConfig::load_env(&mut self) -> Result<(), ConfigError>
// - AppConfig::merge(&mut self, other: Self)
// - AppConfig::validate(&self) -> Result<(), ConfigError>
```

**Environment Variable Support:**

```bash
# Both formats supported:
export host="production.example.com"     # lowercase
export HOST="production.example.com"      # SCREAMING_SNAKE_CASE
```

### 4. `#[derive(ConfigCore)]` - Internal Configuration

Same as `Config` but uses `genies_core::error::ConfigError` instead of `genies::core::error::ConfigError`. Used internally to avoid circular dependencies.

### 5. `#[topic(...)]` - Dapr Topic Consumption

Transforms an async function into a Dapr topic consumer with Redis-based idempotency.

**Attributes:**

| Attribute | Required | Description |
|-----------|----------|-------------|
| `name = "topic_name"` | No | Topic name (default: aggregate type) |
| `pubsub = "pubsub_name"` | No | PubSub component (default: "messagebus") |
| `metadata = "k1=v1,k2=v2"` | No | Topic metadata |

**Example:**

```rust
use genies_derive::topic;
use rbatis::executor::Executor;

#[topic(name = "order-events", pubsub = "messagebus")]
pub async fn handle_order_created(
    tx: &mut dyn Executor,
    event: OrderCreatedEvent,
) -> anyhow::Result<u64> {
    // Process the event
    Order::insert(tx, &order).await?;
    Ok(1)
}
```

**Generated Code:**

1. **Handler Function**: `handle_order_created(tx, event)` - Your business logic
2. **Salvo Handler**: `handle_order_created_hoop` - HTTP handler for Dapr
3. **Dapr Config**: `handle_order_created_dapr()` - Returns `DaprTopicSubscription`
4. **Router**: `handle_order_created_hoop_router()` - Salvo router
5. **Auto-registration**: Via `inventory::submit!`

**Idempotency Flow:**

```
Dapr Message → Parse CloudEvent → Extract event_type
     ↓
Check Redis key: {server}-{handler}-{event_type}-{message_id}
     ↓
If CONSUMING: retry later
If CONSUMED: skip
If not exists: SET NX (atomic) → process → SET CONSUMED
```

### 6. `#[remote(...)]` - HTTP Request Wrapper

Wraps feignhttp requests with automatic JWT token refresh on 401 errors.

**Example:**

```rust
use genies_derive::remote;
use feignhttp::get;

#[remote]
#[get("${GATEWAY}/api/patients/{id}")]
pub async fn get_patient(#[path] id: String) -> Result<Patient, Error> {
    // feignhttp implementation
}
```

**Generated Code:**

```rust
// Original function with Authorization header
pub async fn get_patient_feignhttp(
    #[header] Authorization: &str,
    #[path] id: String
) -> Result<Patient, Error> { ... }

// Wrapper function with auto token refresh
pub async fn get_patient(id: String) -> Result<Patient, Error> {
    let bearer = format!("Bearer {}", REMOTE_TOKEN.lock().unwrap().access_token);
    let result = get_patient_feignhttp(&bearer, id).await;
    
    if result.is_err() && error.contains("401 Unauthorized") {
        // Refresh token from Keycloak
        let new_token = get_temp_access_token(...).await?;
        REMOTE_TOKEN.lock().unwrap().access_token = new_token;
        return get_patient_feignhttp(&new_bearer, id).await;
    }
    result
}
```

### 7. `#[casbin]` - Field-Level Permission Control

Generates custom `Serialize` and Salvo `Writer` implementations for field-level permission filtering.

**Example:**

```rust
use genies_derive::casbin;
use serde::Deserialize;
use salvo::oapi::ToSchema;

#[casbin]
#[derive(Deserialize, ToSchema)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,      // Can be filtered by policy
    pub phone: String,      // Can be filtered by policy
    pub address: Address,   // Nested type - auto-detected
    pub accounts: Vec<BankAccount>,  // Vec type - auto-detected
}
```

**Auto Nested Detection:**

The macro automatically detects non-primitive types and recursively filters them:
- Plain struct fields: `Address`
- Option wrapped: `Option<Address>`
- Vec wrapped: `Vec<BankAccount>`

**Generated Code:**

```rust
impl User {
    pub fn casbin_filter(
        value: &mut serde_json::Value,
        enforcer: &casbin::Enforcer,
        subject: &str,
    ) {
        // 1. Filter own fields
        // 2. Recursively filter nested fields
    }
}

#[async_trait]
impl salvo::writing::Writer for User {
    async fn write(self, req, depot, res) {
        let enforcer = depot.get::<Arc<Enforcer>>("casbin_enforcer");
        let subject = depot.get::<String>("casbin_subject");
        
        let mut value = serde_json::to_value(&self)?;
        Self::casbin_filter(&mut value, enforcer, subject);
        res.render(Json(value));
    }
}
```

**Policy Examples:**

```sql
-- Deny bob from reading User.email field
INSERT INTO casbin_rules (ptype, v0, v1, v2, v3) 
VALUES ('p', 'bob', 'User.email', 'read', 'deny');

-- Deny guest role from reading phone fields
INSERT INTO casbin_rules (ptype, v0, v1, v2, v3) 
VALUES ('p', 'guest', 'User.phone', 'read', 'deny');
```

## Dependencies

- **proc-macro2** / **quote** / **syn** - Procedural macro infrastructure
- **genies_core** - Error types for ConfigCore
- **serde** / **serde_yaml** - Configuration parsing
- **convert_case** - Environment variable name conversion
- **async-trait** - Async trait support

## Integration with Other Crates

| Macro | Integrates With |
|-------|-----------------|
| `Aggregate` | `genies_ddd::aggregate` traits |
| `DomainEvent` | `genies_ddd::event` traits |
| `Config` / `ConfigCore` | `genies_core::error::ConfigError` |
| `#[topic]` | `genies_dapr`, `genies_context::CONTEXT`, Redis |
| `#[remote]` | `genies_core::jwt`, `genies_context::REMOTE_TOKEN` |
| `#[casbin]` | `genies_auth`, `casbin::Enforcer`, Salvo |

## Debug Mode

Enable `debug_mode` feature to print generated code during compilation:

```toml
[dependencies]
genies_derive = { path = "...", features = ["debug_mode"] }
```

## License

MIT
