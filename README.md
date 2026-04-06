English | [简体中文](README.zh-CN.md)

# Genies (神灯)

<p align="center">
  <strong>A Rust-based DDD + Dapr microservice development framework</strong>
</p>

<p align="center">
  <a href="https://github.com/tdcare/genies"><img src="https://img.shields.io/badge/version-1.6.0-blue.svg" alt="version"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-edition%202021-orange.svg" alt="rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="license"></a>
</p>

---

## Table of Contents

- [Introduction](#introduction)
- [Features](#features)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Core Features in Detail](#core-features-in-detail)
- [Configuration Reference](#configuration-reference)
- [Permission Model](#permission-model)
- [API Reference](#api-reference)
- [Example Projects](#example-projects)
- [License](#license)

---

## Introduction

**Genies (神灯)** is a microservice development framework (v1.6.0) designed specifically for the Rust ecosystem. It deeply integrates **DDD (Domain-Driven Design)** principles with the **Dapr microservice runtime**, while maintaining compatibility with **Eventuate**-based Java projects.

The framework provides declarative aggregate roots, domain events, permission control, and configuration management through a **macro-driven architecture**, enabling developers to build enterprise-grade microservice applications with minimal boilerplate code.

### Tech Stack

| Component | Version | Purpose |
|-----------|---------|---------|
| **Rust** | Edition 2021 | Programming language |
| **Salvo** | 0.89 | Web framework |
| **RBatis** | 4.8 | ORM framework |
| **Tokio** | 1.22 | Async runtime |
| **Casbin** | 2.10 | Permission engine |
| **Redis** | - | Cache service |
| **jsonwebtoken** | - | JWT authentication |
| **Dapr** | - | Microservice runtime |

### Design Philosophy

1. **DDD First** - Build business logic around aggregate roots and domain events
2. **Macro-Driven Development** - Reduce boilerplate code through procedural macros for improved productivity
3. **Cloud-Native Ready** - Native support for Dapr and Kubernetes health checks
4. **Java Compatible** - Fully compatible with Eventuate framework message formats

---

## Features

- **Declarative Aggregate Roots** - Quickly define DDD aggregate roots using `#[derive(Aggregate)]`
- **Domain Event Driven** - Mark domain events with `#[derive(DomainEvent)]` for automatic event type identification
- **Dapr Pub/Sub** - Consume events via the `#[topic]` macro with automatic idempotency and retry logic
- **Field-Level Access Control** - Fine-grained field access control using Casbin-based `#[casbin]` macro
- **Flexible Configuration** - Support YAML config files and environment variable overrides with `#[derive(Config)]`
- **Dual-Backend Cache** - Support both Redis and in-memory cache backends, switchable via configuration
- **JWT Auth Middleware** - Built-in JWT validation with Keycloak integration
- **K8s Health Checks** - Out-of-the-box liveness/readiness probes
- **HTTP Wrapper** - `#[remote]` macro for automatic token refresh in cross-service calls

---

## Quick Start

### Prerequisites

- **Rust** >= 1.70.0 (Edition 2021)
- **MySQL** >= 5.7 (for event storage)
- **Redis** >= 6.0 (optional, for caching)
- **Dapr** >= 1.10 (optional, for pub/sub)

### Installing Dependencies

Add dependencies using `cargo add` (automatically fetches the latest version):

```sh
# Main framework (re-exports all sub-modules)
cargo add genies

# Procedural macro library (if using macros independently)
cargo add genies_derive

# Required dependencies
cargo add rbatis --features debug_mode
cargo add tokio --features full
cargo add salvo --features rustls,oapi,affix-state
cargo add serde --features derive
cargo add serde_json
```

You can also manually add dependencies in `Cargo.toml`. Visit [crates.io](https://crates.io) for the latest versions.

### Minimal Example

```rust
use genies::context::CONTEXT;
use genies::k8s::k8s_health_check;
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    // Initialize logging
    genies::config::log_config::init_log();
    
    // Initialize database connection
    CONTEXT.init_mysql().await;
    
    log::info!("Server starting at: http://{}", 
        CONTEXT.config.server_url.replace("0.0.0.0", "127.0.0.1"));
    
    // Build routes
    let router = Router::new()
        .push(k8s_health_check())  // K8s health check
        .push(Router::with_path("/api")
            .hoop(genies::context::auth::salvo_auth)  // JWT auth middleware
            .get(hello));
    
    // Start server
    let acceptor = TcpListener::new(&CONTEXT.config.server_url).bind().await;
    Server::new(acceptor).serve(router).await;
}

#[handler]
async fn hello() -> &'static str {
    "Hello, Genies!"
}
```

### Configuration File

Create `application.yml` in the project root:

```yaml
debug: true
server_name: "my-service"
servlet_path: "/api"
server_url: "0.0.0.0:8080"
database_url: "mysql://user:password@localhost:3306/mydb"
redis_url: "redis://localhost:6379"
cache_type: "redis"
log_level: "debug"
white_list_api:
  - "/actuator/*"
  - "/health/*"
```

---

## Architecture

### Directory Structure

```
genies/
├── Cargo.toml              # Workspace configuration
├── model.conf              # Casbin RBAC model configuration
├── policy.csv              # Casbin policy file
├── crates/
│   ├── genies/             # Main framework aggregation entry
│   ├── core/               # genies_core - Core foundation
│   ├── genies_derive/      # Procedural macro library
│   ├── config/             # genies_config - Configuration management
│   ├── context/            # genies_context - Application context
│   ├── cache/              # genies_cache - Cache service
│   ├── dapr/               # genies_dapr - Dapr integration
│   ├── ddd/                # genies_ddd - DDD core
│   ├── k8s/                # genies_k8s - K8s health checks
│   └── auth/               # genies_auth - Permission example
└── examples/
    └── topic/              # Event subscription example
```

### Crate Dependency Graph

```
                    ┌─────────────────┐
                    │     genies      │  (Main entry, re-exports all sub-crates)
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│ genies_config │   │genies_context │   │  genies_ddd   │
│ Configuration │   │ App Context   │   │   DDD Core    │
└───────┬───────┘   └───────┬───────┘   └───────┬───────┘
        │                   │                   │
        │           ┌───────┴───────┐           │
        │           ▼               ▼           │
        │   ┌───────────────┐ ┌───────────────┐ │
        │   │ genies_cache  │ │  genies_dapr  │◄┘
        │   │ Cache Service │ │Dapr Integration│
        │   └───────────────┘ └───────────────┘
        │
        ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│  genies_core  │   │genies_derive  │   │  genies_k8s   │
│Core Foundation│   │ Proc Macros   │   │  K8s Probes   │
└───────────────┘   └───────────────┘   └───────────────┘
```

### Crate Responsibilities

| Crate | Responsibility |
|-------|----------------|
| **genies** | Main framework aggregation entry; re-exports all sub-crates; provides convenience macros `pool!`, `tx_defer!`, `copy!` |
| **genies_core** | Core infrastructure: error handling, JWT validation, HTTP response models (`RespVO`, `ResultDTO`) |
| **genies_derive** | Procedural macro library: `DomainEvent`, `Aggregate`, `Config`, `topic`, `remote`, `casbin` |
| **genies_config** | Configuration management: `ApplicationConfig`, log configuration; supports YAML + environment variables |
| **genies_context** | Global context (`CONTEXT`), JWT auth middleware, service state management |
| **genies_cache** | Cache abstraction layer: `CacheService` supporting both Redis and in-memory backends |
| **genies_dapr** | Dapr integration: CloudEvent, pub/sub, topic registration |
| **genies_ddd** | DDD core: aggregate root traits, domain event traits, message publisher |
| **genies_k8s** | Kubernetes probes: `/actuator/health/liveness` and `/actuator/health/readiness` |

---

## Core Features in Detail

### 1. Aggregate Root Definition (`#[derive(Aggregate)]`)

Aggregate roots are a core concept in DDD. Genies (神灯) simplifies their definition through the `Aggregate` derive macro:

```rust
use genies_derive::Aggregate;
use serde::{Deserialize, Serialize};

#[derive(Aggregate, Debug, Serialize, Deserialize, Clone)]
#[aggregate_type("me.tdcarefor.order.domain.Order")]  // Optional: specify aggregate type name
#[id_field(id)]                                       // Required: specify ID field
#[initialize_with_defaults]                           // Optional: enable default value initialization
pub struct Order {
    pub id: String,
    pub customer_id: String,
    pub total_amount: f64,
    pub status: String,
}
```

**Attribute Reference:**

| Attribute | Required | Description |
|-----------|----------|-------------|
| `#[aggregate_type("...")]` | No | Custom aggregate type name; defaults to struct name |
| `#[id_field(field_name)]` | Yes | Specifies the field used as the aggregate ID |
| `#[initialize_with_defaults]` | No | Automatically implements the `InitializeAggregate` trait |

**Generated Trait Implementations:**

```rust
impl AggregateType for Order {
    fn aggregate_type(&self) -> String { ... }
    fn atype() -> String { ... }
}

impl WithAggregateId for Order {
    type Id = String;
    fn aggregate_id(&self) -> &Self::Id { &self.id }
}
```

---

### 2. Domain Events (`#[derive(DomainEvent)]`)

Domain events record state changes of aggregate roots. Both struct and enum forms are supported:

```rust
use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

// Struct form
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.order.domain.Order")]
#[event_type("me.tdcarefor.order.event.OrderCreated")]
pub struct OrderCreatedEvent {
    pub order_id: String,
    pub customer_id: String,
    pub total_amount: f64,
}

// Enum form (multiple event types)
#[derive(DomainEvent, Debug, Serialize, Deserialize, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.order.domain.Order")]
pub enum OrderEvent {
    #[event_type("OrderCreated")]
    Created { order_id: String, customer_id: String },
    
    #[event_type("OrderShipped")]
    Shipped { order_id: String, tracking_number: String },
    
    #[event_type("OrderCancelled")]
    Cancelled { order_id: String, reason: String },
}
```

**Attribute Reference:**

| Attribute | Description |
|-----------|-------------|
| `#[event_type("...")]` | Event type identifier, used for deserialization routing |
| `#[event_type_version("...")]` | Event version; defaults to "V0" |
| `#[event_source("...")]` | Event source (typically the fully qualified aggregate root name) |

**Generated Trait Implementations:**

```rust
impl DomainEvent for OrderCreatedEvent {
    fn event_type_version(&self) -> String { "V1".to_string() }
    fn event_type(&self) -> String { "me.tdcarefor.order.event.OrderCreated".to_string() }
    fn event_source(&self) -> String { "me.tdcarefor.order.domain.Order".to_string() }
    fn json(&self) -> String { serde_json::to_string(self).unwrap() }
}
```

---

### 3. Event Publishing and Consuming

#### Publishing Events

Use `DomainEventPublisher` to persist events to the `message` table in the database:

```rust
use genies::ddd::DomainEventPublisher::{publish, publishGenericDomainEvent};

// Publish an event associated with an aggregate root
async fn create_order(tx: &mut dyn Executor, order: &Order) -> Result<()> {
    let event = OrderCreatedEvent {
        order_id: order.id.clone(),
        customer_id: order.customer_id.clone(),
        total_amount: order.total_amount,
    };
    
    // Event will be associated with the Order aggregate root
    publish(tx, order, Box::new(event)).await;
    Ok(())
}

// Publish a generic domain event (not associated with any aggregate root)
async fn send_notification(tx: &mut dyn Executor) -> Result<()> {
    let event = NotificationEvent { message: "Hello".to_string() };
    publishGenericDomainEvent(tx, Box::new(event)).await;
    Ok(())
}
```

**Message Table Schema:**

```sql
CREATE TABLE message (
    id VARCHAR(36) PRIMARY KEY,
    destination VARCHAR(255),
    headers TEXT,
    payload TEXT,
    published INT DEFAULT 0,
    creation_time BIGINT
);
```

#### Consuming Events (`#[topic]` Macro)

Use the `#[topic]` macro to subscribe to events published via Dapr:

```rust
use genies_derive::topic;
use rbatis::executor::Executor;

#[topic(
    name = "me.tdcarefor.order.domain.Order",  // Topic name to subscribe to
    pubsub = "messagebus"                       // Dapr pubsub component name
)]
pub async fn on_order_created(
    tx: &mut dyn Executor,
    event: OrderCreatedEvent
) -> anyhow::Result<u64> {
    log::info!("Received order created event: {:?}", event);
    
    // Process the event...
    // Transactions are managed automatically: commit on success, rollback and retry on failure
    
    Ok(0)
}
```

**`#[topic]` Macro Parameters:**

| Parameter | Required | Description |
|-----------|----------|-------------|
| `name` | No | Topic name; defaults to the aggregate type name |
| `pubsub` | No | Dapr PubSub component name; defaults to "messagebus" |
| `metadata` | No | Additional metadata in the format "key1=value1,key2=value2" |

**Generated Code:**

The `#[topic]` macro automatically generates:
1. A Salvo Handler (`{fn_name}_hoop`) for receiving Dapr messages
2. A Dapr subscription config function (`{fn_name}_dapr`)
3. A route registration function (`{fn_name}_hoop_router`)
4. Automatic idempotency checks (Redis-based)
5. Automatic transaction management and retry logic

#### Registering Event Consumers

```rust
use genies::dapr::dapr_sub::dapr_sub;
use crate::listeners::{on_order_created_hoop, on_order_shipped_hoop};

pub fn event_consumer_router() -> Router {
    Router::new().push(
        Router::with_path("/daprsub/consumers")
            .hoop(on_order_created_hoop)    // Event handler middleware
            .hoop(on_order_shipped_hoop)
            .post(dapr_sub)                  // Dapr response handler
    )
}
```

---

### 4. Configuration Management (`#[derive(Config)]`)

Define configuration structures that support YAML and environment variables using the `Config` macro:

```rust
use genies_derive::Config;
use serde::Deserialize;

#[derive(Config, Debug, Deserialize)]
pub struct MyAppConfig {
    #[config(default = "my-service")]
    pub server_name: String,
    
    #[config(default = "8080")]
    pub port: u32,
    
    #[config(default = "")]
    pub api_key: Option<String>,
    
    #[config(default = "")]
    pub allowed_origins: Vec<String>,
}

// Using the configuration
fn main() {
    let config = MyAppConfig::from_sources("./application.yml").unwrap();
    println!("Server: {}:{}", config.server_name, config.port);
}
```

**Configuration Loading Priority:**

1. Default values (`#[config(default = "...")]`)
2. YAML configuration file
3. Environment variables (supports both `field_name` and `FIELD_NAME` formats)

**Generated Methods:**

```rust
impl MyAppConfig {
    pub fn from_file(path: &str) -> Result<Self, ConfigError>;
    pub fn from_sources(file_path: &str) -> Result<Self, ConfigError>;
    pub fn validate(&self) -> Result<(), ConfigError>;
    pub fn merge(&mut self, other: Self);
    pub fn load_env(&mut self) -> Result<(), ConfigError>;
}
```

---

### 5. Field-Level Access Control (`#[casbin]` Macro)

Dynamic field-level access control powered by Casbin:

```rust
use genies_derive::casbin;
use serde::Deserialize;
use salvo::oapi::ToSchema;

#[casbin]                           // Must be placed first
#[derive(Deserialize, ToSchema)]
pub struct UserProfile {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub credit_card: String,
}

// Usage in a handler
#[endpoint]
async fn get_user_profile(
    req: &mut Request,
    depot: &mut Depot
) -> Json<UserProfile> {
    let enforcer = depot.obtain::<Arc<Enforcer>>().unwrap();
    let current_user = req.query::<String>("user").unwrap_or("guest".into());
    
    let profile = UserProfile {
        id: 1,
        name: "Zhang San".to_string(),
        email: "zhangsan@example.com".to_string(),
        phone: "13800138000".to_string(),
        credit_card: "1234-5678-9012-3456".to_string(),
        enforcer: None,   // Field auto-added by the macro
        subject: None,    // Field auto-added by the macro
    };
    
    // Apply permission policies
    Json(profile.with_policy(Arc::clone(&enforcer), current_user))
}
```

**`#[casbin]` Macro Auto-Generates:**

1. `enforcer` and `subject` fields (marked with `#[serde(skip)]`)
2. `with_policy(enforcer, subject)` method
3. `check_permission(field)` method
4. Custom `Serialize` implementation (filters fields based on permissions)

---

### 6. Cache Service

Genies (神灯) provides a unified cache abstraction supporting both Redis and in-memory backends:

```rust
use genies::context::CONTEXT;
use std::time::Duration;

async fn cache_example() -> Result<()> {
    let cache = &CONTEXT.cache_service;
    
    // String operations
    cache.set_string("key1", "value1").await?;
    let value = cache.get_string("key1").await?;
    cache.del_string("key1").await?;
    
    // With expiration
    cache.set_string_ex("session", "token123", Some(Duration::from_secs(3600))).await?;
    
    // JSON operations
    let user = User { id: 1, name: "test".to_string() };
    cache.set_json("user:1", &user).await?;
    let user: User = cache.get_json("user:1").await?;
    
    // Get TTL
    let ttl = cache.ttl("session").await?;
    
    Ok(())
}
```

**Cache Interface (`ICacheService`):**

```rust
#[async_trait]
pub trait ICacheService: Sync + Send {
    async fn set_string(&self, k: &str, v: &str) -> Result<String>;
    async fn get_string(&self, k: &str) -> Result<String>;
    async fn del_string(&self, k: &str) -> Result<String>;
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String>;
    async fn ttl(&self, k: &str) -> Result<i64>;
}
```

**Switching Cache Backend:**

Configure in `application.yml`:

```yaml
cache_type: "redis"  # or "mem" for in-memory cache
redis_url: "redis://:password@localhost:6379"
```

---

### 7. HTTP Wrapper (`#[remote]` Macro)

Wraps cross-service HTTP calls with automatic token refresh:

```rust
use genies_derive::remote;
use feignhttp::get;

#[remote]
#[get("${gateway}/user-service/api/users/{id}")]
pub async fn get_user_by_id(#[path] id: i64) -> feignhttp::Result<User> {}

// No need to manually pass the Authorization header
async fn example() {
    let user = get_user_by_id(123).await.unwrap();
}
```

**`#[remote]` Macro Features:**

1. Automatically retrieves the access token from `REMOTE_TOKEN`
2. Automatically refreshes the token and retries on 401 errors
3. Seamlessly integrates with feignhttp macros

---

### 8. Kubernetes Health Checks

Genies (神灯) provides built-in K8s readiness and liveness probes:

```rust
use genies::k8s::k8s_health_check;
use salvo::Router;

let router = Router::new()
    .push(k8s_health_check());  // Automatically adds health check routes

// Provided endpoints:
// GET /actuator/health/liveness  - Liveness probe
// GET /actuator/health/readiness - Readiness probe
```

**Modifying Service Status:**

```rust
use genies::context::SERVICE_STATUS;
use std::ops::DerefMut;

fn set_not_ready() {
    let mut status = SERVICE_STATUS.lock().unwrap();
    let map = status.deref_mut();
    map.insert("readinessProbe".to_string(), false);
}
```

---

## Configuration Reference

### ApplicationConfig Full Reference

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `debug` | bool | Debug mode | `true` |
| `server_name` | String | Microservice name | `"my-service"` |
| `servlet_path` | String | Service route prefix | `"/api"` |
| `server_url` | String | Server listen address | `"0.0.0.0:8080"` |
| `gateway` | Option<String> | Gateway address (HTTP) or Dapr mode | `"http://gateway:8080"` |
| `redis_url` | String | Redis cache address | `"redis://:pwd@localhost:6379"` |
| `redis_save_url` | String | Persistent Redis address | `"redis://:pwd@localhost:6380"` |
| `database_url` | String | Database connection string | `"mysql://user:pwd@localhost/db"` |
| `max_connections` | u32 | Maximum connections | `20` |
| `min_connections` | u32 | Minimum connections | `0` |
| `wait_timeout` | u64 | Connection wait timeout (seconds) | `60` |
| `create_timeout` | u64 | Connection creation timeout (seconds) | `120` |
| `max_lifetime` | u64 | Maximum connection lifetime (seconds) | `1800` |
| `log_level` | String | Log level | `"debug,sqlx=warn"` |
| `white_list_api` | Vec<String> | Auth-exempt whitelist | `["/health/*"]` |
| `cache_type` | String | Cache type | `"redis"` or `"mem"` |
| `keycloak_auth_server_url` | String | Keycloak server URL | `"http://keycloak/auth/"` |
| `keycloak_realm` | String | Keycloak realm | `"myrealm"` |
| `keycloak_resource` | String | Keycloak client ID | `"myclient"` |
| `keycloak_credentials_secret` | String | Keycloak client secret | `"xxx-xxx-xxx"` |
| `dapr_pubsub_name` | String | Dapr PubSub component name | `"messagebus"` |
| `dapr_pub_message_limit` | i64 | Message publish batch limit | `50` |
| `dapr_cdc_message_period` | i64 | CDC message polling period (ms) | `5000` |
| `processing_expire_seconds` | i64 | Message processing timeout (seconds) | `60` |
| `record_reserve_minutes` | i64 | Message record retention (minutes) | `10080` |

### Full application.yml Example

```yaml
# Basic configuration
debug: true
server_name: "order-service"
servlet_path: "/order"
server_url: "0.0.0.0:8080"

# Gateway configuration (uses gateway for HTTP, otherwise uses Dapr)
gateway: "http://api-gateway:8080"

# Cache configuration
cache_type: "redis"
redis_url: "redis://:password@redis:6379"
redis_save_url: "redis://:password@redis-persistent:6379"

# Database configuration
database_url: "mysql://root:password@mysql:3306/order_db?serverTimezone=Asia/Shanghai"
max_connections: 20
min_connections: 2
wait_timeout: 60
create_timeout: 120
max_lifetime: 1800

# Log configuration
log_level: "debug,sqlx=warn,hyper=info"

# Keycloak authentication configuration
keycloak_auth_server_url: "http://keycloak:8080/auth/"
keycloak_realm: "myrealm"
keycloak_resource: "order-service"
keycloak_credentials_secret: "your-client-secret"

# Dapr configuration
dapr_pubsub_name: "messagebus"
dapr_pub_message_limit: 50
dapr_cdc_message_period: 5000
processing_expire_seconds: 60
record_reserve_minutes: 10080

# Whitelisted endpoints (no auth required)
white_list_api:
  - "/"
  - "/actuator/*"
  - "/dapr/*"
  - "/swagger-ui/*"
  - "/api-doc/*"
```

### Environment Variable Overrides

Two formats are supported for overriding configuration:

```bash
# Original field name format
export database_url="mysql://prod:password@prod-db:3306/db"

# Uppercase underscore format
export DATABASE_URL="mysql://prod:password@prod-db:3306/db"
export REDIS_URL="redis://:pwd@prod-redis:6379"
export LOG_LEVEL="info"
```

---

## Permission Model

### Casbin RBAC Model

Genies (神灯) uses Casbin for field-level access control. Configuration file `model.conf`:

```ini
[request_definition]
r = sub, obj, act

[policy_definition]
p = sub, obj, act, eft

[role_definition]
g = _, _      # User-role mapping
g2 = _, _     # Resource-resource group mapping

[policy_effect]
e = !some(where (p.eft == deny))

[matchers]
m = g(r.sub, p.sub) && g2(r.obj, p.obj) && r.act == p.act
```

**Model Explanation:**

| Component | Description |
|-----------|-------------|
| `r = sub, obj, act` | Request definition: subject, object, action |
| `p = sub, obj, act, eft` | Policy definition: includes effect (allow/deny) |
| `g = _, _` | User inherits role |
| `g2 = _, _` | Resource inherits resource group |
| `e = !some(where (p.eft == deny))` | Default allow; deny if any deny policy matches |

### Policy File Format

`policy.csv` example:

```csv
# Direct authorization: user alice cannot read UserProfile.email
p, alice, genies_auth.vo.UserProfile.email, read, deny

# Direct authorization: user bob can read User.email
p, bob, genies_auth.vo.User.email, read, allow

# User bob cannot read UserProfile.email
p, bob, genies_auth.vo.UserProfile.email, read, deny

# Role definition: data_group_admin role cannot read data_group
p, data_group_admin, data_group, read, deny

# User-role mapping: alice belongs to data_group_admin role
g, alice, data_group_admin

# Resource-group mapping: these fields belong to data_group
g2, genies_auth.vo.UserProfile.credit_card, data_group
g2, genies_auth.vo.UserProfile.name, data_group
g2, genies_auth.vo.User.phone, data_group
```

### How Field-Level Permissions Work

1. The `#[casbin]` macro modifies the struct by adding `enforcer` and `subject` fields
2. A custom `Serialize` implementation calls `check_permission` before serializing each field
3. `check_permission` constructs a request `(subject, "StructName.field_name", "read")`
4. The Casbin Enforcer decides whether to serialize the field based on policies
5. Denied fields are omitted from the JSON output

---

## API Reference

### Core Traits

#### `DomainEvent` (genies::ddd::event)

```rust
pub trait DomainEvent: Send {
    fn event_type_version(&self) -> String;  // Event version
    fn event_type(&self) -> String;          // Event type identifier
    fn event_source(&self) -> String;        // Event source
    fn json(&self) -> String;                // Serialize to JSON
}
```

#### `AggregateType` (genies::ddd::aggregate)

```rust
pub trait AggregateType {
    fn aggregate_type(&self) -> String;  // Get aggregate type name
    fn atype() -> String;                // Static method to get type name
}
```

#### `WithAggregateId` (genies::ddd::aggregate)

```rust
pub trait WithAggregateId {
    type Id: Debug + Clone + PartialEq + Serialize + DeserializeOwned;
    fn aggregate_id(&self) -> &Self::Id;  // Get aggregate ID
}
```

#### `ICacheService` (genies::cache::cache_service)

```rust
#[async_trait]
pub trait ICacheService: Sync + Send {
    async fn set_string(&self, k: &str, v: &str) -> Result<String>;
    async fn get_string(&self, k: &str) -> Result<String>;
    async fn del_string(&self, k: &str) -> Result<String>;
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String>;
    async fn ttl(&self, k: &str) -> Result<i64>;
}
```

### Core Structs

#### `RespVO<T>` (genies::core)

Standard HTTP response model:

```rust
pub struct RespVO<T> {
    pub code: Option<String>,   // "SUCCESS" or "FAIL"
    pub msg: Option<String>,    // Error message
    pub data: Option<T>,        // Response data
}

impl<T> RespVO<T> {
    pub fn from_result(arg: &Result<T>) -> Self;
    pub fn from(arg: &T) -> Self;
    pub fn from_error(code: &str, arg: &Error) -> Self;
    pub fn from_error_info(code: &str, info: &str) -> Self;
}
```

#### `ResultDTO<T>` (genies::core)

Java-compatible response model:

```rust
pub struct ResultDTO<T> {
    pub status: Option<i32>,    // 1=success, 0=failure
    pub message: Option<String>,
    pub data: Option<T>,
}

impl<T> ResultDTO<T> {
    pub fn success(message: &str, data: T) -> Self;
    pub fn error(message: &str) -> Self;
    pub fn success_empty(message: &str) -> ResultDTO<()>;
}
```

### Convenience Macros

#### `pool!()` (genies)

Get the database connection pool:

```rust
let rb = pool!();
User::select_by_id(rb, 1).await?;
```

#### `tx_defer!()` (genies)

Get a transaction with automatic rollback:

```rust
let mut tx = tx_defer!();
User::insert(&mut tx, &user).await?;
tx.commit().await;  // Auto-rollback if not committed
```

#### `copy!(src, DestType)` (genies)

Field copy conversion:

```rust
let user_dto = copy!(&user_entity, UserDTO);
```

---

## Example Projects

The project includes complete example code in the `examples/` directory:

### Topic Example

Demonstrates event publishing and subscription:

```
examples/topic/
├── Cargo.toml
├── application.yml        # Configuration file
└── src/
    ├── main.rs            # Service entry point
    ├── lib.rs             # Event consumer route configuration
    ├── DeviceUseEvent.rs  # Domain event definition
    └── UseDeviceListeners.rs  # Event handler
```

**Running the example:**

```bash
cd examples/topic
cargo run
```

**Example code snippet:**

```rust
// DeviceUseEvent.rs - Define domain event
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V2")]
#[event_source("me.tdcarefor.tdnis.device.domain.DeptDeviceEntity")]
#[event_type("me.tdcarefor.tdnis.device.event.DeviceUseEvent")]
pub struct DeviceUseEvent {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub deviceNo: Option<String>,
}

// UseDeviceListeners.rs - Event consumer
#[topic(
    name = "me.tdcarefor.tdnis.device.domain.DeptDeviceEntity",
    pubsub = "messagebus"
)]
pub async fn onDeviceUseEvent(
    tx: &mut dyn Executor,
    event: DeviceUseEvent
) -> anyhow::Result<u64> {
    log::info!("Processing device use event: {:?}", event);
    Ok(0)
}
```

---

## License

This project is open-sourced under the **MIT License**.

```
MIT License

Copyright (c) tdcare

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

<p align="center">
  <sub>Built with ❤️ by <a href="https://github.com/tdcare">tdcare</a></sub>
</p>
