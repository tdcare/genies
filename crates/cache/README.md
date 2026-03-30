# genies_cache

Dual-backend caching service for the Genies (神灯) framework, supporting both Redis and in-memory storage.

## Overview

genies_cache provides a unified caching interface with two interchangeable backends:

- **Redis Backend**: Production-ready distributed caching
- **Memory Backend**: Local in-memory caching for testing and development
- **Unified API**: Same interface for both backends via `ICacheService` trait
- **TTL Support**: Time-to-live for automatic key expiration
- **Atomic Operations**: `set_string_ex_nx` for distributed locks and idempotency

## Features

- **Backend Switching**: Change between Redis and memory via `cache_type` config
- **Two Factory Methods**: `new()` for business cache, `new_saved()` for persistent cache
- **Async API**: All operations are async-compatible with Tokio
- **JSON Serialization**: Built-in `set_json` / `get_json` helpers
- **Atomic NX Operation**: Set-if-not-exists for idempotency patterns
- **TTL Management**: Set expiration and query remaining time

## Core Components

| Component | File | Description |
|-----------|------|-------------|
| `ICacheService` | cache_service.rs | Trait defining cache operations |
| `CacheService` | cache_service.rs | Factory that creates appropriate backend |
| `RedisService` | redis_service.rs | Redis implementation |
| `MemService` | mem_service.rs | In-memory implementation |

## API Reference

### ICacheService Trait

```rust
#[async_trait]
pub trait ICacheService: Sync + Send {
    async fn set_string(&self, k: &str, v: &str) -> Result<String>;
    async fn get_string(&self, k: &str) -> Result<String>;
    async fn del_string(&self, k: &str) -> Result<String>;
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String>;
    async fn set_string_ex_nx(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<bool>;
    async fn set_value(&self, k: &str, v: &[u8]) -> Result<String>;
    async fn get_value(&self, k: &str) -> Result<Vec<u8>>;
    async fn set_value_ex(&self, k: &str, v: &[u8], ex: Option<Duration>) -> Result<String>;
    async fn ttl(&self, k: &str) -> Result<i64>;
}
```

### CacheService Methods

| Method | Description |
|--------|-------------|
| `new(cfg)` | Create business cache (uses `redis_url`) |
| `new_saved(cfg)` | Create persistent cache (uses `redis_save_url`) |
| `set_string(k, v)` | Set string value (no expiration) |
| `get_string(k)` | Get string value |
| `del_string(k)` | Delete key |
| `set_string_ex(k, v, ttl)` | Set with optional TTL |
| `set_string_ex_nx(k, v, ttl)` | Atomic set-if-not-exists with TTL |
| `set_json(k, v)` | Serialize and store JSON |
| `get_json(k)` | Retrieve and deserialize JSON |
| `ttl(k)` | Get remaining TTL in seconds |

## Quick Start

### 1. Add Dependency

Use `cargo add` to add dependencies (automatically fetches the latest version):

```sh
cargo add genies_cache genies_config
```

