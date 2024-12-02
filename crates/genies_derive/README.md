# Genies Derive

A powerful derive macro for configuration management in Rust, providing a flexible and type-safe way to handle application configurations.

## Features

- 🔧 Default values via attributes
- 🌍 Environment variable support
- ✅ Configuration validation
- 📁 YAML file configuration
- 🔄 Hot reloading support
- 🏗️ Builder pattern
- 🔄 Type conversion
- 📦 Array and Option type support

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
genies_derive = "0.1.0"
```

## Quick Start

```rust
use genies_derive::Config;
use serde::{Deserialize, Serialize};

#[derive(Config, Debug, Deserialize, Serialize)]
struct ServerConfig {
    #[config(default = "localhost")]
    host: String,
    
    #[config(default = 8080)]
    #[config(validate(range(min = 1, max = 65535)))]
    port: u16,
    
    #[config(default = "topic1,topic2,topic3")]
    topics: Vec<String>,
    
    // Optional fields
    username: Option<String>,
    
    #[config(default = "60")]
    timeout_seconds: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load from file and environment variables
    let config = ServerConfig::from_sources("config.yml")?;
    println!("Config: {:?}", config);

    Ok(())
}
```

## Configuration Sources

Configurations are loaded in the following priority order (highest first):

1. Environment variables
2. Configuration file
3. Default values
4. None (for optional fields)

### Environment Variables

Field names are automatically converted to SCREAMING_SNAKE_CASE:

```bash
export HOST="production.example.com"
export PORT="443"
export TOPICS="prod/events,prod/logs,prod/metrics"
export USERNAME="admin"
export TIMEOUT_SECONDS="120"
```

### YAML Configuration

```yaml
host: "example.com"
port: 8080
topics:
  - "topic1"
  - "topic2"
username: "user"
timeout_seconds: 60
```

## Features

### Default Values

```rust
#[config(default = "localhost")]
host: String
```

### Validation

```rust
#[config(validate(range(min = 1, max = 65535)))]
port: u16
```

### Arrays

```rust
#[config(default = "topic1,topic2,topic3")]
topics: Vec<String>
```

### Optional Fields

```rust
username: Option<String>
```

### Hot Reloading

```rust
let mut config = ServerConfig::from_sources("config.yml")?;
config.reload().await?;
```

## Examples

Check out the [examples](examples/) directory for more detailed examples.

## Contributing

We welcome contributions! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
