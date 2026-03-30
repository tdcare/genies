# genies

The main entry point for the Genies framework, providing a unified re-export of all sub-crates and convenient macros for DDD + Dapr development in Rust.

## Overview

genies is the central hub crate that re-exports all Genies framework components, allowing developers to access the entire framework through a single dependency. It also provides utility macros for common operations like database transactions, object copying, and Dapr gateway configuration.

## Features

- **Unified Re-exports**: Access all sub-crates (core, config, context, cache, dapr, ddd, k8s) through a single import
- **Database Macros**: Convenient macros for RBatis connection pool and transaction management
- **Object Copying**: JSON-based field copying between different struct types
- **Dapr Integration**: Gateway URL configuration for service-to-service communication
- **Topic Collection**: Re-exported functions for Dapr topic subscription routing

## Architecture

### Dependency Graph

```
genies (main entry)
  ├── genies_core      # Core utilities (JWT, error handling, conditions)
  ├── genies_config    # Configuration management
  ├── genies_context   # Application context and auth
  ├── genies_cache     # Redis/Memory caching
  ├── genies_dapr      # Dapr integration (PubSub, CloudEvent)
  ├── genies_ddd       # DDD primitives (Aggregate, DomainEvent)
  └── genies_k8s       # Kubernetes utilities
```

### Re-exported Modules

| Module | Alias | Description |
|--------|-------|-------------|
| `genies_core` | `core` | JWT handling, error types, condition helpers |
| `genies_config` | `config` | YAML/ENV configuration loading |
| `genies_context` | `context` | `CONTEXT` global, auth middleware |
| `genies_cache` | `cache` | Redis and in-memory cache services |
| `genies_dapr` | `dapr` | Dapr PubSub, CloudEvent, topic routing |
| `genies_ddd` | `ddd` | Aggregate, DomainEvent, Message traits |
| `genies_k8s` | `k8s` | Kubernetes integration utilities |

## Quick Start

### 1. Add Dependency

```toml
[dependencies]
genies = { path = "../path/to/genies" }
genies_derive = { path = "../path/to/genies_derive" }
```

### 2. Access Sub-modules

```rust
use genies::core;       // JWT, errors
use genies::config;     // Configuration
use genies::context::CONTEXT;  // Global application context
use genies::cache;      // Caching services
use genies::dapr;       // Dapr integration
use genies::ddd;        // DDD primitives
```

## Macro Reference

### `pool!()` - Get Database Connection

Returns a cloned RBatis connection from the global context.

```rust
use genies::pool;

async fn query_users() -> Result<Vec<User>, Error> {
    let users = User::select_all(pool!()).await?;
    Ok(users)
}
```

### `tx_defer!()` - Transaction with Auto-Rollback Guard

Creates a transaction that automatically rolls back if not explicitly committed.

```rust
use genies::tx_defer;

async fn transfer_funds(from: u64, to: u64, amount: f64) -> Result<(), Error> {
    let mut tx = tx_defer!();
    
    // Deduct from source account
    Account::deduct(&mut tx, from, amount).await?;
    
    // Add to target account
    Account::credit(&mut tx, to, amount).await?;
    
    // Commit transaction (auto-rollback if not called)
    tx.commit().await?;
    Ok(())
}
```

**With custom RBatis instance:**

```rust
let custom_rb = get_custom_rbatis();
let mut tx = tx_defer!(custom_rb);
// ... operations ...
tx.commit().await?;
```

### `copy!()` - Object Field Copying

Copies fields from one struct to another via JSON serialization. Useful for DTO transformations.

```rust
use genies::copy;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct UserEntity {
    id: u64,
    name: String,
    email: String,
    password_hash: String,
}

#[derive(Serialize, Deserialize)]
struct UserDto {
    id: u64,
    name: String,
    email: String,
}

let entity = UserEntity { /* ... */ };
let dto: UserDto = copy!(&entity, UserDto);
```

### `config_gateway!()` - Dapr/Gateway URL Configuration

Configures the gateway URL for service invocation, supporting both Dapr sidecar and direct HTTP gateway modes.

```rust
use genies::config_gateway;

// Define service gateway
static PATIENT_SERVICE: Lazy<String> = config_gateway!("/patient");

// Usage in HTTP client
let url = format!("{}/api/patients/{}", *PATIENT_SERVICE, patient_id);
```

**Behavior:**
- If `gateway` config contains `http://` or `https://`, uses direct gateway URL
- Otherwise, uses Dapr service invocation URL: `http://localhost:3500/v1.0/invoke{service}/method`

## Dapr Topic Functions

Re-exported from `genies_dapr` for convenience:

```rust
use genies::{
    collect_topic_routers,      // Collect all topic handler routers
    collect_topic_subscriptions, // Collect all topic subscriptions
    dapr_subscribe_handler,      // Dapr subscription endpoint handler
    dapr_event_router,           // Event routing handler
};

// In your main.rs
let topic_routers = collect_topic_routers();
let subscriptions = collect_topic_subscriptions();
```

## Integration Example

```rust
use genies::context::CONTEXT;
use genies::{pool, tx_defer, copy};
use genies_derive::{Aggregate, DomainEvent};

#[derive(Aggregate)]
#[aggregate_type("Order")]
#[id_field(id)]
struct Order {
    id: String,
    status: String,
    items: Vec<OrderItem>,
}

async fn create_order(cmd: CreateOrderCommand) -> Result<Order, Error> {
    // Initialize context
    CONTEXT.init_mysql().await;
    
    // Use transaction with auto-rollback
    let mut tx = tx_defer!();
    
    let order = Order {
        id: generate_id(),
        status: "CREATED".to_string(),
        items: cmd.items,
    };
    
    Order::insert(&mut tx, &order).await?;
    tx.commit().await?;
    
    Ok(order)
}
```

## Dependencies

- **rbatis** - ORM framework for database operations
- **serde** / **serde_json** - Serialization for `copy!()` macro
- **once_cell** - Lazy static initialization for `config_gateway!()`
- **log** - Logging in transaction guards

## Related Crates

- [genies_derive](../genies_derive) - Procedural macros (`#[derive(Aggregate)]`, `#[topic]`, etc.)
- [genies_auth](../auth) - Casbin-based permission management
- [genies_context](../context) - Application context and global state

## License

MIT/Apache-2.0