You can also manually add dependencies in `Cargo.toml`. Visit [crates.io](https://crates.io) for the latest versions.

### 2. Configure Backend

In `application.yml`:

```yaml
# Use Redis backend
cache_type: "redis"
redis_url: "redis://:password@127.0.0.1:6379"
redis_save_url: "redis://:password@127.0.0.1:6379"

# Or use memory backend (for testing)
cache_type: "mem"
```

### 3. Create and Use Cache

```rust
use genies_cache::cache_service::CacheService;
use genies_config::app_config::ApplicationConfig;
use std::time::Duration;

// Load config
let config = ApplicationConfig::from_sources("./application.yml")?;

// Create cache service
let cache = CacheService::new(&config);

// Basic operations
cache.set_string("key", "value").await?;
let value = cache.get_string("key").await?;
cache.del_string("key").await?;

// With TTL (expires in 5 minutes)
cache.set_string_ex("session", "data", Some(Duration::from_secs(300))).await?;

// Check remaining TTL
let remaining = cache.ttl("session").await?;
```

## Usage Patterns

### JSON Storage

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

// Store JSON object
let user = User { id: 1, name: "Alice".into() };
cache.set_json("user:1", &user).await?;

// Retrieve JSON object
let user: User = cache.get_json("user:1").await?;
```

### Distributed Lock / Idempotency

```rust
use std::time::Duration;

// Acquire lock (atomic set-if-not-exists)
let lock_acquired = cache.set_string_ex_nx(
    "lock:order:123",
    "processing",
    Some(Duration::from_secs(60))
).await?;

if lock_acquired {
    // Got the lock, process order
    process_order().await?;
    
    // Mark as completed
    cache.set_string_ex(
        "lock:order:123",
        "completed",
        Some(Duration::from_secs(86400))
    ).await?;
} else {
    // Another instance is processing or already completed
    println!("Order already being processed");
}
```

### Message Idempotency Pattern

```rust
async fn handle_message(msg_id: &str, payload: &str) -> Result<()> {
    let key = format!("msg:{}", msg_id);
    let ttl = Some(Duration::from_secs(300)); // 5 minutes
    
    // Try to acquire processing lock
    let lock = cache.set_string_ex_nx(&key, "CONSUMING", ttl).await?;
    
    if !lock {
        // Check if already processed
        let status = cache.get_string(&key).await?;
        if status == "CONSUMED" {
            return Ok(()); // Already processed, skip
        }
        // Still processing by another instance
        return Err(Error::from("Message being processed"));
    }
    
    // Process message
    match process_payload(payload).await {
        Ok(_) => {
            // Mark as completed
            cache.set_string_ex(&key, "CONSUMED", ttl).await?;
            Ok(())
        }
        Err(e) => {
            // Failed, remove lock to allow retry
            cache.del_string(&key).await?;
            Err(e)
        }
    }
}
```

### Two Cache Instances

```rust
// Business cache (volatile, uses redis_url)
let cache = CacheService::new(&config);

// Persistent cache (durable, uses redis_save_url)
let saved_cache = CacheService::new_saved(&config);

// Use business cache for sessions
cache.set_string_ex("session:abc", "data", Some(Duration::from_secs(3600))).await?;

// Use persistent cache for important data
saved_cache.set_string("config:feature_flags", flags_json).await?;
```

## Backend Comparison

| Feature | Redis | Memory |
|---------|-------|--------|
| Distributed | Yes | No |
| Persistence | Optional | No |
| TTL Support | Native | Simulated |
| Atomic NX | Native | Mutex-based |
| Best For | Production | Testing |

## TTL Return Values

The `ttl(k)` method returns:

| Return Value | Meaning |
|--------------|---------|
| `> 0` | Remaining seconds until expiration |
| `-1` | Key exists but has no TTL |
| `-2` | Key does not exist |

## Configuration

```yaml
# Backend selection
cache_type: "redis"  # or "mem"

# Redis URLs
redis_url: "redis://:password@host:port"           # Business cache
redis_save_url: "redis://:password@host:port/db1"  # Persistent cache
```

Redis URL format: `redis://[:password@]host:port[/db]`

## Testing

For integration tests, use memory backend to avoid Redis dependency:

```yaml
# test-application.yml
cache_type: "mem"
```

Or mock the trait:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache() {
        let config = ApplicationConfig {
            cache_type: "mem".to_string(),
            // ... other fields
        };
        let cache = CacheService::new(&config);
        
        cache.set_string("test", "value").await.unwrap();
        assert_eq!(cache.get_string("test").await.unwrap(), "value");
    }
}
```

## Dependencies

- **redis** - Redis client
- **tokio** - Async runtime
- **async-trait** - Async trait support
- **serde** / **serde_json** - JSON serialization
- **genies_core** - Error types
- **genies_config** - ApplicationConfig

## Integration with Other Crates

- **genies_config**: Provides `cache_type`, `redis_url`, `redis_save_url`
- **genies_context**: `CONTEXT.cache()` returns business cache
- **genies_auth**: Uses cache for enforcer version sync
- **genies_ddd**: Uses cache for message idempotency

## License

See the project root for license information.
