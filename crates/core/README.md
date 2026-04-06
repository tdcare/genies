# genies_core

Core utilities and common types for the Genies (神灯) framework, providing response models, error handling, JWT authentication, and conditional evaluation.

## Overview

genies_core provides essential building blocks for Genies-based microservices:

- **Response Models**: `RespVO<T>` and `ResultDTO<T>` for HTTP response formatting
- **Error Handling**: Unified `Error` type with `Result<T>` type alias
- **JWT Utilities**: Keycloak integration for token verification
- **Condition Engine**: JSON-based conditional expression evaluation

## Features

- **Dual Response Models**: Choose between string-code (`RespVO`) and numeric-status (`ResultDTO`) formats
- **Salvo Writer Integration**: Both response types implement `Writer` trait for direct rendering
- **Keycloak JWT Support**: Fetch keys and verify tokens from Keycloak servers
- **Flexible Error Handling**: Convert from various error types (`io::Error`, `rbdc::Error`, etc.)
- **Dapr Pubsub Support**: Built-in response helpers for Dapr message acknowledgment

## Core API Reference

### Response Models

| Type | Code Field | Description |
|------|------------|-------------|
| `RespVO<T>` | `code: String` | Primary model with `CODE_SUCCESS` / `CODE_FAIL` |
| `ResultDTO<T>` | `status: i32` | Java-compatible model with `1` (success) / `0` (fail) |

### Constants

```rust
pub const CODE_SUCCESS: &str = "SUCCESS";
pub const CODE_FAIL: &str = "FAIL";
pub const CODE_SUCCESS_I32: i32 = 1;
pub const CODE_FAIL_I32: i32 = 0;
```

### RespVO<T> Methods

```rust
// Create from successful data
RespVO::from(&data)

// Create from Result
RespVO::from_result(&result)

// Create error response with custom code
RespVO::from_error(code, &error)
RespVO::from_error_info(code, "error message")

// Dapr pubsub responses
resp.is_success()  // {"status": "SUCCESS"}
resp.is_retry()    // {"status": "RETRY"}
```

### ResultDTO<T> Methods

```rust
// Create success response
ResultDTO::success("Operation completed", data)
ResultDTO::success_empty("Done")

// Create error response
ResultDTO::error("Parameter required")
ResultDTO::from_error(code, &error)
ResultDTO::from_error_info(code, "message")

// Create with custom code and message
ResultDTO::from_code_message(200, "OK", &data)
```

### Error Type

```rust
use genies_core::error::Error;

// Create from string
let err = Error::from("Something went wrong");

// Convert from other errors
let err: Error = io_error.into();
let err: Error = rbdc_error.into();
```

### JWT Module

```rust
use genies_core::jwt::{get_keycloak_keys, get_temp_access_token, JWTToken, Keys};

// Fetch Keycloak public keys
let keys: Keys = get_keycloak_keys(
    "http://keycloak.example.com/auth/", 
    "my-realm"
).await?;

// Get service account access token
let token = get_temp_access_token(
    "http://keycloak.example.com/auth/",
    "my-realm",
    "my-client",
    "client-secret"
).await?;

// Verify token with Keycloak keys
let jwt = JWTToken::verify_with_keycloak(&keys, &token)?;

// Access token claims
println!("User: {}", jwt.preferred_username.unwrap_or_default());
println!("Roles: {:?}", jwt.roles);
```

### Condition Module

```rust
use genies_core::condition::{ConditionTree, obj_test};
use serde_json::json;

// Define condition tree
let condition = ConditionTree {
    operator: Some("and".to_string()),
    propertyName: None,
    value: None,
    conditionTrees: Some(vec![
        ConditionTree {
            operator: Some("=".to_string()),
            propertyName: Some("status".to_string()),
            value: Some("active".to_string()),
            conditionTrees: None,
        },
        ConditionTree {
            operator: Some(">".to_string()),
            propertyName: Some("age".to_string()),
            value: Some("18".to_string()),
            conditionTrees: None,
        },
    ]),
};

// Test object against condition
let obj = json!({"status": "active", "age": 25});
let matches = obj_test(&obj, &condition);
```

