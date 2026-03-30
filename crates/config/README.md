# genies_config

Configuration management module for the Genies (神灯) framework, providing YAML-based configuration loading with derive macro support.

## Overview

genies_config provides a simple yet powerful configuration system for Genies microservices:

- **Derive Macro Integration**: Use `#[derive(ConfigCore)]` for automatic config loading
- **YAML Configuration**: Load settings from `application.yml` files
- **Environment Variables**: Override config values via environment variables
- **Type-Safe Access**: Strongly typed configuration structs
- **Default Values**: Support for `#[config(default = "...")]` attribute
- **Validation**: Built-in validation with `#[config(validate(...))]`

## Core Components

| Component | File | Description |
|-----------|------|-------------|
| `ApplicationConfig` | app_config.rs | Main application configuration struct with all fields |
| `LogConfig` | log_config.rs | Logging configuration with `init_log()` function |
| `ConfigCore` macro | genies_derive | Derive macro for automatic config loading |

## ApplicationConfig Fields

| Field | Type | Description |
|-------|------|-------------|
| `debug` | `bool` | Debug mode flag |
| `server_name` | `String` | Microservice name |
| `servlet_path` | `String` | Service route prefix |
| `server_url` | `String` | Server bind address (e.g., "0.0.0.0:5800") |
| `gateway` | `Option<String>` | Gateway URL for cross-service calls |
| `redis_url` | `String` | Redis connection URL (business cache) |
| `redis_save_url` | `String` | Redis URL for persistent cache |
| `database_url` | `String` | MySQL connection URL |
| `max_connections` | `u32` | DB pool max connections |
| `min_connections` | `u32` | DB pool min connections |
| `wait_timeout` | `u64` | DB connection wait timeout (seconds) |
| `create_timeout` | `u64` | DB connection create timeout (seconds) |
| `max_lifetime` | `u64` | DB connection max lifetime (seconds) |
| `log_level` | `String` | Log level filter (e.g., "debug,flyway=info") |
| `white_list_api` | `Vec<String>` | API whitelist (no auth required) |
| `cache_type` | `String` | Cache backend: "redis" or "mem" |
| `keycloak_auth_server_url` | `String` | Keycloak server URL |
| `keycloak_realm` | `String` | Keycloak realm name |
| `keycloak_resource` | `String` | Keycloak client ID |
| `keycloak_credentials_secret` | `String` | Keycloak client secret |
| `dapr_pubsub_name` | `String` | Dapr pubsub component name |
| `dapr_pub_message_limit` | `i64` | Max messages per publish batch |
| `dapr_cdc_message_period` | `i64` | CDC message period (milliseconds) |
| `processing_expire_seconds` | `i64` | Message processing timeout |
| `record_reserve_minutes` | `i64` | Message record retention time |

## Quick Start

### 1. Add Dependencies

Use `cargo add` to add dependencies (automatically fetches the latest version):

```sh
cargo add genies_config genies_derive
cargo add serde --features derive
```