**Supported Operators:**

| Category | Operators |
|----------|-----------|
| Logic | `and`, `or` |
| Comparison | `=`, `<>`, `!=`, `<`, `<=`, `>`, `>=` |
| String | `contain`, `!contain` |
| Array | `arr_size_*`, `arr_exist_*`, `arr_each_*` |

### ID Generation Module

Provides a globally unique Snowflake ID generator based on `rs-snowflake`. Generates 64-bit distributed IDs as `String`, suitable for database VARCHAR primary keys.

#### API

| Function | Signature | Description |
|----------|-----------|-------------|
| `init` | `fn init(machine_id: i32, datacenter_id: i32)` | Initialize the generator (called once at startup by `ApplicationContext`) |
| `next_id` | `fn next_id() -> String` | Generate a globally unique ID |

#### Usage

```rust
// In business code (via genies re-export)
let id = genies::next_id();

// In core libraries (direct access)
let id = genies_core::id_gen::next_id();
```

> **Note**: The generator is automatically initialized during `ApplicationContext::new()`. 
> Do NOT call `init()` manually unless you have a specific reason.

## Quick Start

### 1. Add Dependency

Use `cargo add` to add dependencies (automatically fetches the latest version):

```sh
cargo add genies_core
```

You can also manually add dependencies in `Cargo.toml`. Visit [crates.io](https://crates.io) for the latest versions.

### 2. Use in Salvo Handler

```rust
use salvo::prelude::*;
use genies_core::{RespVO, ResultDTO};

#[endpoint]
async fn get_user() -> RespVO<User> {
    let user = User { id: 1, name: "Alice".into() };
    RespVO::from(&user)
}

#[endpoint]
async fn create_user() -> ResultDTO<String> {
    // Business logic...
    ResultDTO::success("User created", "user_id_123".into())
}

#[endpoint]
async fn handle_error() -> ResultDTO<()> {
    ResultDTO::error("Invalid parameters")
}
```

### 3. JWT Authentication

```rust
use genies_core::jwt::{get_keycloak_keys, JWTToken};

async fn verify_request(token: &str) -> Result<JWTToken, Error> {
    let keys = get_keycloak_keys(
        &config.keycloak_auth_server_url,
        &config.keycloak_realm
    ).await?;
    
    JWTToken::verify_with_keycloak(&keys, token)
}
```

### 4. ID Generation

```rust
use genies_core::id_gen;

// Initialization (usually done automatically by ApplicationContext)
id_gen::init(1, 1);

// Generate unique IDs
let order_id = id_gen::next_id();    // e.g. "7446616570199150889"
let event_id = id_gen::next_id();    // e.g. "7446616570199150890"
```

## When to Use RespVO vs ResultDTO

| Scenario | Recommended |
|----------|-------------|
| New Rust-only services | `RespVO<T>` |
| Interoperating with Java services | `ResultDTO<T>` |
| Need string error codes | `RespVO<T>` |
| Need numeric status codes | `ResultDTO<T>` |
| Dapr pubsub handlers | `RespVO<T>` (has `is_success()`, `is_retry()`) |

## Configuration

JWT functions require Keycloak parameters from `ApplicationConfig`:

```yaml
keycloak_auth_server_url: "http://keycloak.example.com/auth/"
keycloak_realm: "my-realm"
keycloak_resource: "my-client"
keycloak_credentials_secret: "your-client-secret"
```

## Dependencies

- **salvo** - Web framework (Writer trait)
- **serde** / **serde_json** - Serialization
- **jsonwebtoken** - JWT encoding/decoding
- **reqwest** - HTTP client for Keycloak
- **thiserror** - Error derive macros

## Integration with Other Crates

- **genies_config**: JWT functions use `ApplicationConfig` parameters
- **genies_context**: `CONTEXT.config()` provides Keycloak settings
- **genies_auth**: `salvo_auth` middleware uses `JWTToken` for authentication

## License

See the project root for license information.