You can also manually add dependencies in `Cargo.toml`. Visit [crates.io](https://crates.io) for the latest versions.

### 2. Create Configuration File

Create `application.yml` in your project root:

```yaml
debug: true
server_name: "my-service"
servlet_path: "/api"
server_url: "0.0.0.0:5800"

# Redis
cache_type: "redis"
redis_url: "redis://:password@127.0.0.1:6379"
redis_save_url: "redis://:password@127.0.0.1:6379"

# Database
database_url: "mysql://user:pass@127.0.0.1:3306/mydb"
max_connections: 20
min_connections: 0
wait_timeout: 60
create_timeout: 120
max_lifetime: 1800

# Logging
log_level: "debug,flyway=info"

# Keycloak
keycloak_auth_server_url: "http://keycloak.example.com/auth/"
keycloak_realm: "my-realm"
keycloak_resource: "my-client"
keycloak_credentials_secret: "client-secret"

# Dapr
dapr_pubsub_name: "messagebus"
dapr_pub_message_limit: 50
dapr_cdc_message_period: 5000
processing_expire_seconds: 60
record_reserve_minutes: 10080

# API Whitelist
white_list_api:
  - "/"
  - "/actuator/*"
  - "/dapr/*"
```

### 3. Load Configuration

```rust
use genies_config::app_config::ApplicationConfig;

fn main() {
    let config = ApplicationConfig::from_sources("./application.yml").unwrap();
    
    println!("Server: {}", config.server_url);
    println!("Debug: {}", config.debug);
}
```

## Custom Configuration Structs

### Basic Usage

```rust
use genies_derive::ConfigCore;
use serde::Deserialize;

#[derive(ConfigCore, Debug, Deserialize)]
pub struct MyConfig {
    pub host: String,
    pub port: u16,
    pub app_name: String,
}

// Load from file
let config = MyConfig::from_sources("./config.yml")?;
```

### With Default Values

```rust
#[derive(ConfigCore, Debug, Deserialize)]
pub struct ServerConfig {
    #[config(default = "localhost")]
    pub host: String,
    
    #[config(default = 8080)]
    pub port: u16,
}
```

### With Validation

```rust
#[derive(ConfigCore, Debug, Deserialize)]
pub struct ValidatedConfig {
    #[config(default = 8080)]
    #[config(validate(range(min = 1, max = 65535)))]
    pub port: u16,
}
```

### Array Fields

```rust
#[derive(ConfigCore, Debug, Deserialize)]
pub struct ArrayConfig {
    #[config(default = "topic1,topic2,topic3")]
    pub topics: Vec<String>,
    
    #[config(default = "1,2,3")]
    pub numbers: Vec<i32>,
}
```

### Optional Fields

```rust
#[derive(ConfigCore, Debug, Deserialize)]
pub struct OptionalConfig {
    #[config(default = "guest")]
    pub username: Option<String>,
    
    pub password: Option<String>,  // No default, truly optional
}
```

## Logging Configuration

```rust
use genies_config::log_config::{LogConfig, init_log};

// Initialize logging from application.yml
init_log();

// Or load LogConfig manually
let log_config = LogConfig::from_sources("./application.yml")?;
println!("Log level: {}", log_config.log_level);
```

### Log Level Format

The `log_level` field supports tracing-subscriber filter syntax:

```yaml
# Simple level
log_level: "debug"

# Per-module levels
log_level: "debug,flyway=info,sqlx=warn"

# With span filtering
log_level: "debug,flyway=info,[my_span]=trace"
```

## Environment Variable Override

Environment variables automatically override YAML values. Variable names are derived from field names in SCREAMING_SNAKE_CASE:

```bash
# Override server_url
export SERVER_URL="0.0.0.0:9000"

# Override database_url
export DATABASE_URL="mysql://prod:pass@prod-db:3306/app"

# Override arrays (comma-separated)
export WHITE_LIST_API="/,/health,/api/*"
```

## Gateway Configuration

The `gateway` field controls cross-service communication:

```yaml
# Use HTTP gateway
gateway: "http://gateway.example.com:6002"

# Use Dapr sidecar (any non-HTTP value)
gateway: "dapr"
```

When `gateway` starts with `http://` or `https://`, all cross-service calls go through the gateway. Otherwise, Dapr service invocation is used.

## Dependencies

- **genies_derive** - `ConfigCore` derive macro
- **serde** - Deserialization
- **tracing-subscriber** - Log initialization
- **config** (optional) - Advanced configuration loading

## Integration with Other Crates

- **genies_context**: `CONTEXT.config()` returns `ApplicationConfig`
- **genies_cache**: Uses `redis_url`, `redis_save_url`, `cache_type`
- **genies_core**: JWT functions use Keycloak config fields
- **genies_auth**: Uses `white_list_api` for auth bypass

## Configuration Examples

See `crates/config/examples/` for more examples:

- `config_examples.rs` - Various configuration patterns
- `config/basic.yml` - Basic configuration
- `config/array.yml` - Array field configuration
- `config/optional.yml` - Optional field handling
- `config/complex.yml` - Complex configuration scenarios

## License

See the project root for license information.
